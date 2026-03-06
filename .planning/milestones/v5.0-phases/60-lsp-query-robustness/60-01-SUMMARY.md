---
phase: 60-lsp-query-robustness
plan: 01
subsystem: lsp
tags: [rust, lsp, type-checker, ir, queries, hover, goto-def, find-refs]

# Dependency graph
requires:
  - phase: 59-vsix-release-build
    provides: Compiled VSIX and LSP server binary
  - phase: 54-lsp-navigation-and-completions
    provides: queries.rs with expr_at_offset, hover, goto-def, find-refs infrastructure
provides:
  - TypedStmt::Let with type_ann_span and type_ann_def_id fields
  - TypedDecl::Fn with param_name_spans field
  - binding_at_offset(), def_at_offset(), type_ann_def_id_at_offset() in queries.rs
  - Fallback chains in hover, goto-def, and references LSP handlers
affects: [lsp-features, uat-gaps]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "LSP fallback chain: expr_at_offset -> binding_at_offset -> None"
    - "IR carries LSP metadata: name_span, type_ann_span, type_ann_def_id, param_name_spans"
    - "Query functions return Option<T> and are composed via or_else() chains"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs
    - writ-lsp/src/queries.rs
    - writ-lsp/src/backend.rs

key-decisions:
  - "binding_at_offset takes &TypeEnv not &FxHashMap<DefId, FnSig> — avoids re-exporting rustc_hash from writ-lsp and aligns with existing handler signatures"
  - "TypedDecl::Fn param_name_spans is Vec<SimpleSpan> in declaration order — positional indexing aligns with FnSig.params vec"
  - "def_at_offset skips FileId(u32::MAX) synthetic builtins — consistent with existing goto-def builtin filter"
  - "hover fallback uses binding_at_offset only when expr hover is empty/None — avoids false positives from Block nodes"

patterns-established:
  - "LSP queries compose as: try expr_at_offset, else binding_at_offset / type_ann_def_id_at_offset / def_at_offset"
  - "IR fields for LSP use Option<SimpleSpan> — absent when not present in source (inferred types, missing annotations)"

requirements-completed: [LSP-04, LSP-05, LSP-06]

# Metrics
duration: 26min
completed: 2026-03-17
---

# Phase 60 Plan 01: LSP Query Robustness Summary

**Three new LSP query functions (binding_at_offset, def_at_offset, type_ann_def_id_at_offset) and fallback chains in hover/goto-def/find-refs handlers close UAT gaps 7, 8, 9 for declaration-site hover and navigation**

## Performance

- **Duration:** 26 min
- **Started:** 2026-03-17T09:07:53Z
- **Completed:** 2026-03-17T09:33:53Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Extended TypedStmt::Let with `type_ann_span` and `type_ann_def_id` so the type checker records where type annotations are in source
- Extended TypedDecl::Fn with `param_name_spans` so LSP can map fn parameter names to their source positions
- Added `BindingInfo` struct and `binding_at_offset()` for hover on let-binding names and fn param names
- Added `def_at_offset()` for goto-def and find-refs from declaration names (not just use sites)
- Added `type_ann_def_id_at_offset()` for goto-def on type annotations in let bindings
- Wired all three fallbacks into backend.rs hover, goto_definition, and references handlers
- 8 new query unit tests all pass; 0 regressions in the 500+ existing tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Add TypedStmt::Let fields + three new query functions** - `3d5e6ec` (feat)
2. **Task 2: Wire fallback chains into backend handlers** - `18b9b65` (feat)

## Files Created/Modified

- `writ-compiler/src/check/ir.rs` - Added `type_ann_span`, `type_ann_def_id` to TypedStmt::Let; added `param_name_spans` to TypedDecl::Fn
- `writ-compiler/src/check/check_stmt.rs` - Populate new Let fields by capturing annotation span and DefId
- `writ-compiler/src/check/check_decl.rs` - Populate param_name_spans from AstFnDecl.params
- `writ-compiler/src/emit/body/mod.rs` - Add `..` to TypedDecl::Fn match to handle new field
- `writ-compiler/tests/emit_body_tests.rs` - Add `type_ann_span: None, type_ann_def_id: None` to TypedStmt::Let constructions; add `param_name_spans: vec![]` to TypedDecl::Fn constructions
- `writ-compiler/tests/emit_serialize_tests.rs` - Add `param_name_spans: vec![]` to TypedDecl::Fn constructions
- `writ-lsp/src/queries.rs` - BindingInfo struct, binding_at_offset(), def_at_offset(), type_ann_def_id_at_offset() + 8 tests
- `writ-lsp/src/backend.rs` - Fallback chains in hover, goto_definition, and references handlers

## Decisions Made

- `binding_at_offset` takes `&TypeEnv` not raw `&FxHashMap<DefId, FnSig>` — avoids exposing the rustc_hash type as a writ-lsp direct dependency while keeping the API clean
- `TypedDecl::Fn::param_name_spans` uses positional indexing against `FnSig.params` — aligns with how params are stored and avoids duplicating name storage
- `def_at_offset` skips `FileId(u32::MAX)` synthetic builtins — consistent with the existing filter in `goto_definition`
- hover fallback uses `binding_at_offset` only when the expr-based hover text is empty — avoids overriding useful expr-level information with binding-level info for exprs that happen to be inside a let binding

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Changed binding_at_offset signature to use &TypeEnv**
- **Found during:** Task 1 (compiler stage)
- **Issue:** Plan specified `&rustc_hash::FxHashMap<DefId, FnSig>` but `rustc_hash` is not a direct dependency of writ-lsp
- **Fix:** Changed parameter to `&writ_compiler::check::env::TypeEnv` and accessed `type_env.fn_sigs` internally
- **Files modified:** writ-lsp/src/queries.rs, writ-lsp/src/backend.rs (call sites)
- **Verification:** Compiles without errors, all tests pass
- **Committed in:** 3d5e6ec (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minor signature change, no scope creep. All functionality delivered as specified.

## Issues Encountered

- Python script for bulk-inserting `param_name_spans` into test files had a brace-depth counting bug — some constructions were skipped or incorrectly placed. Fixed by manual targeted edits to the affected constructions.

## Next Phase Readiness

- LSP hover, goto-def, and find-refs now work on declaration sites (let bindings, fn params, type annotations, definition names)
- UAT gaps 7, 8, 9 are closed by this plan
- No blockers for remaining phase 60 plans

---
*Phase: 60-lsp-query-robustness*
*Completed: 2026-03-17*

## Self-Check: PASSED

All files verified:
- FOUND: writ-compiler/src/check/ir.rs (type_ann_span, param_name_spans)
- FOUND: writ-lsp/src/queries.rs (binding_at_offset, def_at_offset, type_ann_def_id_at_offset)
- FOUND: writ-lsp/src/backend.rs (all three fallbacks wired)
- FOUND: 60-01-SUMMARY.md
- FOUND: commit 3d5e6ec (Task 1)
- FOUND: commit 18b9b65 (Task 2)
