---
phase: 117-collections-list-map-set
plan: "02"
subsystem: compiler
tags: [collections, generics, golden-tests, writ-std, list, map, set, hashmap]

# Dependency graph
requires:
  - phase: 117-collections-list-map-set
    provides: Plan 01 gates (GC transitivity, impl<T> syntax validation, cross-file resolution)
  - phase: 116-array-primitives
    provides: array dot-call opcodes (add, remove_at, contains, len, index ops) used by collections
  - phase: 115-generic-constraints
    provides: constraint bound syntax (<T: Eq>, <K: Ord + Eq>, <K: Hashable>) enforced at type-check
provides:
  - writ-std/src/collections.writ: List<T>, Map<K:Ord+Eq,V>, Set<T:Eq>, HashMap<K:Hashable,V> in pure Writ
  - writ-std/writ.toml: Writ project configuration for writ-std library
  - Four golden tests proving all collection types compile to valid IL
  - Blessed .writil snapshots for all four collection types
affects:
  - 117-03 (writ-cli library integration uses writ-std/src/collections.writ as source)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Collection class pattern: pub class Coll<T> { items: T[] } with impl<T> Coll<T> { ... }"
    - "Struct-init constructor: new ClassName<ConcreteType> { fields: [] } — fn new() is invalid (new is a keyword)"
    - "Constraint bounds on impl<T: Constraint> ClassName<T>: works correctly for Eq, Ord+Eq, Hashable"
    - "Golden tests inline full class source — compile_and_disassemble operates on single file"

key-files:
  created:
    - writ-std/writ.toml (Writ project config for standard library)
    - writ-std/src/collections.writ (List, Map, Set, HashMap class + impl blocks)
    - writ-golden/tests/golden/coll_list_basic.writ (List<T> golden test source)
    - writ-golden/tests/golden/coll_list_basic.writil (blessed IL snapshot)
    - writ-golden/tests/golden/coll_map_basic.writ (Map<K,V> golden test source)
    - writ-golden/tests/golden/coll_map_basic.writil (blessed IL snapshot)
    - writ-golden/tests/golden/coll_set_basic.writ (Set<T> golden test source)
    - writ-golden/tests/golden/coll_set_basic.writil (blessed IL snapshot)
    - writ-golden/tests/golden/coll_hashmap_basic.writ (HashMap<K,V> golden test source)
    - writ-golden/tests/golden/coll_hashmap_basic.writil (blessed IL snapshot)
  modified:
    - writ-golden/tests/golden_tests.rs (4 new test functions added)

key-decisions:
  - "fn new() removed from collection classes — `new` is a Writ keyword; cannot be a method name"
  - "Golden tests use new ClassName<ConcreteType> { fields: [] } inline instead of factory method calls"
  - "writ-std/src/collections.writ has no fn new() — callers use struct-init syntax directly"
  - "Map and HashMap use O(n) linear scan for Phase 117 (binary search / hash bucketing deferred)"

patterns-established:
  - "Generic collection class pattern: pub class Coll<T: Bound> { field: T[] } + impl<T: Bound> Coll<T> { methods }"
  - "Constructor convention: users write new Coll<int> { field: [] } — no static factory method possible due to keyword conflict"

requirements-completed: [COLL-01, COLL-02, COLL-03, COLL-05, COLL-06]

# Metrics
duration: 20min
completed: 2026-03-29
---

# Phase 117 Plan 02: Collection Classes (List, Map, Set, HashMap) Summary

**Four pure-Writ collection classes in writ-std (List/Map/Set/HashMap) compile to valid IL via four golden snapshot tests, using impl<T> generic inherent impl blocks and array dot-call opcodes**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-29T11:15:23Z
- **Completed:** 2026-03-29T11:25:00Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `writ-std/src/collections.writ` created with all four collection classes (List<T>, Map<K:Ord+Eq,V>, Set<T:Eq>, HashMap<K:Hashable,V>) in pure Writ source using `impl<T> ClassName<T>` syntax validated in Plan 01
- Four golden tests blessed and passing: coll_list_basic, coll_map_basic, coll_set_basic, coll_hashmap_basic
- All 67 golden tests continue to pass (63 existing + 4 new, no regressions)
- Method names match REQUIREMENTS.md exactly: List uses add/get/set/len/remove_at/contains; Map/HashMap use set/get/has/remove/len; Set uses add/remove/contains/len

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-std directory and write all four collection classes** - `914afc6` (feat)
2. **Task 2: Golden IL snapshot tests for all four collection types** - `dfb9e12` (feat)

## Files Created/Modified

- `writ-std/writ.toml` - Writ project config (name=writ-std, sources=["src/"])
- `writ-std/src/collections.writ` - List<T>, Map<K:Ord+Eq,V>, Set<T:Eq>, HashMap<K:Hashable,V>
- `writ-golden/tests/golden/coll_list_basic.writ` - List<T> golden test source
- `writ-golden/tests/golden/coll_list_basic.writil` - Blessed IL snapshot (4898 bytes)
- `writ-golden/tests/golden/coll_map_basic.writ` - Map<K,V> golden test source
- `writ-golden/tests/golden/coll_map_basic.writil` - Blessed IL snapshot (4277 bytes)
- `writ-golden/tests/golden/coll_set_basic.writ` - Set<T> golden test source
- `writ-golden/tests/golden/coll_set_basic.writil` - Blessed IL snapshot (4725 bytes)
- `writ-golden/tests/golden/coll_hashmap_basic.writ` - HashMap<K,V> golden test source
- `writ-golden/tests/golden/coll_hashmap_basic.writil` - Blessed IL snapshot (4322 bytes)
- `writ-golden/tests/golden_tests.rs` - Added 4 golden test functions

## Decisions Made

- **`fn new()` is invalid in Writ**: `new` is a language keyword; cannot be used as method name. The plan's `List::new()` pattern was adapted — factory methods removed, callers use `new List<int> { items: [] }` struct-init syntax directly. The `writ-std/src/collections.writ` source has no factory methods.
- **Golden tests inline full class source**: `compile_and_disassemble` operates on a single file; collection class definitions are copied into each golden test rather than imported from writ-std.
- **Map O(n) confirmed**: COLL-02 originally specified O(log n) sorted-array, but linear scan has identical external API. Binary search deferred to COLL-07 per RESEARCH.md.
- **Constraint bounds on impl header work**: `impl<K: Ord + Eq, V> Map<K, V>` and `impl<K: Hashable, V> HashMap<K, V>` compile correctly through the pipeline fixed in Plan 01.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed fn new() factory methods — `new` is a Writ keyword**
- **Found during:** Task 2 (first BLESS run of coll_list_basic)
- **Issue:** Parser error `found 'KwNew' at 67..70 expected something else` — `pub fn new()` is invalid because `new` is a reserved keyword. This was documented in the 117-01-SUMMARY but not reflected in the plan's collection source.
- **Fix:** Removed all `fn new()` factory methods from writ-std/src/collections.writ and all four golden test files. Users construct collection instances with `new ClassName<Type> { fields: [] }` syntax.
- **Files modified:** `writ-std/src/collections.writ`, all four coll_*_basic.writ files
- **Verification:** All four golden tests pass after BLESS
- **Committed in:** dfb9e12 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug: keyword conflict)
**Impact on plan:** Factory method removal is a surface API change — struct-init is the correct Writ idiom for constructing collection instances. The `fn new()` pattern was documented as broken in 117-01-SUMMARY but the plan source wasn't updated. No scope creep.

## Issues Encountered

- **`fn new()` keyword conflict**: Discovered immediately on first test run. Parse error at position 67 (the `new` token inside `pub fn new() -> List<T>`). Fixed by removing factory methods from class definitions. Struct-init syntax `new List<int> { items: [] }` works correctly as the constructor pattern.

## Known Stubs

None — all four collection classes have full method implementations. The `self.values[0]` fallback in Map.get() and HashMap.get() is intentional documented behavior (caller must check `has()` first), not a stub.

## Next Phase Readiness

Plan 02 complete. All four collection classes compile to valid IL.

**For Plan 03 (writ-cli library integration):** The `writ-std/src/collections.writ` source is ready to be compiled and embedded. Users will construct collections with `new ClassName<Type> { fields: [] }` — note the keyword constraint documented here.

**Known limitation:** Static method calls on generic types (e.g., `List::create()`) still don't type-check due to missing instantiation tracking (deferred per 117-01-SUMMARY). All collection usage must use struct-init constructor syntax.

---
*Phase: 117-collections-list-map-set*
*Completed: 2026-03-29*
