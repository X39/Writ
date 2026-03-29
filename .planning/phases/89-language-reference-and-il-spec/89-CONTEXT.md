# Phase 89: Language Reference and IL Spec - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can browse the complete Writ language spec and IL specification as navigable mdBook chapters with working cross-references between sections. All chapters render correctly with proper table formatting and code blocks.

</domain>

<decisions>
## Implementation Decisions

### Cross-Reference Strategy
- Use mdBook relative links (`[Section X](../language-ref/types.md#enums)`) — standard approach, no preprocessor needed
- Create 3-5 natural cross-references at linkage points: types ↔ structs/enums, entities ↔ components, dialogue ↔ concurrency, IL instruction set → execution model
- Forward references only — no bidirectional linking (marginal benefit, high edit count)

### IL Spec Rendering
- Fix table rendering issues in-place in spec source files — fix alignment issues or missing pipe characters
- IL spec code blocks do NOT get syntax highlighting — IL assembly/pseudocode is a different language from Writ
- Keep the existing SUMMARY.md navigation structure from Phase 87 — Language Reference and IL Specification already in separate sections

### Claude's Discretion
- Specific cross-reference selection (which 3-5 to add)
- Table formatting fixes needed for IL spec
- Any chapter title adjustments for clarity

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/src/SUMMARY.md` — already has 68 chapters wired (29 language-ref + 39 il-spec)
- `docs/src/language-ref/*.md` and `docs/src/il-spec/*.md` — wrapper files with `{{#include}}` directives
- `language-spec/spec/*.md` — source spec files (H1 already stripped in Phase 87)

### Established Patterns
- mdBook wrapper pattern: each chapter is a thin wrapper using `{{#include ../../language-spec/spec/XX_file.md}}`
- Spec files numbered `00_`-`29_` are language spec, `30_`+ are IL spec

### Integration Points
- `docs/src/SUMMARY.md` — chapter navigation (may need title adjustments)
- `language-spec/spec/*.md` — source files where cross-references and table fixes apply

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches for cross-references and table fixes.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
