# Phase 108: Generic Reflection - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — generic type reflection queries + spec)

<domain>
## Phase Boundary

Implement Type.is_generic and Type.type_args() for statically-known generic instantiations, add MethodInfo.attributes() and FieldInfo.attributes() intrinsics, and document the generic reflection limitation in the spec. Satisfies GEN-01 through GEN-04.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked:
- type_args() for runtime-queried generics (via get_type()) returns partial or empty info
- type_args() for statically-known instantiations (via typeof()) returns correct Type array
- is_generic returns true for generic type instantiations, false for non-generic types
- Spec must document the limitation explicitly

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Type TypeDef already has is_generic field (field index 3)
- ReflectionIndex with type/field/method caches
- AttributeInfo allocation helpers from Phase 103
- Existing intrinsic dispatch patterns

### Integration Points
- writ-runtime/src/dispatch/intrinsics.rs — new intrinsic arms
- writ-runtime/src/reflection.rs — type_args allocation
- language-spec/spec/28_1_28_reflection.md — spec updates

</code_context>

<specifics>
## Specific Ideas

No specific requirements.

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
