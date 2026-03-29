---
phase: 78-inner-dispatch-loop
plan: 01
subsystem: runtime
tags: [vm, dispatch, batch, execute_batch, scheduler, performance]

# Dependency graph
requires:
  - phase: 77-frame-register-pool
    provides: RegisterPool threading through execute pipeline; exec_call/exec_ret using pool
  - phase: 76-zero-alloc-call-convention
    provides: Zero-allocation arg passing; push-then-split_at_mut pattern
provides:
  - execute_batch function in dispatch/mod.rs holding &mut Task across instruction batches
  - Restructured run_one_task calling execute_batch instead of execute_one
  - Batch dispatch tests covering DISPATCH-01 through DISPATCH-05
  - Phase 78 fib(40) benchmark: 53.134s median (-11.1% vs Phase 77)
affects: [79-copy-value-enum, future-jit]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "execute_batch inner loop: eliminates per-instruction HashMap task lookup by holding &mut Task across batch"
    - "debug path: execute_batch delegates to execute_one when host.debug_enabled() is true"
    - "atomic-section awareness: limit check only when task.atomic_depth == 0"
    - "outer scheduler loop: retained in run_one_task for concurrency results (SpawnChild, JoinTask, etc.)"

key-files:
  created: []
  modified:
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/tests/vm_tests.rs
    - benchmark/BASELINE.md

key-decisions:
  - "execute_batch holds &mut Task for the entire batch via execute_one calls, not via direct frame access — frame reference is re-acquired each call (DISPATCH-05)"
  - "execute_batch has no #[inline] annotation (same reasoning as execute_one — too large, would bloat callers)"
  - "CmpLtI with swapped operands used for n<=1 base case in fib test (no CmpLeI instruction exists in the VM)"
  - "Branch offsets in test construction are byte-relative; loader converts to instruction indices in pass 2"

patterns-established:
  - "Batch dispatch pattern: hold task reference across full budget, return on any non-Continue result"
  - "Debug fallback: single-instruction path preserves before_instruction hook granularity"

requirements-completed: [DISPATCH-01, DISPATCH-02, DISPATCH-03, DISPATCH-04, DISPATCH-05, VERIFY-01, VERIFY-02, VERIFY-03]

# Metrics
duration: 40min
completed: 2026-03-22
---

# Phase 78 Plan 01: Inner Dispatch Loop Summary

**execute_batch inner dispatch loop eliminates ~300M per-instruction FxHashMap task lookups for fib(40), reducing runtime from 59.800s to 53.134s (-11.1%, -36.2% cumulative vs Phase 75 baseline)**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-03-22
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `execute_batch` to `writ-runtime/src/dispatch/mod.rs`: holds `&mut Task` across entire instruction batch, eliminating the per-instruction `FxHashMap::get_mut` in the scheduler hot path
- Restructured `run_one_task` in `scheduler.rs` to call `execute_batch` instead of `execute_one`, removing the per-instruction limit check and HashMap lookup
- Debug path preserved: `execute_batch` falls back to single-instruction `execute_one` when `host.debug_enabled()` is true
- Atomic-section awareness: limit check only fires when `task.atomic_depth == 0` (never yields mid-atomic)
- Added `batch_dispatch_fib_correctness` test: fib(10)=55 via recursive batch dispatch
- Added `batch_respects_execution_limit` test: loop that needs ~303 instructions stops at limit=10, then completes with unlimited run
- fib(40) median: 53.134s (correct output 102334155 on all 3 runs)
- All 400+ tests pass with zero failures and zero warnings

## Task Commits

1. **Task 1: Add execute_batch and restructure run_one_task** - `3293ff6` (feat)
2. **Task 2: Batch dispatch tests and fib(40) benchmark** - `2a7b318` (test)

## Files Created/Modified

- `/writ-runtime/src/dispatch/mod.rs` - Added `execute_batch` function (52 lines) before `execute_ret`
- `/writ-runtime/src/scheduler.rs` - Replaced `execute_one` import with `execute_batch`; replaced inner loop with single `execute_batch` call; removed `instructions_run` counter
- `/writ-runtime/tests/vm_tests.rs` - Added `batch_dispatch_fib_correctness` and `batch_respects_execution_limit` tests
- `/benchmark/BASELINE.md` - Added Phase 78 section with measured timing data

## Decisions Made

- `execute_batch` has the same parameter signature as `execute_one` plus `limit: u64` (0 = unlimited). This threading approach avoids needing a BatchContext wrapper (research flag resolved: no borrow conflicts surfaced).
- No `CmpLeI` instruction exists in the VM — used `CmpLtI { r_a: 1, r_b: 0 }` (checks `1 < n`, i.e., `n > 1`) with `BrFalse` for the fib base case.
- Branch offsets in test programs are byte-relative; the loader's `decode_and_reindex` converts them to instruction indices in pass 2. Byte layout was computed manually for test correctness.
- `execute_batch` does not use `#[inline]` — consistent with `execute_one` policy (300+ arm match body, would bloat callers).

## Deviations from Plan

None - plan executed exactly as written. The `CmpLeI` limitation noted in the important_notes was handled by using `CmpLtI` with swapped operands as planned.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 79 (Copy-semantic Value enum) is the next optimization: moving `InlineStruct` fields to heap and deriving `Copy` on `Value`. Research flag noted: requires workspace-wide grep for `InlineStruct` before starting, and a GC regression test must pass before changing any `value.rs` match arms.

## Self-Check: PASSED

- FOUND: writ-runtime/src/dispatch/mod.rs (execute_batch at line 513)
- FOUND: writ-runtime/src/scheduler.rs (execute_batch called, execute_one removed)
- FOUND: writ-runtime/tests/vm_tests.rs (batch_dispatch_fib_correctness, batch_respects_execution_limit)
- FOUND: benchmark/BASELINE.md (Phase 78 section)
- FOUND: .planning/phases/78-inner-dispatch-loop/78-01-SUMMARY.md
- COMMIT 3293ff6: feat(78-01): add execute_batch inner dispatch loop; restructure run_one_task
- COMMIT 2a7b318: test(78-01): add batch dispatch tests and record fib(40) performance delta

---
*Phase: 78-inner-dispatch-loop*
*Completed: 2026-03-22*
