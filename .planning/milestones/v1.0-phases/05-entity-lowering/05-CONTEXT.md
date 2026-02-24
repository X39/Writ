# Phase 5: Entity Lowering - Context

**Gathered:** 2026-02-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Desugar `entity` declarations to `struct` + `impl ComponentAccess<T>` blocks + lifecycle hook contract impls. One entity declaration expands to multiple AST declarations. Component definitions are NOT visible at lowering time — cross-module resolution is deferred to the type checker.

</domain>

<decisions>
## Implementation Decisions

### Lifecycle hook representation
- Hooks lower to **contract impls**: `on create { ... }` → `impl OnCreate for Guard { fn on_create(self) { ... } }`
- Three contracts: `OnCreate`, `OnInteract`, `OnDestroy` — assumed pre-defined (extern/builtin), lowering only emits impl blocks
- Hook bodies receive **full expression lowering** (formattable strings, optional sugar, compound assignments, `->` transitions all desugar)
- `[Singleton]` propagates as an `AstAttribute` on the generated `AstStructDecl`

### Component initialization
- `use Health { current: 80, max: 80 }` → struct field with struct literal initializer containing **only the specified overrides**
- `use Speaker {}` → empty struct literal `Speaker {}` — type checker fills all defaults from component definition
- Component fields use **user-unreachable names** (e.g., `$Health`, `$Sprite`) to prevent collisions with user-defined properties
- Entity property fields (name, patrolRoute) retain their original names and visibility unchanged

### ComponentAccess impl shape
- One `impl ComponentAccess<T> for EntityName` per `use` clause
- Single method: `fn get(self) -> T { self.$ComponentName }`
- Entity's own methods go in a **separate inherent impl block** (`impl Guard { fn greet() { ... } }`)

### Emission order
- Claude's discretion — pick a logical, deterministic order suitable for snapshot tests

### Error boundaries
- **Lowering-time errors** (all accumulated, never halt processing):
  - Duplicate `use` clauses (same component used twice)
  - Duplicate property names
  - Unknown lifecycle event names (not create/interact/destroy)
  - Property-component name collisions
- **Deferred to type checker:**
  - Conflicting method names across components (§14.3) — lowering can't see component definitions
  - Missing/invalid field names in use clause overrides
  - Component existence validation

### Member partitioning
- Explicit named `partition_entity_members()` function as a pre-step
- Returns (properties, use_clauses, methods, hooks)
- All duplicate/validation checks happen during partitioning

</decisions>

<specifics>
## Specific Ideas

- Component field naming: use `$ComponentName` convention (dollar-prefix makes names unreachable from user code since `$` is illegal in Writ identifiers)
- Self/this semantics: the spec currently lacks explicit `self` parameter definition for instance methods — lowering should emit `self` matching spec examples, but a spec update for static vs instance methods may be needed
- Error recovery pattern: same as dialogue lowering — emit error, skip problematic member, continue lowering remaining members, report all errors at once

</specifics>

<deferred>
## Deferred Ideas

- Spec update needed: clarify `self` semantics — remove static modifier for non-instance methods, add `self` parameter to indicate instance methods (spec gap, not a Phase 5 task)
- Cross-component method conflict detection — belongs in the type checker phase, not lowering

</deferred>

---

*Phase: 05-entity-lowering*
*Context gathered: 2026-02-26*
