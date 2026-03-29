---
phase: 75-baseline-build-config-and-inline-annotations
plan: 02
subsystem: runtime
tags: [benchmark, baseline, fib, release-profile, lto, timing, measurement]

requires:
  - phase: 75-01
    provides: Release profile with LTO fat/codegen-units=1/panic=abort, FxHashMap across writ-runtime hot paths, inline annotations on dispatch helpers

provides:
  - benchmark/BASELINE.md with fib(40) median 83.297s (3 measured runs under fully-optimized release build)
  - v7.1 pre-optimization reference point for all subsequent phase comparisons (Phases 76-79)
  - Confirmed correct fib(40) output 102334155 in release mode
  - Full workspace test pass confirmation (all tests ok, zero failures) in release mode
  - Zero warnings on release build

affects: [76-zero-alloc-call-convention, 77-frame-register-pool, 78-inner-dispatch-loop, 79-copy-value]

tech-stack:
  added: []
  patterns:
    - "Baseline measurement pattern: 3 cold runs, take median as authoritative figure"
    - "BASELINE.md documents build config, platform, commit hash, and prior reference for historical tracing"

key-files:
  created:
    - benchmark/BASELINE.md
  modified:
    - Cargo.lock

key-decisions:
  - "Median of 3 cold runs (78.798s, 83.297s, 93.202s) = 83.297s is the v7.1 pre-optimization baseline"
  - "Cargo.lock committed as part of plan 02 (rustc-hash entry was missing from prior commit)"

patterns-established:
  - "All future phase benchmarks compare against median 83.297s from this baseline"

requirements-completed: [BUILD-05, VERIFY-01, VERIFY-02, VERIFY-03]

duration: 12min
completed: 2026-03-22
---

# Phase 75 Plan 02: Baseline Build Config and Inline Annotations Summary

**fib(40) measured at 83.297s median (3 runs: 78.8s/83.3s/93.2s) under LTO fat + FxHashMap + inline release build on AMD Ryzen 9 5900X — v7.1 pre-optimization baseline committed to benchmark/BASELINE.md**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-22T05:36:03Z
- **Completed:** 2026-03-22T05:48:55Z
- **Tasks:** 2
- **Files modified:** 2 (benchmark/BASELINE.md created, Cargo.lock updated)

## Accomplishments

- Full workspace release build verified: `cargo build --release` exits 0 with zero warnings (LTO fat build completed in ~2 min)
- Full workspace tests verified: `cargo test --release` passes all tests with zero failures across all crates
- fib(40) correctness verified: output `102334155` confirmed from release binary
- fib(40) measured 3 times: 78.798s, 83.297s, 93.202s — median **83.297s**
- benchmark/BASELINE.md created with all required fields: 3 timing runs, median, build config, platform, commit hash, prior reference
- Cargo.lock updated to include rustc-hash 2.1.1 entry (was uncommitted from Plan 01)

## Task Commits

1. **Task 1: Full workspace verification** - `97618a2` (chore — Cargo.lock update with rustc-hash)
2. **Task 2: Measure fib(40) baseline and create BASELINE.md** - `3bb7b08` (feat)

**Plan metadata commit to follow.**

## Files Created/Modified

- `benchmark/BASELINE.md` - v7.1 pre-optimization baseline timing document (3 runs, median 83.297s, build config, platform)
- `Cargo.lock` - Updated to reflect rustc-hash 2.1.1 dependency in writ-runtime

## Decisions Made

- Median of 3 cold runs (78.798s, 83.297s, 93.202s) = 83.297s is the authoritative v7.1 pre-optimization baseline
- Windows timing shows variance (~15s spread across 3 runs); median chosen over mean to reduce outlier impact
- Cargo.lock committed in this plan (rustc-hash entry was missing from Plan 01 commit — build still succeeded because Cargo.lock is regenerated, but the repo should track the exact dependency version)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Cargo.lock had an untracked change (rustc-hash entry not committed in Plan 01). Added as a chore commit before Task 1 verification commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Baseline established: 83.297s median fib(40) under fully-optimized release build
- Phase 76 (zero-alloc call convention) can begin immediately — baseline is the reference for measuring Phase 76 speedup
- All VERIFY-01/02/03 criteria confirmed passing at this baseline; they must remain passing through Phases 76-79

---
*Phase: 75-baseline-build-config-and-inline-annotations*
*Completed: 2026-03-22*

## Self-Check: PASSED

- benchmark/BASELINE.md: FOUND
- 75-02-SUMMARY.md: FOUND
- Commit 97618a2 (Cargo.lock/chore): FOUND
- Commit 3bb7b08 (BASELINE.md/feat): FOUND
