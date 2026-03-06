---
phase: 48-il-format
plan: 01
subsystem: module-format
tags: [writ-module, binary-format, TypeDefKind, IL, validation]

# Dependency graph
requires:
  - phase: 47-spec-amendments
    provides: "TypeDef.kind=4 for class; format_version=3 spec normative prose"
provides:
  - "TypeDefKind::Class = 4 variant in writ-module crate"
  - "Display impl for TypeDefKind"
  - "DecodeError::InvalidTypeDefKind(u8) error variant"
  - "Reader rejects format_version != 3 with UnsupportedVersion"
  - "Reader rejects unknown TypeDef kind bytes with InvalidTypeDefKind"
  - "ModuleBuilder::add_type_def takes TypeDefKind enum (compile-time safety)"
  - "format_version=3 in both Module::new() and ModuleBuilder::build()"
  - "TypeDefKind re-exported from writ_module crate root"
affects: [49-vm-semantics, 50-compiler-update, writ-assembler, writ-compiler, writ-runtime]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fail-fast version validation: reader rejects format_version != 3 immediately after header parse"
    - "Row-level kind validation: TypeDef kind byte validated in read_type_def, not table dispatch loop"
    - "Enum at API boundary: public builder API uses TypeDefKind enum; TypeDefRow.kind stays u8 (binary mirror)"

key-files:
  created: []
  modified:
    - writ-module/src/tables.rs
    - writ-module/src/error.rs
    - writ-module/src/reader.rs
    - writ-module/src/builder.rs
    - writ-module/src/module.rs
    - writ-module/src/lib.rs
    - writ-module/tests/round_trip.rs
    - writ-module/tests/builder_tests.rs

key-decisions:
  - "TypeDefRow.kind stays u8 (mirrors binary format 1:1); TypeDefKind enum used at all public API boundaries"
  - "Reader validates format_version == 3 exactly (no range, no backward compat); caller gets UnsupportedVersion(version)"
  - "Kind validation at read_type_def row level, not the table dispatch loop, keeps validation co-located with data"
  - "ModuleBuilder::add_type_def changed from kind: u8 to kind: TypeDefKind — compile-time prevention of invalid kinds"

patterns-established:
  - "Binary format: raw bytes in table rows; validated types at reader/builder boundary"
  - "Version bump: format_version=3 is the single canonical version; no mixed-version support"

requirements-completed: [IL-01, IL-02, IL-03]

# Metrics
duration: 8min
completed: 2026-03-12
---

# Phase 48 Plan 01: IL Format - Class Kind Support Summary

**TypeDefKind::Class=4 added to writ-module binary format with format_version=3, strict reader validation, type-safe builder API, and round-trip tests**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-12T18:40:00Z
- **Completed:** 2026-03-12T18:48:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Added `TypeDefKind::Class = 4` variant with `from_u8` match arm and `Display` impl returning "class"
- Bumped `format_version` to 3 in both `Module::new()` and `ModuleBuilder::build()`; reader rejects any other version with `DecodeError::UnsupportedVersion`
- Added `DecodeError::InvalidTypeDefKind(u8)` and reader validation in `read_type_def` — unknown kind bytes rejected at row-parse time
- Changed `ModuleBuilder::add_type_def` from `kind: u8` to `kind: TypeDefKind` — invalid kinds now caught at compile time
- Re-exported `TypeDefKind` from `writ_module` crate root alongside `Module`, `ModuleBuilder`, `MetadataToken`
- Added 4 new tests: `test_class_typedef_round_trip`, `test_format_version_rejection`, `test_invalid_typedef_kind_rejection`, `test_builder_class_type`; all 89 writ-module tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Class variant, Display, validation, format_version bump** - `90ddf08` (feat)
2. **Task 2: Update tests and fix downstream compilation** - `e84d9ea` (test)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `writ-module/src/tables.rs` - Added `Class = 4` variant, updated `from_u8`, added `Display` impl
- `writ-module/src/error.rs` - Added `InvalidTypeDefKind(u8)` variant
- `writ-module/src/reader.rs` - Added format_version==3 check; added kind validation in `read_type_def`
- `writ-module/src/builder.rs` - Changed `TypeDefBuilder.kind: u8` to `TypeDefKind`; `add_type_def` signature updated; `format_version: 3`
- `writ-module/src/module.rs` - `Module::new()` sets `format_version: 3`
- `writ-module/src/lib.rs` - Added `pub use tables::TypeDefKind`
- `writ-module/tests/round_trip.rs` - Added 3 new tests for class round-trip, version rejection, kind rejection
- `writ-module/tests/builder_tests.rs` - Updated 4 call sites to use `TypeDefKind::Struct` (not `.as_u8()`); added `test_builder_class_type`

## Decisions Made
- TypeDefRow.kind stays `u8` to mirror binary format 1:1; all public-facing APIs use `TypeDefKind` enum
- Reader validates `format_version == 3` exactly (strict fail-fast, no compat range)
- Kind validation placed inside `read_type_def` so it is co-located with the data read

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `writ-module` crate now fully supports Class TypeDef kind at the binary format layer
- Downstream crates (writ-compiler, writ-assembler, writ-runtime) will need updating in subsequent plans to use `TypeDefKind::Class` and import `TypeDefKind` from `writ_module` instead of local definitions
- format_version=3 is a breaking change — any v2 modules will be rejected; all consumer crates must rebuild against v3

---
*Phase: 48-il-format*
*Completed: 2026-03-12*
