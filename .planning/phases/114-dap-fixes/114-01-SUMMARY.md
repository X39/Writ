---
phase: 114-dap-fixes
plan: 01
subsystem: dap
tags: [writ-dap, source-attribution, dialogue-interpolation, golden-test]

requires:
  - phase: 109-closure-capture
    provides: closure capture analysis (TYPE-12 fix, used in compile pipeline context)

provides:
  - Per-frame source file attribution in DAP build_stack_frames via method_file_ids
  - compile_and_load and compile_and_load_project return method_file_ids
  - dlg_interp.writ golden fixture exercising {expr} interpolation in dialogue text

affects: [writ-dap, writ-golden, writ-compiler dialogue lowering]

tech-stack:
  added: []
  patterns:
    - "method_file_ids: Vec<Option<FileId>> indexed parallel to module.method_defs, built from per_file_asts name->FileId walk"
    - "run_pipeline returns (bytes, name_file_pairs) for post-compilation method->file attribution"

key-files:
  created:
    - writ-golden/tests/golden/dlg_interp.writ
    - writ-golden/tests/golden/dlg_interp.writil
  modified:
    - writ-dap/src/launch.rs
    - writ-dap/src/server/mod.rs
    - writ-dap/src/server/handlers.rs
    - writ-dap/src/server/inspection.rs
    - writ-golden/tests/golden_tests.rs
    - writ-dap/tests/test_compile_and_load.rs
    - writ-dap/tests/test_debug_session.rs
    - writ-dap/tests/test_quest_system_debug.rs

key-decisions:
  - "method_file_ids built post-compilation via name lookup against name_file_pairs from run_pipeline; avoids module format changes"
  - "run_pipeline returns (Vec<u8>, Vec<(String, FileId)>) so callers can match method names after Module::from_bytes"
  - "Single-file compile_and_load uses vec![Some(FileId(0)); n] — all methods are from the one source file"
  - "Dialogue {expr} interpolation path (lower_dlg_text->lower_fmt_string) was already correct; DAP-02 was a test coverage gap"

patterns-established:
  - "collect_decl_names recurses into Namespace::Block and Entity::inherent_impl to collect all MethodDef-producing declarations"

requirements-completed: [DAP-01, DAP-02]

duration: 12min
completed: 2026-03-29
---

# Phase 114 Plan 01: DAP Fixes Summary

**Per-frame source attribution in DAP via method_file_ids map from compile pipeline, plus dlg_interp.writ golden fixture proving {expr} dialogue text interpolation compiles correctly**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-29T00:20:00Z
- **Completed:** 2026-03-29T00:32:52Z
- **Tasks:** 2
- **Files modified:** 8 (+ 2 new golden fixture files)

## Accomplishments

- DAP-01: `build_stack_frames` now uses `method_file_ids[method_idx]` to pick the correct `source_paths` entry per frame. `source_paths.first()` is only a fallback for synthetic/unknown methods.
- DAP-01: `run_pipeline` returns name->FileId pairs collected from `per_file_asts`; both `compile_and_load` and `compile_and_load_project` build and return `method_file_ids` as a new third tuple element.
- DAP-02: Created `dlg_interp.writ` golden fixture exercising `{name}` (string) and `{count}` (int) interpolation in dialogue text lines through the `lower_dlg_text -> lower_fmt_string` pipeline. All 102 DAP tests pass; all workspace tests pass.

## Task Commits

1. **Task 1: Per-frame source file attribution (DAP-01)** - `ecdaf99` (feat)
2. **Task 2: Dialogue text interpolation golden test (DAP-02)** - `e629afc` (feat)

## Files Created/Modified

- `writ-dap/src/launch.rs` - Extended `run_pipeline` return type; both compile functions return `method_file_ids`; added `collect_decl_names` helper
- `writ-dap/src/server/mod.rs` - Added `method_file_ids: Vec<Option<FileId>>` field to `DapServer`, initialized to `Vec::new()`
- `writ-dap/src/server/handlers.rs` - Destructures new 3-tuple from compile functions; stores `self.method_file_ids`
- `writ-dap/src/server/inspection.rs` - Replaced hardcoded `source_paths.first()` with per-frame `method_file_ids` lookup with fallback
- `writ-golden/tests/golden/dlg_interp.writ` - New fixture: `dlg greet` with `{name}` and `{count}` interpolation in `@npc` speaker lines
- `writ-golden/tests/golden/dlg_interp.writil` - Blessed snapshot
- `writ-golden/tests/golden_tests.rs` - Added `test_dlg_interp` registration
- `writ-dap/tests/test_compile_and_load.rs`, `test_debug_session.rs`, `test_quest_system_debug.rs` - Updated for new 3-tuple return type

## Decisions Made

- Built `method_file_ids` post-compilation via name matching from `run_pipeline`'s `name_file_pairs` return value. This avoids modifying the module binary format or threading DefMap through the serializer.
- Name-based matching is safe because overloads within a file always share the same FileId — the name uniquely identifies the file even when multiple overloads exist.
- DAP-02 was a test coverage gap, not a compiler bug. The `lower_dlg_text -> lower_fmt_string` pipeline was already structurally correct.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Minor: `Ast.items` (not `.decls`) — field name corrected immediately during first build attempt.
- Minor: `AstEntityDecl.inherent_impl` and `AstImplDecl.members` (not `.methods`) — fixed using `AstImplMember::Fn` pattern.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both DAP requirements satisfied (DAP-01 and DAP-02)
- All workspace tests pass (0 failures)
- Phase 114 complete

---
*Phase: 114-dap-fixes*
*Completed: 2026-03-29*
