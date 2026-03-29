---
phase: 121-stdlib-rewrite
plan: 01
subsystem: stdlib
tags: [writ-std, collections, arrays, fixed-size-array-api]

# Dependency graph
requires:
  - phase: 120-array-semantics-correction
    provides: "Fixed-size array API (resize, len, indexed access) — removed growth/removal/membership primitives from T[]"
provides:
  - "All four collection classes (List<T>, Map<K,V>, Set<T>, HashMap<K,V>) rewritten to use only fixed-size array API"
  - "Zero calls to removed array methods (add/remove_at/contains) on any backing array"
affects:
  - 121-02-PLAN.md
  - writ-std compilation via writ-cli

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "resize(old_len+1) + indexed store pattern for array append"
    - "shift-loop + resize(old_len-1) pattern for array element removal"
    - "Manual scan loop for membership testing"

key-files:
  created: []
  modified:
    - writ-std/src/collections.writ

key-decisions:
  - "Method bodies rewritten in-place; all public API signatures kept identical"
  - "result.add() in List.map/filter calls the public List method (not the array method) — left unchanged"
  - "Map.set and HashMap.set use parallel-array resize-both pattern to keep keys[] and values[] in sync"

patterns-established:
  - "Append pattern: old_len = arr.len(); arr.resize(old_len+1); arr[old_len] = item"
  - "Remove-at pattern: shift loop (j < old_len-1), then arr.resize(old_len-1)"

requirements-completed: [STD-01, STD-02, STD-03, STD-04]

# Metrics
duration: 2min
completed: 2026-03-29
---

# Phase 121 Plan 01: Stdlib Rewrite Summary

**All four collection classes rewritten to use only resize/indexed-assignment/len on backing arrays — zero calls to the Phase 120-removed add/remove_at/contains primitives**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-29T21:35:23Z
- **Completed:** 2026-03-29T21:37:16Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- List<T>.add/remove_at/has bodies replaced with resize+store, shift-loop+resize, and manual scan patterns
- Set<T>.add/remove/has bodies replaced with identical patterns
- Map<K,V>.set/remove bodies replaced with parallel-array resize+store and parallel shift-loop+resize
- HashMap<K,V>.set/remove bodies replaced with the same Map patterns
- `cargo build -p writ-cli` succeeds with zero "writ-std compilation failed" warnings
- 12 `.resize()` calls now present across the file confirming all growth/shrink operations use the new API

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite List and Set classes to use fixed-size array API** - `5fecea5` (feat)
2. **Task 2: Rewrite Map and HashMap classes to use fixed-size array API** - `a3c6d13` (feat)

## Files Created/Modified

- `writ-std/src/collections.writ` - All four collection class impl blocks rewritten to use only resize/indexed access/len; public API signatures unchanged

## Decisions Made

- Method bodies replaced in-place with no signature changes — preserves all call sites in user programs
- `result.add()` in `List.map` and `List.filter` correctly calls the public `List<T>.add` method, not the array primitive — left untouched as confirmed correct by plan
- For `Map` and `HashMap`, `set` resizes both `keys` and `values` arrays atomically using a single `old_len` snapshot to guarantee they stay in sync
- For `Map` and `HashMap`, `remove` shifts both parallel arrays in a single loop to keep indices aligned, then resizes both

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- collections.writ is ready for Plan 02 (which handles remaining stdlib files if any, or validation)
- All four collection types compile against the Phase 120 fixed-size array API
- Public API is unchanged — existing callers remain compatible

---
*Phase: 121-stdlib-rewrite*
*Completed: 2026-03-29*
