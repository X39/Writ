---
phase: 89-language-reference-and-il-spec
plan: 01
subsystem: docs
tags: [mdbook, language-spec, il-spec, cross-references, documentation]

# Dependency graph
requires:
  - phase: 87-mdbook-scaffold
    provides: 68 wrapper files wired into SUMMARY.md with {{#include}} directives
  - phase: 88-writ-syntax-highlighting
    provides: Writ syntax highlighting via docs/theme/highlight.js
provides:
  - 5 cross-reference links between spec chapters using mdBook relative paths
  - Types->Structs cross-reference in 06_5_type_system.md
  - Entities->Components cross-reference in 16_15_entities.md
  - Dialogue->Concurrency cross-reference in 15_14_dialogue_blocks_dlg.md
  - Concurrency->IL Execution Model cross-reference in 22_21_concurrency.md
  - IL Calls->IL Execution Model cross-reference in 55_3_7_calls.md
  - Verified clean mdBook build (all 68 chapters, no errors)
affects: [90-getting-started-guide, 92-ci-deploy]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-references go in spec source files (language-spec/spec/), not wrapper files (docs/src/language-ref/ or docs/src/il-spec/)"
    - "Same-directory links use bare filename (structs.md); cross-directory links use relative path (../il-spec/execution.md)"
    - "Anchors for numbered section headings: lowercase, spaces-to-dashes, dots stripped (2.17.3 -> #2173)"

key-files:
  created: []
  modified:
    - language-spec/spec/06_5_type_system.md
    - language-spec/spec/16_15_entities.md
    - language-spec/spec/15_14_dialogue_blocks_dlg.md
    - language-spec/spec/22_21_concurrency.md
    - language-spec/spec/55_3_7_calls.md

key-decisions:
  - "Links written relative to wrapper file location (docs/src/language-ref/ or docs/src/il-spec/), not relative to spec source location (language-spec/spec/), because {{#include}} copies content verbatim and mdBook resolves relative links from the including file"
  - "Forward references only (no bidirectional linking) per locked decision from CONTEXT.md"
  - "Task 2 checkpoint auto-approved (auto_advance=true) — visual verification deferred to Phase 92 CI integration"

patterns-established:
  - "Pattern 2: mdBook Relative Cross-Reference — links in spec source files use paths as-if rendered from docs/src/[section]/"

requirements-completed: [LANG-01, LANG-03, IL-01, IL-02]

# Metrics
duration: 5min
completed: 2026-03-27
---

# Phase 89 Plan 01: Language Reference and IL Spec Summary

**5 mdBook cross-reference links added to spec source files; all 68 chapters verified browsable with clean build**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-27T06:20:00Z
- **Completed:** 2026-03-27T06:21:16Z
- **Tasks:** 2 (1 auto + 1 auto-approved checkpoint)
- **Files modified:** 5

## Accomplishments

- Added all 5 forward cross-references to language-spec/spec/ source files using mdBook relative links
- Verified clean mdBook build (exit 0, no errors, only expected benign admonish version warning)
- All 5 acceptance criteria passed: each grep returns count=1 for the respective cross-reference pattern
- Task 2 checkpoint auto-approved per auto_advance=true config

## Task Commits

1. **Task 1: Add 5 cross-references to spec source files** - `f89de57` (feat)

**Plan metadata:** (final commit — see below)

## Files Created/Modified

- `language-spec/spec/06_5_type_system.md` - Added "See [Structs](structs.md)" after §1.5.2 type inference example
- `language-spec/spec/16_15_entities.md` - Added "See [Components](components.md)" after §1.15.1 Guard entity example
- `language-spec/spec/15_14_dialogue_blocks_dlg.md` - Added "[Concurrency](concurrency.md)" after §1.14.9 suspension paragraph
- `language-spec/spec/22_21_concurrency.md` - Added "[IL Execution Model](../il-spec/execution.md)" after §1.21.1
- `language-spec/spec/55_3_7_calls.md` - Appended "[Execution Model](execution.md#2173-transition-points)" at end of file

## Decisions Made

- Links written relative to wrapper file location (docs/src/language-ref/ or docs/src/il-spec/), not relative to spec source location (language-spec/spec/), because `{{#include}}` copies content verbatim and mdBook resolves relative links from the including file's location — this matches Pitfall 3 from the research document.
- Forward references only (no bidirectional linking) per locked decision from CONTEXT.md.

## Deviations from Plan

None — plan executed exactly as written. All 5 cross-reference locations and link text matched the plan spec. mdBook built cleanly on first attempt.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 89 complete. All 4 requirements satisfied: LANG-01, LANG-03, IL-01, IL-02.
- Phase 90 (Getting Started Guide) and Phase 91 (Rust API docs) are both unblocked — they depend on Phase 87 (scaffold), which is complete.
- One standing blocker before Phase 92 can be fully validated: GitHub Pages source in repo Settings must be switched from "Deploy from a branch" to "GitHub Actions" (documented in STATE.md).

---
*Phase: 89-language-reference-and-il-spec*
*Completed: 2026-03-27*
