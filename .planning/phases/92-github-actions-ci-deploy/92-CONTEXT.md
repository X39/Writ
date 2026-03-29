# Phase 92: GitHub Actions CI/Deploy - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

Every push to master automatically builds the mdBook site and cargo doc output and deploys a merged artifact to GitHub Pages at the /Writ/ path.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key constraints from v9.0 roadmap decisions:
- mdBook 0.4.51 pinned — download as pre-built binary, not via cargo install
- site-url = "/Writ/" — asset paths must resolve at /Writ/ path on GitHub Pages
- Deploy job uses actions/configure-pages@v5, upload-pages-artifact@v3, deploy-pages@v4
- cargo doc redirect template at docs/api-redirect.html (from Phase 91) must be copied into target/doc/index.html during build
- .nojekyll file required in the merged artifact

</decisions>

<code_context>
## Existing Code Insights

Codebase context will be gathered during plan-phase research.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Refer to ROADMAP phase description and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
