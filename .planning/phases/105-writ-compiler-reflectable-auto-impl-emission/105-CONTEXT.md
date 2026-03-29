# Phase 105: writ-compiler Reflectable Auto-Impl Emission - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — compiler auto-generates Reflectable ImplDefs)

<domain>
## Phase Boundary

For every user-defined TypeDef (struct, class, enum, entity), the compiler auto-generates a Reflectable ImplDef with a single get_type() method that dispatches via CALL_VIRT. ImplDefs must be interleaved in TypeDef declaration order (not appended in a post-pass). Satisfies COMP-03, REFL-02.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked in STATE.md:
- Reflectable auto-impl must be emitted interleaved in TypeDef declaration order, not in a post-pass — preserves method_list offset invariant
- Reflectable is contract 19 (0-based index 18) in the virtual module
- Each auto-impl has a single method: get_type() -> Type
- Extern types are excluded from auto-impl (they are host-managed)
- The get_type() method body should emit a TYPEOF instruction with the type's own type index

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Existing ImplDef emission in writ-compiler/src/emit/collect/contracts.rs
- TypeOf instruction emission from Phase 104
- Contract dispatch table in writ-runtime

### Established Patterns
- ImplDef collection during the collect pass
- Method body generation in emit/body/

### Integration Points
- writ-compiler/src/emit/collect/ — ImplDef collection
- writ-compiler/src/emit/body/ — method body generation
- writ-compiler/src/emit/module_builder.rs — module metadata builder

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Follow existing ImplDef emission patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
