---
phase: 64-cross-crate-file-splits
verified: 2026-03-18T05:00:00Z
status: passed
score: 7/7 must-haves verified
re_verification: null
gaps: []
human_verification: []
---

# Phase 64: Cross-Crate File Splits Verification Report

**Phase Goal:** All oversized files in writ-parser, writ-lsp, writ-dap, writ-runtime, and writ-cli are split into focused submodules
**Verified:** 2026-03-18
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `writ_parser::parser::parse` is callable from external test files | VERIFIED | `writ-parser/src/lib.rs` still contains `pub use parser::parse` (line 8); parser is folder-backed — transparent to callers |
| 2 | `writ_parser::parser::type_expr` is callable from external code | VERIFIED | `pub fn type_expr` in `parser/type_expr.rs` line 22; re-exported via `pub use type_expr::type_expr` in `mod.rs` line 62 |
| 3 | All backend.rs calls to `crate::queries::*` functions resolve correctly | VERIFIED | All `crate::queries::position_to_byte_offset`, `hover_text_for_def`, `expr_at_offset` etc. resolve through explicit re-exports in `queries/mod.rs` lines 15–37; `backend.rs` is unchanged |
| 4 | `writ_dap::server::DapServer` is still accessible from external test files | VERIFIED | `pub struct DapServer` in `server/mod.rs` line 26; `pub fn stdio_server` line 153; `pub mod server` in `lib.rs` unchanged |
| 5 | `writ_runtime::Domain` and all `ResolvedRefs` types are re-exported from domain module | VERIFIED | `pub use domain::{Domain, ResolvedRefs, ResolvedType, ResolvedMethod, ResolvedField}` in `lib.rs` line 25; unchanged |
| 6 | writ-cli binary compiles with all command functions accessible | VERIFIED | `commands::cmd_new`, `cmd_build`, `cmd_compile`, `cmd_assemble`, `cmd_disasm`, `cmd_run` all called from `main()` via `commands::` module path |
| 7 | SPLIT-12 and SPLIT-13 no-split rationale documented in source files | VERIFIED | `analysis_host.rs` line 6 contains `## SPLIT-12 review (Phase 64)` with "Conclusion: no split" and "1,025 lines are inline integration tests"; `backend.rs` line 7 contains `## SPLIT-13 review (Phase 64)` with "Conclusion: no split" |

**Score:** 7/7 truths verified

---

## Required Artifacts

### SPLIT-01: writ-parser parser/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-parser/src/parser.rs` | DELETED | VERIFIED | File does not exist |
| `writ-parser/src/parser/mod.rs` | TypePostfix, ExprPostfix + re-exports | VERIFIED | 62 lines; `pub use program::{parse, program_parser}`, `pub use type_expr::type_expr`, `pub use generic_params::generic_params` |
| `writ-parser/src/parser/type_expr.rs` | `pub fn type_expr()` | VERIFIED | 120 lines; `pub fn type_expr<'tokens, 'src: 'tokens, I>()` at line 22 |
| `writ-parser/src/parser/generic_params.rs` | `pub fn generic_params()` | VERIFIED | 50 lines; `pub fn generic_params<...>()` at line 14 |
| `writ-parser/src/parser/pattern.rs` | `pub(super) fn pattern()` | VERIFIED | 141 lines; `pub(super) fn pattern<...>()` at line 19 |
| `writ-parser/src/parser/program.rs` | program_parser, parse, helpers + documented exception | VERIFIED | 2,976+ lines; NOTE comment at line 1; all 5 required functions present |

### SPLIT-02: writ-lsp queries/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/src/queries.rs` | DELETED | VERIFIED | File does not exist |
| `writ-lsp/src/queries/mod.rs` | Re-exports for all public functions | VERIFIED | 37 lines; 20 explicit `pub use` re-exports — no glob exports |
| `writ-lsp/src/queries/walk.rs` | `pub fn position_to_byte_offset`, `pub(super) fn decl_file_id` | VERIFIED | 382 lines; both functions present at lines 19 and 52 |
| `writ-lsp/src/queries/hover.rs` | `pub fn hover_text_for_expr`, `pub struct PatternHoverInfo` | VERIFIED | 476 lines; at lines 19 and 285 |
| `writ-lsp/src/queries/references.rs` | `pub fn collect_references`, `pub struct BindingInfo` | VERIFIED | 562 lines; at lines 19 and 221 |
| `writ-lsp/src/queries/completion.rs` | `pub fn build_identifier_completions`, `pub fn build_signature_help` | VERIFIED | 720 lines; at lines 18 and 268 |
| `writ-lsp/src/queries/semantic.rs` | `pub fn collect_semantic_tokens`, `pub struct RawSemanticToken` | VERIFIED | 622 lines; at lines 49 and 17 |

### SPLIT-06: writ-runtime domain sibling split

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/src/domain.rs` | Has `resolve_refs`, lacks `build_dispatch_table` | VERIFIED | `pub fn resolve_refs` at line 122; `build_dispatch_table` NOT present |
| `writ-runtime/src/domain_dispatch.rs` | `fn build_dispatch_table`, `fn resolve_intrinsic_id`, `impl Domain` | VERIFIED | 267 lines; `impl Domain` at line 17; `pub fn build_dispatch_table` at line 23; `pub fn resolve_intrinsic_id` at line 222 |
| `writ-runtime/src/lib.rs` | `mod domain_dispatch` added | VERIFIED | `mod domain_dispatch` at line 10 (private); `pub use domain::{Domain, ResolvedRefs, ...}` at line 25 unchanged |

### SPLIT-07: writ-dap server/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-dap/src/server.rs` | DELETED | VERIFIED | File does not exist |
| `writ-dap/src/server/mod.rs` | `pub struct DapServer`, `pub fn stdio_server`, module decls | VERIFIED | 158 lines; struct at line 26; stdio_server at line 153; `mod handlers`, `mod helpers`, `mod inspection` at lines 17-19 |
| `writ-dap/src/server/handlers.rs` | `impl DapServer` with handler methods | VERIFIED | 375 lines; `impl<I: Read, O: Write> DapServer<I, O>` at line 19; `handle_set_breakpoints` at line 31; `handle_launch` at line 99 |
| `writ-dap/src/server/helpers.rs` | `fn decode_frame_id`, `pub(crate) fn collect_frame_variables` | VERIFIED | 332 lines; `pub(super) fn decode_frame_id` at line 14; `pub(crate) fn collect_frame_variables` at line 51 |
| `writ-dap/src/server/inspection.rs` | `fn run_until_stop` | VERIFIED | 376 lines; `pub(super) fn run_until_stop` at line 22 |

### SPLIT-12 and SPLIT-13: No-split rationale (writ-lsp)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/src/analysis_host.rs` | SPLIT-12 review comment | VERIFIED | Line 6: `## SPLIT-12 review (Phase 64)`; line 8: "Conclusion: no split"; line 11: "1,025 lines are inline integration tests" |
| `writ-lsp/src/backend.rs` | SPLIT-13 review comment | VERIFIED | Line 7: `## SPLIT-13 review (Phase 64)`; line 9: "Conclusion: no split"; line 10–11: tower-lsp LanguageServer trait impl rationale |

### SPLIT-14: writ-cli commands/ folder

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-cli/src/main.rs` | Has `mod commands`, `mod pipeline`, `fn main()`, lacks `fn cmd_new`, lacks `fn run_pipeline` | VERIFIED | `mod pipeline` at line 12; `mod commands` at line 13; `fn main()` at line 111; no `fn cmd_new` or `fn run_pipeline` in file |
| `writ-cli/src/pipeline.rs` | `pub fn run_pipeline` | VERIFIED | 96 lines; `pub fn run_pipeline` at line 14 |
| `writ-cli/src/commands/mod.rs` | Re-exports for all 6 `cmd_*` functions | VERIFIED | 13 lines; all 6 `pub use` re-exports present |
| `writ-cli/src/commands/new.rs` | `fn cmd_new` | VERIFIED | 172 lines; `pub fn cmd_new` at line 3 |
| `writ-cli/src/commands/build.rs` | `fn cmd_build` | VERIFIED | 80 lines; present |
| `writ-cli/src/commands/compile.rs` | `fn cmd_compile` | VERIFIED | 57 lines; present |
| `writ-cli/src/commands/assemble.rs` | `fn cmd_assemble` | VERIFIED | 52 lines; present |
| `writ-cli/src/commands/disasm.rs` | `fn cmd_disasm` | VERIFIED | 24 lines; present |
| `writ-cli/src/commands/run.rs` | `fn cmd_run` | VERIFIED | 101 lines; present |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-parser/src/lib.rs` | `parser/mod.rs` | `pub use parser::parse` | WIRED | Line 8 of lib.rs; folder module semantics transparent to callers |
| `parser/mod.rs` | `parser/program.rs` | `pub use program::{parse, program_parser}` | WIRED | Line 61 of mod.rs |
| `writ-lsp/src/backend.rs` | `queries/mod.rs` | `crate::queries::` call sites | WIRED | Confirmed: `position_to_byte_offset`, `binding_at_offset`, `def_at_offset`, `hover_text_for_def`, `expr_at_offset` all resolve via mod.rs re-exports |
| `queries/hover.rs` | `queries/walk.rs` | `super::walk::decl_file_id` | WIRED | `pub(super) fn decl_file_id` at walk.rs line 52; accessible across sibling boundary |
| `server/mod.rs` | `server/handlers.rs` | `self.handle_*()` dispatch calls | WIRED | 12+ `self.handle_*` calls in mod.rs dispatch match block confirmed |
| `writ-runtime/src/lib.rs` | `domain.rs` | `pub use domain::{Domain, ResolvedRefs, ...}` | WIRED | Line 25 of lib.rs unchanged |
| `lib.rs` | `domain_dispatch.rs` | `mod domain_dispatch` | WIRED | Line 10 of lib.rs; private module declaration giving `impl Domain` access to crate |
| `writ-cli/src/main.rs` | `commands/mod.rs` | `commands::cmd_new`, etc. | WIRED | Lines 115–121 of main.rs; all 6 cmd_* calls confirmed |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| SPLIT-01 | 64-01 | `writ-parser/src/parser.rs` split into logical submodules | SATISFIED | parser/ folder with 5 files; parser.rs deleted; all API paths preserved |
| SPLIT-02 | 64-02 | `writ-lsp/src/queries.rs` split into hover/completion/reference/semantic modules | SATISFIED | queries/ folder with 6 files; queries.rs deleted; all crate::queries::* paths stable |
| SPLIT-06 | 64-03 | `writ-runtime/src/domain.rs` split into resolution/dispatch modules | SATISFIED | domain.rs retains resolution; domain_dispatch.rs holds dispatch table + resolve_intrinsic_id |
| SPLIT-07 | 64-03 | `writ-dap/src/server.rs` split into request handler modules | SATISFIED | server/ folder with 4 files; server.rs deleted; DapServer path unchanged |
| SPLIT-12 | 64-04 | `writ-lsp/src/analysis_host.rs` reviewed for split opportunities | SATISFIED | Structured `## SPLIT-12 review (Phase 64)` comment with rationale added to module //! doc block |
| SPLIT-13 | 64-04 | `writ-lsp/src/backend.rs` reviewed for split opportunities | SATISFIED | Structured `## SPLIT-13 review (Phase 64)` comment with rationale added to module //! doc block |
| SPLIT-14 | 64-03 | `writ-cli/src/main.rs` split into command modules | SATISFIED | main.rs + pipeline.rs + commands/ folder (6 command files + mod.rs) |

No orphaned requirements. REQUIREMENTS.md phase mapping table confirms all 7 IDs assigned to Phase 64 and marked Complete.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-cli/src/commands/new.rs` | 155 | `// TODO: Add your code here` | Info | Inside a Writ source code template string literal (project scaffold content written to `main.writ`); not an implementation stub — this is intentional user-facing scaffolding |

No blockers. No warnings. The single Info item is a scaffold comment embedded in a string literal, not a code implementation gap.

---

## Structural Quality Notes

**No glob re-exports:** All new `mod.rs` files use explicit `pub use module::item` re-exports — no `pub use module::*` glob patterns. This maintains clear API surfaces.

**Documented exception:** `writ-parser/src/parser/program.rs` (~2,976 lines) exceeds the 500-line target. The documented exception comment at line 1 correctly explains the Chumsky `recursive()` combinator constraint that requires all grammar productions to be visible in a single closure scope.

**File size compliance:** All other new files are well within target limits. The `queries/completion.rs` (720 lines) and `queries/semantic.rs` (622 lines) exceed the 600-line guideline only because of distributed inline tests; production code sections are 531 and ~395 lines respectively.

**Commits verified:** All 6 commits from summaries (8614dba, df4cf7c, a490350, 08827e4, 4b3edd1, ced63db) exist in git history.

---

## Human Verification Required

None. All phase 64 goal criteria are fully verifiable from the codebase structure and file content. The workspace test and clippy gate was confirmed passing by the Plan 04 Task 2 execution (`cargo test --workspace` and `cargo clippy --workspace -- -D warnings` both exit 0 per the summary).

---

_Verified: 2026-03-18T05:00:00Z_
_Verifier: Claude (gsd-verifier)_
