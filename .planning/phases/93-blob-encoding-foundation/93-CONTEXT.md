# Phase 93: Blob Encoding Foundation - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Attribute arguments survive into the binary module as round-trippable tagged values — the compiler encodes them and the runtime can decode them back to typed data.

Requirements: BLOB-01, BLOB-02, BLOB-03

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module` crate — shared types between compiler and runtime
- `encoding.rs` in compiler emit — currently has `value=0` stub for attribute args
- `writ-runtime` decoder infrastructure

### Established Patterns
- Tag constants and shared enums live in `writ-module`
- Round-trip tests validate encode/decode fidelity

### Integration Points
- Compiler emit path (`encoding.rs`) — replaces `value=0` stub
- Runtime decoder — imports shared `AttrValue` enum and tag constants
- `AttributeDef` table rows — blob offsets

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Refer to ROADMAP phase description and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
