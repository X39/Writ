---
phase: quick-260319-nbg
plan: 01
subsystem: dap, compiler
tags: [bug-fix, dap, debug-info, variables, integration-test]
dependency_graph:
  requires: []
  provides: [correct-variable-names-in-dap-variables-panel]
  affects: [writ-dap, writ-compiler]
tech_stack:
  added: []
  patterns: [byte-offset-PC-conversion, debug-local-filtering]
key_files:
  created: []
  modified:
    - writ-compiler/src/emit/serialize.rs
    - writ-dap/src/server/helpers.rs
    - writ-dap/tests/test_protocol.rs
decisions:
  - Updated test_breakpoint_hit_and_inspect breakpoint from line 11 to line 12 so x is already assigned and in scope; the old test was accidentally passing because incorrect PCs made all variables visible everywhere
  - Added final string heap update in translate() after body processing, mirroring the existing blob_heap pattern
metrics:
  duration: ~20min
  completed: "2026-03-19T16:00:52Z"
  tasks: 2
  files: 3
---

# Phase quick-260319-nbg Plan 01: Fix DAP Variables Missing Names Summary

Three-bug fix: byte-offset PC conversion in debug locals, string heap snapshot timing bug, and unnamed-temporary filtering — restoring source-level variable names in the VSCode Variables panel.

## What Was Built

Fixed three inter-related bugs that prevented source-level variable names from appearing in the DAP Variables panel when a breakpoint was hit in VSCode:

**Bug 1 — PC conversion (BUG-15):** `build_debug_locals` in `serialize.rs` was storing instruction-index PCs directly as `start_pc`/`end_pc` in `DebugLocal` entries. The DAP server's `collect_frame_variables` was comparing these against byte-offset PCs, causing scope range mismatches. Fixed by passing `instr_byte_starts` to `build_debug_locals` and converting each PC using `instr_byte_starts.get(instr_idx).copied()`.

**Bug 2 — String heap snapshot timing:** `translate()` copied `builder.string_heap` into `module.string_heap` at the top of the function (line 44), before body processing. `build_debug_locals` then interned variable names into `builder.string_heap` and returned offsets — but those offsets weren't in the already-copied `module.string_heap`. This caused `read_string` to fail at runtime, returning the `"?"` fallback. Fixed by adding a final `module.string_heap = builder.string_heap.data().to_vec()` after all body processing (mirroring the existing `blob_heap` pattern).

**Bug 3 — Unnamed temporaries:** `collect_frame_variables` included all DebugLocal entries regardless of whether they had a name. Unnamed temporaries (computes, intermediate values) had `name_offset = 0` (the empty string sentinel). Fixed by adding `.filter(|dl| dl.name != 0)` before the PC range filter.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix build_debug_locals PC conversion and filter unnamed temporaries | 47c957a | writ-compiler/src/emit/serialize.rs, writ-dap/src/server/helpers.rs |
| 2 | Add integration test validating variable names on breakpoint hit | 0fa2404 | writ-dap/tests/test_protocol.rs |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] String heap snapshot taken before debug local name interning**

- **Found during:** Task 1 verification
- **Issue:** `translate()` copied `builder.string_heap` into `module.string_heap` at the start of the function, before `build_debug_locals` had interned variable names. The DebugLocal offsets pointed past the end of the already-copied heap, causing `read_string` to return `Err` and show `"?"` as the variable name.
- **Fix:** Added `module.string_heap = builder.string_heap.data().to_vec()` after all body processing, after the existing `module.blob_heap` update (same pattern).
- **Files modified:** writ-compiler/src/emit/serialize.rs
- **Commit:** 47c957a (included with Task 1)

**2. [Rule 1 - Bug] test_breakpoint_hit_and_inspect used line 11 (no variables yet)**

- **Found during:** Task 1 verification
- **Issue:** The existing test broke on line 11 (`let x = add(3,4)`) and expected `variables.is_empty() == false`. With the correct PC fix, `x` is only live AFTER line 11 completes — so no named variables are in scope at the line 11 breakpoint (correct behavior). The test was accidentally passing before because all variables had PC=0 due to the uninitialized heap offset bug.
- **Fix:** Updated breakpoint to line 12 and the line assertion from 11 to 12, so `x` is already assigned and visible. Added the empty-name regression guard after the existing non-empty assertion.
- **Files modified:** writ-dap/tests/test_protocol.rs
- **Commit:** 0fa2404 (included with Task 2)

## Self-Check: PASSED

- FOUND: writ-compiler/src/emit/serialize.rs
- FOUND: writ-dap/src/server/helpers.rs
- FOUND: writ-dap/tests/test_protocol.rs
- FOUND commit 47c957a: fix(quick-260319-nbg): fix DAP variables missing names
- FOUND commit 0fa2404: test(quick-260319-nbg): add variable name assertions to DAP integration tests
