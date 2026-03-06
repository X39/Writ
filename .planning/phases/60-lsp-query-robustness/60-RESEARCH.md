# Phase 60: LSP Query Robustness - Research

**Researched:** 2026-03-17
**Domain:** Rust LSP server — typed AST query walking, declaration-site coverage for hover/goto-def/find-refs
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LSP-04 | User can hover any identifier to see its type, signature, or definition info | Requires `binding_at_offset` fallback for let-binding names and fn param names (both exist in TypedStmt, not TypedExpr) |
| LSP-05 | User can go-to-definition on any identifier to jump to its declaration | Requires type-annotation span → DefId side-table so hover on `let x: MyStruct` routes to `MyStruct`'s definition |
| LSP-06 | User can find all references of a definition across all files | Requires `def_at_offset` scanning DefMap.arena for entries whose `name_span` contains the cursor offset, so declaration names resolve to a DefId |
</phase_requirements>

---

## Summary

Phase 54 shipped hover, goto-definition, and find-references handlers that rely entirely on `expr_at_offset` — a walker that traverses `TypedExpr` nodes in function bodies. The UAT (tests 7, 8, 9) revealed three distinct gaps:

**Gap 1 (LSP-04 hover):** Variable declaration names (`let x: int = 42`) and function parameter names (`fn foo(x: int)`) are not `TypedExpr` nodes. They live in `TypedStmt::Let { name_span, ty }` and in `FnSig::params` — but `FnSig` stores `(String, Ty)`, discarding the `AstParam.name_span`. When the cursor is on a declaration name, `expr_at_offset` returns `None` or falls through to the enclosing `Block` whose type is `void`. The hover shows nothing useful.

**Gap 2 (LSP-05 goto-def on type annotations):** When the user writes `let x: MyStruct = ...`, the type annotation `MyStruct` has a span in the AST (`AstStmt::Let { ty: Some(AstType::Named { name, span }) }`) but this span is never preserved in the `TypedStmt::Let`. The checker calls `resolve_ast_type(annotation, ...)` to get a `Ty` but throws away the annotation's span and any `DefId` it resolves to. `find_def_id_at_offset` therefore cannot identify the type reference.

**Gap 3 (LSP-06 find-refs from declaration site):** `DefMap.arena` stores every definition with a `name_span`. When the cursor is on a declaration name (e.g., `fn foo(`) the `name_span` of the `DefEntry` for `foo` contains that position, but `expr_at_offset` never consults `DefMap.arena`. The references handler has no fallback: it calls `expr_at_offset` first, and if that returns `None` (which it does for declaration names), the whole handler returns `None`.

**Primary recommendation:** Three focused additions to `writ-lsp/src/queries.rs` plus one small extension to `TypedStmt::Let` in `writ-compiler/src/check/ir.rs`. No new analysis passes, no new compiler crates.

---

## Standard Stack

No new dependencies required. All changes are within existing crates.

| Crate | Role | Change needed |
|-------|------|---------------|
| `writ-lsp` | LSP handlers and queries | New query functions in `queries.rs`; `backend.rs` hover/goto-def/refs fallbacks |
| `writ-compiler` | Typed IR and type checker | Add `type_ann_span` + `type_ann_def_id` fields to `TypedStmt::Let`; populate in `check_stmt.rs` |

---

## Architecture Patterns

### Current Query Call Chain

```
backend.rs hover()
  -> position_to_byte_offset()         // Position -> byte offset
  -> expr_at_offset(typed_ast, offset) // TypedExpr walker
  -> hover_text_for_expr(expr, ...)    // Format result
  // GAP: returns void if cursor is on a declaration name or binding
```

```
backend.rs goto_definition()
  -> expr_at_offset(typed_ast, offset) // TypedExpr walker
  -> find_def_id_at_offset(expr, ...)  // Extract DefId from expression
  -> def_map.get_entry(def_id)         // Lookup location
  // GAP: no path when cursor is on a type annotation span
```

```
backend.rs references()
  -> expr_at_offset(typed_ast, offset) // TypedExpr walker
  -> find_def_id_at_offset(expr, ...)  // Extract DefId
  -> collect_references(ast, def_id, ...) // Collect use sites
  // GAP: returns None when cursor is on a declaration name
```

### Three-Fix Architecture

**Fix 1: `binding_at_offset` in `queries.rs`**

A new function that walks all `TypedDecl::Fn` bodies and `TypedDecl::Impl` methods looking for the innermost `TypedStmt` that has a binding (let-name or for-binding) whose span contains the offset. Returns a `(Ty, String)` (type + binding name). Also checks fn parameter names stored in `FnSig::params` — but since `FnSig` currently stores `(String, Ty)` without spans, param spans must come from elsewhere (see below).

**Fix 2: Type annotation side-table via `TypedStmt::Let` field extension**

The cleanest approach is to add two optional fields to `TypedStmt::Let`:

```rust
TypedStmt::Let {
    name: String,
    name_span: SimpleSpan,
    ty: Ty,
    mutable: bool,
    value: TypedExpr,
    span: SimpleSpan,
    // NEW:
    type_ann_span: Option<SimpleSpan>,   // span of the type annotation token(s)
    type_ann_def_id: Option<DefId>,      // the DefId the annotation resolves to (if named type)
}
```

`check_stmt.rs` already has `annotation: &AstType` in scope. `resolve_ast_type` returns a `Ty`, but the `DefId` can be recovered with `def_map.get(name)` for `AstType::Named`. This is a compiler IR change but it is **additive only** — all existing matches against `TypedStmt::Let` use `..` destructuring and will not need updating.

Alternative (no IR change): build a `HashMap<SimpleSpan, DefId>` side-table alongside the typed AST in the analysis result. The UAT diagnosis document (`goto-def-references-gaps.md`) mentions both approaches. The side-table approach avoids touching the IR but requires threading extra state through the analysis pipeline.

**Fix 3: `def_at_offset` in `queries.rs`**

A new function that iterates `def_map.arena` (an `id_arena::Arena<DefEntry>`) and finds entries whose `name_span` contains the offset. Returns the `DefId`. This function only looks at top-level names (functions, structs, entities, enums, etc.) — not local variables.

```rust
pub fn def_at_offset(def_map: &DefMap, offset: usize) -> Option<DefId> {
    for (id, entry) in &def_map.arena {
        let span = entry.name_span;
        if offset >= span.start && offset < span.end {
            return Some(id);
        }
    }
    None
}
```

`id_arena::Arena<T>` implements `IntoIterator` yielding `(Id<T>, &T)` pairs — this iteration is O(N) over all definitions, which is acceptable for interactive use.

### Updated Call Chains After Fixes

**Hover (LSP-04):**
```
hover()
  -> expr_at_offset()             // existing: expression nodes
  -> if None or void type:
       binding_at_offset()        // NEW: let-name, for-binding, fn params
  -> format hover text from (name, ty) or expr
```

**Goto-definition (LSP-05):**
```
goto_definition()
  -> expr_at_offset()             // existing
  -> find_def_id_at_offset()      // existing
  -> if None:
       type_ann_def_id_at_offset() // NEW: checks TypedStmt.type_ann_span
  -> if None:
       def_at_offset(def_map, offset) // NEW: declaration names
```

**Find-references (LSP-06):**
```
references()
  -> expr_at_offset()             // existing
  -> find_def_id_at_offset()      // existing
  -> if None:
       def_at_offset(def_map, offset) // NEW: declaration names
  -> collect_references()
```

### Recommended Project Structure (no changes needed)

The existing file layout handles all three fixes:

```
writ-compiler/src/check/
  ir.rs          -- TypedStmt::Let gets 2 new optional fields
  check_stmt.rs  -- populate type_ann_span / type_ann_def_id for Let
writ-lsp/src/
  queries.rs     -- 3 new pub functions + updated hover text helper
  backend.rs     -- updated hover/goto-def/refs handlers (fallback chains)
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Arena iteration | Custom vec scan | `id_arena::Arena` IntoIterator | Already the established pattern; Arena<T> yields (Id<T>, &T) |
| Type annotation resolution | Duplicate `resolve_ast_type` logic | Existing `resolve_ast_type` in `env.rs` + `def_map.get(name)` | `resolve_ast_type` is already called in check_stmt; just capture the DefId alongside |
| Span containment check | Complex range logic | `offset >= span.start && offset < span.end` | SimpleSpan is plain struct; no API needed |
| FnSig param spans | Storing spans in FnSig | Walk `AstFnDecl.params` at hover time or add to TypedDecl::Fn | FnSig is shared across crates; adding spans there has wider blast radius |

**Key insight:** All three gaps are pure query-layer problems. The typed AST already has the right data (spans, types) — it is just not being queried at the right nodes. No new compiler passes, no new type-checking logic.

---

## Common Pitfalls

### Pitfall 1: FnSig Does Not Store Parameter Spans
**What goes wrong:** `FnSig::params` stores `Vec<(String, Ty)>` — the name and type, but NOT the `name_span` from `AstParam`. When hovering a function parameter name (e.g., hovering `x` in `fn foo(x: int)`), there is no way to match the cursor offset to a parameter without the span.
**Why it happens:** FnSig was designed for type checking, not LSP; spans were not needed.
**How to avoid:** Two options:
1. Add `param_spans: Vec<SimpleSpan>` to `FnSig` (parallel to `params`). Requires env.rs + all FnSig consumers to be updated.
2. Walk `AstFnDecl.params` directly at hover time — but the AST is not stored in `AnalysisResult`, only the TypedAst.
3. Add a separate `fn_param_spans: HashMap<DefId, Vec<SimpleSpan>>` to `TypeEnv` or `AnalysisResult`.
**Recommendation:** Extend `TypedDecl::Fn` to store param spans alongside the body, matching the existing `check_fn_decl` flow which already has `fn_decl.params` in scope.
**Warning signs:** Hover on param name returns void or no tooltip; tests pass for hover on `let x` but not for `fn foo(x: int)`.

### Pitfall 2: Arena Iteration Includes Synthetic Builtins
**What goes wrong:** `def_map.arena` contains synthetic entries for `log::trace`, `say`, etc. with `file_id == FileId(u32::MAX)`. If `def_at_offset` returns one of these, `goto_definition` will return `None` (the backend already filters `FileId(u32::MAX)`), but find-references might still try to use it.
**How to avoid:** Filter `entry.file_id == FileId(u32::MAX)` in `def_at_offset`, or let callers handle it (they already do for goto-def). Both work; filtering at the source is cleaner.

### Pitfall 3: Span Overlap Between Annotations and Expressions
**What goes wrong:** For `let x: MyStruct = MyStruct {}`, both the type annotation `MyStruct` and the `new MyStruct {}` expression can have spans that contain the cursor. If the cursor is exactly on the type annotation, `expr_at_offset` might return the `New` expression (which already has the correct DefId), making the annotation side-table redundant for this case. But for `let x: MyStruct = some_fn()`, the `New` expression is not present, so the annotation side-table is the only path.
**How to avoid:** Run `expr_at_offset` first; only fall through to the annotation side-table if `find_def_id_at_offset` returns `None`.

### Pitfall 4: `type_ann_def_id` Only Resolves Named Types
**What goes wrong:** `AstType::Generic`, `AstType::Array`, `AstType::Func` don't have a single `DefId` — they compose multiple types. Attempting to resolve a DefId for `Option<MyStruct>` is ambiguous (is it `Option` or `MyStruct`?).
**How to avoid:** Only populate `type_ann_def_id` for `AstType::Named { name, span }` (direct named types). For complex types, `type_ann_def_id` stays `None`. The hover handler can still show the resolved `Ty` using the interner.
**Warning signs:** Go-to-def on `Option<MyStruct>` doesn't jump anywhere — this is expected and acceptable behavior for generic annotations.

### Pitfall 5: Missing `..` in TypedStmt::Let match arms
**What goes wrong:** Adding fields to `TypedStmt::Let` will break any match arm that binds all fields by name. In `queries.rs`, the `find_in_stmt` and `collect_refs_in_stmt` functions both match `TypedStmt::Let { value, .. }` using `..` — these are safe. But if any code binds the full destructure without `..`, it will fail to compile.
**How to avoid:** `grep` for `TypedStmt::Let {` before the field addition to confirm all match arms use `..`. From reading the code, `check_stmt.rs`, `queries.rs`, and any emitter code all use partial destructuring — verified safe.

---

## Code Examples

### Example: `def_at_offset` (Fix 3)
```rust
// Source: writ-lsp/src/queries.rs (new function)
/// Find the DefId of a top-level definition whose name span contains `offset`.
///
/// Used as a fallback in goto-definition and find-references when `expr_at_offset`
/// returns nothing (cursor is on a declaration name, not a use site).
pub fn def_at_offset(def_map: &DefMap, offset: usize) -> Option<DefId> {
    for (id, entry) in &def_map.arena {
        // Skip synthetic builtins (log::*, dialogue builtins)
        if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
            continue;
        }
        let span = entry.name_span;
        if offset >= span.start && offset < span.end {
            return Some(id);
        }
    }
    None
}
```

### Example: `binding_at_offset` (Fix 1, partial — let-bindings)
```rust
// Source: writ-lsp/src/queries.rs (new function)
/// A binding found at a cursor offset: a local variable or loop binding name.
pub struct BindingInfo {
    pub name: String,
    pub ty: Ty,
    pub name_span: SimpleSpan,
}

/// Find a local binding (let name, for binding) whose name_span contains `offset`.
///
/// Used as a fallback in hover when `expr_at_offset` returns a void block or None.
pub fn binding_at_offset(ast: &TypedAst, offset: usize) -> Option<BindingInfo> {
    for decl in &ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(b) = find_binding_in_expr(body, offset) {
                    return Some(b);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(b) = find_binding_in_expr(body, offset) {
                        return Some(b);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn find_binding_in_expr(expr: &TypedExpr, offset: usize) -> Option<BindingInfo> {
    match expr {
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                if let Some(b) = find_binding_in_stmt(stmt, offset) {
                    return Some(b);
                }
            }
            tail.as_ref().and_then(|t| find_binding_in_expr(t, offset))
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            find_binding_in_expr(condition, offset)
                .or_else(|| find_binding_in_expr(then_branch, offset))
                .or_else(|| else_branch.as_ref().and_then(|e| find_binding_in_expr(e, offset)))
        }
        TypedExpr::Lambda { body, .. } => find_binding_in_expr(body, offset),
        TypedExpr::Match { scrutinee, arms, .. } => {
            find_binding_in_expr(scrutinee, offset)
                .or_else(|| arms.iter().find_map(|arm| find_binding_in_expr(&arm.body, offset)))
        }
        _ => None,
    }
}

fn find_binding_in_stmt(stmt: &TypedStmt, offset: usize) -> Option<BindingInfo> {
    match stmt {
        TypedStmt::Let { name, name_span, ty, .. } => {
            if offset >= name_span.start && offset < name_span.end {
                return Some(BindingInfo { name: name.clone(), ty: *ty, name_span: *name_span });
            }
            None
        }
        TypedStmt::For { binding, binding_span, binding_ty, .. } => {
            if offset >= binding_span.start && offset < binding_span.end {
                return Some(BindingInfo {
                    name: binding.clone(),
                    ty: *binding_ty,
                    name_span: *binding_span,
                });
            }
            None
        }
        TypedStmt::While { condition, body, .. } => {
            find_binding_in_expr(condition, offset)
                .or_else(|| body.iter().find_map(|s| find_binding_in_stmt(s, offset)))
        }
        TypedStmt::Atomic { body, .. } => {
            body.iter().find_map(|s| find_binding_in_stmt(s, offset))
        }
        TypedStmt::Expr { expr, .. } => find_binding_in_expr(expr, offset),
        _ => None,
    }
}
```

### Example: TypedStmt::Let extension (Fix 2)
```rust
// Source: writ-compiler/src/check/ir.rs — TypedStmt::Let variant
TypedStmt::Let {
    name: String,
    name_span: SimpleSpan,
    ty: Ty,
    mutable: bool,
    value: TypedExpr,
    span: SimpleSpan,
    // NEW: type annotation tracking for goto-def on type names
    type_ann_span: Option<SimpleSpan>,
    type_ann_def_id: Option<DefId>,
},
```

```rust
// Source: writ-compiler/src/check/check_stmt.rs — check_stmt Let arm
// After `resolve_ast_type(annotation, ...)`:
let (ann_span, ann_def_id) = if let Some(ref annotation) = ty {
    let ann_span = Some(annotation.span());  // AstType has a span() method (check types.rs)
    let ann_def_id = match annotation {
        AstType::Named { name, .. } => def_map.get(name),
        _ => None,
    };
    (ann_span, ann_def_id)
} else {
    (None, None)
};

TypedStmt::Let {
    name: name.clone(),
    name_span: *name_span,
    ty: final_ty,
    mutable: *mutable,
    value: typed_value,
    span: *span,
    type_ann_span: ann_span,
    type_ann_def_id: ann_def_id,
}
```

Note: `AstType` variants all carry a span but there is no `.span()` method. The span is accessed by destructuring: `AstType::Named { span, .. }`, `AstType::Generic { span, .. }`, etc. A small helper `ast_type_span(ty: &AstType) -> SimpleSpan` is needed, or inline matching.

### Example: Backend fallback for hover (Fix 1 integration)
```rust
// Source: writ-lsp/src/backend.rs — hover handler
let expr = crate::queries::expr_at_offset(typed_ast, byte_offset);

// Check if the expression has a useful type (not void)
let is_void_or_missing = expr.map_or(true, |e| {
    matches!(e.ty(), ty if interner.is_void(ty))
});

if is_void_or_missing {
    // Fallback: check if cursor is on a binding name
    if let Some(binding) = crate::queries::binding_at_offset(typed_ast, byte_offset) {
        let ty_str = interner.display_named(binding.ty, &typed_ast.def_map);
        let hover_text = format!("```writ\n{}: {}\n```", binding.name, ty_str);
        return Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text,
            }),
            range: Some(crate::convert::span_to_range(&source, &binding.name_span)),
        }));
    }
    return Ok(None);
}
```

### Example: Backend fallback for references (Fix 3 integration)
```rust
// Source: writ-lsp/src/backend.rs — references handler
let def_id = crate::queries::find_def_id_at_offset(expr, &typed_ast.def_map)
    .or_else(|| {
        // Fallback: cursor is on a declaration name
        crate::queries::def_at_offset(&typed_ast.def_map, byte_offset)
    });
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| expr_at_offset only | expr_at_offset + binding/def fallbacks | Phase 60 | Hover/goto-def/refs work on declaration names |
| FnSig without spans | FnSig + param spans in TypedDecl::Fn | Phase 60 (if fn param hover addressed) | Hover on fn param shows correct type |

**Note:** Fn parameter hover is a secondary success criterion not in the UAT test list (UAT test 7 says "variable name in a let binding or function parameter"). The plan should address it but it is slightly more complex than the three primary gaps.

---

## Open Questions

1. **Where to store fn parameter name spans**
   - What we know: `FnSig::params` has `(String, Ty)` but no span. `AstFnDecl.params` has `AstParam { name_span }`. The AST is not preserved in `AnalysisResult`.
   - What's unclear: Whether to extend `FnSig` (touching `check/env.rs` and all sig construction), add to `TypedDecl::Fn` (touches just the typed IR and `check_decl.rs`), or add a separate side-table in `TypeEnv`.
   - Recommendation: Add `param_name_spans: Vec<SimpleSpan>` to `TypedDecl::Fn`. This is the most localized change — only `check_decl.rs` needs to populate it, and `queries.rs` can read it during `binding_at_offset`. This avoids modifying `FnSig` which is used by many callers.

2. **`AstType::span()` helper**
   - What we know: `AstType` has no `.span()` method; spans are in each variant.
   - What's unclear: Whether to add a `.span()` method to `AstType` or inline the match in `check_stmt.rs`.
   - Recommendation: Add a small `pub fn span(&self) -> SimpleSpan` to `impl AstType` in `types.rs`. Clean, reusable, zero risk.

3. **Goto-def on type annotations in fn params and struct fields**
   - What we know: UAT test 8 specifically mentions "Works on new X but not on variable types" — the primary gap is `let x: MyType`. Struct field type annotations and fn param type annotations have the same gap but are not mentioned in UAT.
   - What's unclear: Whether Phase 60 should address all type annotation sites or just `let`.
   - Recommendation: Address `let`-binding type annotations first (closes UAT test 8). Note fn param type annotations as a stretch goal — the same approach applies but requires the fn-param span infrastructure (Open Question 1).

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[test]` (cargo test) |
| Config file | none — standard Cargo workspace |
| Quick run command | `cargo test -p writ-lsp --lib 2>&1 \| tail -20` |
| Full suite command | `cargo test -p writ-lsp -p writ-compiler 2>&1 \| tail -30` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-04 | Hover on let-binding name returns correct type | unit | `cargo test -p writ-lsp binding_at_offset --lib -- --nocapture` | ❌ Wave 0 |
| LSP-04 | Hover on fn param name returns correct type | unit | `cargo test -p writ-lsp binding_at_offset_fn_param --lib -- --nocapture` | ❌ Wave 0 |
| LSP-05 | Goto-def on type annotation returns correct DefId | unit | `cargo test -p writ-lsp type_ann_def_id_at_offset --lib -- --nocapture` | ❌ Wave 0 |
| LSP-06 | Find-refs from declaration site returns all refs | unit | `cargo test -p writ-lsp def_at_offset_declaration --lib -- --nocapture` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp --lib 2>&1 | tail -20`
- **Per wave merge:** `cargo test -p writ-lsp -p writ-compiler 2>&1 | tail -30`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Tests for `binding_at_offset` — covers LSP-04 (let binding hover)
- [ ] Tests for `binding_at_offset` with fn params — covers LSP-04 (param hover)
- [ ] Tests for `type_ann_def_id_at_offset` (or `TypedStmt::Let.type_ann_def_id` inspection) — covers LSP-05
- [ ] Tests for `def_at_offset` — covers LSP-06 (find-refs from declaration site)
- [ ] Integration test: full hover pipeline on `let x: int = 42` returns `"x: int"`
- [ ] Integration test: goto-def on `MyStruct` in `let x: MyStruct = ...` jumps to struct def

---

## Sources

### Primary (HIGH confidence)
- Direct code reading: `writ-lsp/src/queries.rs` — complete source of existing query functions
- Direct code reading: `writ-compiler/src/check/ir.rs` — TypedStmt::Let has `name_span` and `binding_span` but no `type_ann_span`
- Direct code reading: `writ-compiler/src/check/check_stmt.rs` — AstStmt::Let destructures `ty: Option<AstType>` in scope
- Direct code reading: `writ-lsp/src/backend.rs` — hover/goto-def/refs handler chains confirmed
- Direct code reading: `.planning/phases/v5.0-milestone-uat/v5.0-UAT.md` — exact root causes and missing items verified from UAT diagnosis

### Secondary (MEDIUM confidence)
- `id_arena` crate: `Arena<T>` implements `IntoIterator<Item = (Id<T>, &T)>` — confirmed by pattern usage in existing codebase (arena.alloc, arena[id] indexing present throughout def_map.rs)

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Gap diagnosis: HIGH — root causes read directly from UAT analysis in v5.0-UAT.md, cross-confirmed with source code
- Fix design: HIGH — all data structures read directly from source; no inference required
- IR change blast radius: HIGH — verified all `TypedStmt::Let` match arms use `..`; change is additive
- fn param spans approach: MEDIUM — two viable paths; `TypedDecl::Fn` extension is recommended but not yet validated against all consumers

**Research date:** 2026-03-17
**Valid until:** 2026-06-17 (stable domain — Rust LSP, no external dependencies changing)
