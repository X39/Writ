---
phase: 107-dynamic-invocation
plan: 01
subsystem: runtime
tags: [reflection, dynamic-dispatch, intrinsics, writ-runtime, field-write, method-invoke]

requires:
  - phase: 106-read-only-introspection-integration-tests-and-lsp
    provides: FieldInfo/MethodInfo heap objects with field_reverse/method_cache maps, IntrinsicId reflection variants, virtual module reflection contracts

provides:
  - FieldInfo.set() intrinsic with readonly enforcement (crashes on let-fields, writes on mut-fields)
  - MethodInfo.invoke() intrinsic that pushes a CallFrame and returns Continue for scheduler-driven execution
  - method_reverse map in ReflectionIndex for O(1) MethodInfo -> (module_idx, method_idx) lookup
  - lookup_method_identity() helper parallel to lookup_field_identity()
  - Two new virtual module contracts: FieldInfo.set and MethodInfo.invoke (total: 48 contracts)

affects: [107-dynamic-invocation-02, future dynamic-invocation callers]

tech-stack:
  added: []
  patterns:
    - "Intrinsic reverse-map pattern: allocate heap object -> insert (href, identity) into reverse map -> use lookup_*_identity() in dispatch arm"
    - "Field readonly check: read abs_idx = td.field_list.saturating_sub(1) + field_offset, check flags & 0x01"
    - "MethodInfo.invoke frame push: push CallFrame with method_idx, set r0=instance, r1..rN=args, return Continue (no inner loop)"

key-files:
  created: []
  modified:
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/intrinsics.rs
    - writ-runtime/src/reflection.rs
    - writ-runtime/src/virtual_module.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/tests/vm_tests.rs

key-decisions:
  - "MethodInfo.invoke pushes a CallFrame and returns Continue — the scheduler drives execution, ensuring cooperative scheduling and defer/crash semantics work naturally"
  - "FieldInfo.set uses absolute field index (td.field_list.saturating_sub(1) + field_offset) to access FieldDefRow.flags for readonly check"
  - "method_reverse map mirrors field_reverse: populated in get_or_alloc_method_info(), keyed by HeapRef, returns (module_idx, method_idx)"
  - "IntrinsicId variants placed after their Phase 103 siblings for organizational clarity"
  - "CallFrame does not store module_idx (existing design) — MethodInfoInvoke only supports same-module invocations in this phase"

patterns-established:
  - "Reverse map pattern: heap object allocation caches (href -> identity) for O(1) dispatch arm lookup"

requirements-completed: [DYN-01, DYN-02, DYN-04]

duration: 25min
completed: 2026-03-28
---

# Phase 107 Plan 01: Dynamic Invocation Summary

**FieldInfo.set() with readonly enforcement and MethodInfo.invoke() with scheduler-driven frame dispatch, completing the P2 dynamic mutation path for writ-runtime reflection**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-28T00:00:00Z
- **Completed:** 2026-03-28T00:25:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `FieldInfoSet` and `MethodInfoInvoke` IntrinsicId variants and all supporting infrastructure
- Implemented `FieldInfoSet` dispatch arm: resolves field identity from reverse map, reads `FieldDefRow.flags` for readonly bit, crashes with `"Reflection write to immutable field '{name}'"` on let-fields, writes via `heap.set_field()` on mut-fields
- Implemented `MethodInfoInvoke` dispatch arm: validates arg count against `param_count`, pushes a `CallFrame` with `r0=instance, r1..rN=args`, returns `Continue` — fully cooperative-scheduler-aware
- Added `method_reverse: FxHashMap<HeapRef, (usize, usize)>` to `ReflectionIndex` struct; populated in `get_or_alloc_method_info()`; exposed as `lookup_method_identity()`
- Added two new virtual module contracts (`FieldInfo.set`, `MethodInfo.invoke`) bringing total to 48; added `ImplDef` entries for FieldInfo and MethodInfo; updated dispatch table count assertions from 62→64

## Task Commits

1. **Task 1: Add method_reverse map, IntrinsicId variants, and virtual module contracts** - `5ac8731` (feat)
2. **Task 2: Implement FieldInfoSet and MethodInfoInvoke intrinsic dispatch arms** - `567cdbf` (feat)

## Files Created/Modified

- `writ-runtime/src/dispatch/mod.rs` — Added `FieldInfoSet` and `MethodInfoInvoke` to `IntrinsicId` enum
- `writ-runtime/src/reflection.rs` — Added `method_reverse` field, populated in `get_or_alloc_method_info()`, added `lookup_method_identity()` public helper
- `writ-runtime/src/virtual_module.rs` — Added `FieldInfo.set` and `MethodInfo.invoke` contracts + ImplDef entries; updated contract count test to 48
- `writ-runtime/src/dispatch/intrinsics.rs` — Implemented `FieldInfoSet` and `MethodInfoInvoke` dispatch arms
- `writ-runtime/src/domain.rs` — Updated dispatch table count assertion from 62 to 64
- `writ-runtime/tests/vm_tests.rs` — Updated dispatch table count assertion from 63 to 65

## Decisions Made

- `MethodInfo.invoke` pushes a `CallFrame` and returns `Continue` — no inner execution loop. The scheduler drives the callee frame, which ensures defer/crash/cancel semantics work correctly across dynamic invocations.
- `CallFrame` does not store `module_idx` (pre-existing design). `MethodInfoInvoke` supports same-module invocations in this phase; cross-module dynamic invocation would require extending `CallFrame` with a module index field (deferred).
- Absolute field index computed as `td.field_list.saturating_sub(1) + field_offset` matching the pattern used throughout the codebase (`domain.rs`, `reflection.rs`, `helpers.rs`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated stale dispatch table count assertions in domain.rs, virtual_module.rs, and vm_tests.rs**
- **Found during:** Task 1 and Task 2 (compiler exhaustive match, then test failures)
- **Issue:** Three tests asserted the old contract count (46) and dispatch table size (62/63); adding 2 new contracts/impls made them fail
- **Fix:** Updated `has_exactly_46_contract_defs` → `has_exactly_48_contract_defs` (46→48), `each_contract_has_one_method` (46→48), dispatch table count (62→64 in domain.rs and dispatch_table_all_intrinsic_types_covered, 63→65 in vm_tests.rs)
- **Files modified:** `writ-runtime/src/virtual_module.rs`, `writ-runtime/src/domain.rs`, `writ-runtime/tests/vm_tests.rs`
- **Verification:** All 156 writ-runtime tests pass
- **Committed in:** `567cdbf` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - stale assertions)
**Impact on plan:** Necessary correctness fix, no scope creep.

## Issues Encountered

None — plan executed cleanly. Compiler's exhaustive match on `IntrinsicId` immediately flagged the new variants needing dispatch arms, guiding the implementation order.

## Next Phase Readiness

- Phase 107-02 can now build on `FieldInfo.set()` and `MethodInfo.invoke()` as available intrinsics
- All 156 writ-runtime tests pass
- DYN-03 (Type.construct) remains explicitly deferred to v12+ per STATE.md

---
*Phase: 107-dynamic-invocation*
*Completed: 2026-03-28*
