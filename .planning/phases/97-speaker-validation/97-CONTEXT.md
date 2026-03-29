# Phase 97: Speaker Validation - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Dialogue blocks using `@speaker` syntax validate that the named speaker is a [Singleton] entity; non-singleton and non-existent entity speakers produce errors.

Requirements: SPKR-01, SPKR-02

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key design notes:
- E0007 for non-[Singleton] entity speakers
- Distinct error for non-existent entity speakers
- Contract-typed speakers must be suppressed (no false E0007)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/lower/dialogue.rs` — dialogue lowering with speaker references
- `writ-compiler/src/resolve/` — resolver infrastructure
- Phase 94 attribute infrastructure ([Singleton] is a builtin attribute)
- `writ-runtime/src/dispatch/entities.rs` — entity dispatch with speaker references

### Integration Points
- Lowering: collect speaker names from dialogue blocks
- Resolver: validate speaker names against entity definitions
- Attribute system: check for [Singleton] attribute on matched entities

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
