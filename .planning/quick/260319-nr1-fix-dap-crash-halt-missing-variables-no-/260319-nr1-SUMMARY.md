---
phase: quick-260319-nr1
plan: 01
subsystem: writ-dap / writ-runtime
tags: [dap, debugger, crash, variables, inspection]
dependency_graph:
  requires: []
  provides: [crash-halt-variable-inspection]
  affects: [writ-dap, writ-runtime]
tech_stack:
  added: []
  patterns: [crash-aware-fallback, register-preservation]
key_files:
  created: []
  modified:
    - writ-runtime/src/error.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-dap/src/server/inspection.rs
    - writ-dap/tests/test_protocol.rs
decisions:
  - "Fall back to self.task_id in resolve_task_id_or_crashed rather than fixing all_task_ids — keeps the exclusion of Cancelled tasks correct for other callers"
  - "Crash frames indexed directly by display_frame_idx (already top-to-bottom) without reversal"
metrics:
  duration: "~6 minutes"
  completed: "2026-03-19"
  tasks: 2
  files: 4
---

# Phase quick-260319-nr1 Plan 01: Fix DAP Crash Halt Missing Variables Summary

**One-liner:** Register values preserved in CrashInfo before unwind, and crash-aware fallback path added to all DAP inspection methods so locals are visible when stopped at a crash halt.

## What Was Built

VSCode halted at a crash (e.g., `x!` unwrap on None) but showed no local variables. Three root causes identified and fixed:

1. `CrashInfo::StackFrame` did not preserve register values — they were lost when the call stack unwound after `execute_crash`.
2. `resolve_task_id` uses `all_task_ids()` which excludes Cancelled tasks, so scopes/variables handlers silently returned empty for crashed threads.
3. `count_active_locals`, `get_variables`, and `do_evaluate` had no fallback path for crash state.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Preserve registers in CrashInfo and add crash-aware DAP inspection | 0b4651a | error.rs, dispatch/mod.rs, inspection.rs |
| 2 | Add integration test for crash halt variable inspection | fc0cb7a | test_protocol.rs |

## Implementation Details

### writ-runtime/src/error.rs
Added `registers: Vec<Value>` field to `StackFrame` with a `use crate::value::Value` import.

### writ-runtime/src/dispatch/mod.rs
In `execute_crash`, cloned `f.registers` for each frame BEFORE the unwind loop so the values are captured at the crash point.

### writ-dap/src/server/inspection.rs
Added `resolve_task_id_or_crashed` helper that first tries `all_task_ids()` (active tasks) then checks `self.task_id` against crash_info for Cancelled tasks.

Updated three inspection methods (`count_active_locals`, `get_variables`, `do_evaluate`) to:
- Use `resolve_task_id_or_crashed` instead of `resolve_task_id`
- Check `call_stack_frames` first (active path)
- Fall back to `crash_info.stack_trace[display_frame_idx]` and its `registers` when the call stack is empty

### writ-dap/tests/test_protocol.rs
Extended `test_halt_on_crash_inspect` with scopes + variables assertions after the existing stackTrace check:
- scopes array is non-empty for crash frame
- `variablesReference` is non-zero
- variables array is non-empty
- variable `x` (the `int?` that is `None`) is present by name

## Deviations from Plan

None - plan executed exactly as written.

## Verification

- `cargo test -p writ-runtime` — 88 passed, 0 failed
- `cargo test -p writ-dap test_halt_on_crash_inspect` — 1 passed
- `cargo test -p writ-dap` — all tests pass (no regressions)

## Self-Check: PASSED

- writ-runtime/src/error.rs — FOUND
- writ-runtime/src/dispatch/mod.rs — FOUND
- writ-dap/src/server/inspection.rs — FOUND
- writ-dap/tests/test_protocol.rs — FOUND
- Commit 0b4651a — FOUND
- Commit fc0cb7a — FOUND
