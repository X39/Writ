---
phase: quick-260319-mdb
plan: 01
subsystem: writ-dap, writ-compiler
tags: [dap, debugger, crash, inspection, force-unwrap, compiler-fix]
dependency_graph:
  requires: []
  provides: [crash-halt-inspection, force-unwrap-correct-codegen]
  affects: [writ-dap, writ-compiler, writ-golden]
tech_stack:
  added: []
  patterns: [crash-info-fallback, terminal-state-guard]
key_files:
  created:
    - writ-golden/tests/golden/crash_unwrap_none.writ
  modified:
    - writ-dap/src/server/handlers.rs
    - writ-dap/src/server/inspection.rs
    - writ-compiler/src/emit/body/patterns.rs
    - writ-dap/tests/test_protocol.rs
    - writ-golden/tests/golden/force_unwrap.writil
decisions:
  - "Detect force-unwrap Option match pattern (Variable arm + Wildcard+Crash arm) in emit_option_propagation and emit IS_NONE + BR_FALSE + CRASH / UNWRAP rather than falling through to emit_literal_match which treats Variable as always-matching"
  - "build_stack_frames returns crash frames already top-to-bottom (no .rev()) since execute_crash captures them with .rev() applied; distinguished via already_reversed flag"
  - "continue-after-crash guard placed before resume_debug check to avoid calling resume_debug on a non-Suspended (Cancelled) task"
metrics:
  duration: ~20min
  completed: 2026-03-19
  tasks: 3
  files: 5
---

# Phase quick-260319-mdb Plan 01: Fix DAP Halt-on-Crash Thread/StackTrace Inspection Summary

After a runtime crash, DAP's `threads` response now returns the crashed task with a descriptive name matching the `stopped` event's `threadId`, and `stackTrace` returns non-empty frames from `CrashInfo.stack_trace`, enabling full crash inspection in VSCode.

## What Was Built

**Task 1: Crash-aware threads and stackTrace handlers**

- `handle_threads` in `handlers.rs`: when `all_task_ids()` is empty (Cancelled tasks are filtered out), check `crash_info` on `self.task_id`; if present, return a thread with the task's actual index as `id` and name `"main (crashed)"` instead of the hardcoded `{id: 0, name: "terminated"}` fallback.

- `build_stack_frames` in `inspection.rs`: when `call_stack_frames` returns empty (crash unwinds all frames via `execute_crash`), fall back to `CrashInfo.stack_trace`. Crash frames are already in top-to-bottom order (captured with `.rev()` in `execute_crash`), so skip the `.rev()` step used for normal call stack frames. Implemented via `(raw_frames, already_reversed)` tuple and a boxed iterator.

**Task 2: Crash fixture and integration test** (+ auto-fixed compiler bug)

- `crash_unwrap_none.writ`: minimal fixture `let x: int? = None; let y = x!;`

- `test_halt_on_crash_inspect`: full crash inspection flow test — validates stopped(exception) event, non-terminated thread name, non-empty stackFrames, stderr crash output, continue-after-crash terminates cleanly with exit_code=1.

- **Auto-fixed (Rule 1 - Bug):** `emit_option_propagation` in `patterns.rs` fell through to `emit_literal_match` for the force-unwrap pattern (`expr!`). `emit_literal_match` treated the `Variable` arm as always-matching, making the `Crash` arm unreachable dead code. The program ran `let y = None` (not the unwrapped value) and returned normally instead of crashing.

  Fix: added `is_option_force_unwrap` detection (Variable arm + Wildcard+Crash arm) and emitted the correct `IS_NONE + BR_FALSE + CRASH / UNWRAP` sequence. Blessed `force_unwrap.writil` golden test with correct output.

**Task 3: Continue-after-crash graceful handling**

- Added terminal-state guard at the top of `run_until_stop`: checks `task_state` before attempting `resume_debug`. If already `Completed` or `Cancelled`, emits `Terminated + Exited` (exit_code=1 for crashes) and returns immediately without trying to resume a non-Suspended task.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Force-unwrap emits dead CRASH code — Option `!` operator never crashes**

- **Found during:** Task 2 (crash fixture `let y = x!` where `x = None` produced `terminated` instead of `stopped(exception)`)
- **Issue:** `emit_option_propagation` called `is_option_propagation` (which checks for `Return` body in arms) for the `!` desugared match. `!` produces Variable+Wildcard/Crash arms — no Return body — so it fell through to `emit_literal_match`. `emit_literal_match` handled Variable patterns as always-matching (inserts binding = scrutinee, emits body, branches to end), making the Crash arm unreachable. Result: `let y = None` (y = the Option itself, not the unwrapped int), no crash.
- **Fix:** Added `is_option_force_unwrap` predicate checking for 2 arms where arm[0] is Variable and arm[1] is Wildcard+Crash. Emits: `IS_NONE`, `BR_FALSE (skip to UNWRAP)`, `CRASH path`, `UNWRAP path + bind variable`.
- **Files modified:** `writ-compiler/src/emit/body/patterns.rs`, `writ-golden/tests/golden/force_unwrap.writil`
- **Commit:** 6210ab6

## Self-Check: PASSED

All key files exist. All commits (021a1bd, 6210ab6, 23d5c32) confirmed in git log.
All DAP tests pass (107 total across 8 test binaries).
All golden tests pass (39 total, force_unwrap.writil blessed).
