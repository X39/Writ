# Phase 13: Lowering — Entity Model and Misc - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Entity lowering produces spec-compliant AST output — all six lifecycle hooks recognized (create, destroy, interact, finalize, serialize, deserialize), components modeled as host-managed slot descriptors (not inline struct fields), implicit self/mut self injected into operator and lifecycle hook methods, a distinct `AstDecl::Entity` variant emitted instead of reusing `AstDecl::Struct`, and the IndexSet contract name corrected from "IndexMut". Requirements: ENT-01, ENT-02, ENT-03, ENT-04, MISC-01.

</domain>

<decisions>
## Implementation Decisions

### Hook self mutability
- All 6 lifecycle hooks get implicit `mut self` — matches spec exactly (§14.6, §14.7.3)
- `on serialize` gets `mut self` even though serialization is logically read-only — the hook might need to park native state
- `on interact(who: Entity)` gets `mut self` plus the explicit `who` parameter

### Self injection scope (ENT-03)
- Lifecycle hooks: implicit `mut self` injected as first parameter
- Operator methods: implicit self injected — Claude decides mutability per operator (IndexSet likely `mut self`, others `self`)
- Applies at the lowering pass level, not as a separate pass

### Claude's Discretion
- **Entity AST structure:** Whether AstEntityDecl carries all sections (properties, component slots, hooks, methods) in one struct or keeps hooks/methods as separate Impl blocks. Should mirror what best serves downstream codegen.
- **Component slot model:** Design of AstComponentSlot struct — structured overrides vs. reusing existing tuple patterns. Overrides nested per slot (maps to source) preferred.
- **ComponentAccess impl generation:** Whether to drop the current ComponentAccess<T> impls (since components are now host-managed slots, not inline fields) or update them. Recommendation: drop them, let codegen emit GET_COMPONENT directly.
- **Override value representation:** Raw AstExpr for component override values (lowering preserves expressions, codegen evaluates).
- **Hook contract naming:** Follow established pattern — OnFinalize/on_finalize, OnSerialize/on_serialize, OnDeserialize/on_deserialize.
- **Operator self mutability:** Per-operator decision — IndexSet gets `mut self`, read-only operators (Add, Sub, Eq, etc.) get immutable `self`.

</decisions>

<specifics>
## Specific Ideas

No specific requirements beyond spec conformance — this phase follows the lowering reference (§14.7, §28.3) precisely.

Key spec references:
- §14.6: All hooks receive implicit `mut self`
- §14.7.1: TypeDef(kind=Entity) with fields, component_slots, component_overrides
- §14.7.3: Hook lowering table with `__on_*` method naming
- §28.3: Entity lowering reference showing complete output shape

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AstFnParam::SelfParam { mutable: bool, span }`: Already exists in ast/decl.rs — use for self injection
- `partition_entity_members()`: Partitions entity members into properties, use clauses, methods, hooks — extend to accept 3 new hook events
- `lower_fn()`, `lower_param()`, `lower_vis()`, `lower_attrs()`: Existing lowering helpers reusable as-is
- `AstStructField`, `AstStructDecl`: Pattern for field/decl structs to follow when designing AstEntityDecl
- `op_symbol_to_contract()`: Contains the IndexMut→IndexSet fix target at line 252 of operator.rs

### Established Patterns
- Entity hooks currently emit as `AstDecl::Impl(AstImplDecl { contract: Some(OnCreate/OnInteract/OnDestroy), ... })` — one impl per hook
- Components currently emit as `$ComponentName` fields on struct + `ComponentAccess<T>` impl per use clause
- Operator lowering extracts ops from impl blocks into standalone contract impls via `op_to_contract_impl()`
- `LoweringError::UnknownLifecycleEvent` already exists for validation

### Integration Points
- `writ-compiler/src/ast/decl.rs`: Add `Entity(AstEntityDecl)` variant to `AstDecl` enum
- `writ-compiler/src/lower/entity.rs`: Main file to modify — `partition_entity_members` + `lower_entity`
- `writ-compiler/src/lower/operator.rs:252`: IndexSet contract name fix
- `writ-compiler/src/lower/mod.rs`: May need to update dispatch for new AstDecl::Entity variant
- `writ-compiler/tests/lowering_tests.rs`: Test file for entity and operator lowering

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 13-lowering-entity-model-and-misc*
*Context gathered: 2026-03-01*
