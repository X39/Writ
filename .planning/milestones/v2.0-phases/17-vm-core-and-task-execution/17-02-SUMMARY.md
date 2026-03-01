---
phase: 17-vm-core-and-task-execution
plan: 02
subsystem: runtime
tags: [vm, dispatch-loop, scheduler, runtime-api, register-machine, instruction-set]

requires:
  - phase: 17-vm-core-and-task-execution
    plan: 01
    provides: Value enum, BumpHeap, CallFrame, Task, RuntimeHost/NullHost, LoadedModule
provides:
  - Complete dispatch loop with all 91 instruction match arms
  - Scheduler with task creation, ready queue, and execution limit enforcement
  - Runtime/RuntimeBuilder public API with tick/confirm/spawn_task
  - ExecutionLimit, TickResult, PendingRequest types
  - 64 per-instruction integration tests
affects: [17-03, 18-entity-gc, 19-contract-dispatch]

tech-stack:
  added: []
  patterns: [match-dispatch loop, ModuleBuilder-based integration tests, run_simple test helper]

key-files:
  created:
    - writ-runtime/src/dispatch.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/tests/vm_tests.rs
  modified:
    - writ-runtime/src/lib.rs

key-decisions:
  - "NullHost auto-confirms all requests synchronously in dispatch loop — tasks never actually suspend with NullHost"
  - "Defer execution during RET/TailCall is stubbed with TODO comments — Plan 03 implements real defer unwinding"
  - "SpawnTask/SpawnDetached/Cancel crash with 'not yet implemented' — Plan 03 implements concurrency"
  - "CALL_VIRT crashes with 'contract dispatch not available' — Phase 19 adds real dispatch"
  - "Switch encoding is 6+4n bytes (not 8+4n), required careful byte offset calculation in tests"
  - "Execution limit check skipped when task.atomic_depth > 0"

patterns-established:
  - "build_runtime(instrs, reg_count) test helper builds ModuleBuilder module from raw instructions"
  - "run_simple(instrs, reg_count) helper spawns task 0, ticks to completion, returns runtime+task_id"
  - "build_two_method_runtime helper for testing CALL/RET between methods"

requirements-completed: [VM-01, VM-03, VM-04, VM-05, VM-06, VM-07, TASK-01, TASK-02]

duration: 15min
completed: 2026-03-02
---

# Phase 17 Plan 02: Dispatch Loop, Scheduler, and Runtime API Summary

**Complete 91-instruction match-dispatch VM with scheduler, Runtime/RuntimeBuilder API, and 64 per-instruction integration tests verifying arithmetic, control flow, calls, objects, strings, and host interaction**

## Performance

- **Duration:** 15 min
- **Started:** 2026-03-02
- **Completed:** 2026-03-02
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- All 91 instructions have match arms in dispatch loop — exhaustive match enforced by compiler
- Scheduler creates tasks, drives execution through Ready/Running/Completed/Cancelled states, enforces instruction limits
- Runtime/RuntimeBuilder provides tick(), confirm(), spawn_task(), register inspection
- 64 integration tests cover every instruction category: data movement, arithmetic, comparison, control flow, calls, conversions, strings, objects, arrays, enums, options, results, boxing, globals, atomics, delegates

## Task Commits

1. **Task 1+2: Dispatch loop, scheduler, Runtime API, and integration tests** - `b354f0d` (feat)

## Files Created/Modified
- `writ-runtime/src/dispatch.rs` - 91-instruction match-dispatch loop, execute_one, execute_ret, dispatch_host_request helpers
- `writ-runtime/src/scheduler.rs` - Scheduler with task map, ready queue, create_task, run_one_task
- `writ-runtime/src/runtime.rs` - Runtime, RuntimeBuilder, ExecutionLimit, TickResult, PendingRequest
- `writ-runtime/src/lib.rs` - Added dispatch, scheduler, runtime module declarations and re-exports
- `writ-runtime/tests/vm_tests.rs` - 64 per-instruction integration tests using ModuleBuilder

## Decisions Made
- NullHost auto-confirms all requests synchronously in dispatch loop; tasks never actually suspend with NullHost (matches CONTEXT.md spec)
- Defer execution during RET/TailCall stubbed for Plan 03
- Concurrency instructions (SpawnTask/SpawnDetached/Cancel) crash with "not yet implemented" for Plan 03
- CALL_VIRT crashes with "contract dispatch not available" for Phase 19
- Execution limit is skipped inside atomic sections (atomic_depth > 0)

## Deviations from Plan

### Auto-fixed Issues

**1. [Build] Rust 2024 edition ref mut pattern error**
- **Found during:** Task 1 (dispatch.rs)
- **Issue:** `ref mut` binding modifier not allowed when implicitly borrowing in Rust 2024 edition
- **Fix:** Removed explicit `ref mut` from ArrayInit and ArraySlice pattern matches
- **Files modified:** dispatch.rs
- **Verification:** Clean build with zero warnings
- **Committed in:** b354f0d

**2. [Build] Switch byte offset miscalculation in test**
- **Found during:** Task 2 (vm_tests.rs)
- **Issue:** Assumed Switch encoding was 16 bytes (with padding) but actual encoding is 6+4n bytes (14 bytes for 2 cases)
- **Fix:** Recalculated offsets: case0=14, case1=30 (from Switch position)
- **Files modified:** vm_tests.rs
- **Verification:** switch_dispatches_by_tag test passes
- **Committed in:** b354f0d

---

**Total deviations:** 2 auto-fixed (2 build fixes)
**Impact on plan:** Both auto-fixes were mechanical corrections. No scope creep.

## Issues Encountered
None beyond the auto-fixed build issues above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All dispatch and scheduler infrastructure ready for Plan 03 defer/crash engine
- Stub comments in dispatch.rs mark exact insertion points for defer execution
- SpawnTask/Cancel stubs ready to be replaced with real implementations
- Runtime public API (tick, confirm) ready for concurrency testing

---
*Phase: 17-vm-core-and-task-execution*
*Completed: 2026-03-02*
