---
phase: 05-entity-lowering
plan: 02
subsystem: compiler
tags: [rust, lowering, entity, snapshot-tests, insta, r12, r13]

# Dependency graph
requires:
  - phase: 05-entity-lowering
    plan: 01
    provides: lower_entity(), partition_entity_members(), AstExpr::StructLit, four entity LoweringError variants
provides:
  - "16 entity lowering snapshot tests (R12 + R13 quality gate)"
  - "10 R12 success-path tests: property fields, use clauses, lifecycle hooks, methods, [Singleton], full declaration, multiple components"
  - "2 R13 component flattening tests: partial override, no override (empty fields vec)"
  - "4 entity error tests: DuplicateUseClause, DuplicateProperty, UnknownLifecycleEvent, PropertyComponentCollision"
  - "Accepted insta snapshot files for all 16 new tests"
affects:
  - type-checker
  - code-generator

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "lower_src_with_errors() for error-path snapshot tests; snapshot (ast, errors) tuple together"
    - "INSTA_UPDATE=always auto-accept workflow; final cargo test without flag confirms stability"

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_component_use_clause.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_empty_use_clause.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_create.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_interact_with_params.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_destroy.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_methods_inherent_impl.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_full_declaration.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_multiple_use_clauses.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_component_partial_override.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_component_no_override.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_use_clause_error.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_unknown_lifecycle_event_error.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_property_component_collision_error.snap
  modified:
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "snapshot (ast, errors) tuple together in error tests — captures both partial AST output and error list in one assertion"
  - "entity_component_partial_override uses only `current: 50` override — confirms absent fields are NOT emitted (type checker fills defaults)"
  - "entity_empty_use_clause vs entity_component_no_override: both test empty StructLit; empty_use_clause exercises Speaker type, no_override exercises Health"

patterns-established:
  - "Error snapshot tests snapshot (ast, errors) tuple — partial AST + errors in one assertion; error accumulation confirmed"
  - "R13 partial-override pattern: only listed fields appear in StructLit fields vec; absent fields confirmed absent"

requirements-completed: [R12, R13]

# Metrics
duration: 10min
completed: 2026-02-27
---

# Phase 05 Plan 02: Entity Lowering Snapshot Tests Summary

**16 insta snapshot tests verifying entity-to-multi-decl lowering: property fields, component use clauses, lifecycle hooks, [Singleton], methods, and all four entity-specific error paths**

## Performance

- **Duration:** 10 min
- **Started:** 2026-02-27T08:02:00Z
- **Completed:** 2026-02-27T08:12:12Z
- **Tasks:** 2
- **Files modified:** 17

## Accomplishments

- Added 10 R12 success-path tests covering all entity member types: property fields (with defaults), full use clause with StructLit initializer, empty use clause (still emits field+impl), all three lifecycle hook events (on create/interact/destroy with params/on destroy), inherent impl for methods, [Singleton] attribute propagation, full four-member entity in deterministic order, and multiple use clauses
- Added 2 R13 component field flattening tests confirming partial override (only specified fields in StructLit) and no-override (empty fields vec) behavior
- Added 4 entity error tests covering the complete error surface: DuplicateUseClause (first use clause still emits), DuplicateProperty (second skipped), UnknownLifecycleEvent (with event name in error), PropertyComponentCollision (both directions)
- All 46 prior tests unchanged; total 62 tests pass with stable snapshots (verified without INSTA_UPDATE)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add R12 entity lowering snapshot tests (success paths)** - `d376180` (test)
2. **Task 2: Add R13 component flattening and entity error snapshot tests** - `d5e0d55` (test)

**Plan metadata:** (docs commit — see state updates below)

## Files Created/Modified

- `writ-compiler/tests/lowering_tests.rs` — Added R12, R13, and entity error test sections (tests 47-62)
- `writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap` — Properties with types and defaults, no inherent impl
- `writ-compiler/tests/snapshots/lowering_tests__entity_component_use_clause.snap` — $Health field with StructLit + ComponentAccess<Health> impl
- `writ-compiler/tests/snapshots/lowering_tests__entity_empty_use_clause.snap` — Empty use clause emits field and impl
- `writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_create.snap` — impl OnCreate for Guard
- `writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_interact_with_params.snap` — impl OnInteract with params
- `writ-compiler/tests/snapshots/lowering_tests__entity_lifecycle_on_destroy.snap` — impl OnDestroy for Guard
- `writ-compiler/tests/snapshots/lowering_tests__entity_methods_inherent_impl.snap` — method in inherent impl (contract: None)
- `writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap` — [Singleton] in AstStructDecl.attrs
- `writ-compiler/tests/snapshots/lowering_tests__entity_full_declaration.snap` — all four member types, deterministic order
- `writ-compiler/tests/snapshots/lowering_tests__entity_multiple_use_clauses.snap` — two $Health/$Sprite fields and impls
- `writ-compiler/tests/snapshots/lowering_tests__entity_component_partial_override.snap` — only `current: 50` in StructLit, no `max`
- `writ-compiler/tests/snapshots/lowering_tests__entity_component_no_override.snap` — empty StructLit fields vec
- `writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_use_clause_error.snap` — DuplicateUseClause with first use still emitted
- `writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap` — DuplicateProperty, second skipped
- `writ-compiler/tests/snapshots/lowering_tests__entity_unknown_lifecycle_event_error.snap` — UnknownLifecycleEvent("explode")
- `writ-compiler/tests/snapshots/lowering_tests__entity_property_component_collision_error.snap` — PropertyComponentCollision for Health/use Health

## Decisions Made

- Snapshot `(ast, errors)` tuple together in error tests rather than separate assertions — single snapshot captures both partial AST output and error list, making intent clear and regression detection comprehensive.
- `entity_component_partial_override` verifies only the overridden field (`current: 50`) appears in the StructLit fields vec; absent fields are intentionally not emitted (type checker fills defaults later).
- Chose `entity_empty_use_clause` (Speaker) and `entity_component_no_override` (Health) as two distinct empty-StructLit tests to exercise both different component names and confirm the pattern is consistent, not coincidental.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None — all entity lowering API surfaces matched the plan's interface definitions. No parser surprises; CST EntityMember variants matched expectations.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 05 complete: entity lowering core implementation (Plan 01) and snapshot quality gate (Plan 02) both done.
- All 15 requirements (R1-R13 + earlier work) for the CST-to-AST lowering pipeline are now covered.
- The full lowering pipeline handles all Writ Item variants: functions, impls, operators, dialogue, entities, namespaces.
- Type checker phase can proceed with confidence in the AST structure produced by lower().

---
*Phase: 05-entity-lowering*
*Completed: 2026-02-27*

## Self-Check: PASSED

- All 17 snapshot files: FOUND
- Task 1 commit d376180: FOUND
- Task 2 commit d5e0d55: FOUND
- Test count: 62 passed, 0 failed (stable without INSTA_UPDATE)
