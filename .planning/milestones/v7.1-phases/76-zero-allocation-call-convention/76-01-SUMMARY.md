---
phase: 76-zero-allocation-call-convention
plan: 01
subsystem: testing
tags: [writ-runtime, vm, tail-call, call-indirect, tdd]

# Dependency graph
requires: []
provides:
  - tail_call_passes_multiple_args test (argc=2, AddI r0+r1->r2, asserts Int(30))
  - call_indirect_passes_args test (argc=1 delegate dispatch, asserts Int(99))
affects: [76-02-zero-allocation-refactor]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two-test safety net pattern: multi-arg tail-call + arg-passing call-indirect written before zero-alloc refactor"

key-files:
  created: []
  modified:
    - writ-runtime/tests/vm_tests.rs

key-decisions:
  - "Instruction::AddI not Add — the integer addition variant in writ-module uses the AddI name; fixed inline per deviation Rule 1"

patterns-established:
  - "build_two_method_runtime helper: main_reg_count/callee_reg_count must cover all registers used by each method"

requirements-completed: [CALL-05]

# Metrics
duration: 5min
completed: 2026-03-22
---

# Phase 76 Plan 01: Zero-Allocation Call Convention — Safety Net Tests Summary

**Two regression-guard tests added to writ-runtime: multi-arg TailCall (argc=2, r0+r1=30) and arg-passing CallIndirect (argc=1, delegate receives 99)**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-22T06:04:30Z
- **Completed:** 2026-03-22T06:08:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added `tail_call_passes_multiple_args`: method 0 loads Int(10) into r0 and Int(20) into r1, issues TailCall (method_idx=0x07000002, r_base=0, argc=2); callee does AddI r0+r1->r2 and returns r2; asserts Value::Int(30)
- Added `call_indirect_passes_args`: method 0 loads Int(99) into r1, creates delegate to method 1 (r2=Void target), issues CallIndirect (r_dst=3, r_delegate=0, r_base=1, argc=1); callee returns r0; asserts Value::Int(99)
- All 90 vm_tests pass (up from 88); all other writ-runtime test suites pass unmodified

## Task Commits

Each task was committed atomically:

1. **Task 1: Add tail_call_passes_multiple_args and call_indirect_passes_args tests** - `10be66d` (test)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `writ-runtime/tests/vm_tests.rs` — Two new test functions inserted: `tail_call_passes_multiple_args` after `tail_call_does_not_grow_stack`, `call_indirect_passes_args` after `new_delegate_and_call_indirect`

## Decisions Made

- Used `Instruction::AddI` (not `Add`) — discovered that the integer add instruction in writ-module is named `AddI`, matching the opcode table. Corrected inline.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected Add -> AddI in tail_call_passes_multiple_args**
- **Found during:** Task 1 (compiling the test)
- **Issue:** Plan's `<interfaces>` block listed `Instruction::Add` but the actual variant in writ-module is `Instruction::AddI`
- **Fix:** Changed `Add { r_dst: 2, r_a: 0, r_b: 1 }` to `AddI { r_dst: 2, r_a: 0, r_b: 1 }`
- **Files modified:** writ-runtime/tests/vm_tests.rs
- **Verification:** `cargo test --release -p writ-runtime` — 90/90 pass
- **Committed in:** 10be66d (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — wrong variant name in plan interfaces)
**Impact on plan:** Trivial one-character fix. No scope creep.

## Issues Encountered

None — tests passed immediately after the AddI correction.

## Next Phase Readiness

- Safety net tests are in place. Plan 02 (zero-allocation call convention refactor) can now proceed knowing that any regression in multi-arg tail-call or arg-passing call-indirect dispatch will be caught immediately.
- No blockers.

---
*Phase: 76-zero-allocation-call-convention*
*Completed: 2026-03-22*
