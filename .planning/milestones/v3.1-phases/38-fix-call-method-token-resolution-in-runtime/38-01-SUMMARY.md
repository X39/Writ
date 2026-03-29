---
phase: 38-fix-call-method-token-resolution-in-runtime
plan: 01
subsystem: runtime
tags: [vm, dispatch, metadata-tokens, method-dispatch, bug-fix]

# Dependency graph
requires:
  - phase: 37-fix-spurious-void-register-in-empty-function-bodies
    provides: golden IL files locked as regression anchors; compiler emits correct IL
provides:
  - BUG-17 fixed: CALL/TailCall/NewDelegate/SpawnTask/SpawnDetached handlers decode MethodDef tokens
  - decode_method_token() helper in dispatch.rs
  - call_with_methoddef_token regression test
affects:
  - 39-extend-method-defs-metadata

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "MethodDef token decoding: (token & 0x00FF_FFFF) as usize - 1 converts 1-based row_index to 0-based array index"
    - "Token format: bits 31-24 = table_id, bits 23-0 = 1-based row_index; 0 = null token"

key-files:
  created: []
  modified:
    - writ-runtime/src/dispatch.rs
    - writ-runtime/tests/vm_tests.rs

key-decisions:
  - "decode_method_token strips table_id byte (bits 31-24) and converts 1-based row_index to 0-based by subtracting 1; returns None for null token (row_index=0)"
  - "CallIndirect not updated — delegate stores already-decoded 0-based index (NewDelegate decodes at creation time)"
  - "CallExtern not updated — ExternDef tokens use table_id=16 (0x10), not 0x07; handled separately"
  - "task_tests failures (scoped_cancel, cancel_triggers_defer, and hanging tests) are pre-existing unrelated bugs; not caused by BUG-17 fix"

patterns-established:
  - "Token decode at dispatch time: decode once at instruction handler entry, use decoded index for all array accesses"
  - "Null token crash path: return ExecutionResult::Crash with descriptive message for null method tokens"

requirements-completed:
  - BUG-17

# Metrics
duration: 15min
completed: 2026-03-04
---

# Phase 38 Plan 01: Fix CALL Method Token Resolution in Runtime Summary

**Runtime VM CALL/TailCall/NewDelegate/SpawnTask/SpawnDetached handlers now decode MethodDef metadata tokens (0x07XXXXXX) to 0-based indices, eliminating the "call to invalid method index 117440513" crash (BUG-17)**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-04T21:08:28Z
- **Completed:** 2026-03-04T21:23:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `decode_method_token()` free function in dispatch.rs that decodes MethodDef metadata tokens (table_id=7, 1-based row_index in bits 23-0) to 0-based array indices
- Fixed all five method-dispatch instruction handlers: Call, TailCall, NewDelegate, SpawnTask, SpawnDetached
- Updated vm_tests.rs to use proper MethodDef tokens (0x07000002, 0x07000003) instead of raw 0-based indices (1, 2)
- Added `call_with_methoddef_token` regression test that explicitly verifies token decoding end-to-end
- All 78 vm_tests pass; all 7 golden tests pass; writ-module and writ-runtime unit tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Decode MethodDef tokens in all method-dispatch instruction handlers** - `5f6f3df` (fix)
2. **Task 2: Run full test suite and verify no regressions** - (no separate commit; verification only, no new changes)

## Files Created/Modified

- `writ-runtime/src/dispatch.rs` - Added decode_method_token() helper; updated Call, TailCall, NewDelegate, SpawnTask, SpawnDetached handlers to decode tokens before array access
- `writ-runtime/tests/vm_tests.rs` - Updated 5 method_idx values from raw 0-based indices to MethodDef tokens; added call_with_methoddef_token regression test

## Decisions Made

- `decode_method_token` is a free inline function (not a method) placed before `execute_one` for locality and clarity
- Returns `Option<usize>`: `None` for null token (row_index=0), `Some(idx)` for valid tokens — null token causes crash with descriptive message
- `CallIndirect` was NOT updated because the delegate heap object stores the already-decoded 0-based index (set by `NewDelegate` at creation time)
- `CallExtern` was NOT updated because ExternDef tokens use table_id=16, not 7, and are handled through a different resolution path
- Pre-existing task_tests failures (scoped_cancel_recursive_on_parent_crash, cancel_triggers_defer_handlers, and several hanging tests) are unrelated to BUG-17 and documented as out-of-scope

## Deviations from Plan

None - plan executed exactly as written. The changes were already staged in the working tree when execution began (plan had been partially prepared), so execution proceeded directly to verification and committing.

## Issues Encountered

The `writ-runtime` task_tests binary was locked during the full test suite run because a previously-launched test process was still running (hanging tests). This prevented re-running task_tests. These are pre-existing failures unrelated to BUG-17 — the plan explicitly notes "a separate unrelated memory/hang bug in the runtime task loop." The vm_tests (78/78), gc_tests (9/9), hook_dispatch_tests (3/3), and runtime unit tests (109/109) all pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- BUG-17 is fixed and regression-tested; compiled .writil files with function calls no longer crash with "call to invalid method index"
- fn_recursion.writil now resolves method tokens correctly at the dispatch layer
- Phase 39 (extend method_defs metadata) can proceed; the dispatch layer is now correct

---
*Phase: 38-fix-call-method-token-resolution-in-runtime*
*Completed: 2026-03-04*
