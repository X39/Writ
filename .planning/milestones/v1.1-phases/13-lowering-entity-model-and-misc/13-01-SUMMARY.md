---
phase: 13-lowering-entity-model-and-misc
plan: 01
subsystem: compiler, lowering
tags: [entity, component-slots, lifecycle-hooks, implicit-self, operator, IndexSet]

# Dependency graph
requires:
  - phase: 10-parser-core-syntax
    provides: CST self/mut-self params, lifecycle hook parsing
  - phase: 12-lowering-dialogue-and-localization
    provides: LoweringContext namespace API patterns
provides:
  - AstDecl::Entity variant with AstEntityDecl struct (ENT-04)
  - AstComponentSlot host-managed descriptors (ENT-02)
  - All 6 lifecycle hooks recognized (ENT-01)
  - Implicit mut self in hooks, self/mut self in operators (ENT-03)
  - IndexSet contract name corrected (MISC-01)
affects: [tests, codegen]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Entity lowering emits AstDecl::Entity instead of AstDecl::Struct + Impl chain"
    - "Components are AstComponentSlot descriptors, not inline struct fields"
    - "Implicit self injection: mut self for hooks and IndexSet, immutable self for other operators"

key-files:
  created: []
  modified:
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/src/lower/entity.rs
    - writ-compiler/src/lower/operator.rs

key-decisions:
  - "AstEntityDecl carries properties, component_slots, hooks, and inherent_impl as separate structured fields"
  - "ComponentAccess<T> impl generation removed — components are host-managed, codegen emits GET_COMPONENT directly"
  - "All 6 hooks get implicit mut self (even serialize) per spec §14.6"
  - "Operator self mutability: IndexSet gets mut self, all others get immutable self"

patterns-established:
  - "AstComponentSlot: component name + overrides vector for host-managed component slots"
  - "AstEntityHook: contract name + method decl pattern for lifecycle hook registrations"

requirements-completed: [ENT-01, ENT-02, ENT-03, ENT-04, MISC-01]

# Metrics
duration: ~15min
completed: 2026-03-01
---

# Phase 13 Plan 01: Entity Model and Misc Implementation Summary

**Entity lowering rewritten to emit AstDecl::Entity with component slots, all 6 hooks with implicit mut self, and IndexSet contract name corrected**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 2
- **Files modified:** 3 (plus 28 snapshot files)

## Accomplishments
- Added AstDecl::Entity variant with AstEntityDecl, AstComponentSlot, AstEntityHook types to AST
- Rewrote entity lowering to emit AstDecl::Entity instead of AstDecl::Struct + Impl chain
- Components modeled as AstComponentSlot descriptors (host-managed) instead of $ComponentName struct fields
- All 6 lifecycle hooks now recognized: create, destroy, interact, finalize, serialize, deserialize
- Implicit mut self injected as first param in all lifecycle hook methods
- Implicit self/mut self injected into operator methods (mut for IndexSet, immutable for others)
- IndexSet contract name fixed from "IndexMut" to "IndexSet"
- Removed ComponentAccess<T> impl generation (no longer needed with host-managed slots)
- All 425 tests pass with 28 snapshots updated

## Files Created/Modified
- `writ-compiler/src/ast/decl.rs` - Added Entity variant, AstEntityDecl, AstComponentSlot, AstEntityHook types
- `writ-compiler/src/lower/entity.rs` - Rewrote lower_entity to emit AstDecl::Entity; partition_entity_members accepts 6 hooks; implicit mut self injection
- `writ-compiler/src/lower/operator.rs` - Fixed IndexSet contract name; added implicit self injection to op_to_contract_impl

## Decisions Made
- AstEntityDecl carries all entity structure in one type (properties, component_slots, hooks, inherent_impl) rather than emitting separate Struct + Impl decls
- ComponentAccess<T> impls dropped since components are now host-managed descriptors — codegen will emit GET_COMPONENT directly
- All 6 hooks get implicit `mut self` per spec §14.6, including serialize (hook might need to park native state)
- Operator self mutability: IndexSet = mut self (modifies container), all others = immutable self (read-only)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 requirements implemented and existing tests pass
- Ready for Plan 13-02 (comprehensive test suite)

---
*Phase: 13-lowering-entity-model-and-misc*
*Plan: 01*
*Completed: 2026-03-01*
