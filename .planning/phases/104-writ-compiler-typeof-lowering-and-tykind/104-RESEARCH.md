# Phase 104: writ-compiler typeof() Lowering and TyKind - Research

**Researched:** 2026-03-28
**Domain:** Writ compiler pipeline — parser, AST lowering, type checker, IL codegen
**Confidence:** HIGH

## Summary

Phase 104 threads `typeof(expr)` through the full compiler pipeline: parser → CST Expr::TypeOf → lowering → AstExpr::TypeOf → type checker TyKind::ReflectionType(Ty) → TypedExpr::TypeOf → IL codegen emitting TypeOf { r_dst, type_idx } with the compile-time TypeDef/TypeRef token baked in.

The runtime and IL instruction are already complete (Phase 103). The compiler has no awareness of `typeof` whatsoever yet — every layer needs new code: a new lexer keyword, a new CST variant, a new AstExpr variant, a new TyKind variant, a new TypedExpr variant, new check arm, new emit arm, and a TypeRef for the `Type` class registered in ModuleBuilder. BOX/UNBOX coercions at reflection API parameter sites are the one architectural wrinkle that needs thought; the approach is: detect when a `TyKind::ReflectionType` value is passed to a parameter typed as something else and insert box/unbox wrappers.

**Primary recommendation:** Follow the exact shape of how `spawn`/`try`/`new` are handled end-to-end — one keyword, one CST variant, one AstExpr variant, one TypedExpr variant, one emit arm. Add a `Type` TypeRef to the module builder alongside the existing `Range` TypeRef.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- typeof(expr) is a static compile-time query that lowers to AstExpr::TypeOf and emits TYPEOF opcode with type_idx baked in — NOT a function call
- The type checker assigns TyKind::ReflectionType(Type) to typeof expressions
- BOX/UNBOX auto-coercions at reflection API parameter/return sites — no TyKind::Any needed
- typeof(Animal) on a polymorphic Dog variable returns Animal (static type); dog.get_type() returns Dog (dynamic)

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked in STATE.md.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COMP-01 | typeof(expr) lowered to AstExpr::TypeOf AST node (not a function call) | New keyword in lexer, new CST variant Expr::TypeOf, new lower_expr arm, new AstExpr::TypeOf variant |
| COMP-02 | TypeOf instruction emitted with compile-time type index baked into instruction | check_expr returns TypedExpr::TypeOf carrying inner Ty; emit arm calls token_for_def on the static type's DefId OR encodes primitive token |
| COMP-04 | BOX/UNBOX coercion auto-inserted at reflection API boundaries | Detect TyKind::ReflectionType in call args/return sites; insert explicit TypedExpr::Box/TypedExpr::Unbox wrappers or emit BOX/UNBOX instructions inline |
| COMP-05 | TyKind::ReflectionType added to type checker for reflection builtin types | Add TyKind::ReflectionType(Ty) variant to ty.rs; add convenience constructor; update display(); update type_sig.rs encode_type(); update extract_type_def_id() if needed |
| REFL-01 | typeof(expr) returns Type for any type expression (structs, classes, enums, entities, contracts, primitives) | The static Ty of the inner expr determines the type_idx; all Ty categories handled; TyKind::ReflectionType is the result type |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-parser (logos + chumsky) | in-workspace | Lexing + parsing | Only parser in project |
| writ-compiler (internal) | in-workspace | Full pipeline | This is the crate being modified |
| writ-module | in-workspace | Instruction encoding | TypeOf instruction already at 0x0A30 |

No external dependencies needed. Everything is internal workspace crates.

---

## Architecture Patterns

### Layer 1: Lexer — `writ-parser/src/lexer.rs`

The lexer uses the `logos` crate. Keywords are declared as `#[token("keyword")] KwFoo` enum variants. `typeof` is not yet a keyword — it lexes as a plain `Ident("typeof")` today.

**Pattern to follow (from `spawn`):**
```rust
// Keywords — Concurrency
#[token("spawn")]
KwSpawn,
```

**New entry needed** (in the "Keywords — Types" or new "Keywords — Reflection" section):
```rust
#[token("typeof")]
KwTypeof,
```

**Placement:** After the existing `KwVoid` in the "Keywords — Types" section, or in a new "Keywords — Reflection" section after "Keywords — Entity".

### Layer 2: CST — `writ-parser/src/cst.rs`

The `Expr<'src>` enum has a variant per expression form. `Spawn`, `Try`, `New` are all present. `TypeOf` is absent.

**New CST variant needed:**
```rust
/// typeof expression: `typeof(expr)`
TypeOf(Box<Spanned<Expr<'src>>>),
```

Note: `typeof(expr)` parses as a keyword followed by a parenthesized expression. The inner expression is the operand. Paren delimiters are consumed by the parser, not stored in the variant (same as how `spawn expr` consumes the keyword and stores the inner expr).

**IMPORTANT:** `lower_expr` in writ-compiler has an exhaustive `match expr` with `no _ =>` wildcard. Adding a new CST variant without a matching lower arm will cause a compile error, ensuring nothing is missed.

### Layer 3: Parser — `writ-parser/src/parser/program.rs`

The expression atom list is in the `choice((...))` block around line 741. Keywords like `spawn`, `try`, `join` use the pattern:

```rust
let spawn_expr = just(Token::KwSpawn)
    .ignore_then(expr.clone())
    .map_with(|e, extra| (cst::Expr::Spawn(Box::new(e)), extra.span()));
```

**typeof pattern** — requires parentheses (spec says `typeof(expr)`):
```rust
let typeof_expr = just(Token::KwTypeof)
    .ignore_then(expr.clone().delimited_by(just(Token::LParen), just(Token::RParen)))
    .map_with(|e, extra| (cst::Expr::TypeOf(Box::new(e)), extra.span()));
```

Added to the atom `choice((...))` before the block-bodied expressions (same position as spawn/try). Must come before `ident_or_path` so `typeof` isn't parsed as an identifier call.

### Layer 4: AstExpr — `writ-compiler/src/ast/expr.rs`

The `AstExpr` enum has a doc comment header listing invariants. All variants carry `span: SimpleSpan`.

**New variant needed:**
```rust
// --- Reflection ---

/// typeof expression: `typeof(expr)` — static compile-time type query.
/// Lowers directly from CST::TypeOf; NOT a function call node.
/// The inner expression is checked for its static type; result type is TyKind::ReflectionType.
TypeOf {
    expr: Box<AstExpr>,
    span: SimpleSpan,
},
```

**Update doc comment invariants** to note: `YES TypeOf — static type query (pass-through like Spawn)`.

### Layer 5: Lowering — `writ-compiler/src/lower/expr.rs`

`lower_expr` has an exhaustive `match expr` over `Expr<'src>` variants. New arm:

```rust
Expr::TypeOf(e) => AstExpr::TypeOf {
    expr: Box::new(lower_expr(*e, ctx)),
    span,
},
```

No desugaring needed — this is a direct structural translation identical to `Spawn`/`Try`/`Join`.

### Layer 6: TypedExpr — `writ-compiler/src/check/ir.rs`

The `TypedExpr` enum carries `ty: Ty` and `span: SimpleSpan` on every variant. Both `.ty()` and `.span()` methods use exhaustive match — adding a new variant requires updating both match arms.

**New TypedExpr variant:**
```rust
TypeOf {
    ty: Ty,            // always TyKind::ReflectionType(inner_ty)
    span: SimpleSpan,
    static_ty: Ty,     // the compile-time type of the inner expression — used by emit
},
```

The `static_ty` field carries the inner expr's type so the emitter knows which type_idx to bake into the TypeOf instruction. The `ty` field carries the result type (TyKind::ReflectionType).

**Update `.ty()` and `.span()` match arms** to add the TypeOf arm.

**Note on inner expr:** The inner `AstExpr` is type-checked but its value is not used at runtime — only its static type matters. Include the inner TypedExpr in TypedExpr::TypeOf for completeness (diagnostic spans, LSP). OR omit it and keep only `static_ty`. Given that the result is purely compile-time, storing only `static_ty` keeps the variant minimal.

### Layer 7: TyKind — `writ-compiler/src/check/ty.rs`

**New TyKind variant:**
```rust
/// Reflection type: the result of a typeof(expr) expression.
/// The inner Ty is the static type of the queried expression.
/// At runtime this produces a Type heap object (writ-runtime builtin class).
ReflectionType(Ty),
```

**Update TyInterner:**
- Add `pub fn reflection_type(&mut self, inner: Ty) -> Ty` convenience constructor
- Update `display()` to show `"Type"` for this variant (or `"Type<{inner}>"`)
- Update `display_named()` to handle the new variant

**Update type_sig.rs `encode_type()`:**
`TyKind::ReflectionType` needs a blob encoding. The `Type` class comes from writ-runtime. Pattern is identical to TypeSpec references: emit a TypeRef token for `"Type"` from `"writ-runtime"`. Use the same approach as Range<T>: encode as `0x10` (TypeRef) + the TypeRef row token.

```rust
TyKind::ReflectionType(_inner) => {
    // Type is a class in writ-runtime. Encode as TypeRef.
    // The TypeRef for "Type" is registered in collect_defs alongside Range.
    if let Some(type_ref_token) = token_for_def_or_type_ref("Type", blob_heap) {
        buf.push(0x10);
        buf.extend_from_slice(&type_ref_token.row().to_le_bytes());
    } else {
        buf.push(0x11); // fallback TypeSpec placeholder
        buf.extend_from_slice(&0u32.to_le_bytes());
    }
}
```

**Simpler approach:** Add `type_ref_token` as a parameter to `encode_type` for the Type class, analogous to how `range_type_token()` works on the builder.

### Layer 8: Type Checker — `writ-compiler/src/check/check_expr/mod.rs`

`check_expr` dispatches on `AstExpr` variants. New arm:

```rust
AstExpr::TypeOf { expr: inner_expr, span } => {
    // Type-check the inner expression to determine its static type.
    // The inner expression value is irrelevant at runtime — only its type matters.
    let typed_inner = check_expr(ctx, inner_expr);
    let static_ty = typed_inner.ty();

    // Result type: TyKind::ReflectionType(static_ty)
    let reflection_ty = ctx.interner.reflection_type(static_ty);

    TypedExpr::TypeOf {
        ty: reflection_ty,
        span: *span,
        static_ty,
    }
}
```

**Error handling:** If `static_ty` is `TyKind::Error` (inner expression had a type error), return `TypedExpr::Error` to suppress cascading errors. This follows the existing pattern used throughout the checker.

### Layer 9: IL Code Generation — `writ-compiler/src/emit/body/expr/mod.rs`

`emit_expr` dispatches on `TypedExpr` variants. New arm for `TypedExpr::TypeOf`:

```rust
TypedExpr::TypeOf { ty, static_ty, .. } => {
    let r_dst = emitter.alloc_reg(*ty);
    let type_idx = resolve_type_idx_for_static_ty(emitter, *static_ty);
    emitter.emit(Instruction::TypeOf { r_dst, type_idx });
    r_dst
}
```

**`resolve_type_idx_for_static_ty` logic:**

```rust
fn resolve_type_idx_for_static_ty(emitter: &BodyEmitter<'_>, static_ty: Ty) -> u32 {
    match emitter.interner.kind(static_ty) {
        // Named user types: look up their TypeDef token via DefId
        TyKind::Struct(def_id)
        | TyKind::Class(def_id)
        | TyKind::Entity(def_id)
        | TyKind::Enum(def_id)
        | TyKind::Contract(def_id) => {
            emitter.builder.token_for_def(*def_id).map(|t| t.0).unwrap_or(0)
        }
        // Primitives: use writ-runtime pseudo-TypeDef tokens
        // Int/Float/Bool/String are pseudo-TypeDefs in the virtual module
        TyKind::Int => emitter.builder.primitive_type_token(PrimitiveTy::Int),
        TyKind::Float => emitter.builder.primitive_type_token(PrimitiveTy::Float),
        TyKind::Bool => emitter.builder.primitive_type_token(PrimitiveTy::Bool),
        TyKind::String => emitter.builder.primitive_type_token(PrimitiveTy::String),
        // Other: 0 (placeholder — handled at runtime gracefully)
        _ => 0,
    }
}
```

**Primitive type token challenge:** The writ-runtime virtual module has TypeDefs for Int/Float/Bool/String at known indices (§1.18). The compiler needs a way to get those tokens. Options:

1. Add `primitive_type_token(PrimitiveTy)` to `ModuleBuilder` — returns a TypeRef token for the primitive pseudo-TypeDef in writ-runtime (same pattern as `range_type_token()`).
2. Hard-code the raw token values: Int is TypeRef 0, Float is TypeRef 1, etc. (brittle but simple).
3. Register TypeRefs for all primitives in `collect_defs` alongside Range/Type.

**Recommended approach:** Register TypeRefs for `"Int"`, `"Float"`, `"Bool"`, `"String"`, `"Type"` from writ-runtime in `collect_defs`, then add lookup methods `primitive_type_token(name: &str)` to ModuleBuilder. This is the same pattern as Range.

**Phase scope note:** For COMP-02 minimum, user-defined types (structs/classes/entities/enums) with real DefIds are sufficient. Primitives can emit type_idx=0 as a known-stub if the runtime handles it gracefully, with a TODO. However, REFL-01 requires all types including primitives, so primitive tokens should be wired in this phase.

### Layer 10: BOX/UNBOX Coercions — COMP-04

The locked decision: compiler auto-inserts BOX/UNBOX coercions at reflection API parameter/return sites. No TyKind::Any needed.

**What "reflection API parameter sites" means:** When a function parameter is typed as `Type` (the reflection class from writ-runtime), and the caller passes a `TyKind::ReflectionType(inner)` value — the types match directly since TyKind::ReflectionType IS the Type class. No boxing is needed in the straightforward call path.

**Where BOX/UNBOX IS needed:** Functions that take `any`-like opaque parameters (e.g., `FieldInfo.get(instance)` which returns an untyped value). Phase 104 doesn't implement those — that's Phase 106+.

**Conclusion for Phase 104:** BOX/UNBOX coercion infrastructure is needed in the type checker's unification logic. When unifying `TyKind::ReflectionType(T)` with a concrete type `T`, the checker should NOT automatically widen — it should require explicit coercion. The auto-insert pattern: in `check_call` (and other sites that unify expected vs actual types), if `expected` is `TyKind::ReflectionType` and `actual` is `TyKind::ReflectionType` → OK. If `expected` is a concrete type and `actual` is `TyKind::ReflectionType` → insert BOX coercion wrapper. If `expected` is `TyKind::ReflectionType` and `actual` is a concrete type → emit UNBOX wrapper.

**For Phase 104 specifically:** The only reflection API usage is `typeof(expr)` as a standalone expression. There are no cross-type API boundary calls yet. The BOX/UNBOX infrastructure should be set up (the detection logic in unify/check_call) but will only trigger in later phases when reflection methods are called. COMP-04 can be satisfied by: implementing the coercion-insertion logic that correctly handles the case, even if no test exercises a cross-boundary call yet. A test that assigns a `typeof(x)` result to a `Type` typed variable would satisfy the requirement.

### Layer 11: Module Builder — `writ-compiler/src/emit/collect/mod.rs`

In `collect_defs`, register TypeRefs for all reflection types and primitives:

```rust
// 2b. TypeRef: reflection types and primitives from writ-runtime
builder.add_type_ref(runtime_mod_idx, "Range", "writ");     // existing
builder.add_type_ref(runtime_mod_idx, "Type", "writ");       // NEW
builder.add_type_ref(runtime_mod_idx, "Int", "writ");        // NEW (primitive pseudo-TypeDef)
builder.add_type_ref(runtime_mod_idx, "Float", "writ");      // NEW
builder.add_type_ref(runtime_mod_idx, "Bool", "writ");       // NEW
builder.add_type_ref(runtime_mod_idx, "String", "writ");     // NEW
```

Add lookup method to ModuleBuilder:
```rust
pub fn type_ref_token_by_name(&self, name: &str) -> Option<u32> {
    for (i, tr) in self.type_refs.iter().enumerate() {
        if self.string_heap.get_str(tr.name) == name {
            return Some(MetadataToken::new(TableId::TypeRef, (i + 1) as u32).0);
        }
    }
    None
}
```

**Note:** `range_type_token()` already does exactly this for `"Range"` — the new method generalizes it. Consider replacing `range_type_token()` with `type_ref_token_by_name("Range")` calls.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type token lookup | Custom index table | `builder.token_for_def(def_id)` | Already works for all user-defined types |
| Cross-module type reference | Hardcoded byte offset | `add_type_ref` + `type_ref_token_by_name` | Same pattern as Range, already proven |
| Keyword parsing | Custom tokenizer | logos `#[token]` attribute | All other keywords use this |
| Type interning | Manual dedup | `TyInterner::intern(TyKind::ReflectionType(inner))` | Structural dedup already implemented |

---

## Common Pitfalls

### Pitfall 1: Exhaustive match arms — compile errors are your friends
**What goes wrong:** Adding `Expr::TypeOf` to the CST without adding it to `lower_expr`, or adding `AstExpr::TypeOf` without handling it in `check_expr`, etc.
**Why it happens:** Missing arms in exhaustive `match` blocks.
**How to avoid:** The Rust compiler will flag all missing arms. Follow the "one new arm per layer" discipline. The existing `no _ =>` comment on `lower_expr` is the key signal.
**Warning signs:** `cargo build` errors mentioning pattern not covered.

### Pitfall 2: TypedExpr::TypeOf's `ty` vs `static_ty` confusion
**What goes wrong:** Emitter uses `ty` (the result type, TyKind::ReflectionType) to resolve type_idx, instead of `static_ty` (the inner expression's type, what we're reflecting on).
**Why it happens:** Other TypedExpr variants only have `ty`. The TypeOf variant intentionally carries both.
**How to avoid:** Name the field `static_ty` (not `inner_ty` or `operand_ty`) to make the distinction explicit.

### Pitfall 3: Missing `.ty()` and `.span()` arms in TypedExpr
**What goes wrong:** Compile error when `TypedExpr::TypeOf` is not in the exhaustive `.ty()` or `.span()` implementations.
**Why it happens:** Both methods use `match self { ... }` with all existing variants.
**How to avoid:** Update both match arms when adding TypedExpr::TypeOf.

### Pitfall 4: Primitive typeof yields type_idx=0 at runtime
**What goes wrong:** `typeof(42)` emits `TypeOf { r_dst, type_idx: 0 }` because Int has no user DefId.
**Why it happens:** `emitter.builder.token_for_def(def_id)` only handles user-registered TypeDefs.
**How to avoid:** Register TypeRef rows for all writ-runtime primitives in `collect_defs` and use `type_ref_token_by_name("Int")` etc.

### Pitfall 5: TyKind::ReflectionType breaks encode_type assertions
**What goes wrong:** `type_sig.rs` hits `debug_assert!(false, "Infer type should not appear...")` or similar when it encounters the new variant.
**Why it happens:** `encode_type_into` has branches for all existing TyKind variants but would fall through to `TyKind::Error` handling for an unknown variant.
**How to avoid:** Add an explicit branch for `TyKind::ReflectionType` before Infer/Error in `encode_type_into`.

### Pitfall 6: Snapshot test failures from new AstExpr variant
**What goes wrong:** Existing lowering snapshot tests fail because `AstExpr` Debug output changed (the enum gained a new variant, changing derive output order).
**Why it happens:** `insta` compares exact Debug output. New enum variants don't change existing variant output.
**How to avoid:** Adding a new variant to AstExpr/TyKind doesn't change existing variants' Debug output. No existing snapshots should break. If any do, it's a different issue.

---

## Code Examples

### Existing: `try` keyword end-to-end (exact pattern to follow)

**Lexer:**
```rust
// writ-parser/src/lexer.rs
#[token("try")]
KwTry,
```

**CST variant:**
```rust
// writ-parser/src/cst.rs
Try(Box<Spanned<Expr<'src>>>),
```

**Parser atom:**
```rust
// writ-parser/src/parser/program.rs
let try_expr = just(Token::KwTry)
    .ignore_then(expr.clone())
    .map_with(|e, extra| (cst::Expr::Try(Box::new(e)), extra.span()));
```

**AstExpr variant:**
```rust
// writ-compiler/src/ast/expr.rs
Try { expr: Box<AstExpr>, span: SimpleSpan },
```

**Lowering arm:**
```rust
// writ-compiler/src/lower/expr.rs
Expr::Try(e) => AstExpr::Try {
    expr: Box::new(lower_expr(*e, ctx)),
    span,
},
```

**TypedExpr variant:**
```rust
// writ-compiler/src/check/ir.rs
// (try desugars in check_expr, so no TypedExpr::Try — TypeOf is different)
```

Note: `try` desugars in the type checker (calls `desugar::desugar_try`). `typeof` does NOT desugar — it remains as a distinct TypedExpr variant all the way to the emitter. The `Spawn` variant IS kept as TypedExpr::Spawn through to emit, making it the better model.

### Existing: How token_for_def works for New instruction
```rust
// writ-compiler/src/emit/body/expr/construction.rs
let type_idx = emitter
    .builder
    .token_for_def(target_def_id)
    .map(|t| t.0)
    .unwrap_or(0);
emitter.emit(Instruction::New { r_dst: r_obj, type_idx });
```

The `typeof` emit follows the same pattern for user types.

### Existing: TypeRef lookup (range_type_token)
```rust
// writ-compiler/src/emit/module_builder.rs
pub fn range_type_token(&self) -> u32 {
    for (i, tr) in self.type_refs.iter().enumerate() {
        let name = self.string_heap.get_str(tr.name);
        if name == "Range" {
            return MetadataToken::new(TableId::TypeRef, (i + 1) as u32).0;
        }
    }
    0
}
```

New `type_ref_token_by_name` generalizes this.

### Existing: TypeOf instruction encoding (already complete)
```rust
// writ-module/src/instruction.rs
TypeOf { r_dst: u16, type_idx: u32 },  // 0x0A30, Shape RI32 (8B)
```

```rust
// writ-runtime/src/dispatch/mod.rs (Phase 103 — already dispatched)
Instruction::TypeOf { r_dst, type_idx } => {
    // ... loads lazy singleton Type object for typedef_0based index
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No `typeof` in Writ | `typeof(expr)` static compile-time query | Phase 104 (this phase) | New compiler keyword |
| No ReflectionType in TyKind | TyKind::ReflectionType(Ty) | Phase 104 | Reflects Type builtin class |
| TypeOf opcode not emitted by compiler | Compiler emits TypeOf with type_idx | Phase 104 | IL generation wired |

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + insta snapshots |
| Config file | `writ-compiler/Cargo.toml` (dev-dependencies: insta) |
| Quick run command | `cargo test -p writ-compiler -- --nocapture 2>&1 | tail -20` |
| Full suite command | `cargo test --workspace 2>&1 | tail -20` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COMP-01 | `typeof(x)` lowers to AstExpr::TypeOf, not Call | unit (lowering_tests.rs) | `cargo test -p writ-compiler lower_typeof` | ❌ Wave 0 |
| COMP-02 | TypeOf instruction emitted with correct type_idx | unit (emit_body_tests.rs or emit_tests.rs) | `cargo test -p writ-compiler emit_typeof` | ❌ Wave 0 |
| COMP-04 | BOX/UNBOX auto-inserted at reflection API boundaries | unit (typecheck_tests.rs) | `cargo test -p writ-compiler typeof_reflection_type` | ❌ Wave 0 |
| COMP-05 | TyKind::ReflectionType assigned to typeof expressions | unit (typecheck_tests.rs) | `cargo test -p writ-compiler tykind_reflection` | ❌ Wave 0 |
| REFL-01 | typeof works for structs, enums, entities, primitives | integration (emit_body_tests.rs) | `cargo test -p writ-compiler typeof_all_type_categories` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler 2>&1 | tail -10`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -20`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-compiler/tests/lowering_tests.rs` — add `lower_typeof_basic`, `lower_typeof_snapshot` test functions
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `tykind_typeof_struct`, `tykind_typeof_primitive`, `typeof_type_error_on_wrong_use`
- [ ] `writ-compiler/tests/emit_body_tests.rs` (or `emit_tests.rs`) — add `emit_typeof_struct`, `emit_typeof_returns_type_ref`
- [ ] No new test files needed — all go into existing test files

*(Framework already installed — insta + cargo test are in use project-wide)*

---

## Open Questions

1. **Primitive type_idx encoding**
   - What we know: writ-runtime virtual module registers Int/Float/Bool/String at known TypeDef indices (0-3 in the virtual module, per §1.18)
   - What's unclear: Do user modules get TypeRef rows for primitives, or do primitives use a special token encoding? The `reflection_tests.rs` runtime test uses `typedef_token(0)` which is a local TypeDef index, not a cross-module reference.
   - Recommendation: Register TypeRef rows for the 4 primitives in `collect_defs` alongside Range and Type, and look up their tokens via `type_ref_token_by_name`. If the runtime's TypeOf dispatch works by 0-based TypeDef index within the virtual module, the TypeRef encoding is correct.

2. **BOX/UNBOX test coverage for COMP-04**
   - What we know: No reflection API methods exist yet (Phase 106+). The BOX/UNBOX coercion path has no callers in Phase 104.
   - What's unclear: Does the test infrastructure need a synthetic test that exercises the coercion insertion, or is it sufficient to test that TyKind::ReflectionType is correctly assigned and does not unify with non-reflection types?
   - Recommendation: For Phase 104, COMP-04 is satisfied by: (a) TyKind::ReflectionType does NOT unify with other types (type error if used where a non-Type value is expected), and (b) a function that accepts a `Type` parameter and receives `typeof(x)` compiles without error. Full BOX/UNBOX IL emission is a Phase 106 concern.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure compiler code changes within the workspace)

---

## Sources

### Primary (HIGH confidence)
- Direct source inspection of `writ-compiler/src/` — lexer, parser, lower, check, emit layers
- Direct source inspection of `writ-parser/src/` — lexer tokens, CST types, program parser
- Direct source inspection of `writ-module/src/instruction.rs` — TypeOf at 0x0A30 confirmed
- `writ-runtime/tests/reflection_tests.rs` — TypeOf usage pattern verified

### Secondary (MEDIUM confidence)
- `writ-compiler/tests/` — test helper patterns for typecheck_tests, emit_tests, lowering_tests
- `writ-runtime/src/dispatch/mod.rs` — TypeOf dispatch verified at line 522

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all layers inspected directly
- Architecture: HIGH — each layer's exact file, function, and line range identified
- Pitfalls: HIGH — derived from actual code structure (exhaustive matches, naming conventions)

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable compiler architecture, low churn)
