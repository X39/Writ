---
phase: 77-frame-register-pool
plan: 02
subsystem: runtime
tags: [register-pool, free-list, allocation, performance, vm, scheduler]

requires:
  - phase: 77-01
    provides: "RegisterPool struct and CallFrame::with_pool in frame.rs"

provides:
  - "RegisterPool fully wired into Scheduler, ExecContext, execute_ret, execute_crash, and all call handlers"
  - "Per-call heap allocation eliminated for recursive functions via free-list reuse"
  - "Phase 77 fib(40) median: 59.800s (-7.179s / -10.7% vs Phase 76)"

affects: [78-inner-dispatch-loop, 79-copy-value]

tech-stack:
  added: []
  patterns:
    - "Pool threading: Scheduler owns pool, ExecContext borrows it, release on every frame pop"
    - "Extract-before-release: read popped.return_register (Copy u16) before moving registers into pool"

key-files:
  created: []
  modified:
    - "writ-runtime/src/scheduler.rs"
    - "writ-runtime/src/dispatch/mod.rs"
    - "writ-runtime/src/dispatch/calls.rs"
    - "writ-runtime/src/runtime.rs"
    - "benchmark/BASELINE.md"

key-decisions:
  - "Extract return_register before pool.release(popped.registers) to avoid partial-move borrow issue"
  - "runtime.rs deferred-crash path (CrashPending debug unwind) also needed pool threading — discovered via compile error"
  - "exec_tail_call left unchanged: reuses existing frame in-place via clear()+resize(), no pop/push needed"

patterns-established:
  - "Pool release on every frame pop: both execute_ret and execute_crash release; tail_call reuses without touching pool"

requirements-completed: [FRAME-05, VERIFY-01, VERIFY-02, VERIFY-03]

duration: 15min
completed: 2026-03-22
---

# Phase 77 Plan 02: Thread RegisterPool Summary

**RegisterPool wired end-to-end through the execution pipeline; fib(40) improved from 66.979s to 59.800s (-10.7%)**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T13:37:04Z
- **Completed:** 2026-03-22T13:49:53Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Threaded `RegisterPool` through Scheduler -> ExecContext -> execute_ret/execute_crash/execute_defer_handler -> exec_call/exec_call_virt/exec_call_indirect
- `execute_ret` releases popped frame's register Vec to pool; `execute_crash` releases every unwound frame's registers to pool
- All three non-tail call handlers acquire frames via `CallFrame::with_pool`
- `create_task` acquires its initial frame from the pool
- fib(40) median dropped from 66.979s (Phase 76) to 59.800s — 10.7% improvement, 28.2% cumulative vs Phase 75 baseline
- Full test suite: zero failures, zero warnings

## Task Commits

1. **Task 1: Thread RegisterPool through pipeline** - `190ed2f` (feat)
2. **Task 2: Record Phase 77 performance delta** - `f28a309` (chore)

## Files Created/Modified

- `writ-runtime/src/scheduler.rs` - Added `pool: RegisterPool` field, `create_task` uses `with_pool`, `execute_one`/`execute_crash` calls pass `&mut self.pool`
- `writ-runtime/src/dispatch/mod.rs` - Added `pool` to ExecContext, updated all dispatch function signatures, `execute_ret` calls `pool.release(popped.registers)`, `execute_crash` calls `pool.release(frame.registers)` in unwind loop
- `writ-runtime/src/dispatch/calls.rs` - `exec_call`, `exec_call_virt` (Method arm), `exec_call_indirect` use `CallFrame::with_pool`; `exec_tail_call` defer handler call updated to pass pool
- `writ-runtime/src/runtime.rs` - Deferred-crash path (CrashPending debug unwind) updated to pass `&mut self.scheduler.pool`
- `benchmark/BASELINE.md` - Phase 77 section with three run times, median, and delta vs Phase 76

## Decisions Made

- Extract `return_register` (Copy u16) before calling `pool.release(popped.registers)` to avoid a partial-move compile error. The `registers` field is moved into the pool while `return_register` is already copied to a local binding.
- `runtime.rs` had a hidden fourth `execute_crash` call site (the debug CrashPending path) that was not listed in the plan's interface notes. Discovered via compile error and fixed immediately (Rule 3 - blocking).
- `exec_tail_call` correctly left unchanged: it in-place replaces the current frame via `clear()+resize()`, never popping or pushing, so pool interaction is not needed or appropriate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] runtime.rs had undocumented fourth execute_crash call site**
- **Found during:** Task 1 (compile step)
- **Issue:** `runtime.rs:292` calls `execute_crash` for the debug-mode deferred crash path (CrashPending suspend reason). The plan's interface notes listed only two call sites in scheduler.rs but missed this one. Compile error: missing 11th argument.
- **Fix:** Added `&mut self.scheduler.pool` as the final argument at `runtime.rs:303`
- **Files modified:** `writ-runtime/src/runtime.rs`
- **Verification:** `cargo build --release` produces zero errors
- **Committed in:** `190ed2f` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Blocking fix necessary for compilation. No scope creep — it was a straightforward parameter addition at the one missed call site.

## Issues Encountered

None beyond the compile-error auto-fix above.

## Next Phase Readiness

- Phase 77 complete: RegisterPool fully integrated, 10.7% improvement measured and recorded
- Phase 78 (inner dispatch loop) can start; the ExecContext struct pattern used here directly supports the batch context extension approach flagged in research
- Cumulative improvement vs Phase 75 baseline: 83.297s → 59.800s (-28.2%). Phase 79 target remains fib(40) < 30s.

## Self-Check: PASSED

All created/modified files verified present. Both task commits verified in git log.

---
*Phase: 77-frame-register-pool*
*Completed: 2026-03-22*
