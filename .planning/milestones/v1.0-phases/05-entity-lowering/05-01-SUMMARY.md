---
phase: 05-entity-lowering
plan: 01
subsystem: compiler
tags: [rust, lowering, entity, ast, cst, struct-lit]

# Dependency graph
requires:
  - phase: 04-dialogue-lowering-and-localization
    provides: lower_dialogue, LoweringContext patterns, dialogue lowering pipeline
  - phase: 03-operator-and-concurrency-lowering
    provides: lower_operator_impls pattern, AstImplDecl/AstImplMember types
provides:
  - "lower/entity.rs with lower_entity() → Vec<AstDecl>"
  - "partition_entity_members() with all 4 validation checks"
  - "AstExpr::StructLit variant for component field initializers"
  - "Entity-specific LoweringError variants (DuplicateUseClause, DuplicateProperty, UnknownLifecycleEvent, PropertyComponentCollision)"
  - "Item::Entity arms wired in both lower() and lower_namespace()"
affects:
  - 05-entity-lowering
  - type-checker
  - code-generator

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Entity-to-multi-decl expansion: one CST node emits Vec<AstDecl> (struct + impls)"
    - "partition_entity_members() pre-step separates scanning/validation from lowering"
    - "Dollar-prefix ($ComponentName) for user-unreachable component struct fields"
    - "ComponentAccess<T> impl with single get() method returning self.$ComponentName"
    - "Lifecycle hooks map to contract impls: OnCreate/OnInteract/OnDestroy"

key-files:
  created:
    - writ-compiler/src/lower/entity.rs
  modified:
    - writ-compiler/src/ast/expr.rs
    - writ-compiler/src/lower/error.rs
    - writ-compiler/src/lower/mod.rs

key-decisions:
  - "Intermediate structs (EntityProperty, EntityUseClause, EntityHook) over deeply nested tuples in partition_entity_members — clearer field access, no ambiguity"
  - "lower_attrs and lower_struct_field made pub(crate) in mod.rs to avoid circular imports; entity.rs uses super:: pattern"
  - "EntityDecl not re-imported in mod.rs — entity.rs handles the full type; mod.rs only destructures Item::Entity(e, e_span)"
  - "Collision detection checks both directions: Property after Use and Use after Property both error via seen_props/seen_components cross-checks"

patterns-established:
  - "Entity lowering uses extend() pattern matching operator_impls: decls.extend(lower_entity(...))"
  - "Empty inherent impl suppressed: methods vec must be non-empty before emitting AstDecl::Impl with contract: None"

requirements-completed: [R12, R13]

# Metrics
duration: 15min
completed: 2026-02-27
---

# Phase 05 Plan 01: Entity Lowering Core Summary

**entity Name { ... } lowering to AstDecl::Struct + ComponentAccess<T> impls + lifecycle hook contract impls with full error accumulation**

## Performance

- **Duration:** 15 min
- **Started:** 2026-02-27T00:00:00Z
- **Completed:** 2026-02-27T00:15:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added AstExpr::StructLit variant to represent component field initializers (Health { current: 80, max: 80 })
- Added four entity-specific LoweringError variants: DuplicateUseClause, DuplicateProperty, UnknownLifecycleEvent, PropertyComponentCollision
- Created lower/entity.rs with partition_entity_members() (4-bucket member validator) and lower_entity() (multi-decl emitter)
- Replaced both todo!() arms in lower() and lower_namespace() — no remaining todo! in the entity lowering path
- All 46 existing snapshot tests pass unchanged; zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Add AstExpr::StructLit, entity error variants, and visibility fixes** - `47eaa98` (feat)
2. **Task 2: Create lower/entity.rs with partition_entity_members and lower_entity** - `ec38661` (feat)

**Plan metadata:** (docs commit — see state updates below)

## Files Created/Modified

- `writ-compiler/src/ast/expr.rs` — Added AstExpr::StructLit { name, name_span, fields, span } variant in Literals section
- `writ-compiler/src/lower/error.rs` — Added DuplicateUseClause, DuplicateProperty, UnknownLifecycleEvent, PropertyComponentCollision variants
- `writ-compiler/src/lower/mod.rs` — Made lower_attrs and lower_struct_field pub(crate); added pub mod entity; added lower_entity import; replaced both Item::Entity todo!() arms
- `writ-compiler/src/lower/entity.rs` — New file: EntityProperty/EntityUseClause/EntityHook structs, partition_entity_members(), lower_entity()

## Decisions Made

- Used intermediate structs (EntityProperty, EntityUseClause, EntityHook) in partition_entity_members() instead of deeply nested tuples — cleaner field access with no ambiguity at tuple index positions.
- EntityDecl is not imported in mod.rs — entity.rs owns the type; mod.rs only pattern-matches Item::Entity((e, e_span)) and passes through to lower_entity.
- Collision detection checks both directions using cross-set membership: a Property added after a Use clause checks seen_components, and a Use clause added after a Property checks seen_props.
- Inherent impl (methods) suppressed when methods vec is empty — mirrors the empty-base-impl-suppressed decision from Phase 3.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None — the plan's interface definitions were accurate. EntityDecl structure matched expectations. lower_type() and lower_expr() APIs worked as specified.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Entity lowering is complete. The full lowering pipeline now handles all Writ Item variants without todo! panics.
- Phase 05 Plan 02 (snapshot tests for entity lowering) can proceed immediately — the entity.rs public API is stable.
- Type checker phase will need to handle ComponentAccess<T> impls and resolve component existence.

---
*Phase: 05-entity-lowering*
*Completed: 2026-02-27*
