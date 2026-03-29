# Phase 96: [Conditional] Semantic Effects - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Functions marked `[Conditional("name")]` are emitted only when the named condition is active, with type-checking of call-site arguments always occurring regardless of elision, and a verified fallback function.

Requirements: COND-01, COND-02, COND-03, COND-04

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key design notes from STATE.md:
- [Conditional] elision must happen at emit time only (EmitCtx, not CheckCtx) — args still type-check when call is elided
- Research gap: `--condition name` vs. `--condition name=bool` CLI syntax unresolved — decide during planning
- Impl-block [Conditional] semantics need spec update before shipping

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 94 attribute infrastructure (AttributeDef pipeline)
- Phase 95 deprecated pattern (TypeEnv metadata maps)
- `writ-compiler/src/emit/` — emitter infrastructure
- `writ-compiler/src/resolve/` — resolver with collector

### Established Patterns
- CompileConfig for passing compiler flags through pipeline
- EmitCtx for emit-time context
- Golden tests for verifying compilation output

### Integration Points
- CLI: `--condition` flag
- Resolver: fallback verification, multiple-condition error
- Emitter: conditional function filtering
- Type checker: type-check passthrough even when elided

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
