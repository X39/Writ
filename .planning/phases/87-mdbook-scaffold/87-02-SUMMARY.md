---
phase: 87-mdbook-scaffold
plan: 02
subsystem: infra
tags: [mdbook, mdbook-admonish, documentation, callout-boxes, github-pages]

requires:
  - phase: 87-01
    provides: "docs/ scaffold with book.toml and 68 chapter wrapper files, mdBook 0.4.51 installed"

provides:
  - "mdbook-admonish 1.20.0 installed and configured in docs/book.toml"
  - "docs/mdbook-admonish.css (356 lines) with Material Design callout styling"
  - "Three admonish callout test blocks (note/warning/tip) added to introduction.md"
  - "mdbook build passes with admonish preprocessor and callout HTML rendered in index.html"

affects: [88-syntax-highlighting, 89-getting-started, 90-arch-overview, 92-ci-deploy]

tech-stack:
  added: [mdbook-admonish 1.20.0]
  patterns:
    - "Admonish callout syntax: fenced code block with 'admonish note/warning/tip' info string"
    - "assets_version field in [preprocessor.admonish] managed by 'mdbook-admonish install' — do not hand-edit"

key-files:
  created:
    - docs/mdbook-admonish.css
  modified:
    - docs/book.toml
    - docs/src/introduction.md

key-decisions:
  - "additional-css path in book.toml must be 'mdbook-admonish.css' (relative to docs/), not 'docs/mdbook-admonish.css' — mdbook-admonish install writes the wrong path when run from repo root"
  - "Version warning 'preprocessor was built against version 0.4.52 but called from 0.4.51' is non-fatal — build succeeds; cargo resolved 0.4.52 as the admonish dep but our installed mdbook is 0.4.51"

patterns-established:
  - "Run 'mdbook-admonish install docs/' from repo root (not from docs/) to inject CSS; fix additional-css path afterward"

requirements-completed: [INFRA-03]

duration: 5min
completed: 2026-03-27
---

# Phase 87 Plan 02: mdBook Admonish Summary

**mdbook-admonish 1.20.0 installed and configured with note/warning/tip callout boxes rendering in the introduction page; mdbook build exits 0**

## Performance

- **Duration:** ~5 min (installation was 45s compile time)
- **Started:** 2026-03-27T03:56:00Z
- **Completed:** 2026-03-27T03:57:30Z
- **Tasks:** 1 auto + 1 checkpoint (auto-approved)
- **Files modified:** 3 (book.toml, introduction.md, mdbook-admonish.css)

## Accomplishments

- Installed mdbook-admonish 1.20.0 via `cargo install`
- Ran `mdbook-admonish install docs/` to inject `docs/mdbook-admonish.css` (356 lines) and `[preprocessor.admonish]` block into book.toml
- Fixed auto-written `additional-css` path from `"docs/mdbook-admonish.css"` to `"mdbook-admonish.css"` (relative to docs/)
- Appended three admonish test callout blocks (note/warning/tip) to `docs/src/introduction.md`
- `mdbook build docs/` exits 0; `docs/target/book/index.html` contains rendered admonish HTML

## Task Commits

1. **Task 1: Install mdbook-admonish and configure book.toml** - `6ed8bed` (feat)
2. **Task 2: Visual verification of rendered site** - auto-approved (auto_advance = true)

**Plan metadata:** (this SUMMARY.md is part of the final metadata commit)

## Files Created/Modified

- `docs/mdbook-admonish.css` - 356-line Material Design callout stylesheet injected by mdbook-admonish install
- `docs/book.toml` - Added `[preprocessor.admonish]` with `command = "mdbook-admonish"` and `assets_version = "3.1.0"`; `additional-css = ["mdbook-admonish.css"]` in `[output.html]`
- `docs/src/introduction.md` - Three admonish callout blocks appended (note, warning, tip)

## Decisions Made

- `additional-css` must be `"mdbook-admonish.css"` not `"docs/mdbook-admonish.css"`: when `mdbook-admonish install` runs from repo root targeting `docs/`, it prepends `docs/` to the path, which is wrong — mdBook resolves `additional-css` relative to the book root (docs/), not the repo root.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed incorrect additional-css path in book.toml**
- **Found during:** Task 1 (install mdbook-admonish and configure book.toml)
- **Issue:** `mdbook-admonish install docs/` set `additional-css = ["docs/mdbook-admonish.css"]` when run from repo root; the CSS file is at `docs/mdbook-admonish.css` on disk but mdBook resolves `additional-css` relative to the book directory (`docs/`), so the correct value is `"mdbook-admonish.css"`
- **Fix:** Changed `additional-css = ["docs/mdbook-admonish.css"]` to `additional-css = ["mdbook-admonish.css"]` in docs/book.toml
- **Files modified:** docs/book.toml
- **Verification:** `mdbook build docs/` exits 0 with admonish content in generated HTML
- **Committed in:** `6ed8bed` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary correction for admonish CSS to load. No scope creep.

## Issues Encountered

- `mdbook-admonish install` was compiled against mdBook 0.4.52 (what cargo resolved as the admonish binary's own dep) but our installed mdBook CLI is 0.4.51. This produces a non-fatal warning: "preprocessor was built against version 0.4.52 of mdbook, but we're being called from version 0.4.51". The build completes successfully; the warning is expected and ignorable for minor patch mismatches.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `mdbook build docs/` is green with admonish callouts rendered
- Phase 87 is complete: scaffold (87-01) + admonish (87-02) are both done
- Phase 88 (syntax highlighting) can proceed — admonish is in place
- Phases 89 and 90 (content) can use admonish callouts in new chapters

## Self-Check: PASSED

- FOUND: docs/book.toml (contains [preprocessor.admonish])
- FOUND: docs/mdbook-admonish.css (356 lines)
- FOUND: docs/src/introduction.md (contains admonish note/warning/tip)
- FOUND: docs/target/book/index.html (contains admonish)
- FOUND commit 6ed8bed (feat(87-02): install mdbook-admonish 1.20.0 and configure callout boxes)

---
*Phase: 87-mdbook-scaffold*
*Completed: 2026-03-27*
