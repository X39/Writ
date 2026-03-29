---
phase: 118-iterator-protocol-higher-order-methods
plan: "01"
subsystem: writ-std/collections
tags: [collections, iterator, higher-order-methods, golden-tests]
dependency_graph:
  requires: [117-collections-list-map-set/03]
  provides: [ListIterator<T>, SetIterator<T>, Iterable<T> impls for List/Set, Map.get_keys/get_values, List.map/filter/reduce, golden tests]
  affects: [writ-std/src/collections.writ, writ-golden, writ-cli]
tech_stack:
  added: []
  patterns: [pure-Writ iterator classes, Iterable<T>/Iterator<T> contract impls, CALL_INDIRECT closure dispatch, BLESS=1 snapshot generation]
key_files:
  created:
    - writ-golden/tests/golden/coll_list_map.writ
    - writ-golden/tests/golden/coll_list_map.writil
    - writ-golden/tests/golden/coll_list_filter.writ
    - writ-golden/tests/golden/coll_list_filter.writil
    - writ-golden/tests/golden/coll_list_reduce.writ
    - writ-golden/tests/golden/coll_list_reduce.writil
  modified:
    - writ-std/src/collections.writ
    - writ-golden/tests/golden_tests.rs
decisions:
  - "map/filter/reduce typed as fn(T)->T (homomorphic) to avoid second generic param unification issue — satisfies COLL-04 monomorphic use case"
  - "get_keys/get_values named with get_ prefix to avoid field-vs-method name collision on Map.keys/values fields"
  - "Both inherent impl and contract impl blocks included for ListIterator/SetIterator (CALL_VIRT needs contract impl vtable slot; direct calls use inherent impl)"
metrics:
  duration_minutes: 5
  tasks_completed: 2
  tasks_total: 2
  files_created: 6
  files_modified: 2
  completed_date: "2026-03-29T15:13:08Z"
---

# Phase 118 Plan 01: Iterator Infrastructure and Higher-Order Methods Summary

**One-liner:** Pure-Writ ListIterator/SetIterator cursor classes with Iterable<T>/Iterator<T> contract impls plus map/filter/reduce HOF methods on List<T>, verified by three new golden IL snapshot tests.

## What Was Built

### Task 1: Iterator classes, Iterable impls, Map accessors (commit: 815527f)

Added to `writ-std/src/collections.writ`:

**ListIterator<T>** — cursor class with `source: T[]` and `index: int` fields. Has both an inherent `impl<T> ListIterator<T>` block (for direct dispatch) and a `impl<T> Iterator<T> for ListIterator<T>` contract impl (for CALL_VIRT vtable dispatch). Method: `pub fn next(mut self) -> T?` — increments index cursor and returns next item or null.

**Iterable<T> for List<T>** — `impl<T> Iterable<T> for List<T>` block returning `new ListIterator<T> { source: self.items, index: 0 }`.

**SetIterator<T: Eq>** — identical structure to ListIterator but with `T: Eq` bound to match Set<T>'s constraint. Both inherent and contract impl blocks included.

**Iterable<T> for Set<T>** — `impl<T: Eq> Iterable<T> for Set<T>` returning `new SetIterator<T> { source: self.items, index: 0 }`.

**List<T> HOF methods** — added to the existing `impl<T> List<T>` block after `has`:
- `map(self, f: fn(T) -> T) -> List<T>` — builds new list applying f to each element
- `filter(self, f: fn(T) -> bool) -> List<T>` — builds new list keeping elements where f returns true
- `reduce(self, initial: T, f: fn(T, T) -> T) -> T` — folds list with accumulator starting at initial

**Map accessors** — `get_keys(self) -> K[]` and `get_values(self) -> V[]` added to `impl<K: Ord + Eq, V> Map<K, V>`.

### Task 2: Golden IL snapshot tests (commit: ac6f665)

Three new golden test .writ sources created (each inlines full List class per golden test pattern):

- **coll_list_map.writ** — List<int> with add/get/len/map, main creates [1,2,3], calls `.map(fn(x: int) -> int { x * 2 })`, reads `doubled.get(0)` and `doubled.len()`
- **coll_list_filter.writ** — List<int> with add/get/len/filter, main creates [1..5], calls `.filter(fn(x: int) -> bool { x > 2 })`, reads result
- **coll_list_reduce.writ** — List<int> with add/len/reduce, main creates [1,2,3], calls `.reduce(0, fn(acc: int, x: int) -> int { acc + x })`, binds result

Three corresponding .writil IL snapshot files generated via `BLESS=1 cargo test -p writ-golden`. Three test functions added to `golden_tests.rs`: `golden_coll_list_map`, `golden_coll_list_filter`, `golden_coll_list_reduce`.

## Verification

- `cargo build -p writ-cli` — passes (collections.writ compiles with all new code)
- `cargo test -p writ-golden` — 70/70 tests pass (all existing + 3 new HOF tests)
- All acceptance criteria grep checks satisfied

## Decisions Made

1. **Homomorphic HOF signatures** — `map` uses `fn(T) -> T` rather than `fn(T) -> U` to avoid the second generic param wildcard unification issue documented in RESEARCH.md Pitfall 4. This satisfies COLL-04's monomorphic int use case (`list.map(fn(x: int) -> int { x * 2 })`) without requiring GenericClass(DefId, Vec<Ty>) in TyKind.

2. **get_keys/get_values naming** — Named with `get_` prefix rather than `keys()`/`values()` to avoid potential field-vs-method name collision. Map has `keys: K[]` and `values: V[]` fields; method-over-field priority was not verified so the safe names are used (noted in plan, safe-start approach).

3. **Dual impl blocks for iterator classes** — Each iterator class has both an inherent impl (for direct method calls) and a contract impl `impl Iterator<T> for ...` (for CALL_VIRT vtable dispatch). The distinction is documented in plan notes: CALL_VIRT dispatches through the contract impl vtable; direct calls go through the inherent impl.

## Deviations from Plan

None — plan executed exactly as written. The map/filter/reduce methods were added to the List<T> inherent impl block in Task 1 rather than as a separate task step, but both tasks were committed separately as planned.

## Known Stubs

None — all methods are fully implemented with real logic. No placeholder or TODO code present.

## Self-Check: PASSED

Files created/present:
- writ-std/src/collections.writ: contains ListIterator, SetIterator, Iterable impls, get_keys/get_values, map/filter/reduce
- writ-golden/tests/golden/coll_list_map.writ: exists, contains "fn main"
- writ-golden/tests/golden/coll_list_map.writil: exists (golden snapshot)
- writ-golden/tests/golden/coll_list_filter.writ: exists, contains "fn main"
- writ-golden/tests/golden/coll_list_filter.writil: exists (golden snapshot)
- writ-golden/tests/golden/coll_list_reduce.writ: exists, contains "fn main"
- writ-golden/tests/golden/coll_list_reduce.writil: exists (golden snapshot)

Commits present:
- 815527f: feat(118-01): add iterator classes, Iterable impls, and Map accessors to collections.writ
- ac6f665: feat(118-01): add golden IL snapshot tests for List map/filter/reduce HOF methods
