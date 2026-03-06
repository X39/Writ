---
phase: 56-dap-advanced-inspection
plan: 02
subsystem: dap
tags: [dap, debugger, variables, scopes, threads, rust]

# Dependency graph
requires:
  - phase: 56-dap-advanced-inspection
    provides: "variables.rs utilities: format_value, decode_type_blob, make/unpack_variables_ref; runtime frame_registers and all_task_ids accessors"
  - phase: 55-dap-server-core
    provides: "DapServer skeleton, breakpoints, step commands, DebugHost, compile_and_load"
provides:
  - "Scopes handler: returns one Locals scope with variablesReference encoding task+frame"
  - "Variables handler: returns DebugLocal-mapped register values with names, values, types"
  - "Evaluate handler: name lookup in DebugLocal table, descriptive error for unknown names"
  - "Threads handler: real task enumeration from runtime with entry method names from string heap"
  - "Globally unique frame IDs: task_idx * 10000 + display_frame_index"
  - "decode_frame_id, build_thread_list, collect_frame_variables, evaluate_local free functions"
affects: ["57-vscode-extension", "dap-server", "debugging"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Module-level free functions for testable DAP handler logic (no DapServer instance needed)"
    - "Globally unique DAP frame IDs: task_idx * 10000 + display_frame_index"
    - "Display-to-actual stack index conversion: actual_idx = frames.len() - 1 - display_frame_idx"

key-files:
  created: []
  modified:
    - writ-dap/src/server.rs

key-decisions:
  - "decode_frame_id and build_thread_list are module-level free functions for testability"
  - "Frame IDs globally unique across tasks: task_idx * 10000 + display_frame_idx"
  - "collect_frame_variables and evaluate_local are module-level free functions"
  - "Stopped events use task_id.index as thread_id (0-based) instead of hardcoded 1"
  - "ScopePresentationhint::String required by dap 0.4.1-alpha1 typed enum (not raw String)"

patterns-established:
  - "TDD approach: write failing tests first, then implement free functions to make them pass"
  - "DAP handler logic isolated as free functions: test without live server or runtime"

requirements-completed: [DAP-04, DAP-06, DAP-07]

# Metrics
duration: 8min
completed: 2026-03-14
---

# Phase 56 Plan 02: DAP Advanced Inspection Summary

**Full Scopes/Variables/Evaluate/Threads DAP handlers with DebugLocal-based variable inspection and globally-unique multi-task frame IDs**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-14T11:27:38Z
- **Completed:** 2026-03-14T11:35:51Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Replaced hardcoded `[Thread { id: 1, name: "main" }]` with real task enumeration via `runtime.all_task_ids()` and method name lookup from string heap
- Implemented Scopes handler returning one "Locals" scope with variablesReference encoding task+frame index
- Implemented Variables handler returning named locals from DebugLocal table, filtered by PC range, with formatted values and decoded type strings
- Implemented Evaluate handler for local variable name lookup with descriptive errors for unknown names
- Updated StackTrace to use globally unique frame IDs (task_idx * 10000 + display_frame_index)
- Updated Stopped events to reference correct thread_id from actual task index
- All 6 VALIDATION.md test cases pass; all 53 workspace tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Multi-task Threads, globally-unique frame IDs, decode_frame_id** - `38beba5` (feat)
2. **Task 2: Scopes, Variables, and Evaluate handler unit tests** - `c01b810` (test)

**Plan metadata:** (docs commit follows)

_Note: Task 1 implemented all free functions (decode_frame_id, build_thread_list, collect_frame_variables, evaluate_local) and all DapServer methods. Task 2 added the unit tests for Task 2's behaviors._

## Files Created/Modified
- `writ-dap/src/server.rs` - Added decode_frame_id, build_thread_list, collect_frame_variables, evaluate_local free functions; added resolve_task_id, count_active_locals, get_variables, do_evaluate DapServer methods; replaced stub Threads/Scopes/Variables handlers; added Command::Evaluate handler; updated build_stack_frames for globally-unique IDs; added 6 unit tests

## Decisions Made
- `decode_frame_id` and `build_thread_list` implemented as module-level free functions so they can be unit-tested without a DapServer instance
- Frame IDs are globally unique across tasks: `task_idx * 10000 + display_frame_idx` — allows Scopes/Variables/Evaluate to unambiguously identify which task+frame the request targets
- `ScopePresentationhint::String(...)` required by dap 0.4.1-alpha1 — the `presentation_hint` field is a typed enum, not a raw String (auto-fixed during Task 1 GREEN phase)
- Stopped events use `task_id.index as i64` instead of hardcoded `1` — correct for multi-task debugging
- `collect_frame_variables` and `evaluate_local` are free functions — makes them directly testable with synthetic Module + registers

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ScopePresentationhint typed enum instead of String**
- **Found during:** Task 1 (GREEN phase compilation)
- **Issue:** Plan's code snippet used `Some("locals".to_string())` for `presentation_hint` but dap 0.4.1-alpha1 defines it as `Option<ScopePresentationhint>` — a typed enum, not `Option<String>`
- **Fix:** Changed to `Some(types::ScopePresentationhint::String("locals".to_string()))`
- **Files modified:** writ-dap/src/server.rs
- **Verification:** Compilation succeeded; tests passed
- **Committed in:** 38beba5 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary fix for the dap crate's actual API. No scope creep.

## Issues Encountered
- `BumpHeap` is in `writ_runtime::heap` / re-exported as `writ_runtime::BumpHeap`, not `writ_runtime::gc::BumpHeap` as the plan suggested — used correct path in tests

## Self-Check: PASSED

- FOUND: writ-dap/src/server.rs
- FOUND: .planning/phases/56-dap-advanced-inspection/56-02-SUMMARY.md
- FOUND commit: 38beba5 (Task 1)
- FOUND commit: c01b810 (Task 2)

## Next Phase Readiness
- DAP Variables panel, Watch panel, and Threads panel are now fully functional for Writ programs
- Phase 57 (VS Code extension bundling) can now ship a complete debugging experience
- Multi-task debugging infrastructure is in place; step commands still target the main task (cooperative scheduling)

---
*Phase: 56-dap-advanced-inspection*
*Completed: 2026-03-14*
