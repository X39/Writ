---
phase: 91-cargo-doc-integration
plan: 01
subsystem: infra
tags: [cargo-doc, rustdoc, workspace-lints, github-pages, redirect]

# Dependency graph
requires:
  - phase: 90-getting-started-architecture
    provides: docs/ directory structure exists
provides:
  - Workspace-wide rustdoc lint suppression via [workspace.lints.rustdoc]
  - All 10 member crates inherit lint config via [lints] workspace = true
  - docs/api-redirect.html redirect template pointing to writ_compiler/index.html
  - Validated cargo doc --workspace --no-deps build with zero warnings
affects: [92-ci-deploy]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "[workspace.lints.rustdoc] in virtual manifest + [lints] workspace = true in members for uniform rustdoc lint suppression"
    - "Meta http-equiv=refresh redirect template committed to docs/ for Phase 92 CI injection into target/doc/index.html"

key-files:
  created:
    - docs/api-redirect.html
  modified:
    - Cargo.toml
    - writ-assembler/Cargo.toml
    - writ-cli/Cargo.toml
    - writ-compiler/Cargo.toml
    - writ-dap/Cargo.toml
    - writ-diagnostics/Cargo.toml
    - writ-golden/Cargo.toml
    - writ-lsp/Cargo.toml
    - writ-module/Cargo.toml
    - writ-parser/Cargo.toml
    - writ-runtime/Cargo.toml

key-decisions:
  - "Workspace lint table [workspace.lints.rustdoc] with 7 allow rules handles all warn-by-default rustdoc lints in one place; no per-file #![allow(...)] needed"
  - "docs/api-redirect.html committed to git as a template; Phase 92 CI copies it to target/doc/index.html at deploy time (target/ is gitignored)"
  - "Redirect target is writ_compiler (underscores) not writ-compiler (hyphens) — cargo doc converts package name hyphens to underscores in output directories"

patterns-established:
  - "Workspace lint inheritance: root [workspace.lints.rustdoc] + member [lints] workspace = true"
  - "cargo doc root redirect: store template in docs/, inject into target/doc/ in CI"

requirements-completed: [CI-03]

# Metrics
duration: 15min
completed: 2026-03-27
---

# Phase 91 Plan 01: cargo doc Integration Summary

**Workspace rustdoc lint suppression via [workspace.lints.rustdoc] across 10 crates, validated zero-warning cargo doc build, and meta-refresh redirect template at docs/api-redirect.html pointing to writ_compiler/index.html for Phase 92 CI injection**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-27T00:00:00Z
- **Completed:** 2026-03-27
- **Tasks:** 2
- **Files modified:** 12 (11 Cargo.toml files + 1 new HTML file)

## Accomplishments

- Added `[workspace.lints.rustdoc]` to root Cargo.toml with 7 "allow" rules covering all warn-by-default rustdoc lints (broken_intra_doc_links, private_intra_doc_links, invalid_html_tags, bare_urls, redundant_explicit_links, invalid_codeblock_attributes, invalid_rust_codeblocks)
- Added `[lints] workspace = true` to all 10 member crate Cargo.toml files for uniform lint inheritance
- Ran `cargo doc --workspace --no-deps` — exits 0 with zero warning lines; CI-safe with `-D warnings`
- Created `docs/api-redirect.html` with `meta http-equiv="refresh"` redirect to `writ_compiler/index.html` for Phase 92 CI to inject at `target/doc/index.html`

## Task Commits

Each task was committed atomically:

1. **Task 1: Configure workspace rustdoc lints and member crate inheritance** - `35c8b90` (chore)
2. **Task 2: Validate cargo doc build and create redirect template** - `0008fbb` (feat)

## Files Created/Modified

- `Cargo.toml` - Added [workspace.lints.rustdoc] block with 7 allow rules
- `writ-assembler/Cargo.toml` - Added [lints] workspace = true
- `writ-cli/Cargo.toml` - Added [lints] workspace = true
- `writ-compiler/Cargo.toml` - Added [lints] workspace = true
- `writ-dap/Cargo.toml` - Added [lints] workspace = true
- `writ-diagnostics/Cargo.toml` - Added [lints] workspace = true
- `writ-golden/Cargo.toml` - Added [lints] workspace = true
- `writ-lsp/Cargo.toml` - Added [lints] workspace = true
- `writ-module/Cargo.toml` - Added [lints] workspace = true
- `writ-parser/Cargo.toml` - Added [lints] workspace = true
- `writ-runtime/Cargo.toml` - Added [lints] workspace = true
- `docs/api-redirect.html` - Meta-refresh redirect template to writ_compiler/index.html

## Decisions Made

- Redirect target is `writ_compiler` (with underscores) — cargo doc converts package name hyphens to underscores in `target/doc/` directory names. Redirecting to `writ-compiler` would 404.
- Template committed to `docs/api-redirect.html` rather than injected into `target/doc/index.html` because `target/` is gitignored. Phase 92 CI copies the template at deploy time.
- All crates suppressed uniformly via workspace inheritance; per-crate overrides deferred until docs are polished (post-1.0).

## Deviations from Plan

None - plan executed exactly as written. `cargo doc` produced zero warnings on first run after adding the workspace lint config; no additional lint entries needed.

## Issues Encountered

None. The workspace lint configuration worked on the first attempt. cargo doc produced zero warnings immediately after the lint config was applied.

## Known Stubs

None — this plan does not render data to any UI.

## Next Phase Readiness

- Phase 92 (CI/deploy) can now run `cargo doc --workspace --no-deps` in CI with `RUSTDOCFLAGS="-D warnings"` without failures
- `docs/api-redirect.html` is the inject target for Phase 92 CI to write to `target/doc/index.html`
- `target/doc/.nojekyll` must also be created by Phase 92 CI to prevent GitHub Pages Jekyll from breaking underscore paths

---
*Phase: 91-cargo-doc-integration*
*Completed: 2026-03-27*
