---
phase: 24-il-codegen-metadata-skeleton
plan: "01"
subsystem: codegen
tags: [emit, metadata, IL, ModuleBuilder, TypeDef, FieldDef, MethodDef, ParamDef, GenericParam]

# Dependency graph
requires:
  - phase: 23-il-typecheck
    provides: "TypedAst with DefMap, TypeEnv with struct/entity/enum/contract field and method signatures"
provides:
  - "emit module foundation: MetadataToken, TableId, 21 row structs, StringHeap, BlobHeap, TypeSig encoding"
  - "ModuleBuilder with add_* staging methods and finalize() for contiguous 1-based row assignment"
  - "collect_defs pass emitting TypeDef/FieldDef/MethodDef/ParamDef/GenericParam/GenericConstraint/ModuleDef/ModuleRef/ExportDef"
  - "26 emit_tests covering token round-trips, heap dedup, collection from TypedAst"
affects:
  - "25-il-codegen-method-bodies: BodyEmitter uses ModuleBuilder and MetadataToken from this phase"
  - "26-cli-integration-e2e-validation: full pipeline uses emit module"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "MetadataToken encoding: (table_id as u32) << 24 | row_1based — matches IL spec 2.16.4"
    - "StringHeap/BlobHeap: FxHashMap deduplication; offset 0 reserved for empty entry"
    - "finalize() list-ownership: children grouped contiguously under parents; parent xxx_list points to first child row"
    - "typecheck() returns (TypedAst, TyInterner, Vec<Diagnostic>) — interner threaded to emit for type_sig encoding"

key-files:
  created:
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/emit/metadata.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/heaps.rs
    - writ-compiler/src/emit/type_sig.rs
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/src/emit/error.rs
    - writ-compiler/tests/emit_tests.rs
  modified:
    - writ-compiler/src/lib.rs

key-decisions:
  - "typecheck() return signature extended to (TypedAst, TyInterner, Vec<Diagnostic>) — interner needed for type_sig encoding in emit phase"
  - "Type sigs encoded during finalize() after row indices known — avoids two-pass token fixup"
  - "alloc_void_reg() uses Ty(4) directly: Void is always 5th pre-interned type per TyInterner::new() fixed ordering"
  - "body_offset, body_size, reg_count on MethodDef rows set to 0 in Phase 24; Phase 25 fills these"

patterns-established:
  - "emit module: same code organization pattern as resolve/ and check/ modules"
  - "collect pass: rebuilds type information from DefMap + original ASTs, parallel to TypeEnv::build()"

requirements-completed: [EMIT-01, EMIT-02, EMIT-04]

# Metrics
duration: ~45min
completed: 2026-03-03
---

# Phase 24 Plan 01: IL Codegen Metadata Skeleton — Core Type Tables Summary

**MetadataToken encoding infrastructure and ModuleBuilder foundation with TypeDef/FieldDef/MethodDef/ParamDef/GenericParam/ModuleDef/ModuleRef/ExportDef emission from TypedAst, verified by 26 passing emit_tests**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-03T00:00:00Z
- **Completed:** 2026-03-03T01:00:00Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Created the full emit module foundation: MetadataToken (spec 2.16.4 encoding), TableId (21 values), all 21 row structs, StringHeap and BlobHeap with deduplication, TypeSig encoding (spec 2.15.3)
- Implemented ModuleBuilder with add_* staging methods and finalize() that assigns contiguous 1-based row indices with list-ownership grouping (children contiguous under parents)
- Implemented collect_defs pass that walks TypedAst/DefMap/ASTs to populate TypeDef, FieldDef, MethodDef, ParamDef, GenericParam, GenericConstraint, ModuleDef, ModuleRef, and ExportDef rows
- All 26 emit_tests pass including token round-trips, heap dedup, entity hook MethodDefs, generic param rows, and ModuleDef/ModuleRef always-present invariants

## Task Commits

1. **Task 1: Emit module foundation with MetadataToken, TableId, row structs, heaps, and ModuleBuilder** - (feat)
2. **Task 2: Collect pass for TypeDef/FieldDef/MethodDef/ParamDef/GenericParam/ModuleDef/ModuleRef/ExportDef** - (feat)

## Files Created/Modified

- `writ-compiler/src/emit/mod.rs` - emit() entry point; coordinates collect_defs + slots + finalize
- `writ-compiler/src/emit/metadata.rs` - MetadataToken, TableId (21 values), 21 row structs, TypeDefKind, HookKind, flag helpers
- `writ-compiler/src/emit/module_builder.rs` - ModuleBuilder staging, finalize() with list-ownership, def_token_map
- `writ-compiler/src/emit/heaps.rs` - StringHeap and BlobHeap with FxHashMap dedup; offset 0 reserved for empty
- `writ-compiler/src/emit/type_sig.rs` - encode_type(): primitives (0x00-0x04), Struct/Entity/Enum (0x10+token), GenericParam (0x12+ordinal), Array (0x20+inner), Func (0x30+blob_offset)
- `writ-compiler/src/emit/collect.rs` - collect_defs() walking all TypedDecl variants for core type tables
- `writ-compiler/src/emit/error.rs` - EmitError enum with Into<Diagnostic>
- `writ-compiler/src/lib.rs` - added pub mod emit
- `writ-compiler/tests/emit_tests.rs` - 26 integration tests for metadata emission

## Evidence (Test Names)

- `module_def_always_present` — ModuleDef row always emitted
- `writ_runtime_moduleref_always_present` — ModuleRef for writ-runtime always emitted
- `pub_items_emit_exportdef` — ExportDef rows for pub declarations
- `struct_emits_typedef` — struct declaration produces TypeDef row
- `struct_fields_emit_fielddefs` — struct fields produce FieldDef rows
- `fn_emits_methoddef` — top-level fn produces MethodDef row
- `fn_params_emit_paramdefs` — function parameters produce ParamDef rows
- `generic_struct_emits_generic_params` — generic struct gets GenericParam row
- `generic_fn_emits_generic_params` — generic fn gets GenericParam row

## Decisions Made

- typecheck() extended to return TyInterner alongside TypedAst — needed for type_sig blob encoding during emit
- Type sigs encoded in finalize() (not collect pass) so token indices are known at encoding time
- body_offset/body_size/reg_count on MethodDef are 0 placeholders; Phase 25 fills real values

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

- Phase 24 Plan 02 can build on ModuleBuilder stubs for contract/impl/global/extern/component/attribute emission
- Plan 24-01 leaves stub add_* methods for all 21 tables; Plan 24-02 fleshes them out

---
*Phase: 24-il-codegen-metadata-skeleton*
*Completed: 2026-03-03*
