# Phase 106: Read-Only Introspection Integration Tests and LSP - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — E2E integration tests and LSP)

<domain>
## Phase Boundary

Write end-to-end integration tests covering all read-only reflection operations (typeof, get_type, Type.fields/methods/attributes/contracts/implements, FieldInfo.get, Type equality, primitive typeof) across struct/class/enum/entity types. Add static-vs-dynamic typeof test. Add GC survival test. Update LSP hover tooltips for typeof expressions and reflection type members. Add golden test .writ files that compile and run. Satisfies REFL-03 through REFL-09, LSP-01, LSP-02.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — integration test phase. Key decisions locked:
- typeof(Animal) on polymorphic Dog returns Animal (static); dog.get_type() returns Dog (dynamic)
- typeof(T) == typeof(T) must be true (interning by TypeDef)
- typeof(T) == typeof(U) must be false for different types
- GC survival: cached Type objects persist after GC with no script-side roots
- LSP shows "Type" as the hover type for typeof expressions

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-golden test infrastructure (compile → disassemble → compare)
- writ-runtime/tests/reflection_tests.rs (unit-level reflection tests from Phase 103)
- writ-lsp hover/diagnostics infrastructure
- Existing E2E test patterns

### Integration Points
- writ-golden/tests/ — golden test .writ source files
- writ-runtime/ — reflection dispatch and GC
- writ-lsp/src/ — hover tooltips and type annotations
- writ-compiler/ — typeof type checking (Phase 104)

</code_context>

<specifics>
## Specific Ideas

- Also address Phase 104's deferred COMP-04 test (typeof passes to Type parameter) if Type becomes resolvable
- Also address Phase 105's deferred E2E test (compiled get_type() returns correct name)

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
