---
phase: 103-writ-runtime-reflectionindex-and-intrinsic-dispatch
plan: 02
subsystem: runtime
tags: [reflection, dispatch, intrinsics, gc, type-fields, attribute-query]

# Dependency graph
requires:
  - phase: 103-01
    provides: ReflectionIndex with get_or_alloc_type/primitive_type, TypeOf stub, IntGetType/FloatGetType/BoolGetType/StringGetType

provides:
  - 22 new IntrinsicId variants for all reflection type methods
  - 22 synthetic reflection method contracts in virtual module
  - ImplDef entries linking Type/FieldInfo/MethodInfo/ParameterInfo/AttributeInfo/ContractInfo to their method contracts
  - get_or_alloc_field_info, get_or_alloc_method_info, get_or_alloc_attribute_info, get_or_alloc_contract_info
  - type_reverse and field_reverse maps for identity recovery from HeapRef
  - TypeFields/TypeMethods/TypeAttributes/TypeContracts/TypeImplements real dispatch arms
  - FieldInfoGet arm reads field from instance using field_offset from reverse map
  - All accessor arms (TypeGetName/Namespace/Kind/IsGeneric, FieldInfoGet*/MethodInfoGet*/etc.)
  - Integration tests: 6 tests covering RT-01 through RT-05

affects:
  - 104 (typeof() lowering in compiler)
  - 105 (Reflectable auto-impl)
  - 106 (full type resolution for FieldInfo.declared_type / MethodInfo.return_type)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Synthetic single-method contracts per reflection type method for CALL_VIRT dispatch"
    - "Reverse map pattern: HeapRef -> identity tuple for intrinsic arms that need to recover allocation context"
    - "Unified TypeAttributes path: replicate Domain::query_attributes_on logic inline in intrinsic arm"
    - "Value::Struct heapref extraction in FieldInfoGet: handle both Value::Ref and Value::Struct instances"

key-files:
  created:
    - writ-runtime/tests/reflection_tests.rs
  modified:
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/intrinsics.rs
    - writ-runtime/src/virtual_module.rs
    - writ-runtime/src/domain_dispatch.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/src/reflection.rs
    - writ-runtime/tests/vm_tests.rs

key-decisions:
  - "Synthetic single-method contracts per reflection method avoids adding a new dispatch mechanism — reuses the existing CALL_VIRT / ImplDef / IntrinsicId pipeline"
  - "type_reverse and field_reverse maps trade a small memory overhead for O(1) identity lookup in intrinsic arms (critical for TypeFields/FieldInfoGet)"
  - "TypeAttributes replicates Domain::query_attributes_on inline (not via ctx.domain) because ExecContext does not hold a Domain reference — RT-05 unified path maintained"
  - "FieldInfoGet handles both Value::Ref and Value::Struct to support structs (which use Value::Struct not Value::Ref) as instances"

requirements-completed: [RT-04, RT-05]

# Metrics
duration: 18min
completed: 2026-03-28
---

# Phase 103 Plan 02: Reflection Intrinsic Dispatch Summary

**All 22 reflection IntrinsicId arms implemented — Type.fields(), Type.attributes(), FieldInfo.get(), and all accessor methods dispatch correctly via CALL_VIRT; AttributeInfo population uses unified Domain::query_attributes_on path (RT-05)**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-03-28T11:10:07Z
- **Completed:** 2026-03-28T11:28:05Z
- **Tasks:** 3
- **Files modified:** 7 modified, 1 created

## Accomplishments

- Added 22 new `IntrinsicId` variants for all reflection type methods
- Added 22 synthetic single-method contracts to the virtual module (Section 8) and corresponding ImplDef entries (Section 10) to wire them into the CALL_VIRT dispatch pipeline
- Added 22 (type_name, method_name) -> IntrinsicId mappings in `domain_dispatch.rs`
- Extended `ReflectionIndex` with `type_reverse` and `field_reverse` maps for O(1) identity recovery, plus 4 new allocation helpers
- Replaced all 22 stub arms with real implementations — TypeFields iterates field range, TypeAttributes replicates Domain::query_attributes_on inline (RT-05), FieldInfoGet reads from struct instances
- Created 6 integration tests covering all 5 RT requirements
- Full test suite: 317 tests pass across all writ-runtime test binaries (0 failures)

## Task Commits

1. **Task 1: Add IntrinsicId variants and register in virtual module** - `5cf1047` (feat)
2. **Task 2: Implement reflection intrinsic dispatch arms and ReflectionIndex helpers** - `53ceb16` (feat)
3. **Task 3: Integration tests for reflection intrinsics and GC survival** - `dacc3e3` (feat)

## Files Created/Modified

- `writ-runtime/src/dispatch/mod.rs` - Added 22 new IntrinsicId variants
- `writ-runtime/src/virtual_module.rs` - Added Section 8 (22 reflection contracts) + Section 10 (22 ImplDef entries); updated test assertions (24→46 contracts, 24→46 methods)
- `writ-runtime/src/domain_dispatch.rs` - Added 22 new (type_name, method_name) -> IntrinsicId mappings in resolve_intrinsic_id
- `writ-runtime/src/dispatch/intrinsics.rs` - Replaced 22 stubs with full implementations; fixed FieldInfoGet to handle Value::Struct instances
- `writ-runtime/src/reflection.rs` - Added type_reverse/field_reverse maps; added get_or_alloc_field_info, get_or_alloc_method_info, get_or_alloc_attribute_info, get_or_alloc_contract_info; added typedef_field_range_pub/typedef_method_range_pub
- `writ-runtime/src/domain.rs` - Updated dispatch count assertions (40→62)
- `writ-runtime/tests/vm_tests.rs` - Updated dispatch count assertion (41→63)
- `writ-runtime/tests/reflection_tests.rs` - New: 6 integration tests

## Decisions Made

- Synthetic single-method contracts per reflection method avoids adding a new dispatch mechanism — reuses the existing CALL_VIRT / ImplDef / IntrinsicId pipeline
- `type_reverse` and `field_reverse` maps trade a small memory overhead for O(1) identity lookup in intrinsic arms (critical for TypeFields/FieldInfoGet which need to recover the typedef identity from a HeapRef)
- `TypeAttributes` replicates `Domain::query_attributes_on` logic inline because `ExecContext` does not hold a `Domain` reference — unified RT-05 path maintained without changing the execution context signature
- `FieldInfoGet` handles both `Value::Ref` and `Value::Struct` instances (structs use `Value::Struct { type_idx, href }` not `Value::Ref`)

## Deviations from Plan

None - plan executed as written. The synthetic contract approach for inherent methods was consistent with the plan's description of "following the existing virtual module pattern for how methods are registered." The FieldInfoGet `Value::Struct` fix was an auto-fixed bug (Rule 1) discovered during testing.

## Known Stubs

- `FieldInfo.declared_type` (field 1 of FieldInfo heap object) is `Value::Void` placeholder — full type resolution deferred to Phase 106
- `MethodInfo.return_type` (field 1) is `Value::Void` placeholder — Phase 106
- `MethodInfo.parameters` (field 2) is empty Array — Phase 106
- `ContractInfo.type` (field 1) is `Value::Void` placeholder — Phase 106

These stubs do not prevent the plan's goal (RT-04, RT-05) from being achieved — the stub fields are documented as intentional Phase 106 work in the plan itself (Open Question 3 in research).

---
*Phase: 103-writ-runtime-reflectionindex-and-intrinsic-dispatch*
*Completed: 2026-03-28*
