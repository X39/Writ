---
phase: 121-stdlib-rewrite
plan: 02
subsystem: stdlib
tags: [writ-std, collections, golden-tests, fixed-size-array-api]

# Dependency graph
requires:
  - phase: 121-01
    provides: "All four collection classes rewritten to use fixed-size array API"
provides:
  - "8 un-ignored collection/iterator golden tests with blessed IL snapshots"
  - "coll_list_basic, coll_map_basic, coll_set_basic, coll_hashmap_basic passing"
  - "coll_list_map, coll_list_filter, coll_list_reduce, iter_for_in_list passing"
  - "All .writil snapshots contain ARRAY_RESIZE instead of ARRAY_ADD/ARRAY_REMOVE"
affects:
  - writ-golden full suite (72 tests, 0 failures)
  - Phase 121 completion

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Golden test driver .writ files use same resize+indexed-store pattern as stdlib"
    - "Standalone driver files are self-contained (no stdlib import) — must mirror API changes"

key-files:
  created: []
  modified:
    - writ-golden/tests/golden_tests.rs
    - writ-golden/tests/golden/coll_list_basic.writ
    - writ-golden/tests/golden/coll_list_basic.writil
    - writ-golden/tests/golden/coll_map_basic.writ
    - writ-golden/tests/golden/coll_map_basic.writil
    - writ-golden/tests/golden/coll_set_basic.writ
    - writ-golden/tests/golden/coll_set_basic.writil
    - writ-golden/tests/golden/coll_hashmap_basic.writ
    - writ-golden/tests/golden/coll_hashmap_basic.writil
    - writ-golden/tests/golden/coll_list_map.writ
    - writ-golden/tests/golden/coll_list_map.writil
    - writ-golden/tests/golden/coll_list_filter.writ
    - writ-golden/tests/golden/coll_list_filter.writil
    - writ-golden/tests/golden/coll_list_reduce.writ
    - writ-golden/tests/golden/coll_list_reduce.writil
    - writ-golden/tests/golden/iter_for_in_list.writ
    - writ-golden/tests/golden/iter_for_in_list.writil

key-decisions:
  - "Golden test .writ drivers are standalone (no stdlib import) — they required the same resize API update as collections.writ"
  - "All driver files updated to use resize(old_len+1)/indexed-store append and shift+resize removal patterns"
  - "Pre-existing non-collection test failures are out of scope per Phase 120-03 notes"

requirements-completed: [STD-01, STD-02, STD-03, STD-04, STD-05]

# Metrics
duration: 5min
completed: 2026-03-29
---

# Phase 121 Plan 02: Golden Test Re-blessing Summary

**8 collection and iterator golden tests un-ignored and re-blessed — all pass with ARRAY_RESIZE in IL snapshots; full suite 72 passed, 0 failed**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-29
- **Completed:** 2026-03-29
- **Tasks:** 2
- **Files modified:** 17

## Accomplishments

- Removed all 8 `#[ignore]` attributes and Phase 121 doc comments from `golden_tests.rs`
- Discovered and fixed that golden test `.writ` driver files are standalone (no stdlib import) — they still contained old array API calls (`.add()`, `.remove_at()`, `.contains()`)
- Updated all 8 `.writ` driver files to use the resize+indexed-store and shift+resize patterns
- Ran BLESS=1 to regenerate all 8 `.writil` snapshots reflecting new ARRAY_RESIZE opcodes
- Full golden test suite: 72 passed, 0 failed, 0 ignored
- `cargo build -p writ-cli` produces no "writ-std compilation failed" warning

## Task Commits

Each task was committed atomically:

1. **Task 1: Un-ignore collection and iterator golden tests** - `f1a9b80` (chore)
2. **Task 2: Re-bless golden IL snapshots and verify all tests pass** - `9ff7736` (feat)

## Files Created/Modified

- `writ-golden/tests/golden_tests.rs` - Removed 8 `#[ignore]` attributes and Phase 121 doc comments
- `writ-golden/tests/golden/coll_list_basic.writ` - Updated List.add to use resize pattern; List.remove_at and List.has use shift+resize and manual scan
- `writ-golden/tests/golden/coll_list_basic.writil` - Blessed snapshot with ARRAY_RESIZE
- `writ-golden/tests/golden/coll_map_basic.writ` - Updated Map.set and Map.remove to use parallel-array resize patterns
- `writ-golden/tests/golden/coll_map_basic.writil` - Blessed snapshot
- `writ-golden/tests/golden/coll_set_basic.writ` - Updated Set.add and Set.remove to use resize patterns
- `writ-golden/tests/golden/coll_set_basic.writil` - Blessed snapshot
- `writ-golden/tests/golden/coll_hashmap_basic.writ` - Updated HashMap.set and HashMap.remove to use parallel-array resize patterns
- `writ-golden/tests/golden/coll_hashmap_basic.writil` - Blessed snapshot
- `writ-golden/tests/golden/coll_list_map.writ` - Updated List.add to use resize pattern
- `writ-golden/tests/golden/coll_list_map.writil` - Blessed snapshot
- `writ-golden/tests/golden/coll_list_filter.writ` - Updated List.add to use resize pattern
- `writ-golden/tests/golden/coll_list_filter.writil` - Blessed snapshot
- `writ-golden/tests/golden/coll_list_reduce.writ` - Updated List.add to use resize pattern
- `writ-golden/tests/golden/coll_list_reduce.writil` - Blessed snapshot
- `writ-golden/tests/golden/iter_for_in_list.writ` - Updated List.add to use resize pattern
- `writ-golden/tests/golden/iter_for_in_list.writil` - Blessed snapshot

## Decisions Made

- Golden test driver files are self-contained and do not import from writ-std; they required the same API change as the stdlib itself
- The `.writ` driver files serve as spec for what the public collection API should look like from a user perspective — they were updated to mirror the fix applied to collections.writ in Plan 01
- `result.add()` calls in `coll_list_map` and `coll_list_filter` correctly call the public `List<T>.add()` method (not the array primitive), which itself uses resize — this is correct and unchanged

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Golden test .writ driver files contained removed array API calls**
- **Found during:** Task 2 (BLESS=1 run)
- **Issue:** The plan assumed driver files import from stdlib; they are actually standalone files defining their own `List<T>`, `Set<T>`, `Map<K,V>`, `HashMap<K,V>` classes — those local class bodies still called `.add()`, `.remove_at()`, `.contains()` on arrays
- **Fix:** Updated all 8 `.writ` driver files with the same resize+indexed-store patterns used in collections.writ (Plan 01)
- **Files modified:** All 8 `.writ` golden driver files
- **Commit:** `9ff7736`

## Known Stubs

None — all 8 tests produce real IL output with correct ARRAY_RESIZE opcodes.

## Self-Check: PASSED

- `f1a9b80` exists: confirmed (chore - un-ignore tests)
- `9ff7736` exists: confirmed (feat - blessed snapshots)
- `writ-golden/tests/golden/coll_list_basic.writil` contains ARRAY_RESIZE: confirmed (2 occurrences)
- All 8 collection/iterator golden tests pass: confirmed (8 passed, 0 failed)
- Full suite: 72 passed, 0 failed, 0 ignored: confirmed
- writ-cli BUILD CLEAN: confirmed

---
*Phase: 121-stdlib-rewrite*
*Completed: 2026-03-29*
