# Phase 95: [Deprecated] Semantic Effects - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Referencing a deprecated item produces a compiler warning with the user's message string and the LSP surfaces that warning as a diagnostic with the message in hover.

Requirements: DEPR-01, DEPR-02

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key design notes:
- W0006 warning code for deprecated item references
- Self-deprecation suppression: no warning when call site is in same module as deprecated item
- LSP must show DiagnosticSeverity::Warning squiggle AND hover tooltip with deprecation message

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/check/` — type checker with env_build for building environments
- `writ-compiler/src/check/check_expr/` — expression checking where call-site warnings would fire
- `writ-lsp/src/queries/hover.rs` — hover information
- `writ-lsp/src/queries/semantic.rs` — semantic diagnostics
- Attribute infrastructure from Phase 93-94

### Established Patterns
- Warning/error diagnostics use diagnostic codes (E0001-E0008 existing)
- LSP queries in writ-lsp/src/queries/

### Integration Points
- env_build: build deprecated_items map from attribute data
- check_expr: emit W0006 at reference sites
- LSP hover: show deprecation message
- LSP diagnostics: show warning squiggles

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
