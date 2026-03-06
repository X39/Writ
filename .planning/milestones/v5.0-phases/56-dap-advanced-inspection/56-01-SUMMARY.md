---
phase: 56-dap-advanced-inspection
plan: 01
subsystem: dap
tags: [rust, dap, debugger, runtime, variables, type-decoding]

# Dependency graph
requires:
  - phase: 55-dap-server-core
    provides: DapServer with initialize/launch/breakpoints/stepping/stackTrace handlers
provides:
  - Runtime::frame_registers(task_id, frame_index) -> Option<Vec<Value>> accessor
  - Runtime::all_task_ids() -> Vec<TaskId> accessor (non-terminal tasks only)
  - writ-dap/src/variables.rs with format_value, decode_type_blob, make_variables_ref, unpack_variables_ref
affects: [56-02-dap-advanced-inspection, future DAP Scopes/Variables/Evaluate/Threads handlers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "variablesReference encoding: (task_idx as i64) << 32 | frame_idx as i64 for DAP protocol"
    - "format_value delegates heap reads to GcHeap trait for all Ref variants"
    - "decode_type_blob matches first byte tag, reads LE u32 for TypeDef row lookup"

key-files:
  created:
    - writ-dap/src/variables.rs
  modified:
    - writ-runtime/src/runtime.rs
    - writ-dap/src/lib.rs

key-decisions:
  - "frame_registers uses call_stack.get(frame_index) — 0 is bottom/oldest frame, N-1 is top/innermost"
  - "all_task_ids filters with matches!(state, Completed | Cancelled) — excludes only terminal states"
  - "format_value uses {:?} for String heap objects (quoted), Display for primitives"
  - "decode_type_blob returns 'Type' (not '?') for TypeDef row out-of-range — distinguishes decode success from unknown tag"
  - "Invalid ref test uses two heaps: allocate on heap_a to get HeapRef, test against empty heap_b since HeapRef.0 is pub(crate)"

patterns-established:
  - "Test pattern for runtime tests: insert_task() helper directly manipulates scheduler.tasks to bypass method index validation"
  - "Type blob decoding: offset 0 -> '?', empty blob -> '?', tag 0x10 reads u32 LE as 1-based TypeDef row"

requirements-completed: [DAP-04, DAP-07]

# Metrics
duration: 4min
completed: 2026-03-14
---

# Phase 56 Plan 01: DAP Advanced Inspection Foundations Summary

**Runtime frame-register and task-enumeration accessors plus a variables.rs utility module with value formatting, type blob decoding, and variablesReference encoding for DAP variable inspection**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-14T11:21:01Z
- **Completed:** 2026-03-14T11:24:42Z
- **Tasks:** 2
- **Files modified:** 3 (runtime.rs, variables.rs created, lib.rs)

## Accomplishments
- Added `frame_registers(task_id, frame_index)` to Runtime — returns cloned register vec for any call frame by index (0=bottom, N-1=top)
- Added `all_task_ids()` to Runtime — returns non-terminal task IDs (Ready/Running/Suspended), excludes Completed/Cancelled
- Created `writ-dap/src/variables.rs` with four public functions: `format_value`, `decode_type_blob`, `make_variables_ref`, `unpack_variables_ref`
- 33 new unit tests across both crates — all passing, no regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: frame_registers and all_task_ids accessors** - `179f4e1` (feat)
2. **Task 2: variables.rs utility module** - `94a6b2d` (feat)

**Plan metadata:** (docs commit to follow)

_Note: TDD tasks combined RED+GREEN in single commits due to immediate implementation clarity_

## Files Created/Modified
- `writ-runtime/src/runtime.rs` - Added `frame_registers` and `all_task_ids` public methods plus 8 unit tests
- `writ-dap/src/variables.rs` - Created with `format_value`, `decode_type_blob`, `make_variables_ref`, `unpack_variables_ref` plus 25 unit tests
- `writ-dap/src/lib.rs` - Added `pub mod variables;` declaration

## Decisions Made
- `frame_registers` indexes into `call_stack` directly — 0 is the oldest frame (matches DAP stackFrames ordering)
- `all_task_ids` uses `matches!` macro pattern for clean terminal-state exclusion
- `format_value` uses `{:?}` (Debug) for String heap objects to produce quoted output matching debugger conventions
- `decode_type_blob` returns `"Type"` (not `"?"`) for valid-but-out-of-range TypeDef rows to distinguish decode success from unknown tags
- Tests use `insert_task()` helper that directly manipulates `scheduler.tasks` since `HeapRef.0` is `pub(crate)` and `spawn_task` rejects method index 0 in an empty module

## Deviations from Plan

None - plan executed exactly as written. The only issue encountered (HeapRef private field in tests) was resolved within the task scope using a two-heap test pattern.

## Issues Encountered
- `HeapRef(u32)` inner field is `pub(crate)` — cannot construct `HeapRef(999)` from outside the crate for the invalid-ref test. Fixed by allocating on `heap_a`, using that ref against an empty `heap_b` (ref is out-of-range on the empty heap, triggering the error path).

## Next Phase Readiness
- Plan 02 can now wire Scopes, Variables, Evaluate, and Threads DAP handlers using these tested utilities
- `frame_registers` and `all_task_ids` are the only Runtime accessors needed for Plan 02
- `format_value` and `decode_type_blob` handle all value types that will appear in variable responses

---
*Phase: 56-dap-advanced-inspection*
*Completed: 2026-03-14*
