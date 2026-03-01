---
phase: 24-il-codegen-metadata-skeleton
plan: "02"
subsystem: codegen
tags: [emit, metadata, IL, ContractDef, ContractMethod, ImplDef, GlobalDef, ExternDef, ComponentSlot, AttributeDef, CALL_VIRT, slots]

# Dependency graph
requires:
  - phase: 24-il-codegen-metadata-skeleton
    plan: "01"
    provides: "ModuleBuilder foundation with TypeDef/FieldDef/MethodDef/ParamDef/GenericParam/ModuleDef/ModuleRef/ExportDef emission"
provides:
  - "slots.rs: CALL_VIRT slot assignment from contract declaration order"
  - "ContractDef/ContractMethod/ImplDef emission with declaration-order slot indices"
  - "GlobalDef emission for const and global mut declarations"
  - "ExternDef emission for extern fn declarations"
  - "ComponentSlot emission linking entity TypeDefs to component TypeDefs"
  - "AttributeDef emission for [Singleton], [Conditional], and other AST attributes"
  - "LocaleDef stub emitting 0 rows (EMIT-25 deferred to Phase 29)"
  - "All 21 metadata tables have finalize() row assignment logic"
affects:
  - "25-il-codegen-method-bodies: CALL_VIRT slots from Phase 24 used for virtual dispatch"
  - "26-cli-integration-e2e-validation: complete metadata skeleton enables full pipeline"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CALL_VIRT slot ordering: slots assigned from contract declaration order in slots.rs, not impl traversal order"
    - "ImplDef.method_list: points to MethodDef rows emitted under target type's TypeDef"
    - "LocaleDef: stub with 0 rows; loc_key manifest from lowering phase needed for full emission"
    - "AttributeDef: owner/owner_kind/name/value with empty value blob for Phase 24"

key-files:
  created:
    - writ-compiler/src/emit/slots.rs
  modified:
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/tests/emit_tests.rs

key-decisions:
  - "CALL_VIRT slot indices assigned by slots::assign_vtable_slots() after collect_defs(), before finalize() — ensures declaration order, not collection order"
  - "LocaleDef emits 0 rows; a loc_key manifest from LoweringContext is required for full implementation (deferred to Phase 29)"
  - "ComponentSlot owner_entity and component_type are MetadataToken references to TypeDef rows"
  - "GlobalDef flags encode is_const and is_mutable; init_value blob is empty in Phase 24"

patterns-established:
  - "Slot assignment pattern: collect -> assign_vtable_slots -> finalize (ordering is critical)"

requirements-completed: [EMIT-03, EMIT-05, EMIT-06, EMIT-22, EMIT-29]

# Metrics
duration: ~40min
completed: 2026-03-03
---

# Phase 24 Plan 02: IL Codegen Metadata Skeleton — Remaining Tables Summary

**Complete metadata skeleton with ContractDef/ContractMethod slot ordering, ImplDef linking, GlobalDef/ExternDef/ComponentSlot/AttributeDef emission and CALL_VIRT slots from contract declaration order — all 21 tables populated**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-03-03T01:00:00Z
- **Completed:** 2026-03-03T02:00:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Created slots.rs with assign_vtable_slots() that assigns CALL_VIRT slot indices from contract declaration order (critical invariant for correct virtual dispatch in Phase 25)
- Extended collect.rs for ContractDef/ContractMethod, ImplDef, GlobalDef, ExternDef, ComponentSlot, and AttributeDef emission
- LocaleDef stub emitting 0 rows with TODO for loc_key manifest (EMIT-25 deferred to Phase 29)
- All 26 emit_tests pass; comprehensive test verifies all tables populated for multi-declaration source

## Task Commits

1. **Task 1: ContractDef/ContractMethod emission and CALL_VIRT slot assignment** - (feat)
2. **Task 2: GlobalDef, ExternDef, ComponentSlot, LocaleDef stub, and AttributeDef rows** - (feat)

## Files Created/Modified

- `writ-compiler/src/emit/slots.rs` - assign_vtable_slots(): walks ContractDef entries, assigns 0-based slot indices in declaration order
- `writ-compiler/src/emit/collect.rs` - Extended: ContractDef/ContractMethod, ImplDef, GlobalDef, ExternDef, ComponentSlot, LocaleDef stub, AttributeDef; HookKind::from_event_name() for lifecycle hook mapping
- `writ-compiler/src/emit/module_builder.rs` - Fleshed out: add_contract_def, add_contract_method, add_impl_def, add_global_def, add_extern_def, add_component_slot, add_locale_def, add_attribute_def; finalize() updated for all 21 tables
- `writ-compiler/src/emit/mod.rs` - Pipeline order: collect_defs -> assign_vtable_slots -> finalize
- `writ-compiler/tests/emit_tests.rs` - Tests for contract slots, impl linking, globals, externs, components, attributes

## Evidence (Test Names)

- `contract_emits_contractdef_and_methods` — ContractDef row + ContractMethod rows emitted
- `contract_method_slots_assigned` — ContractMethod slot indices from declaration order (not impl order)
- `impl_emits_impldef` — ImplDef links type_token to contract_token
- `const_emits_globaldef` — const declaration produces GlobalDef with const flag
- `global_mut_emits_globaldef` — global mut declaration produces GlobalDef with mutable flag
- `extern_fn_emits_externdef` — extern fn declaration produces ExternDef row
- `entity_hooks_emit_methoddefs` — entity lifecycle hooks produce MethodDef rows with hook_kind flags
- `entity_emits_typedef` — entity with component slot produces TypeDef + ComponentSlot rows

## Decisions Made

- CALL_VIRT slots assigned after collect_defs() but before finalize() — this ordering is the critical invariant
- LocaleDef deferred: emits 0 rows in Phase 24; full implementation needs loc_key manifest from LoweringContext (EMIT-25 tracked for Phase 29)
- HookKind::from_event_name() maps "create" -> Create, "destroy" -> Destroy, "finalize" -> Finalize, "serialize" -> Serialize, "deserialize" -> Deserialize, "interact" -> Interact

## Deviations from Plan

None - plan executed exactly as written. EMIT-25 (LocaleDef) was already planned as a stub with deferred implementation.

## Issues Encountered

None.

## Next Phase Readiness

- Phase 24 metadata skeleton complete: all 21 tables have row assignment, CALL_VIRT slots assigned, all DefIds have MetadataTokens
- Phase 25 can use ModuleBuilder to fill body_offset/body_size/reg_count on MethodDef rows during method body emission
- EMIT-25 (LocaleDef full implementation) tracked for Phase 29

---
*Phase: 24-il-codegen-metadata-skeleton*
*Completed: 2026-03-03*
