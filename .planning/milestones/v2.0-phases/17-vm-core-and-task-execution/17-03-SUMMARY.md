---
phase: 17-vm-core-and-task-execution
plan: 03
subsystem: runtime
tags: [vm, defer, crash, atomic, concurrency, task-lifecycle, scheduler]

requires:
  - phase: 17-vm-core-and-task-execution
    plan: 02
    provides: Dispatch loop, Scheduler, Runtime/RuntimeBuilder, ExecutionLimit, TickResult

provides:
  - Defer LIFO execution on RET, TailCall, and crash unwind
  - Crash propagation with full call stack unwinding and CrashInfo
  - Secondary crash swallowing in defer handlers
  - Atomic section isolation (exempt from execution limits)
  - SPAWN_TASK/SPAWN_DETACHED creating scoped and root-level tasks
  - JOIN suspension until target task completes
  - CANCEL with recursive scoped child cancellation
  - TaskId packing/unpacking in Value::Int
  - Runtime::call_sync, run_task, crash_info, host accessor methods
  - 26 task lifecycle integration tests

affects: [18-entity-gc, 19-contract-dispatch]

tech-stack:
  added: []
  patterns: [RecordingHost for log inspection, byte_offset_of helper for DeferPush tests, round-robin with execution limits for cancel tests]

key-files:
  created:
    - writ-runtime/tests/task_tests.rs
  modified:
    - writ-runtime/src/dispatch.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/src/value.rs

key-decisions:
  - "TaskId packed into Value::Int as (index << 32 | generation) for register storage"
  - "Defer handlers are code regions within the same method body, executed by saving/restoring PC"
  - "execute_crash builds CrashInfo BEFORE unwinding, then iterates all frames running defers at each level"
  - "Secondary crashes in defer handlers are logged at Error level and swallowed"
  - "Concurrency instructions (Spawn/Join/Cancel) return structured ExecutionResult variants to the scheduler"
  - "Scheduler restructured to avoid borrow checker issues: task references re-acquired per match arm"
  - "cancel_task_tree is depth-first recursive: children cancelled before parent"
  - "JOIN handled entirely in scheduler (not via host) with join_waiters HashMap"
  - "Runtime::host()/host_mut() accessors added for test inspection"

patterns-established:
  - "RecordingHost captures log messages for verifying secondary crash logging"
  - "byte_offset_of(instrs, n) computes DeferPush byte offsets from instruction arrays"
  - "Round-robin execution with limits ensures child tasks execute DeferPush before parent cancels"

requirements-completed: [TASK-03, TASK-04, TASK-05, TASK-06, TASK-07]

duration: 12min
completed: 2026-03-02
---

# Phase 17 Plan 03: Defer, Crash, Atomic, Concurrency Summary

**Complete defer/crash unwinding engine, atomic section isolation, cooperative task concurrency (SPAWN/JOIN/CANCEL), and 26 task lifecycle integration tests verifying all TASK-03 through TASK-07 requirements**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-02
- **Completed:** 2026-03-02
- **Tasks:** 2
- **Files modified:** 5 (4 modified + 1 created)

## Accomplishments
- Defer handlers execute in LIFO order on RET, TailCall, and crash unwind
- Full crash propagation: CrashInfo built before unwinding, all frames' defers execute
- Secondary crashes in defer handlers logged at Error level and swallowed (unwinding continues)
- Atomic sections prevent task interleaving and exempt tasks from execution limits
- SPAWN_TASK creates scoped children, SPAWN_DETACHED creates root-level tasks
- JOIN suspends caller until target reaches terminal state, delivering return value
- CANCEL recursively cancels scoped children (depth-first) before parent
- Runtime API complete: call_sync, run_task, crash_info, host accessors
- 26 integration tests covering defer, crash, atomic, spawn, join, cancel, and runtime API
- All 124 writ-runtime tests pass (34 unit + 64 VM + 26 task)

## Task Commits

1. **Task 1+2: Defer/crash engine, concurrency, and task lifecycle tests** - `20ce369` (feat)

## Files Created/Modified
- `writ-runtime/src/dispatch.rs` - ExecutionResult extended with SpawnChild/SpawnDetachedTask/JoinTask/CancelTask/DeferComplete; execute_defer_handler, execute_crash, updated execute_ret and TailCall with defer LIFO
- `writ-runtime/src/scheduler.rs` - Full task management with join_waiters, cancel_task_tree, wake_joiners, handling all new ExecutionResult variants
- `writ-runtime/src/runtime.rs` - call_sync, run_task, crash_info, task_count, host/host_mut methods
- `writ-runtime/src/value.rs` - pack_task_id and unpack_task_id helpers
- `writ-runtime/tests/task_tests.rs` - 26 integration tests with RecordingHost, byte_offset_of helper

## Decisions Made
- TaskId packed as (index << 32 | generation) in Value::Int for register storage
- Defer handlers executed by saving/restoring PC within same method body
- Concurrency instructions return structured ExecutionResult variants (not handled inline in dispatch)
- Scheduler restructured to drop task references before re-borrowing self (Rust borrow checker)
- JOIN handled in scheduler with join_waiters HashMap, not via host requests
- Cancel tests use round-robin with execution limits to ensure child runs DeferPush before being cancelled

## Deviations from Plan

### Auto-fixed Issues

**1. [Build] Borrow checker errors in scheduler.rs**
- **Found during:** Task 1 (scheduler.rs)
- **Issue:** Holding mutable borrow on task while trying to borrow self for spawn/join/cancel
- **Fix:** Restructured run_one_task to re-acquire task reference per match arm
- **Files modified:** scheduler.rs
- **Verification:** Clean build

**2. [Test] Cancel tests saw Void instead of expected global values**
- **Found during:** Task 2 (task_tests.rs)
- **Issue:** Child tasks hadn't executed their DeferPush before being cancelled (child was in ready queue but hadn't run)
- **Fix:** Added Nops to both parent and child, used execution limits for round-robin so child runs DeferPush before parent runs Cancel
- **Files modified:** task_tests.rs
- **Verification:** All 26 tests pass

**3. [Build] Private host field access from integration tests**
- **Found during:** Task 2 (task_tests.rs)
- **Issue:** Runtime.host field is pub(crate), not accessible from integration tests
- **Fix:** Added host() and host_mut() public accessor methods to Runtime
- **Files modified:** runtime.rs, task_tests.rs
- **Verification:** Clean build, test accesses log_messages via host()

---

**Total deviations:** 3 auto-fixed (1 build fix, 1 test logic fix, 1 API addition)
**Impact on plan:** All auto-fixes were mechanical. No scope creep.

## Issues Encountered
None beyond the auto-fixed issues above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All Phase 17 requirements (VM-01 through VM-07, TASK-01 through TASK-08) are implemented
- Complete VM with 91 instructions, defer/crash engine, atomic sections, cooperative concurrency
- Runtime public API ready for entity-component integration (Phase 18)
- Contract dispatch stub ready for Phase 19

---
*Phase: 17-vm-core-and-task-execution*
*Completed: 2026-03-02*
