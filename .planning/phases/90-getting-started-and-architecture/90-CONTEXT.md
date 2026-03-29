# Phase 90: Getting Started and Architecture - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

New prose chapters covering installation, hello world, CLI reference, compiler architecture, and crate map — enabling a new user to install Writ, write and run a program, and understand the compiler's structure.

</domain>

<decisions>
## Implementation Decisions

### Content Structure and Tone
- 3 new chapters in a "Getting Started" section before Language Reference: Installation, Hello World, CLI Reference
- 2 new chapters in an "Architecture" section: Compiler Pipeline, Crate Map
- Concise technical writing tone — direct instructions with code examples, minimal narrative, matching the spec's existing style
- Hello World program uses a simple `fn main()` with a `say` dialogue — demonstrates both basic function syntax and Writ's unique dialogue feature in ~5 lines
- Crate dependency diagram presented as an ASCII table showing crate name, purpose, and dependencies — renders cleanly in mdBook without external tools

### CLI Documentation Scope
- Every subcommand documented with all flags: `writ compile`, `writ run`, `writ build` each with flags, descriptions, and examples
- Architecture page uses prose + table/diagram only, no code snippets (code belongs in cargo doc)
- Minimal "Contributing" section at the end of architecture page pointing to crate map and test commands — not a separate page

### Claude's Discretion
- Exact wording and prose for each page
- CLI flag discovery from actual binary help output
- Crate relationship details from Cargo.toml inspection
- SUMMARY.md placement of new sections

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/src/SUMMARY.md` — existing navigation with 68 chapters
- `docs/src/introduction.md` — existing landing page
- `writ-cli/` — CLI binary with subcommand definitions
- `Cargo.toml` workspace — lists all 9 crates

### Established Patterns
- mdBook wrapper files in `docs/src/` with `{{#include}}` directives
- New content goes directly in `docs/src/` (not wrapper files for spec)

### Integration Points
- `docs/src/SUMMARY.md` — must add new sections
- `docs/book.toml` — no changes needed (existing config works)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard technical documentation approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
