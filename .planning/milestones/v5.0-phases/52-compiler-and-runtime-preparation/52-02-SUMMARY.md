---
phase: 52-compiler-and-runtime-preparation
plan: 02
subsystem: runtime
tags: [rust, writ-runtime, dap, debug, breakpoint, vm-dispatch]

# Dependency graph
requires:
  - phase: 52-compiler-and-runtime-preparation/52-01
    provides: source location tracking (SourceSpan line/col fix) that debug hooks consume
provides:
  - DebugAction enum (6 variants) in writ-runtime::host
  - RuntimeHost debug hooks: debug_enabled(), before_instruction(), on_function_enter(), on_function_exit()
  - SuspendReason enum (HostRequest, Breakpoint, DebugStep) in writ-runtime::task
  - Task::suspend_reason field — distinguishes why a task suspended
  - ExecutionResult::DebugSuspend variant — signals debug-triggered suspension
  - Runtime::resume_debug() method — DAP server resumes debug-suspended tasks
  - lookup_source_location() helper in dispatch/mod.rs
affects: [55-dap-server, writ-dap]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "debug_enabled() guard pattern: zero-cost debug hooks when disabled (no branch in hot path when false)"
    - "SuspendReason on Task: every suspension is tagged with its cause for DAP introspection"
    - "Default-impl debug methods on RuntimeHost: production hosts (NullHost, CliHost) compile with zero changes"

key-files:
  created: []
  modified:
    - writ-runtime/src/host.rs
    - writ-runtime/src/task.rs
    - writ-runtime/src/lib.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/calls.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/runtime.rs

key-decisions:
  - "debug_enabled() is a &self method (not &mut self) so it can be checked without exclusive borrow"
  - "DebugSuspend is a separate ExecutionResult variant (not reusing Suspended(RequestId)) because debug resumes are host-free — DAP controls resume, not the host response cycle"
  - "SuspendReason::HostRequest is NOT set on the JoinTask suspension path (RequestId(0)) — that is an internal scheduler synchronization, not a true host request"
  - "on_function_enter/exit hooks fire in exec_call, exec_call_virt, exec_call_indirect (all frame-push sites) and execute_ret (frame-pop site)"

patterns-established:
  - "Debug guard pattern: if host.debug_enabled() { ... } wraps every debug call site — zero cost when disabled"
  - "SuspendReason is set at the suspension site and cleared at the resume site (confirm() and resume_debug())"

requirements-completed: [PREP-03, PREP-04]

# Metrics
duration: 3min
completed: 2026-03-13
---

# Phase 52 Plan 02: Debug Hooks and SuspendReason Summary

**RuntimeHost debug hooks (before_instruction, on_function_enter/exit, debug_enabled) + SuspendReason enum on Task with VM dispatch integration — enabling zero-overhead DAP breakpoint/stepping support**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-13T23:18:00Z
- **Completed:** 2026-03-13T23:21:11Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `DebugAction` enum (6 variants: Continue, Break, StepOver, StepInto, StepOut, Disconnect) and 4 debug hook methods to `RuntimeHost` trait — all with default no-op implementations so NullHost/CliHost compile unchanged
- Added `SuspendReason` enum and `suspend_reason: Option<SuspendReason>` field to `Task` — the DAP server can now distinguish breakpoint suspensions from host-request suspensions
- Integrated debug hooks into `execute_one` (before_instruction guard), `exec_call/virt/indirect` (on_function_enter), and `execute_ret` (on_function_exit) — guarded by `host.debug_enabled()` for zero overhead in production
- Added `ExecutionResult::DebugSuspend` and `Runtime::resume_debug()` — the DAP server can suspend and resume tasks without going through the host request/response cycle

## Task Commits

Each task was committed atomically:

1. **Task 1: Define DebugAction, SuspendReason, and add debug hooks to RuntimeHost** - `ae670a1` (feat)
2. **Task 2: Integrate debug hooks into VM dispatch and set SuspendReason on suspensions** - `558d60e` (feat)

## Files Created/Modified

- `writ-runtime/src/host.rs` — DebugAction enum, 4 debug hook methods on RuntimeHost trait, tests for NullHost defaults
- `writ-runtime/src/task.rs` — SuspendReason enum, suspend_reason field on Task, tests for all variants
- `writ-runtime/src/lib.rs` — Re-exports DebugAction and SuspendReason
- `writ-runtime/src/dispatch/mod.rs` — lookup_source_location() helper, debug hook call site in execute_one, on_function_exit in execute_ret, ExecutionResult::DebugSuspend
- `writ-runtime/src/dispatch/calls.rs` — on_function_enter after frame push in exec_call/exec_call_virt/exec_call_indirect
- `writ-runtime/src/scheduler.rs` — SuspendReason::HostRequest set on Suspended path, DebugSuspend arm handled
- `writ-runtime/src/runtime.rs` — suspend_reason cleared in confirm(), resume_debug() method added

## Decisions Made

- `debug_enabled()` is `&self` (not `&mut self`) so the host can be queried without exclusive borrow during the instruction fetch loop
- `DebugSuspend` is a separate `ExecutionResult` variant rather than reusing `Suspended(RequestId)` — debug resumes are controlled by the DAP server, not the host response cycle. This avoids polluting PendingRequest lists with fake request IDs.
- `SuspendReason::HostRequest` is intentionally NOT set on the `JoinTask` suspension path (which uses `RequestId(0)`) — that is internal scheduler synchronization, not a real host request.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

- `cargo build --workspace` shows pre-existing failures in `writ-compiler` (unrelated to this plan — those files were already modified before this plan started). `cargo test -p writ-runtime` and `cargo build -p writ-runtime` both pass cleanly.

## Next Phase Readiness

- PREP-03 and PREP-04 are complete: `RuntimeHost` has `before_instruction` hook; `SuspendReason` distinguishes Breakpoint, DebugStep, and HostRequest suspensions
- Phase 55 (DAP server) can now implement a `RuntimeHost` that sets `debug_enabled() = true` and receives `before_instruction` callbacks to implement breakpoints and stepping
- The `resume_debug(task_id)` API is ready for DAP Continue/Step commands

---
*Phase: 52-compiler-and-runtime-preparation*
*Completed: 2026-03-13*

## Self-Check: PASSED

- FOUND: writ-runtime/src/host.rs (DebugAction enum + debug hooks)
- FOUND: writ-runtime/src/task.rs (SuspendReason enum + suspend_reason field)
- FOUND: writ-runtime/src/dispatch/mod.rs (debug_enabled guard + lookup_source_location)
- FOUND: .planning/phases/52-compiler-and-runtime-preparation/52-02-SUMMARY.md
- FOUND: ae670a1 feat(52-02): add DebugAction, SuspendReason, and debug hooks to RuntimeHost
- FOUND: 558d60e feat(52-02): integrate debug hooks into VM dispatch and set SuspendReason on suspensions
