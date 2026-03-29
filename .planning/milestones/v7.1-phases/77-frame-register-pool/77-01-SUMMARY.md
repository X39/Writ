---
phase: 77-frame-register-pool
plan: 01
subsystem: runtime
tags: [register-pool, free-list, vm, performance, allocation]

# Dependency graph
requires:
  - phase: 76-zero-alloc-call-convention
    provides: zero-allocation arg passing pattern established; writ-runtime call dispatch baseline
provides:
  - RegisterPool struct with acquire/release free-list in writ-runtime/src/frame.rs
  - CallFrame::with_pool constructor for pool-based frame allocation
  - 5 pool correctness tests covering FRAME-01/02/03/04/06
affects:
  - 77-02 (plan 02 integrates RegisterPool into dispatch hot-path)
  - 78-inner-dispatch-loop (builds on reduced per-call allocation cost)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Free-list pool: cap at POOL_CAP=64 entries; release fills+clears (preserving capacity); acquire scans from back for capacity >= reg_count"
    - "TDD for pool logic: behavior spec drives test structure before integration"

key-files:
  created:
    - writ-runtime/tests/pool_tests.rs
  modified:
    - writ-runtime/src/frame.rs
    - writ-runtime/src/lib.rs

key-decisions:
  - "fill-then-clear release sequence: v.fill(Value::Void) drops any heap-allocated values in registers (FRAME-03 safety), v.clear() resets len to 0 while preserving capacity for the next acquire's resize"
  - "rposition scan from back: most recently released Vec is most likely to have matching capacity for recursive workloads — amortizes search cost"
  - "POOL_CAP=64: bounds free-list memory use; 64 entries covers all realistic concurrent call depths"

patterns-established:
  - "Pattern 1: Pool acquire always returns len==reg_count all-Void — callers never need to initialize registers after acquire"
  - "Pattern 2: Pool release is fire-and-forget — if free-list full, Vec is dropped silently (no error)"

requirements-completed: [FRAME-01, FRAME-02, FRAME-03, FRAME-04, FRAME-06]

# Metrics
duration: 10min
completed: 2026-03-22
---

# Phase 77 Plan 01: RegisterPool — Free-List Register Vec Reuse Summary

**RegisterPool free-list with acquire/release in frame.rs: fill-then-clear release, capacity-checked reuse, POOL_CAP=64 cap, and 5 FRAME-01/02/03/04/06 correctness tests**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-22T13:30:00Z
- **Completed:** 2026-03-22T13:40:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- RegisterPool struct added to frame.rs: acquire scans free-list from back for capacity >= reg_count, resize+return; release fills with Value::Void, clears, caps at 64 entries
- CallFrame::with_pool constructor delegates register Vec acquisition to pool (hot-path entry point for plan 02)
- RegisterPool re-exported from writ-runtime/src/lib.rs for use by dispatch module in plan 02
- 5 pool correctness tests covering all FRAME requirements: fresh alloc, capacity check, reuse clearing, cap enforcement, with_pool constructor

## Task Commits

Each task was committed atomically:

1. **Task 1: RegisterPool struct and pool-aware CallFrame constructor** - `bf19391` (feat)
2. **Task 2: Pool correctness tests** - `e78ccc8` (test)

**Plan metadata:** (docs commit below)

## Files Created/Modified
- `writ-runtime/src/frame.rs` - Added RegisterPool struct, acquire/release methods, POOL_CAP constant, CallFrame::with_pool constructor
- `writ-runtime/src/lib.rs` - Added RegisterPool to public re-export
- `writ-runtime/tests/pool_tests.rs` - 5 pool correctness tests (FRAME-01/02/03/04/06)

## Decisions Made
- **fill-then-clear release sequence:** `v.fill(Value::Void)` drops held heap values (GC-safety, FRAME-03), then `v.clear()` resets len to 0 while preserving capacity. This ensures acquire's `resize(reg_count, Value::Void)` works from an empty base.
- **rposition (back-to-front scan):** Most recently released Vec sits at the back of the free-list and is most likely to be the right size for recursive workloads, giving near-O(1) hit rate in practice.
- **POOL_CAP=64:** Bounds worst-case memory overhead. At 8+ words per Value and typical 8-32 regs per frame, 64 entries uses <1MB even at max register count.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- RegisterPool is complete and re-exported; plan 02 can integrate it directly into exec_call / exec_call_virt / exec_call_indirect dispatch handlers
- CallFrame::with_pool constructor is the integration entry point — plan 02 replaces `CallFrame::new(...)` with `CallFrame::with_pool(&mut pool, ...)` and adds release on frame pop
- Full test suite green; zero warnings

---
*Phase: 77-frame-register-pool*
*Completed: 2026-03-22*

## Self-Check: PASSED

- FOUND: writ-runtime/src/frame.rs
- FOUND: writ-runtime/src/lib.rs
- FOUND: writ-runtime/tests/pool_tests.rs
- FOUND: .planning/phases/77-frame-register-pool/77-01-SUMMARY.md
- FOUND commit: bf19391 (feat: RegisterPool struct)
- FOUND commit: e78ccc8 (test: pool correctness tests)
