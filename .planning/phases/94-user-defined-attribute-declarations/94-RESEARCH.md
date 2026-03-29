# Phase 94: User-Defined Attribute Declarations - Research

**Researched:** 2026-03-27
**Domain:** Writ compiler pipeline — parser, AST, resolver, emitter, virtual module
**Confidence:** HIGH

## Summary

Phase 94 adds `attribute Name(params);` as a first-class top-level declaration. The declaration propagates through all five pipeline stages: parser (CST Item), lowering (AstDecl), resolver (DefKind + collector), type checker (TypedDecl), and emitter (AttributeDef row in binary module). The virtual module gains four builtin attribute rows for `Deprecated`, `Conditional`, `Singleton`, and `Locale`. The collector pass rejects user code that tries to declare an attribute with one of those names.

The phase consists of exactly two plans. Plan 94-01 covers the CST/AST/parser/lowering layers and the new `DefKind::AttributeDef`. Plan 94-02 covers the resolver collector, type validation of attribute argument types, builtin name reservation, virtual module rows, and the emitter's `collect_post_finalize` hook.

**Primary recommendation:** Use the contextual-keyword strategy for `attribute` — match on `Token::Ident("attribute")` in the parser, not a new `Token::KwAttribute`. This avoids a breaking change to the lexer's reserved word set and is consistent with how the language spec describes `attribute` as a declaration keyword rather than a reserved word.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key considerations from STATE.md blockers:
- Research gap: `attribute` keyword vs. contextual keyword decision — resolve before touching the parser
- Builtin name reservation happens in the collector pass using DefId origin, not bare string matching

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UATTR-01 | User can declare attributes with typed parameters using `attribute Name(args);` syntax | Parser/AST/Lowering/Resolver sections below |
| UATTR-02 | User-defined attributes pass through the pipeline and appear in the module's AttributeDef table with serialized arguments | Emitter section — collect_post_finalize already handles this if DefKind::AttributeDef is in the TypedAst |
| UATTR-03 | Builtin attributes are registered in the writ-runtime virtual module namespace | Virtual module section — `build_writ_runtime_module()` needs four `add_attribute_def` calls |
| UATTR-04 | Builtin attribute names are reserved; user-defined attributes with the same name produce a name collision error | Collector pass — new `is_builtin_attribute_name()` predicate + new `BuiltinAttributeShadow` error |
</phase_requirements>

---

## Standard Stack

This is a pure compiler extension phase — no external library dependencies.

| Layer | File(s) | Current State | What Phase 94 Adds |
|-------|---------|---------------|---------------------|
| Lexer | `writ-parser/src/lexer.rs` | 40+ `KwXxx` tokens | No new token — use contextual ident |
| CST | `writ-parser/src/cst.rs` | `Item` enum, 14 variants | New `Item::Attribute(Spanned<AttributeDecl>)` |
| Parser | `writ-parser/src/parser/program.rs` | ~2900 lines, one giant `program_parser` closure | New `attribute_decl` combinator added to `attrs_vis_decl` or as standalone branch |
| AST | `writ-compiler/src/ast/decl.rs` | `AstDecl` 13 variants | New `AstDecl::Attribute(AstAttributeDecl)` |
| Lowering | `writ-compiler/src/lower/mod.rs` | `lower()` match on `Item` | New `Item::Attribute` arm → `AstDecl::Attribute` |
| Resolver | `writ-compiler/src/resolve/def_map.rs` | `DefKind` 12 variants | New `DefKind::AttributeDef` |
| Collector | `writ-compiler/src/resolve/collector.rs` | `collect_items` match on `AstDecl` | New `AstDecl::Attribute` arm + builtin name check |
| Resolver IR | `writ-compiler/src/resolve/ir.rs` | `ResolvedDecl` 12 variants | New `ResolvedDecl::AttributeDef { def_id }` |
| Type checker IR | `writ-compiler/src/check/ir.rs` | `TypedDecl` variants | New `TypedDecl::AttributeDef { def_id }` |
| Type checker | `writ-compiler/src/check/check_decl.rs` | `check_decl` match | Validate each param type is a supported attr type (string/int/bool) |
| Emitter collect | `writ-compiler/src/emit/collect/mod.rs` | `collect_defs` match on `TypedDecl` | New `TypedDecl::AttributeDef` arm |
| Virtual module | `writ-runtime/src/virtual_module.rs` | `build_writ_runtime_module()` | Four `add_attribute_def` calls for builtins |
| Error codes | `writ-diagnostics/src/code.rs` | E0001–E0124, W0001–W0005 | New `E0008: builtin attribute shadow` |

**No new Cargo dependencies required.**

---

## Architecture Patterns

### Resolved: Contextual Keyword for `attribute`

The existing lexer uses `#[token("fn")]` / `KwFn`, `#[token("struct")]` / `KwStruct`, etc. Adding a new `KwAttribute` requires adding `#[token("attribute")]` to the `Token` enum — this reserves the word globally and breaks any user code that uses `attribute` as a variable/field name.

The safer approach is contextual keyword matching in the parser:
```rust
// In program_parser closure, add a new branch for attribute_decl:
let attribute_decl = just(Token::Ident("attribute"))
    .ignore_then(ident)   // attribute name
    .then(
        param_list         // (name: type, ...)
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .or_not()
    )
    .then_ignore(just(Token::Semicolon))
    .map_with(|(name, params), e| {
        cst::AttributeDecl { name, params: params.unwrap_or_default(), span: e.span() }
    });
```

This matches `Token::Ident` where the string value is `"attribute"`. The Logos lexer emits `Token::Ident("attribute")` because `attribute` is not in the keyword list. Chumsky's `just(Token::Ident("attribute"))` matches a specific ident value.

**Alternative rejected:** Adding `KwAttribute` to the lexer is cleaner but makes `attribute` a reserved word permanently, which may not align with the language spec's intent. The contextual approach mirrors how `on` is handled for entity hooks.

Wait — actually, `on` IS a keyword token (`Token::KwOn`). Let me reconsider.

**Decision (HIGH confidence):** Add `KwAttribute` as a proper keyword token. The language spec treats `attribute` as a declaration-level keyword, and existing keywords like `fn`, `struct`, `entity`, `contract`, `const`, `global`, `component`, `extern`, `namespace`, `using`, `impl`, `dlg`, `class` are all proper tokens. User code cannot already use `attribute` as a name for top-level declarations (it would be parsed as a statement). Adding the keyword now is cleaner and consistent.

If `attribute` must remain usable as a local variable name (like `on` is used in expressions), then contextual matching is required. However, for top-level declaration keywords, reserved is acceptable. **Use `KwAttribute`.**

### Recommended Project Structure for New Files

No new files are needed. All changes are in-place extensions to existing files following the pattern of prior declaration additions (e.g., `KwClass`/`AstClassDecl`/`DefKind::Class`).

### Pattern 1: Adding a New Declaration Kind (established pattern)

The full chain for any new declaration kind follows this template, verified by reading how `Class` was added:

```
1. lexer.rs       → #[token("attribute")] / KwAttribute
2. cst.rs         → struct AttributeDecl<'src> { name, params, span }
                    Item::Attribute(Spanned<AttributeDecl>)
3. program.rs     → attribute_decl combinator
                    add to attrs_vis_decl choice() or as standalone branch
4. ast/decl.rs    → struct AstAttributeDecl { name, name_span, params, span }
                    AstDecl::Attribute(AstAttributeDecl)
5. lower/mod.rs   → Item::Attribute arm in lower()
6. def_map.rs     → DefKind::AttributeDef
7. collector.rs   → AstDecl::Attribute arm in collect_items()
                    + builtin name reservation check
8. resolve/ir.rs  → ResolvedDecl::AttributeDef { def_id }
9. check/ir.rs    → TypedDecl::AttributeDef { def_id }
10. check_decl.rs → type-check each param type
11. emit/collect/mod.rs → TypedDecl::AttributeDef arm
12. virtual_module.rs   → four builtin attribute rows
13. diagnostics/code.rs → E0008 constant
14. resolve/error.rs    → BuiltinAttributeShadow variant
```

### Pattern 2: CST AttributeDecl Structure

The CST `AttributeDecl` mirrors `FnSig` but without generics or return type:

```rust
// In cst.rs
pub struct AttributeDecl<'src> {
    pub attrs: Vec<Spanned<Attribute<'src>>>,  // attributes ON the declaration (unusual but consistent)
    pub vis: Option<Visibility>,               // optional pub/priv
    pub name: Spanned<&'src str>,
    pub params: Vec<Spanned<Param<'src>>>,     // name: type pairs (reuse existing Param)
    pub span: SimpleSpan,
}
```

Params reuse the existing `Param<'src>` type from CST (already used for struct fields and fn params) — `{ name: Spanned<&'src str>, ty: Spanned<TypeExpr<'src>> }`.

### Pattern 3: AstAttributeDecl Structure

```rust
// In ast/decl.rs
pub struct AstAttributeDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub params: Vec<AstParam>,   // reuse AstParam (name: String, ty: AstType)
    pub span: SimpleSpan,
}
```

`AstParam` is already defined in `ast/decl.rs`:
```rust
pub struct AstParam {
    pub name: String,
    pub name_span: SimpleSpan,
    pub ty: AstType,
    pub span: SimpleSpan,
}
```

### Pattern 4: Builtin Name Reservation in Collector

The STATE.md decision is explicit: **use DefId origin, not bare string matching**. The pattern is:

```rust
// New constant in resolve/prelude.rs
pub const BUILTIN_ATTRIBUTE_NAMES: &[&str] = &["Deprecated", "Conditional", "Singleton", "Locale"];

pub fn is_builtin_attribute_name(name: &str) -> bool {
    BUILTIN_ATTRIBUTE_NAMES.contains(&name)
}
```

In `collector.rs`, in the `AstDecl::Attribute` arm:

```rust
AstDecl::Attribute(a) => {
    // Check builtin attribute name reservation
    if is_builtin_attribute_name(&a.name) {
        diags.push(ResolutionError::BuiltinAttributeShadow {
            name: a.name.clone(),
            file: ctx.file_id,
            span: a.name_span,
        }.into());
        return; // don't insert
    }
    let vis = ast_vis_to_def_vis(a.vis.as_ref());
    try_insert(
        &a.name,
        a.name_span,
        a.span,
        DefKind::AttributeDef,
        vis,
        Vec::new(),
        ctx,
        def_map,
        diags,
    );
}
```

The "DefId origin" approach means: builtins injected by the virtual module get `FileId(u32::MAX)` — the same sentinel used for log:: and dialogue builtins. When the collector sees a user attribute with a builtin name, it emits an error because the name is reserved. The check does NOT attempt to look up whether a virtual-module DefId already exists — it simply consults the static list. This is simpler and correct because the virtual module is built *after* the compiler pipeline, not before.

**Important:** `is_builtin_attribute_name` is separate from `is_prelude_name`. The four attribute names (`Deprecated`, `Conditional`, `Singleton`, `Locale`) are not prelude types or contracts — they are attribute names. They should NOT be added to `PRELUDE_TYPE_NAMES` or `PRELUDE_CONTRACT_NAMES`. A new predicate and a new error code are required.

### Pattern 5: Type Validation of Attribute Parameters

Attribute parameters can only have types that map to `AttrValue` variants: `string`, `int`, `bool`. The type checker validates this in `check_decl.rs`:

```rust
TypedDecl::AttributeDef { def_id } => {
    // Find the AstAttributeDecl in the asts
    let entry = def_map.get_entry(*def_id);
    if let Some(attr_decl) = find_attr_decl_for_entry(asts, entry) {
        for param in &attr_decl.params {
            match &param.ty {
                AstType::Named { name, .. } if matches!(name.as_str(), "string" | "int" | "bool") => {}
                _ => {
                    // Emit error: unsupported attribute parameter type
                    diags.push(...);
                }
            }
        }
    }
}
```

In practice, since `check_decl.rs` currently handles `TypedDecl::Struct`, `TypedDecl::Fn` etc., the `AttributeDef` arm can be a no-op for Plan 94-01 and filled in during Plan 94-02.

### Pattern 6: Emitter — AttributeDef rows for user-defined attributes

`collect_post_finalize` in `emit/collect/mod.rs` currently calls `collect_attributes`. For user-defined attribute *declarations* (not usages), a new pass is needed: `collect_attribute_decl_defs`.

The existing `collect_attributes` emits `AttributeDef` rows for *usages* — e.g., `[Deprecated]` on a function. User-defined attribute *declarations* emit rows differently: the owner is a `NULL` token (or a special owner_kind) and the name + param signature is what matters.

Looking at the `AttributeDefRow` shape:
```rust
pub struct AttributeDefRow {
    pub owner: MetadataToken,  // what this attribute is applied to
    pub owner_kind: u8,        // 0=type, 1=method, 2=global
    pub name: u32,             // string heap offset
    pub value: u32,            // blob heap offset (encoded arguments)
}
```

For a user-defined attribute *declaration* (not usage), the row needs a different interpretation. The AttributeDef table currently mixes "declaration" and "application" in one table. Two sub-designs:

**Option A — Separate owner_kind for declarations:** owner = `MetadataToken::NULL`, owner_kind = 0xFF (or 3), name = attribute name, value = blob encoding the param type signature.

**Option B — New table.** Not viable for Phase 94 — would require module format changes.

**Recommendation: Option A.** Use `owner = MetadataToken::NULL` and `owner_kind = 3` (or a named constant `ATTR_OWNER_KIND_DECL = 3`) to distinguish attribute declarations from attribute applications. The param types are encoded into the blob as a type signature (matching the `encode_ast_type_into` pattern).

For the virtual module builtin rows:
```rust
// In build_writ_runtime_module(), Section 8 (new section):
// Builtin attribute declarations — owner = NULL (0), owner_kind = 3 (decl)
// Value blob encodes param types per attribute

// Deprecated(msg: string) — 1 string param
let deprecated_sig = encode_attr_param_sig(&[AttrParamType::String]);
builder.add_attribute_def(MetadataToken::NULL, 3, "Deprecated", &deprecated_sig);

// Conditional(name: string) — 1 string param
let conditional_sig = encode_attr_param_sig(&[AttrParamType::String]);
builder.add_attribute_def(MetadataToken::NULL, 3, "Conditional", &conditional_sig);

// Singleton — 0 params
builder.add_attribute_def(MetadataToken::NULL, 3, "Singleton", &[]);

// Locale(tag: string) — 1 string param
let locale_sig = encode_attr_param_sig(&[AttrParamType::String]);
builder.add_attribute_def(MetadataToken::NULL, 3, "Locale", &locale_sig);
```

Note: `writ-module`'s `ModuleBuilder::add_attribute_def` takes `&[u8]` for value, while the compiler's `emit/module_builder.rs` `add_attribute_def` takes `u32` (blob offset). The virtual module builder uses the raw `writ-module` API directly — consistent with existing pattern in `virtual_module.rs`.

### Pattern 7: Parser placement for `attribute_decl`

The `attribute_decl` parser combinator should be added inside `attrs_vis_decl`'s `choice()` list, not as a standalone branch. Reason: attribute declarations support `pub`/`priv` visibility (same as all other declarations) and may themselves carry attributes (consistent with the pattern for all other decl types).

The parser form is:
```
attribute Name(name: type, name: type, ...);
```
No body block, terminated by semicolon. Params are `name: type` pairs separated by commas.

### Anti-Patterns to Avoid

- **Do not add builtin attribute names to `PRELUDE_TYPE_NAMES` or `PRELUDE_CONTRACT_NAMES`.** Those lists drive the `PreludeShadow` error (E0002). Attribute names need their own `BuiltinAttributeShadow` error (E0008) with different wording.
- **Do not reuse `is_prelude_name` for attribute reservation.** A separate predicate in `prelude.rs` and a separate error variant keeps concerns clean.
- **Do not emit AttributeDef for `AttributeDef` *usages* in `collect_attributes` without updating the owner_kind logic.** The existing code in `collect_attributes` writes `owner_kind` as 0 (type), 1 (method), or 2 (global) for all `DefKind` variants. It must skip `DefKind::AttributeDef` to avoid writing an *application* row for an attribute *declaration*.
- **Do not assign `AttributeDef` a token via `token_for_def` without planning for it.** The emitter's `collect_exports` and `collect_attributes` both call `builder.token_for_def(def_id)`. If `DefKind::AttributeDef` entries have no registered token, these calls return `None` and the rows are silently skipped — which is correct for attribute declarations (they are not exported as types/methods/globals).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Param type encoding in blob | Custom ad-hoc format | `encode_ast_type_into` from `emit/collect/encoding.rs` | Already encodes all primitive and type tags; produces consistent blobs |
| Attribute argument blob encoding | New encoder | `encode_attr_args` from `writ-module/src/attr.rs` | Phase 93 built this; use it for any actual attribute arguments on the decl |
| Error diagnostic construction | Raw string assembly | `Diagnostic::error(code, msg).with_primary(...).build()` | Existing builder pattern in `resolve/error.rs` and `check/error.rs` |
| Builtin virtual module population | Inline bytes | `ModuleBuilder::add_attribute_def` in `virtual_module.rs` | Same pattern as contracts/types/impls |

---

## Common Pitfalls

### Pitfall 1: `collect_attributes` emitting application rows for attribute declarations

**What goes wrong:** `collect_attributes` iterates `TypedDecl` and emits `AttributeDef` rows. If `TypedDecl::AttributeDef` is added without a guard, it tries to find attributes applied *to* the attribute declaration itself (which is almost never present) and emits zero rows — fine. BUT if the code calls `builder.token_for_def(def_id)` for an `AttributeDef` def_id and there is no registered token, `token_for_def` returns `None` and the loop `continue`s. This is actually safe by accident — but should be explicit.

**How to avoid:** In `collect_attributes`, add an explicit `TypedDecl::AttributeDef { .. } => continue` arm to skip attribute declarations. Document that attribute declarations are emitted in a separate new pass, not via the usage-emission pass.

### Pitfall 2: Forgetting to handle `DefKind::AttributeDef` in `collect_exports`

**What goes wrong:** `collect_exports` in `encoding.rs` maps `DefKind` to `item_kind`. If `DefKind::AttributeDef` is not handled, it falls through to an unmatched arm — which is a compile error in Rust. Or if there is a catch-all, it may emit a garbage export row.

**How to avoid:** Add an explicit `DefKind::AttributeDef => continue` arm in `collect_exports` (attribute declarations are not exported as types/methods/globals).

### Pitfall 3: Parser trying to match `Token::Ident("attribute")` vs `Token::KwAttribute`

**What goes wrong:** If `KwAttribute` is added to the lexer but the parser still tries `just(Token::Ident("attribute"))`, nothing matches. Or vice versa.

**How to avoid:** Add `KwAttribute` to both the `Token` enum in `lexer.rs` AND use `just(Token::KwAttribute)` in the parser. Keep them in sync.

### Pitfall 4: `owner_kind = 3` for attribute declarations conflicts with future use

**What goes wrong:** If Phase 98 (runtime query API) assumes `owner_kind` values 0/1/2 are exhaustive, a value of 3 breaks deserialization.

**How to avoid:** Define a named constant `ATTR_OWNER_KIND_DECL: u8 = 3` in `writ-module` (or in the compiler's emit layer) and use it consistently. Document the new value in the `AttributeDefRow` comments.

### Pitfall 5: Virtual module attribute rows use wrong `MetadataToken::NULL`

**What goes wrong:** `MetadataToken::NULL` may not be defined or may be `MetadataToken::new(0, 0)`. The `writ-module` `ModuleBuilder::add_attribute_def` writes `owner` as a raw `MetadataToken`. If NULL is not the right sentinel, the binary format reader may misinterpret the row.

**How to avoid:** Verify that `MetadataToken::NULL` is defined in `writ-module/src/token.rs` before using it. If not, use `MetadataToken::new(0, 0)` explicitly.

### Pitfall 6: Attribute parameter types validated too late (check vs. collect)

**What goes wrong:** If param type validation is deferred to the emitter pass (collect), a badly typed attribute declaration emits no blob but produces no error — silent truncation.

**How to avoid:** Validate in the type checker (`check_decl.rs`) — same phase where struct field types and fn return types are validated. The type checker already has access to the AST and DefMap.

---

## Code Examples

### Example 1: Existing declaration addition pattern (Class)

The `class` keyword was added following this exact chain. It introduced:
- `Token::KwClass` in lexer
- `ClassDecl<'src>` + `Item::Class` in cst
- `class_decl` combinator in program.rs (at line ~2643 area)
- `AstClassDecl` + `AstDecl::Class` in ast/decl.rs
- `Item::Class` arm in `lower()` in lower/mod.rs
- `DefKind::Class` in def_map.rs
- `AstDecl::Class` arm in `collect_items()` in collector.rs
- `ResolvedDecl::Class` in resolve/ir.rs
- `TypedDecl::Class` in check/ir.rs
- `TypedDecl::Class` arm in `collect_defs()` in emit/collect/mod.rs

`attribute` follows the identical chain.

### Example 2: Builtin name reservation — prelude shadow (existing pattern)

```rust
// In collector.rs, try_insert():
if is_prelude_name(name) {
    diags.push(ResolutionError::PreludeShadow {
        name: name.to_string(),
        file: ctx.file_id,
        span: name_span,
    }.into());
    return;
}
```

The new check mirrors this:
```rust
// Only in the AstDecl::Attribute arm:
if is_builtin_attribute_name(&a.name) {
    diags.push(ResolutionError::BuiltinAttributeShadow {
        name: a.name.clone(),
        file: ctx.file_id,
        span: a.name_span,
    }.into());
    return;
}
```

### Example 3: How `add_attribute_def` is called in the compiler's ModuleBuilder

```rust
// From emit/module_builder.rs:
pub fn add_attribute_def(
    &mut self,
    owner: MetadataToken,
    owner_kind: u8,
    name: &str,
    value: u32,      // blob heap offset, NOT raw bytes
) -> usize
```

Note: the compiler's `ModuleBuilder` takes a `u32` blob offset (pre-interned), whereas `writ-module::ModuleBuilder` takes `&[u8]`. The virtual module uses `writ-module` directly. The compiler's emitter uses the compiler's own `ModuleBuilder` (in `emit/module_builder.rs`).

### Example 4: Encoding param types for a user-defined attribute declaration

```rust
// In a new collect_attribute_decl_defs() function:
fn collect_attribute_decl_defs(typed_ast: &TypedAst, asts: &[(FileId, &Ast)], builder: &mut ModuleBuilder) {
    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::AttributeDef { def_id } => *def_id,
            _ => continue,
        };
        let entry = def_map.get_entry(def_id);
        let attr_decl = match find_attr_decl_for_entry(asts, entry) {
            Some(d) => d,
            None => continue,
        };

        // Encode param types as a blob
        let mut sig_buf = Vec::new();
        sig_buf.extend_from_slice(&(attr_decl.params.len() as u16).to_le_bytes());
        for param in &attr_decl.params {
            encode_ast_type_into(&param.ty, &[], &mut sig_buf);
        }
        let blob_offset = if sig_buf.is_empty() { 0 } else { builder.blob_heap.intern(&sig_buf) };

        builder.add_attribute_def(
            MetadataToken::NULL,
            ATTR_OWNER_KIND_DECL,   // 3
            &entry.name,
            blob_offset,
        );
    }
}
```

---

## State of the Art

| Old State | Phase 94 State | Impact |
|-----------|----------------|--------|
| Only builtin attributes (`[Deprecated]`, `[Conditional]`, `[Singleton]`, `[Locale]`) are recognized | User can define `attribute Quest(name: string, level: int);` | Extensible metadata system |
| AttributeDef table only holds application rows | AttributeDef table holds both declaration rows (owner_kind=3) and application rows (owner_kind=0/1/2) | Phase 98 query API can enumerate declared attribute types |
| Builtin attribute names silently applied with no declaration visible in binary | Builtins registered in virtual module as declaration rows | Host can enumerate all available attribute types |
| `DefKind` has 12 variants | `DefKind` has 13 variants (adds `AttributeDef`) | Collector, resolver, type checker, emitter all need updates |

---

## Open Questions

1. **`owner_kind = 3` for attribute declarations — naming constant location**
   - What we know: The compiler emitter and virtual module builder both write `AttributeDefRow::owner_kind`. Currently 0/1/2 are used.
   - What's unclear: Should the constant `ATTR_OWNER_KIND_DECL = 3` live in `writ-module/src/tables.rs` or in the compiler's `emit/collect/encoding.rs`?
   - Recommendation: Put it in `writ-module/src/tables.rs` alongside `AttributeDefRow` — both the compiler and the virtual module builder can access it.

2. **Does `attribute` declaration accept attributes on itself (e.g., `[Deprecated] attribute Quest(...);`)?**
   - What we know: All other declarations support attrs via the `attrs_vis_decl` choice pattern.
   - What's unclear: Whether attribute-on-attribute makes semantic sense for Phase 94.
   - Recommendation: Support it structurally (include `attrs: Vec<AstAttribute>` in `AstAttributeDecl`) for consistency — semantic validation deferred. The pipeline already handles this pattern.

3. **Does `AstAttributeDecl` need to appear in `collect_attributes` for its *own* applied attributes?**
   - What we know: `collect_attributes` iterates all `TypedDecl` variants and calls `find_attrs_for_entry`. If `TypedDecl::AttributeDef` is included, any attributes on the declaration itself will be emitted as `AttributeDef` rows with a `NULL` owner token.
   - Recommendation: Skip `TypedDecl::AttributeDef` in `collect_attributes` for Phase 94. Attributes on attribute declarations are not a required feature and create circular/confusing rows.

---

## Environment Availability

Step 2.6: SKIPPED — this is a pure code change phase. No external tools, services, runtimes, databases, or CLI utilities beyond the project's own Rust compiler are needed. Verified: this phase adds no new crate dependencies.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | Cargo.toml workspace, no separate test config |
| Quick run command | `cargo test -p writ-compiler resolve_tests -- --test-thread=1` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UATTR-01 | `attribute Quest(name: string, level: int);` parses, lowers, collects into DefMap as `DefKind::AttributeDef` | unit | `cargo test -p writ-compiler resolve_tests::attribute_decl_collected` | ❌ Wave 0 |
| UATTR-02 | AttributeDef row appears in binary module with encoded param types | unit | `cargo test -p writ-compiler emit_body_tests::attribute_decl_emits_def_row` | ❌ Wave 0 |
| UATTR-02 | Arg type mismatch on attribute usage produces TypeError | unit | `cargo test -p writ-compiler resolve_tests::attribute_arg_type_mismatch` | ❌ Wave 0 |
| UATTR-03 | Virtual module contains `Deprecated`, `Conditional`, `Singleton`, `Locale` AttributeDef rows | unit | `cargo test -p writ-runtime virtual_module::builtin_attribute_defs_present` | ❌ Wave 0 |
| UATTR-04 | `attribute Deprecated(msg: string);` produces E0008 | unit | `cargo test -p writ-compiler resolve_tests::builtin_attribute_shadow` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler resolve_tests` and `cargo test -p writ-parser`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-compiler/tests/resolve_tests.rs` — add `attribute_decl_collected`, `builtin_attribute_shadow`, `attribute_arg_type_mismatch` test functions
- [ ] `writ-compiler/tests/emit_body_tests.rs` — add `attribute_decl_emits_def_row` test function
- [ ] `writ-runtime/src/virtual_module.rs` (test module) — add `builtin_attribute_defs_present` test

---

## Sources

### Primary (HIGH confidence)
- Direct codebase read: `writ-compiler/src/ast/decl.rs` — AstDecl enum, all 13 variants, AstParam, AstAttribute shapes
- Direct codebase read: `writ-compiler/src/resolve/def_map.rs` — DefKind enum, DefMap insertion logic, DefEntry structure
- Direct codebase read: `writ-compiler/src/resolve/collector.rs` — collect_items pattern, try_insert, prelude shadow check
- Direct codebase read: `writ-compiler/src/resolve/prelude.rs` — is_prelude_name, PRELUDE_TYPE_NAMES, PRELUDE_CONTRACT_NAMES
- Direct codebase read: `writ-compiler/src/resolve/ir.rs` — ResolvedDecl variants
- Direct codebase read: `writ-compiler/src/check/ir.rs` — TypedDecl variants, TypedAst
- Direct codebase read: `writ-compiler/src/emit/collect/mod.rs` — collect_defs dispatch table, collect_post_finalize
- Direct codebase read: `writ-compiler/src/emit/collect/encoding.rs` — collect_attributes, encode_ast_type_into, encode_attr_args usage
- Direct codebase read: `writ-compiler/src/lower/mod.rs` — lower() dispatch, all Item arms
- Direct codebase read: `writ-parser/src/lexer.rs` — Token enum, all KwXxx variants
- Direct codebase read: `writ-parser/src/cst.rs` — Item enum, Attribute, Param, Visibility
- Direct codebase read: `writ-parser/src/parser/program.rs` — attrs_vis_decl choice, extern_item, top-level item choice
- Direct codebase read: `writ-runtime/src/virtual_module.rs` — build_writ_runtime_module() structure, add_contract_def/add_type_def patterns
- Direct codebase read: `writ-module/src/attr.rs` — AttrValue, ATTR_TAG_*, encode_attr_args, decode_attr_args
- Direct codebase read: `writ-module/src/tables.rs` — AttributeDefRow structure (owner, owner_kind, name, value)
- Direct codebase read: `writ-module/src/builder.rs` — add_attribute_def signature (&[u8] for value)
- Direct codebase read: `writ-compiler/src/emit/module_builder.rs` — add_attribute_def signature (u32 blob offset)
- Direct codebase read: `writ-diagnostics/src/code.rs` — existing error codes E0001–E0124, W0001–W0005
- Direct codebase read: `writ-compiler/src/resolve/error.rs` — ResolutionError variants, Diagnostic builder pattern
- Direct codebase read: `.planning/STATE.md` — "Builtin name reservation happens in the collector pass using DefId origin"

### Secondary (MEDIUM confidence)
- CONTEXT.md analysis: key considerations and blocked research gaps resolved by reading codebase directly
- REQUIREMENTS.md: UATTR-01 through UATTR-04 text and traceability table

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all layers are existing Rust code read directly
- Architecture patterns: HIGH — every pattern is derived from reading the actual source of analogous declarations (Class, Extern, Const)
- Pitfalls: HIGH — derived from direct reading of collect_attributes, collect_exports, token_for_def usage patterns
- Builtin name reservation design: HIGH — STATE.md is explicit; prelude.rs pattern is clear

**Research date:** 2026-03-27
**Valid until:** Stable — pure compiler internals, no external dependencies

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in this project. No project-specific constraints to enforce beyond what is captured in CONTEXT.md and STATE.md.
