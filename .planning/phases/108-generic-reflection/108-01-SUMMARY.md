---
phase: 108-generic-reflection
plan: 01
subsystem: runtime
tags: [reflection, generics, intrinsics, type-system, writ-runtime]

requires:
  - phase: 107-dynamic-invocation
    provides: FieldInfo.set and MethodInfo.invoke intrinsics, existing reflection infrastructure

provides:
  - "Type.is_generic populates from GenericParam table scan (not hardcoded false)"
  - "Type.type_args() intrinsic reads field 4 (Array<Type>) from Type heap object"
  - "get_or_alloc_typespec_type() allocates Type with type_args for Array<T> TypeSpec signatures"
  - "MethodInfo.attributes() intrinsic scans attribute_defs filtered by MethodDef table_id"
  - "FieldInfo.attributes() intrinsic scans attribute_defs filtered by FieldDef table_id"
  - "method_attr_cache and field_attr_cache in ReflectionIndex for per-member attribute caching"
  - "Virtual module has 51 contract_defs (up from 48)"
  - "TypeOf dispatch branches on table_id=4 (TypeSpec) for generic instantiation"

affects:
  - 108-02-PLAN
  - any plan consuming Type.type_args() or per-member reflection attributes

tech-stack:
  added: []
  patterns:
    - "TypeSpec-backed Type objects: cached with key (module_idx, usize::MAX-1-typespec_idx) to avoid collision with typedef keys"
    - "Phase-scoped caches in ReflectionIndex: method_attr_cache, field_attr_cache follow same (module_idx, idx, ordinal) key pattern as attr_cache"
    - "Shared allocate_attribute_info() helper: extracted from get_or_alloc_attribute_info to avoid triplication across type/method/field attribute allocation"

key-files:
  modified:
    - writ-runtime/src/reflection.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/intrinsics.rs
    - writ-runtime/src/virtual_module.rs
    - writ-runtime/src/domain_dispatch.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/tests/vm_tests.rs
    - writ-runtime/tests/reflection_tests.rs

key-decisions:
  - "TYPE_FIELD_COUNT bumped from 4 to 5 (field 4 = type_args Array<Type>)"
  - "TypeSpec cache key uses (module_idx, usize::MAX-1-typespec_idx) to avoid collision with typedef keys"
  - "TypeOf dispatch uses table_id bit-field (bits 31-24) to detect TypeSpec vs TypeDef tokens"
  - "get_or_alloc_attribute_info refactored to use shared allocate_attribute_info() private helper"

requirements-completed: [GEN-01, GEN-02, GEN-03]

duration: 25min
completed: 2026-03-28
---

# Phase 108 Plan 01: Generic Reflection Summary

**Type.is_generic from GenericParam scan, Type.type_args() Array field, MethodInfo/FieldInfo.attributes() intrinsics, and TypeOf TypeSpec token dispatch — 51 virtual module contracts, 90/90 runtime tests pass.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-03-28T00:00:00Z
- **Completed:** 2026-03-28T00:25:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- TYPE_FIELD_COUNT bumped from 4 to 5; Type heap objects now have `type_args` (Array<Type>) as field 4
- `is_generic` field populated by scanning the GenericParam table for the typedef (not hardcoded false)
- `get_or_alloc_typespec_type()` allocates Type objects for Array<T> and generic TypeSpec signatures, populating type_args from the blob signature
- TypeOf dispatch updated to route TypeSpec tokens (table_id=4) to the new typespec path
- MethodInfoAttributes and FieldInfoAttributes intrinsics scan attribute_defs by MethodDef/FieldDef table_id
- Virtual module contracts: 48 → 51 (Type.type_args, MethodInfo.attributes, FieldInfo.attributes added)
- 3 new IntrinsicId variants wired end-to-end: virtual module → domain_dispatch → intrinsics.rs
- All 90 writ-runtime tests pass

## Task Commits

1. **Task 1: reflection.rs** - `27066e9` (feat)
2. **Task 2: dispatch + virtual_module** - `196ae92` (feat)

## Files Created/Modified

- `writ-runtime/src/reflection.rs` - TYPE_FIELD_COUNT=5, GenericParam scan, type_args field, method/field attr caches, shared allocate_attribute_info helper, get_or_alloc_typespec_type
- `writ-runtime/src/dispatch/mod.rs` - 3 new IntrinsicId variants, TypeOf TypeSpec branch
- `writ-runtime/src/dispatch/intrinsics.rs` - 3 new dispatch arms, ReflectionIndex import
- `writ-runtime/src/virtual_module.rs` - Type typedef gains type_args field, 3 new contracts + impls
- `writ-runtime/src/domain_dispatch.rs` - 3 new resolve_intrinsic_id match arms
- `writ-runtime/src/domain.rs` - Updated dispatch table count 64 → 67
- `writ-runtime/tests/vm_tests.rs` - Updated dispatch count assertion 65 → 68
- `writ-runtime/tests/reflection_tests.rs` - Updated GC root count assertion to >= 5

## Decisions Made

- TypeSpec cache key uses `(module_idx, usize::MAX-1-typespec_idx)` — avoids collision with typedef keys which use small indices; `usize::MAX` is already reserved for primitive types so `usize::MAX-1-idx` gives a distinct range
- `get_or_alloc_attribute_info` refactored to share `allocate_attribute_info()` — three attribute allocation paths (type, method, field) use identical logic; DRY with a private helper
- TypeOf dispatch uses `(*type_idx >> 24) as u8` for table_id extraction — matches the decode_method_token pattern used throughout dispatch code

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed 3 failing tests after bumping TYPE_FIELD_COUNT**
- **Found during:** Task 2 (virtual module + dispatch)
- **Issue:** `type_typedef_is_class_with_four_fields` expected 4 fields; `dispatch_table_virtual_module_has_36_intrinsic_entries` and `dispatch_table_all_intrinsic_types_covered` expected 64 entries; `call_virt_user_defined_contract_dispatch_table_populated` expected 65 entries
- **Fix:** Updated assertions to 5 fields (Type.type_args added), 67 entries, 68 entries
- **Files modified:** virtual_module.rs, domain.rs, vm_tests.rs, reflection_tests.rs
- **Verification:** `cargo test -p writ-runtime` passes 90/90
- **Committed in:** 196ae92 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - existing test assertions stale after struct field addition)
**Impact on plan:** Required update — these tests enforce the exact struct shape and dispatch table size; updating them is a direct consequence of the planned changes.

## Issues Encountered

None — the only complications were stale test assertions for count invariants that correctly reflected the old state.

## Next Phase Readiness

- Plan 108-02 can proceed: Type.type_args() and MethodInfo/FieldInfo.attributes() dispatch is live
- TypeOf compiler lowering for TypeSpec tokens (typeof(Array<int>)) is ready to test end-to-end

---
*Phase: 108-generic-reflection*
*Completed: 2026-03-28*
