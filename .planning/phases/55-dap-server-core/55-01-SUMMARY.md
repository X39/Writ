---
phase: 55-dap-server-core
plan: 01
subsystem: dap
tags: [rust, dap, debug-adapter-protocol, writ-dap, writ-runtime, breakpoints, stepping]

# Dependency graph
requires:
  - phase: 52-compiler-and-runtime-preparation
    provides: "SuspendReason, DebugAction, before_instruction hooks, SourceSpan data"
  - phase: 52-compiler-and-runtime-preparation
    provides: "5-stage compilation pipeline (run_pipeline), emit_debug_info"
provides:
  - "writ-dap crate with dap 0.4.1-alpha1 dependency (confirmed compiles)"
  - "DebugHost: RuntimeHost impl with breakpoint + step over/into/out state machine"
  - "BreakpointTable: source line -> (method_idx, pc) with snap-to-nearest"
  - "compile_and_load: single-file .writ compilation for DAP launch"
  - "Runtime::suspend_reason() and Runtime::call_stack_frames() accessors"
affects:
  - 55-dap-server-core (plan 02 — DapServer uses DebugHost and BreakpointTable)

# Tech tracking
tech-stack:
  added: ["dap 0.4.1-alpha1", "serde_json 1"]
  patterns:
    - "BreakpointTable built from Module.method_bodies[i].source_spans at construction time"
    - "StepMode enum state machine checked in before_instruction hot path"
    - "Per-task call depth HashMap updated by on_function_enter/on_function_exit"
    - "pending_stop taken via take_pending_stop() — cleared after DAP server reads it"

key-files:
  created:
    - "writ-dap/Cargo.toml"
    - "writ-dap/src/lib.rs"
    - "writ-dap/src/main.rs"
    - "writ-dap/src/launch.rs"
    - "writ-dap/src/server.rs"
    - "writ-dap/src/breakpoints.rs"
    - "writ-dap/src/debug_host.rs"
  modified:
    - "Cargo.toml (workspace members)"
    - "writ-runtime/src/runtime.rs (suspend_reason, call_stack_frames)"

key-decisions:
  - "dap 0.4.1-alpha1 compiles successfully against workspace toolchain — no fallback needed"
  - "BreakpointTable snap-to-nearest: prefers line >= requested, falls back to line < requested"
  - "StepOver checks both depth <= origin_depth AND line/method differs from origin — avoids stopping on same-line instructions"
  - "compile_and_load returns (&'static str source, Module) pair — source retained for breakpoint mapping in Plan 55-02"
  - "DebugHost.on_request auto-confirms all game host requests (DAP server is not a real game host)"

patterns-established:
  - "BreakpointTable: instantiate once per debug session from compiled Module, rebuild on module reload"
  - "StepMode: caller sets mode before resuming task, DebugHost resets to None after stop"
  - "Test helper make_module_with_spans builds minimal Module using Module::new() + manual method_bodies"

requirements-completed: [DAP-01, DAP-02, DAP-03]

# Metrics
duration: 45min
completed: 2026-03-14
---

# Phase 55 Plan 01: DAP Server Core Foundation Summary

**writ-dap crate with DebugHost (step over/into/out + breakpoints), BreakpointTable (snap-to-nearest source line mapping), compile_and_load pipeline, and Runtime DAP inspection accessors — 22 tests passing**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-14T09:40:00Z
- **Completed:** 2026-03-14T10:25:00Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Created `writ-dap` crate with `dap 0.4.1-alpha1` (confirmed compiles on workspace toolchain — no fallback required)
- Implemented `DebugHost` with full stepping state machine: StepOver (skips callees), StepInto (any line change), StepOut (waits for call depth decrease)
- Implemented `BreakpointTable` with snap-to-nearest logic: maps source lines to `(method_idx, pc)` using SourceSpan data from compiled module
- Implemented `compile_and_load` in `launch.rs` — duplicates the 5-stage pipeline from writ-cli with `emit_debug_info=true` always set for DAP
- Added `Runtime::suspend_reason()` and `Runtime::call_stack_frames()` accessors needed by the DAP server in Plan 55-02

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-dap crate, validate dap dependency, add Runtime accessors** - `d599cca` (feat)
2. **Task 2: DebugHost with stepping state machine and BreakpointTable** - `ccad346` (feat)

## Files Created/Modified

- `writ-dap/Cargo.toml` - Crate manifest with dap 0.4.1-alpha1 and writ-* dependencies
- `writ-dap/src/lib.rs` - Module declarations: debug_host, breakpoints, launch, server
- `writ-dap/src/main.rs` - Minimal binary placeholder
- `writ-dap/src/launch.rs` - compile_and_load: 5-stage pipeline, returns (Module, &'static str)
- `writ-dap/src/server.rs` - Stub for Plan 55-02 DapServer
- `writ-dap/src/breakpoints.rs` - BreakpointTable: line_index, snap-to-nearest, pc_lookup, 8 tests
- `writ-dap/src/debug_host.rs` - DebugHost: RuntimeHost impl, StepMode state machine, 14 tests
- `Cargo.toml` - Added writ-dap to workspace members
- `writ-runtime/src/runtime.rs` - Added suspend_reason() and call_stack_frames() public accessors

## Decisions Made

- `dap 0.4.1-alpha1` compiles successfully — the pre-release crate was a documented risk in STATE.md. No serde_json fallback needed.
- `BreakpointTable::snap_to_nearest` prefers line >= requested (forward snap), falls back to nearest line below. This matches VS Code's expectation that "line 7 snaps to the next valid line".
- `StepOver` checks `depth <= origin_depth && (line != origin_line || method != origin_method)` — the combined condition avoids false stops on same-line instructions (multiple instructions at same source line).
- `compile_and_load` returns `(Module, &'static str)` — the leaked source string is retained so Plan 55-02 can correlate source line numbers to breakpoint requests without re-reading the file.
- `DebugHost::on_request` auto-confirms all game-host requests with neutral values (Void, Int(0), Confirmed) — the DAP server needs execution to proceed through extern calls without a real game engine.

## Deviations from Plan

None — plan executed exactly as written. The `dap` crate compiled successfully on first try (the stated risk in STATE.md was validated and resolved).

## Issues Encountered

None — all tests passed on first run after implementation.

## Next Phase Readiness

- `DebugHost` and `BreakpointTable` are ready for use by `DapServer` in Plan 55-02
- `compile_and_load` provides the compiled module and source text needed for launch
- `Runtime::suspend_reason()` and `call_stack_frames()` enable DAP stack trace responses
- The `writ-dap/src/server.rs` stub is the entry point for Plan 55-02

## Self-Check: PASSED

All created files confirmed to exist. Both task commits verified in git log.

- FOUND: writ-dap/Cargo.toml
- FOUND: writ-dap/src/lib.rs
- FOUND: writ-dap/src/debug_host.rs
- FOUND: writ-dap/src/breakpoints.rs
- FOUND: writ-dap/src/launch.rs
- FOUND: .planning/phases/55-dap-server-core/55-01-SUMMARY.md
- FOUND commit: d599cca (Task 1)
- FOUND commit: ccad346 (Task 2)

---
*Phase: 55-dap-server-core*
*Completed: 2026-03-14*
