---
phase: 28-codegen-bug-fixes
plan: "02"
subsystem: codegen
tags: [writ-compiler, emit, IL, range, defer, instruction-patching]

# Dependency graph
requires:
  - phase: 28-codegen-bug-fixes
    provides: BF-02 (Range Nop emission) and BF-03 (DeferPush placeholder) bug fixes
provides:
  - "emit_range(): Range<T> struct construction via New + 4x SetField sequence"
  - "emit_defer(): DeferPush handler offset patching via post-emission Vec mutation"
  - "range_type_token() helper in ModuleBuilder for Range TypeRef lookup"
affects:
  - 28-codegen-bug-fixes (plan 03 and beyond)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Post-emission instruction patching: emit placeholder, record index, patch after body emission"
    - "Range struct construction: New + 4x SetField for start/end/start_inclusive/end_inclusive fields"
    - "Defer handler layout: DeferPush -> DeferPop -> Br (skip handler) -> handler body -> DeferEnd"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/tests/emit_body_tests.rs

key-decisions:
  - "emit_defer() uses instruction-index patching (not byte-offset): DeferPush.method_idx holds instruction array index of handler start, not byte position"
  - "Br instruction inserted after DeferPop to skip handler body on normal execution path; handler only reachable by runtime defer mechanism"
  - "range_type_token() falls back to 0 if no TypeRef named Range is registered, matching the ArrayInit elem_type=0 placeholder pattern used elsewhere"
  - "start_inclusive is always true in Writ syntax (ranges always include start); only end_inclusive varies by ..= vs .. syntax"

patterns-established:
  - "Post-emission patching pattern: record instruction index BEFORE emit, patch AFTER dependent instructions are emitted"
  - "TDD RED-GREEN cycle: write failing tests first, commit them, then implement to GREEN"

requirements-completed:
  - EMIT-12
  - EMIT-15

# Metrics
duration: 10min
completed: 2026-03-03
---

# Phase 28 Plan 02: BF-02 Range Construction and BF-03 DeferPush Handler Offset Summary

**Range expressions now emit `New + 4x SetField` struct construction (not `Nop`); DeferPush.method_idx holds the correct handler instruction index via post-emission patching**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-03T17:11:18Z
- **Completed:** 2026-03-03T17:21:00Z
- **Tasks:** 2 (TDD: RED commit + GREEN commit per task)
- **Files modified:** 3

## Accomplishments
- BF-02 fixed: `TypedExpr::Range` now dispatches to `emit_range()` instead of emitting `Instruction::Nop`
- `emit_range()` constructs a `Range<T>` struct with 4 SetField instructions (start, end, start_inclusive, end_inclusive)
- BF-03 fixed: `emit_defer()` uses post-emission patching to set `DeferPush.method_idx` to the handler's instruction index
- Added `Br` instruction after `DeferPop` to skip handler body on normal execution path
- Added `range_type_token()` helper to `ModuleBuilder` for future Range TypeRef resolution
- 4 new tests added and passing; 86 total writ-compiler emit tests pass with 0 failures

## Task Commits

Each task committed atomically following TDD RED-GREEN cycle:

1. **Task 1 RED: Failing tests for Range and Defer** - `39201d6` (test)
2. **Task 1 GREEN: Implement emit_range and fix emit_defer** - `ecd5d07` (feat)

_Note: Tasks 1 and 2 from plan were merged — tests (Task 2 spec) were written first as RED phase of Task 1 TDD cycle_

## Files Created/Modified
- `writ-compiler/src/emit/body/expr.rs` - Replaced Range Nop with `emit_range()` call; rewrote `emit_defer()` with post-emission patching and Br skip; added `emit_range()` function
- `writ-compiler/src/emit/module_builder.rs` - Added `range_type_token()` method that searches TypeRef entries for "Range" with 0 fallback
- `writ-compiler/tests/emit_body_tests.rs` - Added 4 new tests: `test_range_emits_new_and_set_field`, `test_range_inclusive_emits_load_true_for_end`, `test_defer_emits_correct_handler_offset`, `test_defer_handler_offset_matches_handler_start`

## Decisions Made
- `DeferPush.method_idx` holds the instruction ARRAY INDEX (not byte offset) of the handler body start — this matches the IL spec's intent and allows the runtime to jump directly to the correct instruction
- `Br` instruction is placed AFTER `DeferPop` (not before) so normal exit disarms the defer before branching past the handler
- The `emit_range()` function uses `Ty(0)` (Int) as fallback for None start/end, consistent with how the type system pre-interns primitives
- `range_type_token()` uses a 0 fallback (same as ArrayInit elem_type) since cross-module TypeRef wiring is deferred to a future phase

## Deviations from Plan

### Auto-fixed Issues

None. However, the TDD cycle was adjusted: both Task 1 (implementation) and Task 2 (tests) were combined into a single RED-GREEN cycle. Tests were written first (RED commit `39201d6`), then implementation (GREEN commit `ecd5d07`). This is the correct TDD approach and matches the plan's intent — the tests specified in Task 2 served as the RED phase for Task 1.

## Issues Encountered
- The Edit tool could not patch `expr.rs` due to an auto-formatter rewriting the file between reads; resolved by using Python to insert the `emit_range` function at the correct line offset
- Pre-existing linter auto-format on `expr.rs` updated `emit_tail_call` signature (added `callee_def_id` parameter) as part of Plan 28-01 changes — this was already present and not a regression

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- BF-02 and BF-03 are resolved; Range expressions and defer blocks will emit correct IL
- Phase 28 Plan 03 can now address any remaining codegen bugs
- The post-emission patching pattern established here (`defer_push_idx` / `br_skip_idx`) can be reused for other control-flow constructs that need forward reference resolution

---
*Phase: 28-codegen-bug-fixes*
*Completed: 2026-03-03*
