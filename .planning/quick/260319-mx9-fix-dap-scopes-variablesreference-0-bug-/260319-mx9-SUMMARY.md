---
phase: quick-260319-mx9
plan: 01
subsystem: writ-dap
tags: [dap, variables, bugfix]
dependency_graph:
  requires: []
  provides: [FIX-DAP-VARS-REF-0]
  affects: [writ-dap/src/variables.rs, writ-dap/tests/test_protocol.rs]
tech_stack:
  added: []
  patterns: ["+1 offset encoding for protocol sentinel value avoidance"]
key_files:
  created: []
  modified:
    - writ-dap/src/variables.rs
    - writ-dap/tests/test_protocol.rs
decisions:
  - "+1 offset in make_variables_ref rather than special-casing (0,0): keeps encoding uniform"
metrics:
  duration: "~2 minutes"
  completed_date: "2026-03-19"
  tasks_completed: 2
  files_modified: 2
---

# Phase quick-260319-mx9 Plan 01: Fix DAP scopes variablesReference=0 bug Summary

**One-liner:** Fixed `make_variables_ref(0,0)` returning 0 (DAP sentinel for "no children") by adding a +1 offset, preventing VSCode from silently skipping the variables request.

## What Was Done

The DAP protocol reserves `variablesReference=0` to mean "this scope has no children / is not expandable". When `task_idx=0` and `frame_idx=0` (the common case during single-task debugging), the old bit-packing produced exactly 0. VSCode interprets 0 as "no variables available" and never sends a `variables` request, so the Locals pane appeared permanently empty.

### Fix Applied

**`writ-dap/src/variables.rs`** — `make_variables_ref` / `unpack_variables_ref`:

```rust
// Before (broken):
pub fn make_variables_ref(task_idx: u32, frame_idx: u32) -> i64 {
    ((task_idx as i64) << 32) | (frame_idx as i64)  // (0,0) → 0
}

// After (fixed):
pub fn make_variables_ref(task_idx: u32, frame_idx: u32) -> i64 {
    (((task_idx as i64) << 32) | (frame_idx as i64)) + 1  // (0,0) → 1
}

pub fn unpack_variables_ref(r: i64) -> (u32, u32) {
    let r = r - 1;  // subtract offset before unpacking
    ((r >> 32) as u32, (r & 0xFFFF_FFFF) as u32)
}
```

The +1 offset is transparent to callers — `unpack_variables_ref` reverses it. The roundtrip invariant `unpack(make(t, f)) == (t, f)` holds for all valid inputs.

**`writ-dap/tests/test_protocol.rs`** — `test_breakpoint_hit_and_inspect`:

Added `assert_ne!(vars_ref, 0, ...)` immediately after extracting `variablesReference` from the scopes response, ensuring any regression is caught at test time.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Fix make_variables_ref/unpack_variables_ref with +1 offset | 0bfc125 | writ-dap/src/variables.rs |
| 2 | Add assert_ne!(vars_ref, 0) in integration test | 017fc30 | writ-dap/tests/test_protocol.rs |

## Test Results

- `cargo test -p writ-dap --lib variables`: 27 passed (all unit tests in variables.rs)
- `cargo test -p writ-dap -- --nocapture`: 108 passed across all test files, 0 failed

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- writ-dap/src/variables.rs: modified, contains `make_variables_ref` with +1 offset
- writ-dap/tests/test_protocol.rs: modified, contains `vars_ref != 0` assertion
- Commit 0bfc125: FOUND
- Commit 017fc30: FOUND
- All tests pass
