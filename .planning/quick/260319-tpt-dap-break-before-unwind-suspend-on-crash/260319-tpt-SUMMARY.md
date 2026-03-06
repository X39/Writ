---
phase: quick-260319-tpt
plan: 01
subsystem: writ-runtime, writ-dap
tags: [dap, debugging, crash, break-before-unwind, suspend-reason]
dependency_graph:
  requires: []
  provides: [CrashPending suspend reason, debug-gated crash handling, deferred crash unwind on resume]
  affects: [writ-runtime/src/task.rs, writ-runtime/src/scheduler.rs, writ-runtime/src/runtime.rs, writ-dap/src/server/inspection.rs]
tech_stack:
  added: []
  patterns: [break-before-unwind, deferred unwind, debug-gated behavior]
key_files:
  created: []
  modified:
    - writ-runtime/src/task.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/runtime.rs
    - writ-dap/src/server/inspection.rs
    - writ-dap/tests/test_protocol.rs
decisions:
  - "Use SuspendReason::CrashPending { message } as the signal — task stays Suspended with live call stack, resume_debug performs deferred unwind"
  - "wake_joiners made pub(crate) on Scheduler so runtime.rs can call it during deferred unwind"
  - "CrashPending detection in run_until_stop pre-check: check before calling resume_debug, emit terminated+exited(1) after deferred unwind without entering tick loop"
metrics:
  duration: ~15min
  completed: 2026-03-19T20:34:42Z
  tasks: 2
  files: 5
---

# Phase quick-260319-tpt Plan 01: DAP Break-Before-Unwind (Suspend on Crash) Summary

**One-liner:** Break-before-unwind via SuspendReason::CrashPending: task stays Suspended with live call stack on crash in debug mode; VSCode inspects real frames/registers; Continue triggers deferred unwind.

## What Was Built

### Task 1: Add CrashPending variant and gate crash handling on debug mode

Added `SuspendReason::CrashPending { message: String }` to `writ-runtime/src/task.rs`. This variant signals that a task crashed but was suspended before unwind so the debugger can inspect the live call stack.

Modified the `ExecutionResult::Crash(msg)` match arm in `writ-runtime/src/scheduler.rs` to check `host.debug_enabled()` before unwinding:
- Debug mode: set `task.state = Suspended`, `task.suspend_reason = Some(CrashPending { message })`, return `DebugSuspend` — the live call stack is preserved, all existing DAP inspection methods (stack frames, registers, variables) work via the primary path with no fallback needed.
- Non-debug mode: identical to previous behavior (immediate `execute_crash` → defers → Cancelled + CrashInfo).

Modified `resume_debug()` in `writ-runtime/src/runtime.rs` to detect `CrashPending` and perform the deferred crash unwind (calls `execute_crash`, cancels scoped children, releases global locks, wakes joiners) instead of re-queuing the task. Made `Scheduler::wake_joiners` `pub(crate)` to allow access from `runtime.rs`.

### Task 2: Update DAP inspection to handle CrashPending suspend reason

Modified `run_until_stop()` in `writ-dap/src/server/inspection.rs`:

1. **Pre-check (top of function):** When task is `Suspended`, detect `CrashPending` before calling `resume_debug()`. After the deferred unwind, emit `terminated + exited(1)` immediately and return — bypasses the tick loop entirely.

2. **Tick loop Suspended arm:** Added `CrashPending` to the `is_debug_suspend` matches. When detected, emit `output(stderr)` with the crash message followed by `stopped(exception)` with the crash message as `description` and `text`. This uses the live-stack primary inspection path.

Updated `writ-dap/tests/test_protocol.rs` comments in `test_halt_on_crash_inspect` to document the break-before-unwind behavior (live frames, Suspended task, deferred unwind on Continue).

## Verification

- `cargo test -p writ-runtime`: 257 tests, 0 failures
- `cargo test -p writ-dap`: 109 tests, 0 failures (including `test_halt_on_crash_inspect`)
- Golden tests: 30 pre-existing failures (unrelated to this change, confirmed by checking before/after)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Scheduler::wake_joiners was private**
- **Found during:** Task 1
- **Issue:** `runtime.rs` needed to call `self.scheduler.wake_joiners()` for the deferred unwind path, but the method was `fn` (private to Scheduler impl).
- **Fix:** Changed `fn wake_joiners` to `pub(crate) fn wake_joiners` in `scheduler.rs`.
- **Files modified:** `writ-runtime/src/scheduler.rs`
- **Commit:** 00ccb48

**2. [Rule 1 - Bug] Explicit `ref` binding not allowed in implicitly-borrowing pattern**
- **Found during:** Task 2
- **Issue:** `if let Some(SuspendReason::CrashPending { ref message }) = runtime.suspend_reason(task_id)` — compiler rejected explicit `ref` binding when pattern implicitly borrows.
- **Fix:** Changed to `{ message }` (let the implicit borrow mode handle the reference).
- **Files modified:** `writ-dap/src/server/inspection.rs`
- **Commit:** 033dd3e

## Commits

| Hash | Message |
|------|---------|
| 00ccb48 | feat(quick-260319-tpt): add CrashPending suspend reason and debug-gated crash handling |
| 033dd3e | feat(quick-260319-tpt): update DAP inspection to handle CrashPending suspend reason |

## Self-Check: PASSED

All modified files exist. Both task commits verified in git log.
- FOUND: writ-runtime/src/task.rs
- FOUND: writ-runtime/src/scheduler.rs
- FOUND: writ-runtime/src/runtime.rs
- FOUND: writ-dap/src/server/inspection.rs
- FOUND: writ-dap/tests/test_protocol.rs
- FOUND commit: 00ccb48
- FOUND commit: 033dd3e
