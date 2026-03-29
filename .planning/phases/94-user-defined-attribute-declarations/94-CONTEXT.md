# Phase 94: User-Defined Attribute Declarations - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Users can declare custom attributes with typed parameters, those declarations survive the full compiler pipeline into the binary module, and builtin attribute names are reserved so user attributes cannot shadow them.

Requirements: UATTR-01, UATTR-02, UATTR-03, UATTR-04

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key considerations from STATE.md blockers:
- Research gap: `attribute` keyword vs. contextual keyword decision — resolve before touching the parser
- Builtin name reservation happens in the collector pass using DefId origin, not bare string matching

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module/src/attr.rs` — AttrValue enum and tag constants (from Phase 93)
- `writ-compiler/src/emit/collect/encoding.rs` — attribute argument encoding (from Phase 93)
- `writ-runtime/src/virtual_module.rs` — virtual module infrastructure
- Parser declaration handling patterns in `writ-parser/src/parser/program.rs`

### Established Patterns
- DefKind enum in resolver for declaration types
- AstDecl enum in AST for declaration nodes
- Collector pass for gathering definitions
- Virtual module for builtin type registration

### Integration Points
- Parser → AST → Lowering → Resolver → Type checker → Emitter pipeline
- Virtual module AttributeDef table for builtins
- Collector pass for builtin name reservation

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Refer to ROADMAP phase description and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
