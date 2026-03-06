# Phase 63: writ-compiler File Splits - Research

**Researched:** 2026-03-18
**Domain:** Rust module splitting — structural refactoring without behavior change
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SPLIT-03 | `writ-compiler/src/check/check_expr.rs` (2,134 lines) split by expression category | File read + section analysis: 9 natural sections identified by `// ====` comment banners |
| SPLIT-04 | `writ-compiler/src/emit/collect.rs` (1,687 lines) split by declaration type | File read: clear per-declaration-kind groupings, 12 section banners |
| SPLIT-05 | `writ-compiler/src/emit/body/expr.rs` (1,470 lines) split by expression category | File read: comment markers delineate literals/binary/control-flow/builtins/construction |
| SPLIT-08 | `writ-compiler/src/emit/module_builder.rs` (1,063 lines) reviewed for split opportunities | File read: single mega-struct with add/query/finalize method groups — internal cohesion argues against split |
| SPLIT-09 | `writ-compiler/src/check/env.rs` (1,032 lines) reviewed for split opportunities | File read: TypeEnv + LocalEnv structs + find_* helpers + build_* helpers + resolve_ast_type — two distinct groups |
| SPLIT-10 | `writ-compiler/src/lower/dialogue.rs` (858 lines) reviewed for split opportunities | File read: singleton-speaker scan + loc-key logic + text lowering + choice/control flow lowering — borderline |
| SPLIT-11 | `writ-compiler/src/resolve/resolver.rs` (849 lines) reviewed for split opportunities | File read: resolve_bodies + process_usings + resolve_decl_list (large) + resolve_ast_type — cohesive pass logic |
</phase_requirements>

---

## Summary

Phase 63 is a structural refactoring: split 7 oversized files in `writ-compiler` into focused submodules without changing any behavior. No new features, no bug fixes, no test changes. The only build artifact that must change is the file layout and `mod` declarations.

All 7 files are within a single Rust crate (`writ-compiler`). Rust's module system means a split is: (1) create new `.rs` files under the parent directory, (2) add `pub mod` declarations in the parent mod file or in the original file which becomes `mod.rs`, (3) fix visibility (`pub(super)`, `pub(crate)`, or `pub`) on relocated items. All existing tests import by crate path (`writ_compiler::check::check_expr::CheckCtx`), so the requirement that `mod` declarations are explicit and not hidden behind `pub use *` glob re-exports is important.

The two mandatory splits (SPLIT-03, SPLIT-04, SPLIT-05) have clear natural boundaries already marked by `// ====` section comment banners. The four "reviewed" requirements (SPLIT-08 through SPLIT-11) require judgment: three of the four files should NOT be split (the cohesion analysis below shows splitting would hurt readability), and one (SPLIT-09 `env.rs`) has a genuine split opportunity.

**Primary recommendation:** Split the three large files with clear category boundaries (check_expr.rs, collect.rs, emit/body/expr.rs). Document rationale for not splitting the other four.

---

## Standard Stack

This phase uses only the Rust toolchain already present in the project — no new dependencies.

| Tool | Version | Purpose |
|------|---------|---------|
| rustc/cargo | edition 2024 (Cargo.toml) | Build + test |
| cargo test | project standard | Verify no regressions |
| cargo clippy | project standard | Zero-warning requirement from Phase 62 |

**Installation:** No new packages needed.

---

## Architecture Patterns

### Rust Module Split Pattern

The canonical pattern for splitting a large `foo.rs` into submodules:

**Option A — Folder conversion (most common for large files):**
```
// Before: check/check_expr.rs (one large file)

// After:
check/check_expr/          ← new folder
check/check_expr/mod.rs    ← re-exports, shared types (CheckCtx)
check/check_expr/ident.rs  ← check_ident, check_path
check/check_expr/binary.rs ← check_binary, check_unary_prefix
...
```
All external callers using `super::check_expr::{CheckCtx, check_expr}` continue to work
because `check_expr` is still a module, just now backed by a folder instead of a single file.

**Option B — Sibling files (for "reviewed" files where splitting is lighter):**
```
// Before: check/env.rs (one large file with TypeEnv + build helpers + LocalEnv)

// After:
check/env.rs         ← TypeEnv, LocalEnv, FnSig, ImplEntry + re-export pub items
check/env_build.rs   ← build_fn_sig, build_struct_fields, build_impl_entry, etc.
```
In `env.rs`: `mod env_build;` + `use env_build::*;` or explicit items.

### Recommended Project Structure After Phase 63

```
writ-compiler/src/check/
├── check_expr/
│   ├── mod.rs          # CheckCtx struct + check_expr dispatch + check_block/_stmts + check_assign_mutability
│   ├── ident.rs        # check_ident, check_path (lines 353-583)
│   ├── binary.rs       # check_binary, check_unary_prefix (lines 584-787)
│   ├── call.rs         # check_call, check_call_with_sig, check_contract_bounds, check_generic_call (lines 788-1209)
│   ├── control.rs      # check_if, check_block (lines 1211-1319)
│   ├── access.rs       # check_member_access, check_bracket_access (lines 1320-1591)
│   ├── match_.rs       # check_match, check_pattern (lines 1592-1815)
│   ├── lambda.rs       # check_lambda (lines 1817-1895)
│   └── construction.rs # check_new_construction, check_array_lit (lines 1896-2060)
│
├── env.rs              # TypeEnv, LocalEnv, FnSig, ImplEntry — keep as-is (see SPLIT-09 analysis)
...

writ-compiler/src/emit/
├── collect/
│   ├── mod.rs          # collect_defs, collect_post_finalize, collect_exports, collect_attributes
│   ├── types.rs        # collect_struct, collect_entity, collect_enum, collect_class
│   ├── functions.rs    # collect_fn, collect_extern_fn, collect_component
│   ├── contracts.rs    # collect_contract, collect_impl, collect_extern_class, collect_extern_component
│   ├── builtins.rs     # inject_log_extern_defs, inject_dialogue_extern_defs
│   ├── walker.rs       # collect_called_def_ids, walk_expr, walk_stmt
│   └── encoding.rs     # encode_type_from_ast, encode_fn_sig, encode_empty_sig, ast_type_to_ty_simple
│
├── body/
│   ├── expr/
│   │   ├── mod.rs         # emit_expr dispatch + emit_literal
│   │   ├── binary.rs      # emit_binary (lines 438-687)
│   │   ├── control.rs     # emit_if, emit_spawn, emit_defer (lines 688-845)
│   │   ├── construction.rs # emit_range, emit_array_lit, emit_new (lines 845-1247)
│   │   ├── builtins.rs    # try_emit_builtin_method (lines 944-1157)
│   │   └── string.rs      # try_collect_str_build_parts, emit_str_build, collect_string_chain (lines 1247-1325)
│   ...
```

### Anti-Patterns to Avoid

- **Splitting into units that share all local private helpers:** If two "submodules" need the same private helper, either the helper must be `pub(super)` in a shared parent, or the split is wrong.
- **`pub use *` glob re-exports from parent mod.rs:** The phase requirement explicitly prohibits this. Use explicit `pub use submod::ItemName`.
- **Moving `CheckCtx` out of `check_expr`:** External callers (`check_decl.rs`, `check_stmt.rs`, `desugar.rs`, `writ-lsp`) use `super::check_expr::{check_expr, CheckCtx}`. `CheckCtx` must stay at the `check_expr` module root (i.e., in `check_expr/mod.rs`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Detecting circular imports after split | Manual trace | `cargo check` immediately catches cycles |
| Verifying public API surface unchanged | Custom script | `cargo test --workspace` — existing tests will fail if anything breaks |
| Finding all callers of a private function | Manual grep | Rust compiler: make it `pub(crate)` and `cargo check` reports all callers |

---

## Common Pitfalls

### Pitfall 1: Visibility Escalation Cascade
**What goes wrong:** When a private helper function is moved to a submodule, Rust requires visibility annotation to make it accessible from sibling submodules. The natural reaction is `pub`, but `pub(super)` is the correct scope.
**Why it happens:** Moving `fn check_ident(ctx: &mut CheckCtx, ...)` from `check_expr.rs` to `check_expr/ident.rs` means `check_expr/mod.rs` (where `check_expr` calls it) can no longer see it as `fn`. It must be `pub(super) fn check_ident`.
**How to avoid:** Use `pub(super)` for all helpers only used within the containing module tree. Use `pub(crate)` only if needed cross-module within the crate. Reserve bare `pub` for items that were already public.
**Warning signs:** If `pub use *` starts appearing in mod.rs, or items get bare `pub` — investigate whether that visibility was already required.

### Pitfall 2: Circular Module References
**What goes wrong:** `check_expr/call.rs` imports `check_expr/mod.rs::check_expr`, which imports `check_expr/call.rs::check_call`. This is actually fine in Rust — modules within the same parent can call each other.
**Why it happens:** Confusion between Rust's module system (no circular imports) and function call cycles (fine). Within a single file you can have mutually recursive functions; after splitting into submodules, those same functions can still call each other IF they are in sibling submodules under the same parent module.
**How to avoid:** The dispatch function `check_expr` in `mod.rs` calls `check_ident` in `ident.rs` — `ident.rs` is a submodule, so `mod.rs` can call `pub(super) fn check_ident`. `ident.rs` can call back to `check_expr` via `super::check_expr`. Rust allows this.
**Warning signs:** `cargo check` errors about "module `X` is private" or "use of undeclared crate or module".

### Pitfall 3: Insta Snapshot Test Invalidation
**What goes wrong:** Snapshot tests in `writ-compiler/tests/` use insta. If any type name or module path appears in a snapshot output and the module path changes, snapshots will mismatch.
**Why it happens:** The tests do not directly reference internal file paths, but Debug/Display impls may include type names. The refactoring changes no behavior, so this should not occur — but verify.
**How to avoid:** After all splits: run `cargo test -p writ-compiler` and check all snapshot tests pass without any `cargo insta review` needed.

### Pitfall 4: Splitting `check_expr.rs` Breaks External Path References
**What goes wrong:** `writ-lsp/src/queries.rs` does NOT import `check_expr` internals — it only uses `check::ir::TypedAst`, `check::ty::Ty`, `check::env::TypeEnv`, `check::env::FnSig`. The `CheckCtx` struct is never imported by external crates.
**Why it happens:** Could be confused with `check::check_expr::CheckCtx` appearing in mod.rs's construction of CheckCtx.
**How to avoid:** No external crate references `check_expr` module internals — only the `check::mod.rs` entry point uses `check_expr::CheckCtx` locally. External paths are safe.

### Pitfall 5: module_builder.rs Split Creates Incoherent State
**What goes wrong:** `ModuleBuilder` is a single struct with 40+ fields. If its method impls are split across files (e.g., `add_methods.rs`, `query_methods.rs`, `finalize.rs`), you need to use Rust's `impl ModuleBuilder` in multiple files under the same module — which is legal but requires `mod.rs` to declare the impl submodules.
**Why it happens:** `impl` blocks for a type can only appear in the same crate but can span multiple files if they are all submodules.
**How to avoid:** The research conclusion is NOT to split `module_builder.rs` (see SPLIT-08 analysis below). If future owners want to split it, the approach is: move impl blocks to separate files, each declaring `impl ModuleBuilder { ... }`.

---

## Per-File Split Analysis

### SPLIT-03: check/check_expr.rs (2,134 lines) — MANDATORY SPLIT

**Natural sections (identified by `// ====` comment banners and function groups):**

| New File | Lines Approx | Content |
|----------|-------------|---------|
| `check_expr/mod.rs` | ~100 | `CheckCtx` struct + `impl CheckCtx` + `check_expr` dispatch (lines 1-351) + `check_block`/`check_block_stmts` (1277-1319) + `check_assignment_mutability` (2061-2090) + `find_root_binding`/`find_fn_def_id` (2089-end) |
| `check_expr/ident.rs` | ~115 | `check_ident` (353-468) |
| `check_expr/path.rs` | ~115 | `check_path` (469-583) |
| `check_expr/binary.rs` | ~130 | `check_binary` (584-713), `check_unary_prefix` (714-787) |
| `check_expr/call.rs` | ~290 | `check_call` (788-974), `check_call_with_sig` (975-1054), `check_contract_bounds` (1055-1114), `check_generic_call` (1115-1210) |
| `check_expr/control.rs` | ~100 | `check_if` (1211-1276), `check_block` inline (already in mod.rs) |
| `check_expr/access.rs` | ~270 | `check_member_access` (1324-1450), `check_bracket_access` (1456-1591) |
| `check_expr/match_.rs` | ~220 | `check_match` (1596-1667), `check_pattern` (1668-1815) |
| `check_expr/lambda.rs` | ~80 | `check_lambda` (1821-1895) |
| `check_expr/construction.rs` | ~170 | `check_new_construction` (1900-2004), `check_array_lit` (2009-2055) |

**External API preserved:** `pub use check_expr::{CheckCtx, check_expr, check_block_stmts, check_assignment_mutability}` — same as today.

**Visibility:** All split helper functions use `pub(super)` so `mod.rs`'s `check_expr` dispatch function can call them.

### SPLIT-04: emit/collect.rs (1,687 lines) — MANDATORY SPLIT

**Natural sections (identified by `// ====` section banners):**

| New File | Lines Approx | Content |
|----------|-------------|---------|
| `collect/mod.rs` | ~160 | `collect_defs` (20-116), `collect_post_finalize` (117-133), `find_module_name` (138-156), `build_generic_map` (162-169) |
| `collect/types.rs` | ~400 | `collect_struct` (174-222), `collect_entity` (227-274), `collect_enum` (279-316), `collect_class` (849-893) |
| `collect/functions.rs` | ~200 | `collect_fn` (321-367), `collect_extern_fn` (782-807), `collect_component` (511-544) |
| `collect/contracts.rs` | ~230 | `collect_contract` (372-406), `collect_impl` (412-506), `collect_extern_class` (898-930), `collect_extern_component` (935-967) |
| `collect/builtins.rs` | ~130 | `inject_log_extern_defs` (562-588), `inject_dialogue_extern_defs` (600-644) |
| `collect/walker.rs` | ~130 | `collect_called_def_ids` (653-672), `walk_expr` (673-751), `walk_stmt` (752-777) |
| `collect/globals.rs` | ~80 | `collect_const` (972-990), `collect_global` (991-1009) |
| `collect/encoding.rs` | ~350 | `collect_exports` (1014-1043), `collect_attributes` (1048-1091), `collect_locale_defs` (1101-1149), `find_attrs_for_entry` (1150-1187), `collect_component_slots` (1192-1225), `ast_type_to_ty_simple` (1238-1273), `encode_type_from_ast` (1274-1284), `encode_ast_type_into` (1285-1346), `encode_empty_sig` (1347-1354), `encode_fn_sig` (1355-1394), `encode_fn_sig_from_ast_sig` (1395-end) |

**External API preserved:** `pub use collect::{collect_defs, collect_post_finalize, inject_log_extern_defs, inject_dialogue_extern_defs}` — same as today.

### SPLIT-05: emit/body/expr.rs (1,470 lines) — MANDATORY SPLIT

**Natural sections (identified by `// ─── section ───` comment markers):**

| New File | Lines Approx | Content |
|----------|-------------|---------|
| `expr/mod.rs` | ~100 | `emit_expr` dispatch (1-397) |
| `expr/literal.rs` | ~40 | `emit_literal` (399-434) |
| `expr/binary.rs` | ~255 | `emit_binary` (438-687) |
| `expr/control.rs` | ~200 | `emit_if` (688-748), `emit_spawn` (749-805), `emit_defer` (806-844) |
| `expr/construction.rs` | ~300 | range (`emit_range` 858-911), `emit_array_lit` (912-943), `emit_new` (1158-1246), `emit_str_build` (1288-1324), `try_collect_str_build_parts` (1247-1263), `collect_string_chain` (1264-1287) |
| `expr/builtins.rs` | ~215 | `try_emit_builtin_method` (944-1157) |
| `expr/eq.rs` | ~105 | `emit_struct_eq` (1325-1375), `emit_struct_neq` (1376-1433), `emit_field_eq` (1434-end) |

**External API preserved:** `pub use expr::emit_expr` — same as today.

### SPLIT-08: emit/module_builder.rs (1,063 lines) — RECOMMEND NO SPLIT

**Analysis:** `ModuleBuilder` is a single struct with 40+ fields. Its `impl` block contains three method groups:
1. `new()` constructor (lines 130-176)
2. "Add" methods — Pass 1 population (lines 178-559) — 14 methods
3. `finalize()` — Pass 2 index assignment (lines 560-658) — 1 large method
4. "Query" methods — post-finalize access (lines 660-1043) — ~30 methods

All methods read or write `self`'s fields directly. Splitting the `impl` into files would:
- Require all split files to be submodules of the same parent module
- Not reduce complexity: callers would still use `builder.add_typedef(...)`, `builder.finalize()`, etc.
- Create confusion about which file to look in for a given method

**Conclusion:** Document as "reviewed, no split needed — single struct with related methods, splitting impl blocks across files adds file navigation overhead without clarity gain." The 1,063 lines is under 2x the 500-line target. **Document rationale as required by success criterion 1.**

### SPLIT-09: check/env.rs (1,032 lines) — SPLIT OPPORTUNITY: PARTIAL

**Analysis:** The file has two clearly distinct responsibilities:

1. **Type data structures + TypeEnv::build** (lines 1-278): `FnSig`, `EnumVariantSig`, `ImplEntry`, `TypeEnv`, `TypeEnv::build`. These are the public API items consumed by `check_decl.rs`, `check_stmt.rs`, `check_expr.rs`, and `writ-lsp`.

2. **AST-to-type builder helpers** (lines 280-982): `decl_def_id`, `find_fn_decl` through `find_global_decl`, `build_generic_map`, `resolve_ast_type`, `resolve_ast_type_with_file`, `build_fn_sig`, `build_struct_fields`, etc. — private helpers for `TypeEnv::build`.

3. **LocalEnv** (lines 984-1033): `LocalEnv`, `Mutability` — local variable environment.

**Recommendation:** Extract the large private build helpers to `env_build.rs`:
```
check/env.rs      — public structs (FnSig, TypeEnv, LocalEnv, Mutability) + TypeEnv::build
check/env_build.rs — private: decl_def_id, find_*_decl, build_fn_sig, build_struct_fields, etc.
```
`env.rs` calls `mod env_build;` and uses `env_build::` for internal helpers. This gets `env.rs` from 1,032 lines to ~300 lines of public types. `env_build.rs` holds ~720 lines of private helpers. Neither file is "oversized" and each has a clear single responsibility.

**External API preserved:** `writ_compiler::check::env::{TypeEnv, FnSig, LocalEnv, Mutability}` — unchanged.

### SPLIT-10: lower/dialogue.rs (858 lines) — RECOMMEND NO SPLIT

**Analysis:** The file contains one coherent operation: lowering a `DlgDecl` CST node into an `AstFnDecl`. The internal sections are:

1. `DlgLowerState` struct (lines 21-29) — session state
2. `lower_dialogue` entry point (lines 48-167) — orchestrates the whole pass
3. Speaker collection helpers (174-246) — `collect_singleton_speakers`, `collect_singleton_speakers_inner`, `collect_dlg_if_else`
4. Line lowering (251-393) — `lower_dlg_lines`
5. Speaker resolution (395-418) — `resolve_speaker`
6. Localization key computation (419-474) — `compute_or_use_loc_key`, `fnv1a_32`
7. Text lowering (480-569) — `raw_text_content`, `expr_to_slot_text`, `lower_dlg_text`
8. Say construction (580-622) — `make_say`, `make_say_localized`
9. Choice lowering (628-718) — `lower_choice`
10. Control flow (723-826) — `lower_dlg_if`, `lower_dlg_else`, `lower_dlg_match`
11. Transition (831-end) — `lower_transition`

All sections share `DlgLowerState` and participate in a single-pass transformation. Splitting would create artificial file boundaries in what is a single algorithmic pipeline. At 858 lines it is under 2x the 500-line target.

**Conclusion:** Document as "reviewed, no split needed — cohesive single-pass transformation, all sections share session state and are tightly coupled to `DlgLowerState`." **Document rationale.**

### SPLIT-11: resolve/resolver.rs (849 lines) — RECOMMEND NO SPLIT

**Analysis:** The file contains Pass 2 of name resolution: `resolve_bodies` (22-51), `detect_file_namespace` (54-61), `process_usings` (64-221), `resolve_decl_list` (224-636), `resolve_ast_type` (637-779), `find_enum_variants` (780-811), `make_fqn` (812-821), `get_suggestion` (822-827), `check_generic_shadows` (828-end).

`resolve_decl_list` is 413 lines — a large match on `AstDecl` variants. However, it is a single unified algorithm (resolve names in every AST node type). Breaking it apart would create:
- Artificial split: `resolve_struct_decl.rs`, `resolve_fn_decl.rs`, etc. — each 30-50 lines
- Harder to follow the control flow of the resolver

At 849 lines this is under 2x the 500-line target. `resolve_ast_type` (143 lines) is the most separable unit, but it is tightly coupled with `resolve_decl_list`'s inline type resolution calls.

**Conclusion:** Document as "reviewed, no split needed — single-pass resolver; `resolve_decl_list` is a match on all AST decl kinds and extracting individual handlers would fragment the algorithm without clarity gain." **Document rationale.**

---

## Code Examples

### Creating a Folder Module (canonical Rust pattern)

```
# 1. Rename check_expr.rs to check_expr/mod.rs
#    (or create check_expr/ directory and move content)

# File layout:
writ-compiler/src/check/check_expr/mod.rs      # was check_expr.rs — keep CheckCtx here
writ-compiler/src/check/check_expr/ident.rs    # new
writ-compiler/src/check/check_expr/binary.rs   # new
```

```rust
// Source: Rust Reference — https://doc.rust-lang.org/reference/items/modules.html

// In check_expr/mod.rs:
pub mod ident;    // check_ident, check_path
pub mod binary;   // check_binary, check_unary_prefix
pub mod call;     // check_call, check_call_with_sig, etc.
pub mod control;  // check_if
pub mod access;   // check_member_access, check_bracket_access
pub mod match_;   // check_match, check_pattern
pub mod lambda;   // check_lambda
pub mod construction; // check_new_construction, check_array_lit

// Re-use existing pub items by declaring them here (they stay here):
pub struct CheckCtx<'def> { ... }
pub fn check_expr(ctx: &mut CheckCtx, expr: &AstExpr) -> TypedExpr { ... }
pub fn check_block_stmts(...) -> TypedExpr { ... }
pub fn check_assignment_mutability(...) { ... }

// Call into submodules:
use self::ident::check_ident;
use self::binary::{check_binary, check_unary_prefix};
// ... etc.
```

```rust
// In check_expr/ident.rs:
use super::CheckCtx;  // accesses mod.rs's CheckCtx
use crate::check::ir::TypedExpr;
// ...

pub(super) fn check_ident(ctx: &mut CheckCtx, name: &str, span: SimpleSpan) -> TypedExpr {
    // ... move from check_expr.rs lines 353-468
}

pub(super) fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    // ... move from check_expr.rs lines 469-583
}
```

### Sibling-File Pattern (for env.rs)

```rust
// Source: Rust Reference — module system

// In check/env.rs (existing file, trimmed):
mod env_build;  // private submodule

pub struct TypeEnv { ... }
pub struct FnSig { ... }
pub struct LocalEnv { ... }
pub enum Mutability { ... }

impl TypeEnv {
    pub fn build(...) -> (TypeEnv, Vec<Diagnostic>) {
        // still here — calls env_build helpers
        use env_build::*;
        // ...
    }
}

// In check/env_build.rs (new file):
use super::*;  // or explicit imports

pub(super) fn build_fn_sig(...) -> FnSig { ... }
pub(super) fn build_struct_fields(...) -> Vec<...> { ... }
// etc.
```

---

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| `mod foo;` in lib.rs only | Hierarchical `mod.rs` files | Rust 2018 edition allows `foo/mod.rs` OR `foo.rs` sibling style |
| Monolithic single file | Folder module with subfiles | Standard for large Rust crates |
| Bare `pub` on everything | `pub(super)` / `pub(crate)` | Rust idiom: minimize visibility |

**Note on `mod.rs` vs `foo.rs` sibling style:**
Rust 2018 introduced the ability to write `foo/bar.rs` without a `foo/mod.rs` — just have `foo.rs` as the module root. However, when `foo.rs` itself needs to become a module directory, the canonical approach is `foo/mod.rs`. Both styles compile identically. The project uses `mod.rs` style already (e.g., `emit/body/mod.rs`, `check/mod.rs`). Continue with `mod.rs`.

---

## Open Questions

1. **Does `check_expr/mod.rs` get too long with the dispatch match?**
   - What we know: `check_expr` dispatch (lines 1-351) is 351 lines on its own.
   - What's unclear: Whether to keep the dispatch inline or put it in a `dispatch.rs`.
   - Recommendation: Keep in `mod.rs` — the dispatch is the entry point and having it in mod.rs makes navigation obvious.

2. **Should `collect/encoding.rs` be further split?**
   - What we know: `encode_fn_sig` + `encode_ast_type_into` are 350+ lines of type-encoding helpers.
   - Recommendation: Keep as `encoding.rs` — it's a cohesive group of type-signature encoding functions.

3. **`try_emit_builtin_method` in `emit/body/expr.rs` is 215 lines — does it stay in `builtins.rs`?**
   - Recommendation: Yes — it handles Option/Result/Array builtins as a unit, clear responsibility.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner + insta 1.x (snapshot tests) |
| Config file | `writ-compiler/Cargo.toml` (dev-dependencies: insta) |
| Quick run command | `cargo test -p writ-compiler` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| SPLIT-03 | check_expr.rs split: all typecheck tests still pass | regression | `cargo test -p writ-compiler typecheck` | YES (typecheck_tests.rs) |
| SPLIT-04 | collect.rs split: all emit tests still pass | regression | `cargo test -p writ-compiler emit` | YES (emit_tests.rs, emit_serialize_tests.rs) |
| SPLIT-05 | expr.rs split: all body emission tests still pass | regression | `cargo test -p writ-compiler emit_body` | YES (emit_body_tests.rs) |
| SPLIT-08 | module_builder.rs: review documented + zero new warnings | build | `cargo clippy -p writ-compiler` | N/A |
| SPLIT-09 | env.rs split: all typecheck + LSP tests pass | regression | `cargo test --workspace` | YES |
| SPLIT-10 | dialogue.rs: review documented + lowering tests pass | regression | `cargo test -p writ-compiler lowering` | YES (lowering_tests.rs) |
| SPLIT-11 | resolver.rs: review documented + resolve tests pass | regression | `cargo test -p writ-compiler resolve` | YES (resolve_tests.rs) |

### Sampling Rate

- **Per file split:** `cargo test -p writ-compiler` (full compiler test suite, ~30 seconds)
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** All workspace tests green + `cargo clippy --workspace` zero warnings before verification

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. All test files exist.

---

## Sources

### Primary (HIGH confidence)
- Direct file inspection: all 7 source files read in full
- Rust Reference (module system): https://doc.rust-lang.org/reference/items/modules.html — HIGH confidence on module patterns
- Project source code: `writ-compiler/src/check/mod.rs`, `emit/mod.rs`, `lower/mod.rs`, `resolve/mod.rs` — actual current structure
- Cross-reference check: `writ-lsp/src/queries.rs` and `analysis_host.rs` — confirmed external API surface

### Secondary (MEDIUM confidence)
- N/A for this phase — all knowledge comes from direct file inspection, no web search needed

### Tertiary (LOW confidence)
- N/A

---

## Metadata

**Confidence breakdown:**
- File sizes and line counts: HIGH — directly measured with `wc -l`
- Natural section boundaries: HIGH — identified from `// ====` and `// ───` comment markers in source
- Split recommendations (mandatory): HIGH — clear category-based sections
- Split recommendations (reviewed): HIGH — confirmed by reading the files + checking external callers
- Visibility requirements (`pub(super)` strategy): HIGH — standard Rust module idiom
- Test coverage: HIGH — all test files verified to exist

**Research date:** 2026-03-18
**Valid until:** 2026-04-18 (stable codebase — no fast-moving dependencies)
