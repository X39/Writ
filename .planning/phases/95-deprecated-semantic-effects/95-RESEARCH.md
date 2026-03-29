# Phase 95: [Deprecated] Semantic Effects - Research

**Researched:** 2026-03-27
**Domain:** Writ compiler type checker + LSP — attribute-driven warnings and hover integration
**Confidence:** HIGH

## Summary

Phase 95 adds semantic effects for the `[Deprecated("msg")]` builtin attribute. The attribute already passes through the full pipeline (parser -> lowering -> AST -> resolver -> emitter) as of Phase 94. What's missing is the checker side: a `deprecated_items` map in `TypeEnv` (built in `env_build`) that the check_expr layer queries to emit W0006 at call sites, and an LSP layer that reflects those warnings as squiggles and decorates hover tooltips with the deprecation message.

The entire implementation is additive — no existing pipeline stages need structural changes. The two natural seams are (1) `TypeEnv::build` in `env.rs` where new per-DefId metadata maps are routinely added (precedent: `const_types`, `global_types`), and (2) the already-identified injection points in `check_expr/call.rs` where `check_call_with_sig` and `resolve_overloaded_call` both have the callee `DefId` in hand.

The self-deprecation suppression rule is implemented by comparing `ctx.current_file` (the file being checked) with `DefEntry.file_id` for the deprecated item. The project does not currently use module identity (module name) for cross-module tracking; file identity is the correct and sufficient comparator given the single-module-per-compilation model.

**Primary recommendation:** Build `deprecated_items: FxHashMap<DefId, String>` in `TypeEnv`, populate it during `env_build` by scanning AST attributes, then emit W0006 in `check_call_with_sig` / `check_ident` at reference sites where `DefEntry.file_id != ctx.current_file`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices. Key design notes supplied:
- W0006 warning code for deprecated item references
- Self-deprecation suppression: no warning when call site is in same module as deprecated item
- LSP must show DiagnosticSeverity::Warning squiggle AND hover tooltip with deprecation message

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DEPR-01 | Referencing a `[Deprecated("msg")]` item produces a compiler warning containing the user's message string | `TypeEnv` gains `deprecated_items: FxHashMap<DefId, String>`; `check_call_with_sig` and `check_ident` query it; `Diagnostic::warning(code::W0006, ...)` emitted |
| DEPR-02 | LSP surfaces `[Deprecated]` as `DiagnosticSeverity::Warning` and shows the deprecation message on hover | Existing `writ_diag_to_lsp` already maps `Severity::Warning` to `DiagnosticSeverity::WARNING` (confirmed); hover needs augmented text prepended to the existing `hover_text_for_expr` / `hover_text_for_def` output |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ_diagnostics::Diagnostic::warning` | (in-crate) | Emit W0006 | Established builder API; `Severity::Warning` already maps to `DiagnosticSeverity::WARNING` in `convert.rs` |
| `writ_diagnostics::code` | (in-crate) | W0006 constant | All warning/error codes are defined here as `pub const` strings |
| `rustc_hash::FxHashMap` | (in-crate) | deprecated_items storage in TypeEnv | All TypeEnv maps use FxHashMap (confirmed across `fn_sigs`, `const_types`, etc.) |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `writ_compiler::ast::decl::AstAttribute` / `AstAttributeArg` | (in-crate) | Read `[Deprecated("msg")]` from AST | Used in `env_build` phase to scan attrs on decl nodes |
| `emit/collect/lookup::find_attrs_for_entry` | (in-crate) | Existing attr lookup helper | Could be reused conceptually; env_build needs its own scan since it works from `asts` + `entry` |

**Installation:** No new dependencies.

## Architecture Patterns

### Recommended Project Structure
No new files required. Changes touch existing files at well-defined seams:

```
writ-compiler/src/
├── check/
│   ├── env.rs           # add deprecated_items field to TypeEnv
│   ├── env_build.rs     # populate deprecated_items during build
│   └── check_expr/
│       ├── call.rs      # emit W0006 in check_call_with_sig + resolve_overloaded_call
│       └── ident.rs     # emit W0006 when ident resolves to a deprecated Fn/Const/Global
└── ...
writ-diagnostics/src/
└── code.rs              # add W0006 constant
writ-lsp/src/
└── queries/
    └── hover.rs         # prepend deprecation notice in hover_text_for_expr / hover_text_for_def
```

### Pattern 1: Adding metadata to TypeEnv (HIGH confidence)

**What:** `TypeEnv` gains a new `FxHashMap<DefId, String>` field. `TypeEnv::build` populates it by scanning the AST attributes for each resolved decl, extracting the first positional string arg of `[Deprecated("msg")]`.

**When to use:** Any time per-definition metadata needs to flow from AST attributes into the checker.

**Example:**
```rust
// In env.rs — add field to TypeEnv
pub struct TypeEnv {
    // ... existing fields ...
    /// DefIds marked [Deprecated("msg")], mapping to the user's message string.
    pub deprecated_items: FxHashMap<DefId, String>,
}
```

```rust
// In env.rs — TypeEnv::build initializer
let mut env = TypeEnv {
    // ... existing fields ...
    deprecated_items: FxHashMap::default(),
};
```

```rust
// In env_build.rs — helper to extract deprecation message from attrs list
pub(super) fn extract_deprecated_msg(attrs: &[AstAttribute]) -> Option<String> {
    for attr in attrs {
        if attr.name == "Deprecated" {
            // First positional string arg is the message
            for arg in &attr.args {
                if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                    return Some(value.clone());
                }
            }
            // [Deprecated] with no args: emit warning with no message
            return Some(String::new());
        }
    }
    None
}
```

```rust
// In env.rs TypeEnv::build — after building per-kind metadata, scan for [Deprecated]
// This scan covers Fn, Struct, Class, Entity, Enum, Contract, Component,
// ExternFn, Const, Global (all declaration kinds that carry attrs in the AST).
for decl in &resolved.decls {
    let def_id = env_build::decl_def_id(decl);
    let entry = resolved.def_map.get_entry(def_id);
    let attrs = find_attrs_for_entry_in_asts(asts, entry);
    if let Some(msg) = env_build::extract_deprecated_msg(&attrs) {
        env.deprecated_items.insert(def_id, msg);
    }
}
```

The `find_attrs_for_entry` helper already exists in `emit/collect/lookup.rs`. Env_build should duplicate the relevant pattern (or factor it into a shared utility) since `env_build` is in `writ-compiler` and `emit/collect` is also in `writ-compiler`. However, the cleanest approach is to implement the attr-lookup inline in `env_build` using the same `find_attrs_for_entry` pattern, or directly call the function from `emit::collect::lookup` since it's in the same crate. Either is valid; the latter avoids duplication.

### Pattern 2: Emitting W0006 at call sites (HIGH confidence)

**What:** After `check_call_with_sig` resolves the callee `DefId`, before returning the `TypedExpr::Call`, query `ctx.type_env.deprecated_items`. If present AND `ctx.current_file != def_entry.file_id`, push a `Diagnostic::warning(code::W0006, ...)`.

**When to use:** Whenever a known-`DefId` function or type reference is resolved in the checker.

**Example:**
```rust
// In check_call_with_sig, after resolving def_id but before return:
if let Some(msg) = ctx.type_env.deprecated_items.get(&def_id) {
    let entry = ctx.def_map.get_entry(def_id);
    // Suppress self-deprecation: same file means same module
    if entry.file_id != ctx.current_file {
        let warning_msg = if msg.is_empty() {
            format!("`{}` is deprecated", fn_name)
        } else {
            format!("`{}` is deprecated: {}", fn_name, msg)
        };
        ctx.diags.push(
            Diagnostic::warning(code::W0006, warning_msg)
                .with_primary(ctx.current_file, name_span, "deprecated item used here")
                .with_secondary(entry.file_id, entry.name_span, "declared deprecated here")
                .build(),
        );
    }
}
```

**Self-deprecation suppression detail:** `ctx.current_file` is the `FileId` of the file being type-checked (set in `check_fn_decl` from `entry.file_id`). The deprecated item's `entry.file_id` is where it was declared. When they match, the call site is in the same file as the declaration — this is the "same module" condition. Confirmed by reading `check_decl.rs`: `ctx.current_file = file_id` is set from the function's own `DefEntry.file_id`.

### Pattern 3: Ident references to deprecated items (HIGH confidence)

**What:** `check_ident` in `ident.rs` resolves `DefKind::Fn | ExternFn | Const | Global` by name. When a deprecated DefId is found, a warning should also fire there (not just at call sites). This covers cases like passing a deprecated function as a value, or reading a deprecated constant.

**When to use:** When `check_ident` resolves to a def in the public or private DefMap.

**Example:**
```rust
// In check_ident, after finding def_id via def_map.get(name) or file_private:
if let Some(msg) = ctx.type_env.deprecated_items.get(&def_id) {
    let entry = ctx.def_map.get_entry(def_id);
    if entry.file_id != ctx.current_file {
        let warning_msg = ...;
        ctx.diags.push(Diagnostic::warning(code::W0006, warning_msg)
            .with_primary(ctx.current_file, span, "deprecated item used here")
            .build());
    }
}
```

**Caveat:** Function idents that immediately become a call go through `check_call` -> `resolve_overloaded_call` -> `check_call_with_sig` — NOT through `check_ident`. So the W0006 in `check_call_with_sig` covers function calls. The `check_ident` path covers function-as-value (passing `deprecated_fn` as argument), constant references, and global references. Be careful not to double-emit for function calls.

**Double-emission avoidance:** `resolve_overloaded_call` is entered when the callee is `AstExpr::Ident` AND the name resolves to a function. In that path, `check_ident` is NOT called (the function lookup is done directly via `find_fn_candidates`). So there is no double-emission risk between `check_call_with_sig` and `check_ident` for function calls.

However, the `TypedExpr::Var` case in `check_ident` for `DefKind::Fn` could still be hit if the function name is used as a value (not a call). In that scenario only `check_ident` fires, not `check_call_with_sig`. So placing the warning in both is correct.

### Pattern 4: LSP hover deprecation notice (HIGH confidence)

**What:** `hover_text_for_expr` and `hover_text_for_def` in `hover.rs` receive `type_env` as a parameter. When the hovered item has a deprecated DefId, prepend a deprecation notice to the hover markdown.

**When to use:** Any hover response for a deprecated item.

**Helper to add:**
```rust
// In hover.rs — extract deprecation notice if present
fn deprecation_notice(def_id: DefId, type_env: &TypeEnv) -> Option<String> {
    type_env.deprecated_items.get(&def_id).map(|msg| {
        if msg.is_empty() {
            "**Deprecated**".to_string()
        } else {
            format!("**Deprecated:** {}", msg)
        }
    })
}
```

```rust
// In hover_text_for_expr, for the Call { callee_def_id: Some(def_id) } arm:
if let Some(sig) = type_env.fn_sigs.get(def_id) {
    let sig_text = format_fn_sig_hover(sig, def_map, interner);
    let mut parts = vec![sig_text];
    if let Some(notice) = deprecation_notice(*def_id, type_env) {
        parts.insert(0, notice);  // or append — prepend is conventional
    }
    // also append doc comment if present...
    return parts.join("\n\n");
}
```

**For `hover_text_for_def`:** Same pattern. `def_id` is already available at the call site in all arms.

**For `hover_text_for_expr` Var arm:** This arm currently does a name-based lookup to find a matching def. If a `DefId` is found and it is in `deprecated_items`, prepend the notice.

### Anti-Patterns to Avoid

- **Storing the deprecation message in `DefEntry`:** `DefEntry` intentionally has no attribute data. The established pattern (per Phase 94) is to leave DefEntry minimal and build derived lookup tables in `TypeEnv` during `env_build`. Do not add `deprecated_msg: Option<String>` to `DefEntry`.

- **Scanning AST attributes at every call site:** Attribute scanning is O(n) over the AST. The `deprecated_items` map in `TypeEnv` amortizes this to a one-time O(n) build + O(1) per lookup. Do not inline attribute scanning in check_expr.

- **Checking `entry.namespace` for self-deprecation:** The requirement says "same module" but the project uses file-per-module conventions. `entry.file_id == ctx.current_file` is the correct and simplest comparator. Namespace comparison is fragile (nested namespaces, declarative namespace form, etc.).

- **Emitting warnings for `check_new_construction` (types):** When a deprecated struct/entity/enum is referenced in a `new Type { ... }` expression, `check_new_construction` in `construction.rs` resolves the `target_def_id`. That path should also emit W0006. Check if `construction.rs` has the def_id available at the relevant point.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Warning emission | Custom diagnostic struct | `Diagnostic::warning(code::W0006, ...).with_primary(...).build()` | Established builder pattern; already maps to `DiagnosticSeverity::WARNING` in LSP convert layer |
| Attribute scanning | Inline match in env_build | `find_attrs_for_entry` pattern (lookup.rs) | Identical pattern already exists for emit phase; avoid divergence |
| LSP severity mapping | Custom severity logic | Existing `severity_to_lsp` in `convert.rs` | Already handles `Severity::Warning -> DiagnosticSeverity::WARNING` |

**Key insight:** The LSP warning squiggle pipeline already works end-to-end — `Diagnostic::warning(...)` -> `Severity::Warning` -> `severity_to_lsp` -> `DiagnosticSeverity::WARNING` -> editor squiggle. Phase 95 just needs to emit W0006 diagnostics correctly and the rest of the chain handles it for free.

## Common Pitfalls

### Pitfall 1: Missing new Type reference sites
**What goes wrong:** W0006 fires for function calls but not when a deprecated type is used as a type annotation, constructor, or enum variant.
**Why it happens:** Type annotation checking does not go through `check_call_with_sig` or `check_ident`. `check_new_construction` in `construction.rs` does resolve `target_def_id` directly.
**How to avoid:** Add the W0006 check inside `check_new_construction` when `target_def_id` is resolved, similar to call sites.
**Warning signs:** Test case `new DeprecatedStruct { ... }` produces no squiggle.

### Pitfall 2: Double-emission for type references that are also called
**What goes wrong:** Two W0006 diagnostics appear for the same call site.
**Why it happens:** If the ident check fires AND the call check fires.
**How to avoid:** Confirmed by code reading — `resolve_overloaded_call` bypasses `check_ident` entirely for direct function calls. No double-emission risk on that path. Type value references (not calls) only go through `check_ident`. The two code paths are disjoint for functions.
**Warning signs:** Unit test for a simple deprecated function call produces two warnings.

### Pitfall 3: `TypeEnv::build` is called before `asts` has attribute nodes
**What goes wrong:** The deprecated_items map is empty even though the source has `[Deprecated]` on a function.
**Why it happens:** `find_attrs_for_entry` scans the original `asts` slice. If the function lookup in `env_build` does not find the AstFnDecl (e.g., because `find_fn_decl` returns `None`), the attribute scan is skipped.
**How to avoid:** Do the attr scan in a separate pass over `resolved.decls` after the per-kind field building, using the same `asts` reference. This ensures the scan still runs even if the fn sig was already built.
**Warning signs:** Integration test with `[Deprecated("msg")] fn foo() {}` produces no warning at call site.

### Pitfall 4: Hover shows deprecation notice for declaration site but not use site
**What goes wrong:** Hover on the deprecated function declaration shows the notice; hover on a call site does not.
**Why it happens:** `hover_text_for_def` handles declaration sites; `hover_text_for_expr` handles use sites. They need the same deprecation notice logic.
**How to avoid:** Share a `deprecation_notice()` helper called from both functions.
**Warning signs:** LSP E2E test shows notice in one hover but not the other.

### Pitfall 5: Self-deprecation scope is wider than expected
**What goes wrong:** A function in a multi-file project calls a deprecated function in a different file of the same module but no warning fires, because `file_id` equality is too coarse.
**Why it happens:** The requirement says "same module as deprecated item". In the Writ project, module = namespace = file (one namespace per file convention). `file_id` equality is the correct comparator. If the project ever allows multi-file namespaces this would need revisiting, but that is out of scope.
**How to avoid:** Use `entry.file_id != ctx.current_file` as specified. This is the correct "same module" test for the current project structure.
**Warning signs:** None — this is correct behavior per spec.

## Code Examples

Verified patterns from in-codebase sources:

### Warning emission pattern (from check_stmt.rs W0005)
```rust
// Source: writ-compiler/src/check/check_stmt.rs:265
ctx.diags.push(
    writ_diagnostics::Diagnostic::warning(
        writ_diagnostics::code::W0005,
        "array literal containing a range in for loop ...",
    )
    .with_primary(ctx.current_file, *span, "this iterates ...")
    .with_help("remove the brackets ...")
    .build(),
);
```

### Adding a field to TypeEnv (from env.rs)
```rust
// Source: writ-compiler/src/check/env.rs
pub struct TypeEnv {
    pub fn_sigs: FxHashMap<DefId, FnSig>,
    pub const_types: FxHashMap<DefId, Ty>,
    // ... new field:
    pub deprecated_items: FxHashMap<DefId, String>,
}
```

### find_attrs_for_entry pattern (from emit/collect/lookup.rs:169)
```rust
// Source: writ-compiler/src/emit/collect/lookup.rs
pub(super) fn find_attrs_for_entry(asts: &[(FileId, &Ast)], entry: &DefEntry) -> Vec<AstAttribute> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            match decl {
                AstDecl::Fn(f) if f.name == entry.name && f.name_span == entry.name_span => {
                    return f.attrs.clone();
                }
                // ... other decl kinds ...
                _ => {}
            }
        }
    }
    Vec::new()
}
```

### check_call_with_sig injection point (from call.rs)
```rust
// Source: writ-compiler/src/check/check_expr/call.rs:336
pub(super) fn check_call_with_sig(
    ctx: &mut CheckCtx,
    fn_name: &str,
    def_id: DefId,
    sig: FnSig,
    args: &[AstArg],
    span: SimpleSpan,
    name_span: SimpleSpan,
) -> TypedExpr {
    let entry = ctx.def_map.get_entry(def_id);
    // W0006 injection here: entry.file_id vs ctx.current_file
    ...
}
```

### Hover deprecation pattern (from hover.rs structure)
```rust
// Source: writ-lsp/src/queries/hover.rs
// Pattern: hover_text_for_expr and hover_text_for_def both receive type_env.
// Current signature already has all needed data:
pub fn hover_text_for_expr(
    expr: &TypedExpr,
    def_map: &DefMap,
    interner: &TyInterner,
    type_env: &writ_compiler::check::env::TypeEnv,
    source: &str,
    ast: &TypedAst,
) -> String { ... }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No attribute semantic effects | Attributes pass through to module binary only (Phase 93-94) | Phase 93-94 | Phase 95 adds the first compiler-semantic effect |

**Deprecated/outdated:**
- None — this is a new feature.

## Open Questions

1. **Where does `[Deprecated]` on an enum type warn? On enum variant construction?**
   - What we know: Enum variants are constructed via `TypedExpr::Literal { value: TypedLiteral::EnumVariant { .. } }` in `check_path.rs`. The enum `DefId` is known at construction.
   - What's unclear: Whether the requirement covers enum type references (only at `new Type`, not at variant construction, or both).
   - Recommendation: Treat as a type-level deprecation: warn when `TypedExpr::New { target_def_id }` resolves to a deprecated type. Enum variant construction via `Color::Red` is a separate path — cover types first, add enum variant path if tests require it.

2. **`[Deprecated]` with no argument (bare `[Deprecated]`)**
   - What we know: `AstAttribute.args` will be empty. The spec says "with the user's message string" — but what if there is no message?
   - Recommendation: Treat empty message as `"deprecated"` or emit the warning without the `": ..."` suffix. Use `Option<String>` internally and format accordingly.

## Environment Availability

Step 2.6: SKIPPED — pure in-codebase Rust changes, no external tool dependencies.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-compiler deprecated 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEPR-01 | Calling `[Deprecated("msg")]` fn produces W0006 with message | unit | `cargo test -p writ-compiler deprecated_warning -x` | ❌ Wave 0 |
| DEPR-01 | No W0006 when call site is in same file as deprecated decl | unit | `cargo test -p writ-compiler deprecated_self_suppress -x` | ❌ Wave 0 |
| DEPR-01 | Deprecated const/type reference also produces W0006 | unit | `cargo test -p writ-compiler deprecated_const -x` | ❌ Wave 0 |
| DEPR-02 | LSP hover shows deprecation message for deprecated fn | integration | `cargo test -p writ-lsp deprecated_hover` | ❌ Wave 0 |
| DEPR-02 | LSP diagnostics include W0006 as DiagnosticSeverity::Warning | integration | `cargo test -p writ-lsp deprecated_diagnostic` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler --lib 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Unit tests in `writ-compiler/tests/` for W0006 emission (DEPR-01)
- [ ] LSP integration tests in `writ-lsp/tests/` for hover and diagnostic (DEPR-02)

*(No existing test infrastructure covers DEPR-01 or DEPR-02)*

## Sources

### Primary (HIGH confidence)
- `writ-compiler/src/check/env.rs` — TypeEnv field patterns, build structure
- `writ-compiler/src/check/env_build.rs` — how per-decl metadata is populated
- `writ-compiler/src/check/check_expr/call.rs` — check_call_with_sig, resolve_overloaded_call
- `writ-compiler/src/check/check_expr/ident.rs` — check_ident DefMap lookup
- `writ-compiler/src/check/check_decl.rs` — current_file setting pattern
- `writ-compiler/src/check/error.rs` — TypeError -> Diagnostic conversion pattern
- `writ-compiler/src/check/check_stmt.rs:265` — W0005 warning emission precedent
- `writ-diagnostics/src/code.rs` — W0001-W0005 constants; W0006 must be added here
- `writ-diagnostics/src/diagnostic.rs` — Diagnostic::warning builder API
- `writ-lsp/src/queries/hover.rs` — hover_text_for_expr, hover_text_for_def signatures
- `writ-lsp/src/convert.rs` — severity_to_lsp: Warning -> DiagnosticSeverity::WARNING
- `writ-lsp/src/backend.rs` — publish_grouped_diagnostics pipeline
- `writ-compiler/src/emit/collect/lookup.rs:169` — find_attrs_for_entry pattern
- `writ-compiler/src/ast/decl.rs` — AstAttribute, AstAttributeArg structures

### Secondary (MEDIUM confidence)
- None needed — all findings verified directly from source.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are in-crate; no new dependencies
- Architecture: HIGH — all injection points verified by direct code reading; patterns confirmed by W0005 precedent
- Pitfalls: HIGH — identified by tracing the actual code paths call-by-call
- LSP integration: HIGH — `Severity::Warning` -> `DiagnosticSeverity::WARNING` confirmed in `convert.rs`

**Research date:** 2026-03-27
**Valid until:** 60 days (stable Rust codebase, no external dependency drift)
