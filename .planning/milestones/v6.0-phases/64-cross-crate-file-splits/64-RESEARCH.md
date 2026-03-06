# Phase 64: Cross-Crate File Splits - Research

**Researched:** 2026-03-18
**Domain:** Rust module splitting — structural refactoring across 5 crates without behavior change
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SPLIT-01 | `writ-parser/src/parser.rs` (3,345 lines) split into logical submodules | File read + function inventory: 6 top-level parsers with clear grammar-category boundaries |
| SPLIT-02 | `writ-lsp/src/queries.rs` (2,768 lines) split into hover/completion/reference/definition modules | File read + `// ====` section banners: 5 distinct query categories already delineated |
| SPLIT-06 | `writ-runtime/src/domain.rs` (1,149 lines) split into resolution/dispatch modules | File read: two coherent groups — ref resolution (lines 100-561) and dispatch table construction (lines 362-612) |
| SPLIT-07 | `writ-dap/src/server.rs` (1,140 lines) split into request handler modules | File read: single `handle_request` match with 15+ `Command::*` arms; free helper functions are naturally separable |
| SPLIT-12 | `writ-lsp/src/analysis_host.rs` (1,415 lines) reviewed for split opportunities | File read: single struct `AnalysisHost` with two public methods (`analyze_standalone`, `analyze_project`) + 3 private helpers + inline tests |
| SPLIT-13 | `writ-lsp/src/backend.rs` (888 lines) reviewed for split opportunities | File read: `LanguageServer` impl (lines 49-651) + private helpers impl (lines 653-843) + free functions |
| SPLIT-14 | `writ-cli/src/main.rs` (703 lines) split into command modules | File read: 6 `cmd_*` functions (cmd_new, cmd_build, cmd_compile, cmd_assemble, cmd_disasm, cmd_run) + shared `run_pipeline` |
</phase_requirements>

---

## Summary

Phase 64 is a structural refactoring parallel to Phase 63: split oversized files across five crates (`writ-parser`, `writ-lsp`, `writ-dap`, `writ-runtime`, `writ-cli`) without changing any behavior. No new features, no bug fixes, no test modifications required.

The same Rust module-splitting mechanics from Phase 63 apply here: convert a monolithic file into a folder module (`foo.rs` → `foo/mod.rs` + `foo/subfile.rs`), or split into sibling files. The critical constraint is that all external paths (`writ_parser::parser::parse`, `writ_lsp::backend::Backend`, etc.) must remain stable — existing tests import these exact paths and must pass without modification.

The most important discovery is the **natural split boundaries already present**: `queries.rs` has 5 explicit `// ====` section comment banners; `parser.rs` has 6 top-level public parsers that are structurally independent; `main.rs` has 6 `cmd_*` functions already section-delimited by `// ─── Subcommand: X` banners. For `domain.rs`, two "reviewed" requirements (SPLIT-12, SPLIT-13) have genuine but borderline split cases that benefit from careful cohesion analysis before committing.

**Primary recommendation:** Split the three files with obvious categorical boundaries (parser.rs, queries.rs, main.rs). Document rationale for domain.rs split vs. keep. For analysis_host.rs and backend.rs: document rationale with strong preference for no-split given tight async coupling in backend.rs and the single-struct cohesion of analysis_host.rs.

---

## Standard Stack

This phase uses only the Rust toolchain already present — no new dependencies.

| Tool | Version | Purpose |
|------|---------|---------|
| rustc/cargo | edition 2024 (workspace Cargo.toml) | Build + test |
| cargo test | project standard | Regression verification after each split |
| cargo clippy | project standard | Zero-warning requirement maintained from Phase 62 |

**Installation:** No new packages needed.

---

## Architecture Patterns

### Rust Module Split Pattern (established in Phase 63)

**Option A — Folder conversion (mandatory for files being split into many pieces):**
```
// Before: parser.rs (one file, 3,345 lines)

// After:
parser/            ← new folder
parser/mod.rs      ← shared types + pub fn parse() entry point
parser/types.rs    ← TypePostfix, ExprPostfix helper enums
parser/type_expr.rs  ← pub fn type_expr()
parser/generic_params.rs  ← pub fn generic_params()
parser/pattern.rs  ← fn pattern() (private)
parser/strings.rs  ← fn parse_formattable_string(), split_dlg_text_segments(), parse_expr_from_source()
parser/program.rs  ← pub fn program_parser() (3,294-line main parser)
```

External callers use `writ_parser::parser::parse` — this path is unchanged because `parser` is still a module (now folder-backed). `writ_parser::parser::type_expr` also remains valid.

**Option B — Sibling files (for "reviewed" files where splitting is lighter):**
```
// Before: domain.rs (single file, 1,149 lines)

// After (if splitting):
domain.rs        ← Domain struct + ResolvedRefs types + core add/resolve_refs methods
domain_refs.rs   ← resolve_module_refs, resolve_parent_type, find_* helpers
domain_dispatch.rs ← build_dispatch_table, resolve_type_key, resolve_contract_key_for_impl
```

**The no-split pattern (for reviewed requirements):**
Add a `## SPLIT-XX review (Phase 64)` block to the module-level `//!` doc comment, exactly as done in Phase 63 for SPLIT-08/10/11.

### Pub Visibility Discipline

Consistent with Phase 63 decisions:
- Items shared across sibling submodules: `pub(super)`
- Items needed outside the parent module: `pub` or `pub(crate)`
- Items strictly internal: `pub(crate)` or private
- No `pub use *` glob re-exports that obscure origin

### Test Path Preservation

External test files import by crate path. These must remain stable:

| Test file | Critical import | Split impact |
|-----------|-----------------|--------------|
| `writ-parser/tests/parser_tests.rs` | `writ_parser::parser::parse` | `parse` must stay re-exported from `parser` mod |
| `writ-lsp/tests/test_hover_protocol.rs` | `writ_lsp::backend::Backend` | `Backend` stays in `backend` mod |
| `writ-dap/tests/test_compile_and_load.rs` | `writ_dap::launch::compile_and_load` | `launch` module unchanged |
| `writ-dap/tests/test_initialize_sequence.rs` | (reads `server` module indirectly) | `server::DapServer` must stay |
| `writ-runtime/tests/vm_tests.rs` | (no direct `domain` imports) | tests use `Runtime` not `Domain` |

---

## File-by-File Analysis

### SPLIT-01: writ-parser/src/parser.rs (3,345 lines) — MANDATORY SPLIT

**Current structure (verified by source read):**

| Function | Lines (approx) | Visibility | Role |
|----------|---------------|------------|------|
| `TypePostfix` enum | 28-35 | private | Type parsing helper |
| `ExprPostfix` enum | 36-57 | private | Expression parsing helper |
| `type_expr()` | 72-170 | `pub` | Type expression parser |
| `generic_params()` | 176-213 | `pub` | Generic parameter parser |
| `pattern()` | 224-347 | `fn` (private) | Pattern parser for match arms |
| `parse_formattable_string()` | 353-462 | `fn` (private) | String interpolation parser |
| `split_dlg_text_segments()` | 477-607 | `fn` (private) | DLG text interpolation |
| `parse_expr_from_source()` | 613-654 | `fn` (private) | Single-expr parse helper |
| `program_parser()` | 674-3293 | `pub` | Full grammar (2,620 lines!) |
| `parse()` | 3307-end | `pub` | Entry point |

**Recommended split (folder conversion):**
```
writ-parser/src/parser/
├── mod.rs           # TypePostfix, ExprPostfix + pub use parse, type_expr, generic_params, program_parser
├── type_expr.rs     # type_expr() (lines 72-170, ~100 lines)
├── generic_params.rs # generic_params() (lines 176-213, ~38 lines)
├── pattern.rs       # pattern() (lines 224-347, ~124 lines)
├── strings.rs       # parse_formattable_string, split_dlg_text_segments, parse_expr_from_source (~300 lines)
└── program.rs       # program_parser() + parse() (lines 674-end, ~2,640 lines)
```

**Note:** `program_parser()` at 2,640 lines cannot itself be easily split — it is a single `recursive()` closure with deeply nested combinators. It is a Chumsky parser definition where all grammar productions are mutually referential. The file split gives `program.rs` ~2,640 lines (above the 500-line target) but this is a justified exception: splitting Chumsky grammar parsers mid-recursive-closure is not structurally sound. Document rationale.

**Key concern:** `parse_formattable_string` and `split_dlg_text_segments` call `parse_expr_from_source`, which in turn calls `parse()`. If `parse()` is in `mod.rs` and strings.rs imports it, this works — but circular module dependencies must be avoided. Solution: put `parse_expr_from_source` and `parse()` in `program.rs` and have `strings.rs` call `super::program::parse_expr_from_source` (made `pub(super)`).

**External API preserved:**
- `writ_parser::parser::parse` — stays as `pub use program::parse` in `mod.rs`
- `writ_parser::parser::type_expr` — stays as `pub use type_expr::type_expr` in `mod.rs`

---

### SPLIT-02: writ-lsp/src/queries.rs (2,768 lines) — MANDATORY SPLIT

**Current structure — 5 explicit `// =====` section banners:**

| Section | Line range | Public fns | Category |
|---------|-----------|-----------|---------|
| Shared position/walk utilities | 1-272 | `position_to_byte_offset`, `expr_at_offset`, `find_def_id_at_offset` | Core walkers |
| Hover and References | 273-866 | `hover_text_for_expr`, `hover_text_for_def`, `extract_doc_comment`, `pattern_at_offset`, `collect_references` | Hover + refs |
| Binding/Def/TypeAnn fallbacks | 868-1091 | `binding_at_offset`, `def_at_offset`, `type_ann_def_id_at_offset` | Goto-def fallbacks |
| Completion | 1092-1612 | `build_identifier_completions`, `build_dot_completions`, `build_signature_help` | Completions + sighelp |
| Semantic tokens | 1613-1996 | `collect_semantic_tokens`, `collect_dialogue_speaker_tokens` | Semantic tokens |
| Tests | 1998-end | (inline `#[cfg(test)]`) | Tests |

**Recommended split (folder conversion):**
```
writ-lsp/src/queries/
├── mod.rs          # pub use re-exports for all public API + shared types (BindingInfo, RawSemanticToken, PatternHoverInfo)
├── walk.rs         # position_to_byte_offset, expr_at_offset, find_def_id_at_offset, decl_file_id + all find_in_* / update_best helpers (~272 lines)
├── hover.rs        # hover_text_for_expr, hover_text_for_def, extract_doc_comment, pattern_at_offset + format_fn_sig_hover (~594 lines)
├── references.rs   # collect_references + collect_refs_in_* helpers, binding_at_offset, def_at_offset, type_ann_def_id_at_offset (~420 lines)
├── completion.rs   # build_identifier_completions, build_dot_completions, build_signature_help + format_fn_sig_oneliner, extract_callee_name, find_enclosing_call (~520 lines)
└── semantic.rs     # collect_semantic_tokens, collect_dialogue_speaker_tokens + collect_tokens_in_* helpers, RawSemanticToken, token type constants (~383 lines)
```

**Shared helpers:** `decl_file_id` is used by `walk.rs`, `hover.rs`, `references.rs` — keep in `walk.rs` as `pub(super)` so siblings can call `super::walk::decl_file_id`.

**Test placement:** The inline `#[cfg(test)]` block at line 1998 tests `position_to_byte_offset`, hover, and signature help. Keep tests in `mod.rs` (referencing submodules via `use super::*`) or distribute to each submodule. Recommend distributing: move tests to their respective submodule files.

**`mod.rs` re-export pattern:**
```rust
// queries/mod.rs
pub mod walk;
pub mod hover;
pub mod references;
pub mod completion;
pub mod semantic;

pub use walk::{position_to_byte_offset, expr_at_offset, find_def_id_at_offset};
pub use hover::{hover_text_for_expr, hover_text_for_def, extract_doc_comment, pattern_at_offset, PatternHoverInfo};
pub use references::{collect_references, binding_at_offset, def_at_offset, type_ann_def_id_at_offset, BindingInfo};
pub use completion::{build_identifier_completions, build_dot_completions, build_signature_help};
pub use semantic::{collect_semantic_tokens, collect_dialogue_speaker_tokens, RawSemanticToken};
```

**Callers (backend.rs):** All calls to `queries::hover_text_for_expr(...)` etc. continue to work unchanged since `mod.rs` re-exports the full public surface.

---

### SPLIT-06: writ-runtime/src/domain.rs (1,149 lines) — REVIEW (split recommended)

**Current structure:**

| Group | Lines | Content |
|-------|-------|---------|
| Resolution types | 19-83 | `ResolvedType`, `ResolvedMethod`, `ResolvedField`, `ResolvedContract`, `ResolvedRefs` |
| Domain struct | 84-130 | `Domain` struct + `new()`, `add_module()`, `resolve_refs()` |
| Ref resolution | 131-360 | `resolve_module_refs()`, `resolve_parent_type()`, `find_module_by_name()`, `find_type_def_by_name()`, `find_contract_def_by_name()`, `find_method_in_type()`, `find_field_in_type()` |
| Dispatch table | 361-561 | `build_dispatch_table()`, `resolve_type_key()`, `resolve_contract_key_for_impl()`, `get_contract_method_count()`, `get_type_name()` |
| Free function | 563-612 | `resolve_intrinsic_id()` |
| Tests | 614-1149 | 22 inline `#[cfg(test)]` functions (535 lines of tests) |

**Analysis:** The two functional groups (resolution + dispatch) are relatively cohesive but do not call each other directly — `resolve_refs` builds `ResolvedRefs`; `build_dispatch_table` uses `ResolvedRefs` indirectly via the already-resolved `modules[idx].resolved_refs`. This is a clean boundary.

**Recommended approach: sibling file split (Option B):**
```
writ-runtime/src/domain.rs         # Domain struct, ResolvedRefs types, new/add_module/resolve_refs + resolution helpers (~560 lines)
writ-runtime/src/domain_dispatch.rs # build_dispatch_table, resolve_type_key, resolve_contract_key_for_impl, get_contract_method_count, get_type_name, resolve_intrinsic_id (~250 lines)
```

In `domain.rs`: `mod domain_dispatch;` + `pub use domain_dispatch::...` for items the runtime uses. Tests stay in `domain.rs` (they test both halves equally and use `use super::*`).

**Alternative — no split with documented rationale:** The file is 1,149 lines with 535 lines of tests. The test section alone accounts for 46% of the file. The two functional groups are each under 300 lines of code. "No split" is defensible and follows the Phase 63 pattern for files under 1,200 lines where the test block is the primary driver of size.

**Recommendation:** Split is worth doing. The dispatch table construction logic (`build_dispatch_table`, `resolve_intrinsic_id`) is conceptually distinct from ref resolution and the `resolve_intrinsic_id` match at 50 lines is a natural extraction point.

---

### SPLIT-07: writ-dap/src/server.rs (1,140 lines) — MANDATORY SPLIT

**Current structure:**

| Group | Lines | Content |
|-------|-------|---------|
| Free helper fns | 22-145 | `decode_frame_id`, `build_thread_list`, `collect_frame_variables`, `instr_to_byte_pc`, `evaluate_local` |
| `DapServer` struct | 146-176 | Struct + fields |
| `run()` + `handle_request()` | 177-571 | Main dispatch loop + 15 `Command::*` arms |
| Internal helpers | 572-929 | `run_until_stop`, `resolve_task_id`, `count_active_locals`, `get_variables`, `do_evaluate`, `build_stack_frames`, `current_position` |
| Constructor | 928-930 | `pub fn stdio_server()` |
| Tests | 946-end | Inline tests |

**`handle_request` Command arms (15 handlers):**
- `Initialize` — send capabilities
- `SetBreakpoints` — pre/post-launch breakpoint management
- `ConfigurationDone` — acknowledge
- `Launch` — compile and start execution
- `Threads` — list active tasks
- `StackTrace` — build frame list
- `Scopes` — variable scope structure
- `Variables` — variable values
- `Continue` — resume execution
- `Next` / `StepIn` / `StepOut` — stepping
- `Evaluate` — expression evaluation
- `Disconnect` — shutdown
- `Terminate` — terminate session

**Recommended split (folder conversion):**
```
writ-dap/src/server/
├── mod.rs           # DapServer struct, run(), handle_request() dispatch match, stdio_server()
├── handlers.rs      # All Command::* arm bodies extracted as private methods on DapServer
│                    #   OR: handle_initialize, handle_set_breakpoints, handle_launch,
│                    #       handle_threads, handle_stack_trace, handle_scopes,
│                    #       handle_variables, handle_continue, handle_step, handle_evaluate
├── helpers.rs       # Free functions: decode_frame_id, build_thread_list, collect_frame_variables,
│                    #   instr_to_byte_pc, evaluate_local (pub(crate) stays as-is)
└── inspection.rs    # run_until_stop, build_stack_frames, current_position, get_variables,
                     #   resolve_task_id, count_active_locals, do_evaluate
```

**Alternative approach — fewer files:** Extract just the free helpers and keep the rest together:
```
writ-dap/src/server/
├── mod.rs     # DapServer struct + handle_request + all impl methods (~800 lines, still large)
└── helpers.rs # decode_frame_id, build_thread_list, collect_frame_variables, instr_to_byte_pc, evaluate_local (~145 lines)
```

**Recommendation:** Use the 3-file approach (mod.rs + handlers.rs + inspection.rs). The `handle_request` match dispatch stays in `mod.rs` as orchestration; each arm calls a method like `self.handle_launch(req, args)` defined in `handlers.rs`.

**Test consideration:** Inline `#[cfg(test)]` tests in `server.rs` test `decode_frame_id`, `build_thread_list`, `collect_frame_variables`, `instr_to_byte_pc`. These tests should follow the helpers to `helpers.rs`.

**External API:** `writ_dap::server::DapServer` and `writ_dap::server::stdio_server` stay in `server/mod.rs` — paths unchanged.

---

### SPLIT-12: writ-lsp/src/analysis_host.rs (1,415 lines) — REVIEW

**Current structure:**

| Group | Lines | Content |
|-------|-------|---------|
| `AnalysisResult` struct | 10-21 | Result type |
| `AnalysisHost` struct | 23-26 | Stateless unit struct |
| `analyze_standalone` | 37-125 | Single-file pipeline |
| `analyze_project` | 127-352 | Multi-file project pipeline (225 lines) |
| Private helpers | 354-388 | `internal_stage_panic_diag`, `io_error_diag`, `config_error_diag` (35 lines) |
| Tests | 390-1415 | 1,025 lines of inline integration tests |

**Analysis:** The test block is 72% of the file. The actual production code is only ~390 lines — well under the 500-line target. A split here would move tests to a separate integration test file rather than splitting functional code.

**Recommendation: No split — document rationale.** The production code section is cohesive (single `AnalysisHost` struct with two public methods) and already small. If test extraction were desired, tests could move to `writ-lsp/tests/test_analysis_host.rs` but this goes against the Phase 64 success criteria which specify "all existing tests pass without modification."

**Rationale comment to add:**
```
## SPLIT-12 review (Phase 64)
Reviewed for split opportunities at 1,415 lines. Conclusion: no split.
The production code section is ~390 lines (one struct, two public methods, three
private helpers). The remaining 1,025 lines are inline integration tests. Splitting
the test block into a separate integration test file would require test file
modification (violating the "tests pass without modification" success criterion).
Splitting the production code alone would fragment a tightly coupled, sequential
5-stage pipeline analysis (parse → lower → resolve → typecheck) that shares local
variables across all stages.
```

---

### SPLIT-13: writ-lsp/src/backend.rs (888 lines) — REVIEW

**Current structure:**

| Group | Lines | Content |
|-------|-------|---------|
| `Backend` struct | 1-47 | Struct + `new()` |
| `LanguageServer` impl | 49-651 | `initialize`, `shutdown`, `did_open`, `did_change`, `did_save`, `did_close`, `hover`, `goto_definition`, `references`, `completion`, `signature_help`, `semantic_tokens_full` (12 async handlers) |
| Private `impl Backend` | 653-843 | `publish_diagnostics_for`, `publish_grouped_diagnostics`, `identifier_completion` |
| Free functions | 845-888 | `resolve_trigger_file_id`, `display_path_to_url` |

**Analysis:** The 12 async LSP handlers in the `LanguageServer` impl cannot be moved to sibling files because Rust requires all methods of a trait impl to be in the same impl block. Splitting async trait methods across files with `mod` boundaries is only possible by delegating to helper functions — meaning the split adds indirection but doesn't reduce the impl block size.

**Recommendation: No split — document rationale.** The file is 888 lines total; the practical code size is reasonable. The `LanguageServer` trait constraint (tower-lsp) requires all async handlers in a single `impl` block. The delegation pattern would just add boilerplate.

**Rationale comment to add:**
```
## SPLIT-13 review (Phase 64)
Reviewed for split opportunities at 888 lines. Conclusion: no split.
The `LanguageServer` trait impl (tower-lsp) requires all 12 async handlers in a
single `impl LanguageServer for Backend` block — Rust does not allow splitting
a trait impl across files. A delegation pattern (each handler calls a helper in
a separate module) would add boilerplate without reducing cognitive load. The
private `impl Backend` section (publish_diagnostics_for, publish_grouped_diagnostics,
identifier_completion) is already at natural granularity.
```

---

### SPLIT-14: writ-cli/src/main.rs (703 lines) — MANDATORY SPLIT

**Current structure:**

| Group | Lines | Content |
|-------|-------|---------|
| CLI definition | 23-112 | `Cli` + `Commands` clap structs |
| `main()` | 115-133 | Dispatch match |
| `cmd_new` | 137-306 | Project scaffolding (170 lines) |
| `cmd_new` test | 288-307 | Inline test (20 lines) |
| `run_pipeline` | 321-403 | Shared 5-stage compile helper (83 lines) |
| `cmd_build` | 407-482 | Multi-file project compile (76 lines) |
| `cmd_compile` | 485-537 | Single file compile (53 lines) |
| `cmd_assemble` | 540-586 | Assemble .writil → .writc (47 lines) |
| `cmd_disasm` | 589-607 | Disassemble .writc → .writil (19 lines) |
| `cmd_run` | 610-703 | Execute .writc module (94 lines) |

**`writ-cli` is a `[[bin]]` crate (not `lib`).** There is no `lib.rs`, so there is no public API surface to preserve for external tests. The integration tests at `writ-cli/tests/cli_integration.rs` and `e2e_compile_tests.rs` invoke the `writ` binary via `std::process::Command` — they do not import Rust symbols from `writ-cli`.

**This changes the split pattern significantly:** Since `main.rs` is a binary crate, submodules are declared with `mod` in `main.rs` and become siblings (`src/commands/new.rs`, etc.). The `Commands` clap enum must stay in `main.rs` but handler function bodies can move.

**Recommended split (sibling modules declared from main.rs):**
```
writ-cli/src/
├── main.rs              # Cli/Commands structs + main() + mod declarations (~50 lines)
├── bom_utils.rs         # Existing — unchanged
├── cli_host.rs          # Existing — unchanged
├── pipeline.rs          # run_pipeline() (~83 lines)
└── commands/
    ├── mod.rs           # Re-exports for cmd_* functions
    ├── new.rs           # cmd_new() (~170 lines)
    ├── build.rs         # cmd_build() (~76 lines)
    ├── compile.rs       # cmd_compile() (~53 lines)
    ├── assemble.rs      # cmd_assemble() (~47 lines)
    ├── disasm.rs        # cmd_disasm() (~19 lines)
    └── run.rs           # cmd_run() (~94 lines)
```

**In `main.rs`:**
```rust
mod cli_host;
mod bom_utils;
mod pipeline;
mod commands;

use commands::{cmd_new, cmd_build, cmd_compile, cmd_assemble, cmd_disasm, cmd_run};
```

**Each command file** needs `use crate::pipeline::run_pipeline` or `use super::super::pipeline::run_pipeline` as appropriate. Since `pipeline.rs` is a top-level sibling, commands use `use crate::pipeline::run_pipeline`.

**`cmd_new` inline test:** The test `#[cfg(test)] fn main() { ... }` inside `cmd_new` can move to `commands/new.rs` — no change to external test files.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rust circular module deps | Manual indirection layers | Restructure with `pub(super)` and `pub(crate)` | Rust's module system forbids circular `mod` imports |
| Chumsky parser splitting | Try to split recursive closures | Keep `program_parser()` whole in `program.rs` | Chumsky's `recursive()` requires the closure to see all parsers simultaneously |
| tower-lsp async trait splitting | Move async handlers to separate files | Keep `impl LanguageServer` whole in `backend.rs` | Rust trait impls cannot be split across files |

**Key insight:** File splits in Rust are limited by the language's module and trait rules. The planner must identify which boundaries are genuinely clean vs. which appear clean but hit language constraints (async trait impls, recursive closures).

---

## Common Pitfalls

### Pitfall 1: Circular Module References
**What goes wrong:** `strings.rs` calls `parse()` which is in `program.rs`. If both are modules of `parser`, Rust forbids `strings` importing from `program` if `program` also imports from `strings`.
**How to avoid:** Put `parse_expr_from_source` in `program.rs` (not `strings.rs`). `strings.rs` accepts the function as a parameter, or calls it via `pub(super)` path: `super::program::parse_expr_from_source`.
**Detection:** `cargo build` fails with "cycle detected" or "unresolved import" errors.

### Pitfall 2: Breaking External Test Paths
**What goes wrong:** Splitting `parser.rs` into `parser/` changes the module — but if `parse` is not re-exported from `parser/mod.rs`, the import `writ_parser::parser::parse` breaks.
**How to avoid:** Every public function from the original file must have an explicit `pub use submodule::fn_name;` in `mod.rs`. Run `cargo test --workspace` after each split before moving on.
**Detection:** Integration test failures with "unresolved import" or "no function `parse` in module `parser`".

### Pitfall 3: Private Function Visibility Across Submodules
**What goes wrong:** Moving `decl_file_id` from `queries.rs` to `queries/walk.rs` makes it `fn decl_file_id` (private to `walk`). `hover.rs` then cannot call it.
**How to avoid:** Change to `pub(super)` in `walk.rs` — then `hover.rs` (a sibling of `walk.rs`) can use `super::walk::decl_file_id`.
**Detection:** `cargo build` fails with "function `decl_file_id` is private" errors.

### Pitfall 4: Inline Tests Become Invalid After Split
**What goes wrong:** An inline `#[cfg(test)]` block uses `use super::*` — after splitting, `super` refers to the submodule not the crate root, missing imports from other submodules.
**How to avoid:** When distributing tests, update `use super::*` to explicit imports or add `use super::super::*` as needed. Alternatively, consolidate tests in `mod.rs` with explicit `use crate::queries::walk::*` style imports.
**Detection:** Test compilation errors after split.

### Pitfall 5: Clippy Warnings from New File Structure
**What goes wrong:** Moving `#[allow(dead_code)]` constants (`TOKEN_TYPE_KEYWORD`, etc. in `queries.rs`) to a new submodule may cause clippy to re-evaluate and produce new warnings.
**How to avoid:** Check `cargo clippy --workspace` after each split. The constants in `queries.rs` already have `#[allow(dead_code)]` annotations — ensure these travel with the constants to `semantic.rs`.
**Detection:** `cargo clippy` exits non-zero after a split that previously left it clean.

---

## Code Examples

### Folder Module Declaration Pattern
```rust
// queries/mod.rs
// Source: Phase 63 established pattern (63-RESEARCH.md)

pub mod walk;
pub mod hover;
pub mod references;
pub mod completion;
pub mod semantic;

// Re-export the full public surface so callers don't change
pub use walk::{position_to_byte_offset, expr_at_offset, find_def_id_at_offset};
pub use hover::{hover_text_for_expr, hover_text_for_def, extract_doc_comment, pattern_at_offset, PatternHoverInfo};
pub use references::{collect_references, binding_at_offset, def_at_offset, type_ann_def_id_at_offset, BindingInfo};
pub use completion::{build_identifier_completions, build_dot_completions, build_signature_help};
pub use semantic::{collect_semantic_tokens, collect_dialogue_speaker_tokens, RawSemanticToken};
```

### Cross-Submodule Access Pattern
```rust
// queries/hover.rs — calling a helper from walk.rs
// Source: Phase 63 established pattern

pub(super) fn decl_file_id_in_hover(decl: &TypedDecl, def_map: &DefMap) -> FileId {
    // Use walk module's helper via pub(super) path
    super::walk::decl_file_id(decl, def_map)
}
```

### Sibling File Declaration in Binary Crate
```rust
// writ-cli/src/main.rs
mod cli_host;   // existing
mod bom_utils;  // existing
mod pipeline;   // NEW: run_pipeline
mod commands;   // NEW: folder module

use commands::{cmd_new, cmd_build, cmd_compile, cmd_assemble, cmd_disasm, cmd_run};
use cli_host::CliHost;
use bom_utils::{strip_bom_and_decode, add_utf8_bom};
```

### "No Split" Rationale Comment Pattern
```rust
//! ## SPLIT-XX review (Phase 64)
//!
//! Reviewed for split opportunities at N lines. Conclusion: no split.
//! [reason: tight coupling / single concept / test-dominated / trait constraint / etc.]
```

---

## Recommended Plan Structure

Based on file sizes and cohesion analysis, Phase 64 should have 3-4 plans:

| Plan | Files | Requirements | Approximate size |
|------|-------|-------------|-----------------|
| 64-01 | `writ-parser/src/parser.rs` → `parser/` folder | SPLIT-01 | Large (3,345 lines → 6 files) |
| 64-02 | `writ-lsp/src/queries.rs` → `queries/` folder | SPLIT-02 | Large (2,768 lines → 6 files) |
| 64-03 | `writ-dap/src/server.rs` → `server/` folder; `writ-runtime/src/domain.rs` → `domain.rs + domain_dispatch.rs`; `writ-cli/src/main.rs` split | SPLIT-06, SPLIT-07, SPLIT-14 | Medium (3 separate splits) |
| 64-04 | Document rationale for `analysis_host.rs` (SPLIT-12), `backend.rs` (SPLIT-13); full workspace verification | SPLIT-12, SPLIT-13 | Light (rationale comments + test run) |

---

## State of the Art

| Old Approach | Current Approach | Phase 63 Outcome |
|--------------|------------------|-----------------|
| Single monolithic `check_expr.rs` (2,155 lines) | `check_expr/` folder with 9 subfiles | Complete |
| Single monolithic `collect.rs` (1,700 lines) | `collect/` folder with 9 subfiles | Complete |
| Single monolithic `body/expr.rs` (1,478 lines) | `expr/` folder with category files | Complete |
| `module_builder.rs`, `dialogue.rs`, `resolver.rs` | Reviewed, documented rationale, kept intact | Complete |

Phase 64 follows the same proven workflow: inventory sections → decide split/no-split → folder conversion → verify public API stability → `cargo test --workspace` → `cargo clippy --workspace`.

---

## Validation Architecture

> `workflow.nyquist_validation` key is absent from `.planning/config.json` — treating as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in) |
| Config file | workspace `Cargo.toml` |
| Quick run command | `cargo test -p writ-parser -p writ-lsp -p writ-dap -p writ-runtime -p writ-cli` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPLIT-01 | parser.rs split; `parse()` path preserved | integration | `cargo test -p writ-parser` | ✅ `writ-parser/tests/parser_tests.rs` |
| SPLIT-02 | queries.rs split; all query fns accessible | integration | `cargo test -p writ-lsp` | ✅ `writ-lsp/tests/test_hover_protocol.rs` |
| SPLIT-06 | domain.rs split; resolution + dispatch work | unit + integration | `cargo test -p writ-runtime` | ✅ `writ-runtime/tests/vm_tests.rs` + inline |
| SPLIT-07 | server.rs split; DAP handlers intact | integration | `cargo test -p writ-dap` | ✅ `writ-dap/tests/test_initialize_sequence.rs` etc. |
| SPLIT-12 | analysis_host.rs reviewed, rationale documented | n/a (doc only) | `cargo build -p writ-lsp` | ✅ inline tests |
| SPLIT-13 | backend.rs reviewed, rationale documented | n/a (doc only) | `cargo build -p writ-lsp` | ✅ no separate test file |
| SPLIT-14 | main.rs split; CLI e2e tests pass | e2e | `cargo test -p writ-cli` | ✅ `writ-cli/tests/cli_integration.rs` |

### Sampling Rate
- **Per task commit:** `cargo test -p <crate_being_split>` (quick feedback)
- **Per plan completion:** `cargo test --workspace && cargo clippy --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
None — existing test infrastructure covers all phase requirements. No new test files need to be created for this structural refactoring phase.

---

## Open Questions

1. **`program_parser()` line count after split**
   - What we know: `program_parser()` + `parse()` together are ~2,640 lines — far above the 500-line target
   - What's unclear: Whether the planner should document an explicit exception or attempt a sub-split
   - Recommendation: Document the exception. Chumsky's `recursive()` combinator creates a single closure scope where all grammar productions must be visible simultaneously. A "sub-split" would require either duplicating the recursive type or indirection that degrades parser performance. The 500-line target is a guideline; `program.rs` as a documented exception is correct.

2. **`domain.rs` inline tests after split**
   - What we know: 22 tests spanning 535 lines test both resolution and dispatch table logic using `use super::*`
   - What's unclear: Whether tests should be split with the code or stay in `domain.rs`
   - Recommendation: Keep all tests in `domain.rs`. `use super::*` will still import from the `domain` module root. Tests reference both `Domain` (resolution) and `DispatchTable` (dispatch) — keeping them together avoids test-file modification.

3. **`queries.rs` shared `PatternHoverInfo` struct placement**
   - What we know: `PatternHoverInfo` is defined near line 558 in the hover section, but used only by `pattern_at_offset` (hover) and called from `backend.rs`
   - What's unclear: Whether it belongs in `hover.rs` or `mod.rs`
   - Recommendation: Define in `hover.rs`, re-export from `mod.rs`. Backend.rs uses `queries::PatternHoverInfo` — this path is stable with the re-export.

---

## Sources

### Primary (HIGH confidence)
- Direct file reads — `writ-parser/src/parser.rs`, `writ-lsp/src/queries.rs`, `writ-lsp/src/analysis_host.rs`, `writ-lsp/src/backend.rs`, `writ-dap/src/server.rs`, `writ-runtime/src/domain.rs`, `writ-cli/src/main.rs`
- Phase 63 established patterns — `63-RESEARCH.md`, `63-01-PLAN.md` through `63-04-PLAN.md`
- `.planning/REQUIREMENTS.md` — requirement definitions and line counts
- `.planning/STATE.md` — phase decisions and ordering rationale

### Secondary (MEDIUM confidence)
- Rust Reference on modules (well-established language semantics — folder modules, pub(super), trait impl constraints)
- Chumsky library behavior (recursive combinator closure scope — consistent with codebase usage patterns)

### Tertiary (LOW confidence)
- None — all findings are verified against source files directly

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — project uses standard Rust toolchain, no new deps
- File split boundaries (SPLIT-01, SPLIT-02, SPLIT-14): HIGH — natural section markers already present in source
- File split boundaries (SPLIT-06, SPLIT-07): HIGH — clear functional groups identified by direct read
- No-split rationale (SPLIT-12, SPLIT-13): HIGH — language constraints (trait impl, test-dominated) are definitive
- Architecture patterns: HIGH — established in Phase 63, directly applicable

**Research date:** 2026-03-18
**Valid until:** Stable (pure Rust module mechanics don't change; source files are stable between phases)
