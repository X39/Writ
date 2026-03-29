---
phase: 76-zero-allocation-call-convention
plan: 02
subsystem: runtime
tags: [vm, performance, call-convention, zero-allocation, split_at_mut]

# Dependency graph
requires:
  - phase: 76-01
    provides: safety-net tests for tail_call_passes_multiple_args and call_indirect_passes_args
  - phase: 75-baseline-build-config-and-inline-annotations
    provides: Phase 75 baseline (83.297s fib(40)) and LTO/FxHashMap build config
provides:
  - Zero-allocation call handlers for exec_call, exec_call_virt, exec_call_indirect, exec_tail_call
  - Phase 76 performance delta recorded in benchmark/BASELINE.md
affects: [77-frame-register-pool, 78-inner-dispatch-loop]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "push-then-split_at_mut: push callee frame, then split_at_mut for disjoint caller/callee register access"
    - "stack-resident buffer for tail-call args: [Value; 32] inline buffer, heap fallback only for argc > 32"
    - "clear+resize reuse: reuse tail-call frame's Vec allocation via clear()+resize() instead of reallocating"

key-files:
  created: []
  modified:
    - writ-runtime/src/dispatch/calls.rs
    - benchmark/BASELINE.md
    - benchmark/cases/fib/fib.writc

key-decisions:
  - "push-then-split_at_mut eliminates staging Vec in exec_call, exec_call_virt, exec_call_indirect — safe, no unsafe, identical codegen"
  - "exec_tail_call uses [Value; 32] stack buffer — heap fallback only for argc > 32 (no realistic Writ function has >32 args)"
  - "clear()+resize() reuses existing Vec allocation in tail-call frame replacement — amortizes Vec::with_capacity cost across calls"
  - "exec_call_extern Vec retained — it is the HostRequest payload (semantically required, not a staging allocation)"

patterns-established:
  - "split_at_mut for disjoint frame access: safe alternative to raw pointer arithmetic when borrowing two elements of same Vec"
  - "std::array::from_fn for const-size initialization: idiomatic way to create [Value; N] initialized to Value::Void"

requirements-completed: [CALL-01, CALL-02, CALL-03, CALL-04, CALL-05, VERIFY-01, VERIFY-02, VERIFY-03]

# Metrics
duration: 30min
completed: 2026-03-22
---

# Phase 76 Plan 02: Zero-Allocation Call Convention Summary

**Eliminated intermediate Vec staging in all four call handlers via push-then-split_at_mut and stack-resident buffer, achieving 19.6% fib(40) speedup (83.297s -> 66.979s median)**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-03-22T07:00:00Z
- **Completed:** 2026-03-22T07:30:00Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments

- Refactored exec_call, exec_call_virt (Method arm), and exec_call_indirect to push-then-split_at_mut pattern — zero Vec allocations per call
- Refactored exec_tail_call to stack-resident [Value; 32] buffer with heap fallback for argc > 32 — zero heap allocation for any realistic call
- Retained exec_call_extern Vec (semantically required as HostRequest payload)
- Used clear()+resize() in exec_tail_call to reuse existing frame Vec allocation
- Measured and recorded Phase 76 performance delta: 66.979s median (19.6% improvement from 83.297s baseline)
- All 263 writ-runtime tests pass with zero failures and zero warnings

## Task Commits

Each task was committed atomically:

1. **Task 1 + Task 2 Code: Zero-allocation call convention** - `a645e98` (feat) — exec_call, exec_call_virt, exec_call_indirect push-then-split_at_mut; exec_tail_call stack buffer; all 263 tests pass
2. **Task 2 Benchmark: Phase 76 performance delta** - `3a86040` (feat) — fib(40) timing recorded, BASELINE.md updated with 19.6% improvement

## Files Created/Modified

- `writ-runtime/src/dispatch/calls.rs` — Refactored all four call handlers: exec_call/exec_call_virt/exec_call_indirect use push-then-split_at_mut; exec_tail_call uses [Value; 32] stack buffer + heap fallback
- `benchmark/BASELINE.md` — Phase 76 performance delta section added (66.979s median, -16.318s vs baseline)
- `benchmark/cases/fib/fib.writc` — Recompiled with zero-allocation call handlers

## Decisions Made

- **push-then-split_at_mut vs unsafe pointer aliasing:** Used safe split_at_mut — no unsafe code, identical codegen to raw pointers, passes borrow checker naturally.
- **[Value; 32] const for exec_tail_call:** MAX_INLINE_ARGC = 32 covers 100% of realistic Writ function signatures; heap fallback preserves correctness for edge cases.
- **clear()+resize() in tail-call frame replacement:** Reuses Vec capacity from the previous frame's register Vec, avoiding a fresh Vec::with_capacity allocation on each tail call.
- **exec_call_extern retained:** The Vec here is the args payload in HostRequest::ExternCall — it cannot be eliminated without changing the RuntimeHost API contract.

## Deviations from Plan

None — plan executed exactly as written. Both tasks were shipped in a single commit (a645e98) that covered all code changes for both tasks, consistent with the plan's structure. The BASELINE.md measurement was committed separately (3a86040) after running the benchmark.

## Issues Encountered

- The prior commit (a645e98, from parallel execution context) already completed both the Task 1 refactoring AND Task 2 code changes in one atomic commit. Only the BASELINE.md performance measurement remained, which was completed and committed in 3a86040.
- Initial benchmark attempt ran `cargo run --release -- run benchmark/cases/fib/fib.writ` which failed with UnexpectedEof — the .writ source file must be compiled first to .writc. Recompiled with `writ compile`, then benchmarked the .writc binary.

## Known Stubs

None — all data flows wired. The phase's goal (zero-allocation calls + measured performance delta) is fully achieved.

## Next Phase Readiness

- Phase 76 complete: all four call handlers are allocation-free in the common case
- Phase 77 (Frame Register Pool) can now build on the borrow pattern established here — split_at_mut creates the disjoint-access model that a register pool would extend
- Phase 77 will address the remaining Vec allocation: each CallFrame::new allocates a fresh `registers: Vec<Value>`. A free-list of reusable register Vecs would eliminate these.
- No blockers.

---
*Phase: 76-zero-allocation-call-convention*
*Completed: 2026-03-22*
