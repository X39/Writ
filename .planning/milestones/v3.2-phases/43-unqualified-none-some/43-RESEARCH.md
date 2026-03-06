# Phase 43: Unqualified None/Some - Research

**Researched:** 2026-03-06
**Domain:** Writ compiler — name resolution, type checking, parser extension, spec update
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Phase 43 implements a general `using EnumName::*;` glob variant import mechanism (not just Option-specific)
- `None` and `Some` are pre-injected at sub-prelude priority automatically — no `using` needed
- `using Status::*;` brings all Status variants into scope unqualified (general mechanism)
- `using Option::*;` is also valid (redundant since None/Some are pre-injected, but consistent and must not error)
- Selective import (`using Option::None;`) is NOT required for this phase — glob only
- Unqualified `None`/`Some` must work in both expression AND pattern position
- `let x = None;` with no type annotation → error: "cannot infer type for `None` — add a type annotation: `let x: T? = None`"
- `let x = Some(42);` → infers `int?` from the argument type
- `foo(None)` where `foo` takes `bool?` → infers `bool?` from the parameter type (bidirectional inference from context)
- All user definitions silently shadow injected symbols (no warning)
- Two using-glob conflicts → ambiguity error (reuses existing `LookupResult::Ambiguous`)
- Spec gets a new subsection in the existing imports/using section

### Claude's Discretion
- Implementation approach for sub-prelude injection: new `LookupResult` variant (e.g., `OptionConstructor`) vs. rewriting to qualified path before resolution vs. handling in type checker
- Whether `using Status::*;` is handled in the resolver scope chain (new `ScopeLayer::GlobEnum`) or expanded at the `using` declaration site
- Exact spec section number for the new imports subsection

### Deferred Ideas (OUT OF SCOPE)
- Selective import: `using Option::None;` (import one variant) — not required, defer to future
- `using Enum::*;` scoped to a block rather than a file — deferred
- Warning on shadowing built-in None/Some — explicitly rejected; no warning per user decision
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LANG-02 | User can write `None` and `Some(value)` without `Option::` prefix — resolver injects both as sub-prelude symbols that do not shadow user-defined names | Sub-prelude injection in `scope.rs` + check layer typing for `None`/`Some` idents; emitter already handles them by name |
</phase_requirements>

---

## Summary

Phase 43 adds two related features: (1) automatic sub-prelude injection of `None` and `Some` so they resolve without `Option::` qualification, and (2) a general `using EnumName::*;` glob import mechanism that brings all variants of any user-defined enum into unqualified scope.

The emitter already handles `"None"` and `"Some"` by name regardless of path — both `TypedExpr::Path { segments: [..., "None"] }` and `TypedExpr::Var { name: "None" }` in call position already emit `LOAD_NULL`/`WRAP_SOME` correctly (confirmed in `emit/body/expr.rs:1054-1095`). This means **no emitter changes are needed**. The work is entirely in the resolver, type checker, parser (for `*` in `using` paths), and spec.

The key architectural insight is that `check_ident` and `check_path` in `check_expr.rs` need to recognize `"None"` and `"Some"` as Option constructors when they appear unqualified. The cleanest approach is to add a new `LookupResult::OptionConstructor(String)` variant (or equivalent) in `scope.rs` and handle it in `check_ident`/`check_path` in `check_expr.rs` to produce typed nodes with the correct `TyKind::Option(InferVar)` type.

**Primary recommendation:** Add `LookupResult::BuiltinVariant(String)` to `scope.rs`; handle it in `check_ident`/`check_path` to produce `TypedExpr::Var`/`TypedExpr::Path` with `Option<InferVar>` type; implement `using Status::*;` by expanding to per-variant `UsingEntry` records in `process_usings`; update the parser's `qualified_name` to allow a trailing `::*` terminal in the `using_decl` parser only.

---

## Standard Stack

### Core
| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| `resolve/scope.rs` | `writ-compiler` | `LookupResult` enum + `ScopeChain` | Needs new `BuiltinVariant` variant and sub-prelude lookup step |
| `resolve/prelude.rs` | `writ-compiler` | Named constant tables for prelude items | Needs new `SUB_PRELUDE_VARIANT_NAMES` constant |
| `resolve/resolver.rs` | `writ-compiler` | `process_usings` — parses and registers `using` declarations | Needs glob-enum expansion branch |
| `check/check_expr.rs` | `writ-compiler` | `check_ident`, `check_path` | Need to handle `BuiltinVariant` result → produce correctly typed `TypedExpr` |
| `writ-parser/src/parser.rs` | `writ-parser` | `using_decl` parser, `qualified_name` | Needs `::*` terminal acceptance |
| `language-spec/spec/24_23_modules_namespaces.md` | spec | §23.4 using declarations | Needs new subsection for glob imports and sub-prelude builtins |

### No Changes Needed
| Component | Why |
|-----------|-----|
| `emit/body/expr.rs` | Already matches `"None"`/`"Some"` by name in both Path and Var callee positions |
| `lower/expr.rs` | `null` → `Path { segments: ["Option", "None"] }` still correct; unqualified `None` follows separate path |
| `check/check_stmt.rs` | Type annotation unification already works; bare `None` inference error is a new `TypeError` variant |

---

## Architecture Patterns

### Recommended Implementation Structure

```
writ-compiler/src/resolve/
├── prelude.rs           # Add SUB_PRELUDE_VARIANT_NAMES: &[&str]
├── scope.rs             # Add LookupResult::BuiltinVariant(String)
│                        # Add sub-prelude lookup step in resolve_value/resolve_type
└── resolver.rs          # Add glob-enum expansion in process_usings

writ-compiler/src/check/
└── check_expr.rs        # Handle BuiltinVariant in check_ident + check_path
                         # Add bare-None type inference error

writ-parser/src/
└── parser.rs            # Extend using_decl to accept ::* terminal

language-spec/spec/
└── 24_23_modules_namespaces.md   # New §23.4.X subsection
```

### Pattern 1: Sub-Prelude Lookup in `resolve_value`

**What:** After local bindings and generics, before using-imports, inject a check for `"None"`/`"Some"`.
**When to use:** When looking up an identifier in expression or pattern position.
**Priority:** Locals > Generics > Primitives > Prelude types > User namespace > **Sub-prelude builtins** > Using-imports > Root namespace

```rust
// Source: resolve/scope.rs - resolve_value (to be added after local/generic checks)
// Sub-prelude builtins: injected BELOW user namespace but ABOVE using-imports
// This means user defs in current namespace shadow them, but using-imports do NOT.
// Actually: sub-prelude means they lose to EVERYTHING user-defined.
// Correct priority: after prelude check, after user namespace, before using-imports or after?
// Decision: after root namespace lookup fails -> check sub-prelude builtins last
// (so ANY user def shadows them)
```

**IMPORTANT NOTE on priority order:** The spec says "sub-prelude priority" means user-defined names always win at any scope level. This means the sub-prelude check must come LAST in `resolve_value` — after locals, generics, primitives, prelude, namespace, file-private, using-imports, and root namespace. Only when all of those return `NotFound` do we check sub-prelude builtins.

```rust
// In resolve_value, append after step 7 (currently "NotFound"):
// Step 8: Check sub-prelude builtins
if prelude::SUB_PRELUDE_VARIANT_NAMES.contains(&name) {
    return LookupResult::BuiltinVariant(name.to_string());
}
LookupResult::NotFound
```

### Pattern 2: `LookupResult::BuiltinVariant` in Type Checker

**What:** `check_ident` must produce a `TypedExpr::Var` with type `Option<InferVar>` when it sees a `BuiltinVariant` result.
**When to use:** When `check_ident` or the ident-fast-path in `check_call` encounters `"None"` or `"Some"`.

```rust
// Source: check/check_expr.rs - check_ident (conceptual, not existing code)
// After all DefMap lookups fail, check resolver scope chain:
// (The check layer currently does NOT call scope.resolve_value — it uses def_map directly)
// This means check_ident needs its own sub-prelude check:

// For "None" (zero-arg constructor):
"None" => {
    let infer = ctx.interner.intern(TyKind::Infer(ctx.unify.new_var()));
    let opt_ty = ctx.interner.option(infer);
    TypedExpr::Var { ty: opt_ty, span, name: "None".to_string() }
}
// For "Some" (one-arg constructor):
"Some" => {
    // Return as a callable — type is fn(T) -> Option<T>
    // But emitter handles it by name, so just need a non-error type
    let infer = ctx.interner.intern(TyKind::Infer(ctx.unify.new_var()));
    let opt_ty = ctx.interner.option(infer);
    // Some as a value (not called) is a constructor reference
    // For now, returning opt_ty is sufficient for the emitter path
    TypedExpr::Var { ty: opt_ty, span, name: "Some".to_string() }
}
```

**Critical insight:** The check layer (`check_ident`, `check_path`) does NOT currently go through the resolver `ScopeChain`. It queries `def_map` directly. This means adding `LookupResult::BuiltinVariant` to `scope.rs` is necessary for the resolver pass, but `check_expr.rs` needs its own direct check for `"None"` and `"Some"` by name.

### Pattern 3: `using Status::*;` Glob Expansion

**What:** When `process_usings` encounters a path ending in `"*"`, it finds the enum by its prefix, gets all variants from the def_map, and registers one `UsingEntry` per variant with `target_fqn`.
**When to use:** When the `AstUsingDecl.path` last segment is `"*"`.

```rust
// Source: resolve/resolver.rs - process_usings (to be added)
if path.last().map(|s| s == "*").unwrap_or(false) && path.len() >= 2 {
    // Glob import: using Status::*;
    let enum_path = &path[..path.len() - 1];
    let enum_fqn = enum_path.join("::");

    if let Some(enum_def_id) = scope.def_map.get(&enum_fqn) {
        let entry = scope.def_map.get_entry(enum_def_id);
        if entry.kind == DefKind::Enum {
            // Expand each variant as a separate UsingEntry with target_fqn
            // ... register one entry per variant
        }
    }
    // Mark as GlobEnum type for "used" tracking (or skip unused-import warning for globs)
}
```

**Variant enumeration:** The def_map stores enum variants as children; they appear as `"EnumName::VariantName"` FQNs. The existing `scope.def_map.namespace_members` or a variant-enumeration helper can be used.

### Pattern 4: Parser Extension for `::*`

**What:** The `qualified_name` parser currently only accepts `Token::Ident` segments separated by `::`. The `using_decl` needs to accept a trailing `*` terminal.
**When to use:** Only in the `using_decl` parser — NOT in `qualified_name` generally (that would break type expressions and paths).

```rust
// Source: writ-parser/src/parser.rs - using_decl (to be extended)
// Current: .then(qualified_name.clone())
// New: accept qualified_name OR qualified_name + ::* terminal
let using_path = qualified_name.clone()
    .then(
        just(Token::ColonColon)
            .ignore_then(just(Token::Star))  // :: *
            .or_not()
    )
    .map(|(mut path, star)| {
        if star.is_some() {
            path.push(("*", star_span));  // or use a sentinel
        }
        path
    });
```

**Token check:** `Token::Star` must exist in the lexer for `*`. Confirm this exists (it is used for multiplication expressions, so it should be `Token::Star` or similar).

### Anti-Patterns to Avoid

- **Adding `None`/`Some` to `PRELUDE_TYPE_NAMES`:** Prelude types cannot be shadowed by user code; sub-prelude must be separate and checked last.
- **Modifying `null` lowering:** `null` → `Path { segments: ["Option", "None"] }` already works; don't change it.
- **Using a new `ScopeLayer::GlobEnum` layer:** The existing `UsingEntry` mechanism is sufficient — just register one entry per variant with `target_fqn`. No new layer type needed.
- **Changing the emitter:** The emitter already handles `"None"`/`"Some"` by last-segment name in both `TypedExpr::Path` and `TypedExpr::Var`. No emitter changes.
- **Generating an error type for unqualified `None`/`Some`:** The check layer must produce a real `Option<InferVar>` type, not Error, so that type annotation unification in `check_stmt.rs` can resolve the infer variable.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type inference for bare `None` | Custom unification logic | `ctx.interner.intern(TyKind::Infer(...))` + existing `UnifyCtx` | Already works for other infer contexts |
| Ambiguity detection for two globs | New ambiguity tracking | `LookupResult::Ambiguous(Vec<(DefId, String)>)` | Already implemented in `scope.rs` and handled in `resolver.rs` |
| Unused import warning suppression | New "glob used" flag | Don't emit unused-import warnings for glob entries (or mark all as used immediately) | Glob imports are inherently harder to track usage on; simpler to always mark used |

**Key insight:** The emitter's name-matching for `"None"`/`"Some"` was designed to be path-agnostic. Unqualified names flow through to the same emitter paths naturally.

---

## Common Pitfalls

### Pitfall 1: Check Layer Does Not Use `ScopeChain`
**What goes wrong:** Adding `LookupResult::BuiltinVariant` to `scope.rs` then expecting `check_ident` to see it automatically.
**Why it happens:** `check_ident` queries `def_map` directly (not through `ScopeChain::resolve_value`). The resolver and type checker are separate passes.
**How to avoid:** Add the `"None"`/`"Some"` check directly in `check_ident` by name — a simple `match name { "None" => ..., "Some" => ... }` — in addition to (or instead of) the `LookupResult::BuiltinVariant` in the resolver.
**Warning signs:** If `None` resolves in the resolver pass but still fails in the type checker with "undefined variable", this is the cause.

### Pitfall 2: `None` Without Type Annotation Produces Unconstrained Infer Variable
**What goes wrong:** `let x = None;` — the infer variable created for the `Option<InferVar>` type is never resolved, producing a confusing error or panic.
**Why it happens:** No constraint on the inner type is provided when `None` is used as a bare value.
**How to avoid:** After checking the value expression in `check_stmt.rs`, if the result type is `Option<Infer(?)>` and no annotation is present, emit a specific "cannot infer type for None" error (`TypeError::NoneWithoutAnnotation` or similar) before it reaches unification.
**Warning signs:** Panic in `resolve_ty` or an unconstrained infer variable at the end of type checking.

### Pitfall 3: `using Status::*;` Expands to Variants Not in DefMap
**What goes wrong:** For prelude enums like `Option`, there are no DefMap entries for `None`/`Some` variants because Option is a prelude type (not a user-defined enum).
**Why it happens:** `scope.def_map.get("Option::None")` returns `None`.
**How to avoid:** `using Option::*;` is the "valid but redundant" case — when the enum FQN resolves to a prelude type (not a user-defined `DefKind::Enum`), silently accept the declaration without registering any UsingEntry (the sub-prelude injection already handles `None`/`Some`). Only expand user-defined enums.
**Warning signs:** If `using Option::*;` errors with "cannot find Option in DefMap".

### Pitfall 4: Parser `*` Token Conflict
**What goes wrong:** Adding `*` to `qualified_name` makes it parseable in type positions, breaking multiplication or type expressions.
**Why it happens:** `qualified_name` is reused for namespace declarations too.
**How to avoid:** Add the `::*` optional tail only to the `using_decl` parser locally — do NOT modify `qualified_name` itself.
**Warning signs:** Parse errors in arithmetic or type expressions after the change.

### Pitfall 5: Shadowing Resolution Order
**What goes wrong:** A local variable named `None` causes a "variable already declared" error or lookup returns the sub-prelude builtin instead of the local.
**Why it happens:** Sub-prelude check placed too early in `resolve_value` (before locals).
**How to avoid:** The sub-prelude check MUST be the very last fallback in `resolve_value` (after root namespace lookup). Any user-defined symbol at any scope level must shadow it.
**Warning signs:** If `let None = 5;` causes an error instead of silently shadowing.

### Pitfall 6: `AstUsingDecl.path` Containing `"*"` Breaks Resolver
**What goes wrong:** The existing `process_usings` code tries `path.join("::")` to form an FQN, producing `"Status::*"` and failing to find it in def_map.
**Why it happens:** The glob case is not branched before the existing FQN logic.
**How to avoid:** Check for trailing `"*"` as the very first branch in `process_usings`, before all other path-length checks.
**Warning signs:** `ResolutionError::UnresolvedName { name: "Status::*" }` in output.

---

## Code Examples

### Existing: How `Option::None` flows through the emitter (no changes needed)
```rust
// Source: writ-compiler/src/emit/body/expr.rs:1047-1065
// TypedExpr::Path callee with last segment "None" and 0 args -> LOAD_NULL
TypedExpr::Path { segments, .. } => {
    let name = match segments.last() {
        Some(n) => n.as_str(),
        None => return None,
    };
    match name {
        "None" if args.is_empty() => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::LoadNull { r_dst });
            return Some(r_dst);
        }
        "Some" if args.len() == 1 => { /* WRAP_SOME */ }
        // ...
    }
}
// TypedExpr::Var { name: "None", .. } also handled identically at lines 1083-1095
```

### Existing: `null` lowering (no changes needed)
```rust
// Source: writ-compiler/src/lower/expr.rs:63-67
// null keyword → Path { segments: ["Option", "None"] }
Expr::NullLit => AstExpr::Path {
    segments: vec!["Option".to_string(), "None".to_string()],
    span,
},
// This still works because emitter matches "None" as last segment
```

### Existing: `LookupResult` variants in scope.rs (add BuiltinVariant here)
```rust
// Source: writ-compiler/src/resolve/scope.rs:42-60
pub enum LookupResult {
    Def(DefId),
    Primitive(PrimitiveTag),
    GenericParam(String),
    PreludeType(String),
    PreludeContract(String),
    NotFound,
    Ambiguous(Vec<(DefId, String)>),
    VisibilityError(DefId),
    // NEW:
    // BuiltinVariant(String),  // e.g., "None", "Some" — sub-prelude constructors
}
```

### Existing: prelude.rs pattern (add sub-prelude array here)
```rust
// Source: writ-compiler/src/resolve/prelude.rs:10
pub const PRELUDE_TYPE_NAMES: &[&str] = &["Option", "Result", "Range", "Array", "Entity"];
// ADD:
// pub const SUB_PRELUDE_VARIANT_NAMES: &[&str] = &["None", "Some"];
```

### Existing: check_ident structure (add builtin check here)
```rust
// Source: writ-compiler/src/check/check_expr.rs:351-448
fn check_ident(ctx: &mut CheckCtx, name: &str, span: SimpleSpan) -> TypedExpr {
    // 1. Local env
    // 2. DefMap (root, file-private)
    // ... (currently falls through to UndefinedVariable error)
    // ADD BEFORE error emit:
    // match name {
    //   "None" => { create Option<InferVar> node }
    //   "Some" => { create Option<InferVar> or constructor type node }
    //   _ => {}
    // }
}
```

### Existing: `process_usings` structure (add glob branch)
```rust
// Source: writ-compiler/src/resolve/resolver.rs:63-133
fn process_usings(items: &[AstDecl], scope: &mut ScopeChain<'_>, diags: &mut Vec<Diagnostic>) {
    for item in items {
        if let AstDecl::Using(using) = item {
            let path = &using.path;
            // ADD FIRST:
            // if path.last() == Some(&"*".to_string()) { /* glob handling */ continue; }
            if path.len() == 1 { /* namespace import */ }
            else if path.len() >= 2 { /* specific import */ }
        }
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|-----------------|--------|
| `Option::None` required | `None` (unqualified) | Phase 43 adds this |
| No glob enum imports | `using Status::*;` | Phase 43 adds this |
| `null` keyword only | `null` AND `None` | `None` becomes a parallel spelling |

---

## Open Questions

1. **Does `Some` in non-call position need special treatment?**
   - What we know: `Some(42)` appears as `AstExpr::Call { callee: AstExpr::Ident { name: "Some" }, args: [42] }` — the callee is checked by `check_ident`, then `check_call` fires the constructor path via the `TypedExpr::Var { name: "Some" }` emitter branch.
   - What's unclear: If `Some` appears as a bare identifier (not called), e.g., passed as a function value, what type should it get? This is a corner case not in the success criteria.
   - Recommendation: Assign `Option<InferVar>` type as a reasonable default; the emitter will only be reached when `Some` is a callee anyway.

2. **Where in the check layer does `foo(None)` context inference happen?**
   - What we know: `check_stmt.rs` unifies annotation type with inferred type after `check_expr`. For function arguments, `check_call_with_sig` unifies param type with arg type.
   - What's unclear: Whether the infer variable created for bare `None` gets resolved via `ctx.unify.unify(param_ty, arg_ty)` in `check_call_with_sig`.
   - Recommendation: Yes — if `None` produces `Option<InferVar>` and the param type is `Option<bool>`, unification resolves `InferVar = bool`. This is the standard inference path. The existing `UnifyCtx` handles this.

3. **Token name for `*` in the parser**
   - What we know: The parser uses `Token::Star` or similar for the multiplication operator.
   - What's unclear: The exact `Token` variant name without reading `lexer.rs` fully.
   - Recommendation: Check `writ-parser/src/lexer.rs` for the `*` token variant name before writing the parser change.

4. **`using Option::*;` expansion — prelude enum variants**
   - What we know: `Option` has no `DefId` and no variants in `def_map`. It is a `PreludeType`.
   - What's unclear: Whether `using Option::*;` should silently no-op or emit a "not a user-defined enum" error.
   - Recommendation: Silently no-op — treat it as a valid but vacuous import (no entries registered). This matches the "valid but redundant" decision in CONTEXT.md.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + `writ-compiler/tests/` integration tests |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-compiler unqualified` |
| Full suite command | `cargo test -p writ-compiler && cargo test -p writ-golden` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LANG-02-A | `let x: bool? = None;` compiles, emits LOAD_NULL | unit (typecheck + emit) | `cargo test -p writ-compiler none_unqualified_with_annotation` | ❌ Wave 0 |
| LANG-02-B | `let y = Some(true);` compiles, emits WRAP_SOME | unit (typecheck + emit) | `cargo test -p writ-compiler some_unqualified_infers_type` | ❌ Wave 0 |
| LANG-02-C | `let None = 5;` compiles — user def shadows builtin | unit (typecheck) | `cargo test -p writ-compiler user_none_shadows_builtin` | ❌ Wave 0 |
| LANG-02-D | `Option::None` and `Option::Some(v)` still compile | unit (typecheck) | `cargo test -p writ-compiler qualified_option_still_works` | ✅ (fn_optional.writ golden) |
| LANG-02-E | `let x = None;` (no annotation) → specific error | unit (typecheck) | `cargo test -p writ-compiler bare_none_no_annotation_error` | ❌ Wave 0 |
| LANG-02-F | `using Status::*;` brings variants into scope | unit (resolve + typecheck) | `cargo test -p writ-compiler using_enum_glob` | ❌ Wave 0 |
| LANG-02-G | Two glob conflicts → ambiguity error | unit (resolve) | `cargo test -p writ-compiler using_glob_conflict_ambiguous` | ❌ Wave 0 |
| LANG-02-H | `match x { None => ..., Some(v) => ... }` pattern | unit (typecheck) | `cargo test -p writ-compiler none_some_in_pattern_position` | ❌ Wave 0 |
| LANG-02-I | `using Option::*;` is valid (no error) | unit (resolve) | `cargo test -p writ-compiler using_option_glob_redundant_no_error` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler unqualified`
- **Per wave merge:** `cargo test -p writ-compiler && cargo test -p writ-golden`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add tests LANG-02-A through LANG-02-E, LANG-02-H
- [ ] `writ-compiler/tests/resolve_tests.rs` — add tests LANG-02-F, LANG-02-G, LANG-02-I
- [ ] `writ-compiler/tests/emit_body_tests.rs` — add emit-level tests for LOAD_NULL/WRAP_SOME via unqualified names (optional, high confidence the emitter path already covers this)

*(LANG-02-D already covered by the existing `fn_optional` golden test — no new test needed for that case.)*

---

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `writ-compiler/src/resolve/scope.rs` — LookupResult enum, ScopeChain::resolve_value, ScopeChain::resolve_type
- Direct source inspection: `writ-compiler/src/resolve/resolver.rs` — process_usings, full resolver architecture
- Direct source inspection: `writ-compiler/src/resolve/prelude.rs` — PRELUDE_TYPE_NAMES pattern
- Direct source inspection: `writ-compiler/src/check/check_expr.rs` — check_ident, check_path, check_call, EnumDestructure pattern handling
- Direct source inspection: `writ-compiler/src/check/check_stmt.rs` — let binding unification flow
- Direct source inspection: `writ-compiler/src/check/ty.rs` — TyKind::Option, TyKind::Infer, TyInterner
- Direct source inspection: `writ-compiler/src/emit/body/expr.rs:1047-1110` — None/Some/Ok/Err name-based emitter dispatch
- Direct source inspection: `writ-compiler/src/lower/expr.rs:63-67` — null → Option::None lowering
- Direct source inspection: `writ-parser/src/parser.rs:2393-2445` — qualified_name and using_decl parsers
- Direct source inspection: `writ-parser/src/cst.rs:174-181` — UsingDecl structure
- Direct source inspection: `writ-golden/tests/golden/fn_optional.writ` — existing Option::None/Some golden test

### Secondary (MEDIUM confidence)
- `.planning/phases/43-unqualified-none-some/43-CONTEXT.md` — confirmed design decisions and codebase scouting

---

## Metadata

**Confidence breakdown:**
- Emitter path (no changes needed): HIGH — confirmed by direct code inspection
- Resolver sub-prelude injection design: HIGH — confirmed by reading full scope.rs and prelude.rs
- Check layer typing for None/Some: HIGH — confirmed check_ident does not go through ScopeChain; direct name check is required
- Parser `::*` extension: HIGH — confirmed using_decl structure and qualified_name; token name for `*` needs one more file read
- using-glob expansion: HIGH — process_usings structure fully read; UsingEntry mechanism is clean fit
- Spec section location: HIGH — confirmed §23.4 is the correct location in 24_23_modules_namespaces.md

**Research date:** 2026-03-06
**Valid until:** 2026-04-05 (stable codebase, 30-day window)
