---
phase: 87-mdbook-scaffold
plan: 01
subsystem: infra
tags: [mdbook, documentation, spec, markdown, github-pages]

requires: []
provides:
  - "docs/ directory with mdBook 0.4.51 project (book.toml, SUMMARY.md, 68 chapter wrapper files)"
  - "All 70 language-spec/spec/*.md files with shared document-level H1 stripped and headings promoted"
  - "docs/target/book/ HTML output with site-url=/Writ/ for GitHub Pages path routing"
affects: [88-syntax-highlighting, 89-getting-started, 90-arch-overview, 91-cargo-doc, 92-ci-deploy]

tech-stack:
  added: [mdbook 0.4.51]
  patterns:
    - "Wrapper file pattern: docs/src/language-ref/*.md and docs/src/il-spec/*.md each contain a single {{#include}} directive pointing to language-spec/spec/*.md"
    - "H1 promotion: shared document-level H1 stripped from all 70 spec files; former H2 becomes H1 for sidebar title display"
    - "build-dir = target/book keeps generated HTML inside gitignored target/ directory"

key-files:
  created:
    - docs/book.toml
    - docs/src/SUMMARY.md
    - docs/src/introduction.md
    - docs/src/language-ref/ (29 wrapper files)
    - docs/src/il-spec/ (39 wrapper files)
    - strip_h1.py
  modified:
    - language-spec/spec/00_preamble.md through 69_b_il_decision_log.md (70 files, H1 stripped + headings promoted)

key-decisions:
  - "Wrapper files contain only the {{#include}} directive with no H1 of their own; the spec file's promoted H1 serves as the chapter heading"
  - "build-dir = target/book (not the default book/) to keep output inside already-gitignored target/ directory"
  - "site-url = /Writ/ set before first build to ensure correct GitHub Pages asset paths"
  - "00_preamble.md and 01_table_of_contents.md have no wrapper files; preamble content merged into introduction.md, TOC excluded (broken anchor links)"

patterns-established:
  - "Wrapper files are single-line {{#include}} directives — never copy spec content into docs/src/"
  - "spec files in language-spec/spec/ are the single source of truth; docs/ wrapper files just include them"

requirements-completed: [INFRA-01, INFRA-02, LANG-02]

duration: 65min
completed: 2026-03-27
---

# Phase 87 Plan 01: mdBook Scaffold Summary

**mdBook 0.4.51 scaffold with 68 chapter wrapper files, site-url=/Writ/ config, and all 70 spec files H1-stripped and heading-promoted so every chapter shows its own title in the sidebar**

## Performance

- **Duration:** ~65 min (including mdBook compile time ~62s)
- **Started:** 2026-03-27T03:47:00Z
- **Completed:** 2026-03-27T03:52:28Z
- **Tasks:** 1
- **Files modified:** 142 (70 spec files + 69 new docs/ files + strip_h1.py)

## Accomplishments

- Installed mdBook 0.4.51 via `cargo install`
- Stripped shared document-level H1 from all 70 spec files (`language-spec/spec/*.md`) and promoted all heading levels by one (H2 -> H1, H3 -> H2, etc.) using `strip_h1.py`
- Created `docs/book.toml` with `site-url = "/Writ/"` and `build-dir = "target/book"` for GitHub Pages path routing and gitignored output
- Created `docs/src/SUMMARY.md` with 68 chapters (29 Language Reference + 39 IL Specification)
- Created `docs/src/introduction.md` as the landing page
- Created 29 `docs/src/language-ref/*.md` wrapper files and 39 `docs/src/il-spec/*.md` wrapper files, each containing a single `{{#include}}` directive
- `mdbook build docs/` exits 0 and produces `docs/target/book/index.html`

## Task Commits

1. **Task 1: Install mdBook, strip H1s, and create docs/ scaffold** - `7976533` (feat)

**Plan metadata:** (this SUMMARY.md is part of the final metadata commit)

## Files Created/Modified

- `docs/book.toml` - mdBook configuration: title, site-url=/Writ/, build-dir=target/book, git-repository-url, search enabled
- `docs/src/SUMMARY.md` - 68-chapter master chapter list with Introduction prefix chapter, Language Reference and IL Specification parts
- `docs/src/introduction.md` - Landing page merging preamble content (Draft v0.5, file extension, section overview)
- `docs/src/language-ref/*.md` (29 files) - Wrapper files with `{{#include ../../../language-spec/spec/NN_*.md}}`
- `docs/src/il-spec/*.md` (39 files) - Wrapper files with `{{#include ../../../language-spec/spec/NN_*.md}}`
- `strip_h1.py` - Python script to strip shared H1 and promote headings in spec files
- `language-spec/spec/*.md` (70 files) - H1 stripped, all headings promoted one level

## Decisions Made

- Wrapper files have no H1 of their own: the spec file's promoted H1 is the chapter heading, and SUMMARY.md provides the sidebar title
- `00_preamble.md` excluded from wrapper files (content merged into `introduction.md`)
- `01_table_of_contents.md` excluded from wrapper files (contains broken anchor links)
- `strip_h1.py` kept in repo root as a one-time utility script; tracked in git for reproducibility
- `docs/target/book/` is gitignored via the existing `target/` rule in `.gitignore`

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `grep -l "^# Writ IL Specification" language-spec/spec/*.md` returned 2 false-positive files after stripping — `30_29_lowering_reference.md` and `67_4_2_opcode_assignment_table.md` contain these patterns as embedded section headings within the file body (not as first-line document-level H1). The first-line check confirmed 0 files start with the shared H1 pattern. No action required.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `docs/` scaffold is fully built; `mdbook build docs/` exits 0
- Phase 88 (syntax highlighting) can now add mdbook-admonish and Writ syntax highlighting
- Phases 89 (getting started) and 90 (architecture overview) can add content chapters to the already-wired SUMMARY.md
- Phase 92 (CI/deploy) can reference `docs/target/book/` as the artifact output directory

## Self-Check: PASSED

- FOUND: docs/book.toml
- FOUND: docs/src/SUMMARY.md
- FOUND: docs/src/introduction.md
- FOUND: docs/src/language-ref/overview.md
- FOUND: docs/src/il-spec/vm.md
- FOUND: docs/target/book/index.html
- FOUND commit 7976533 (feat(87-01): create mdBook scaffold with H1-stripped spec files)

---
*Phase: 87-mdbook-scaffold*
*Completed: 2026-03-27*
