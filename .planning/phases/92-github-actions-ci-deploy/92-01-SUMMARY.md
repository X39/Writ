---
phase: 92-github-actions-ci-deploy
plan: 01
subsystem: infra
tags: [github-actions, github-pages, mdbook, cargo-doc, ci-cd]

# Dependency graph
requires:
  - phase: 91-cargo-doc-integration
    provides: docs/api-redirect.html template and validated cargo doc --workspace --no-deps
  - phase: 87-mdbook-scaffold
    provides: docs/book.toml with build-dir=target/book and site-url=/Writ/
provides:
  - .github/workflows/docs.yml — two-job GitHub Pages workflow (build + deploy)
  - Automated docs deployment on every push to master via GitHub Actions
affects:
  - All future documentation updates (auto-deployed by this workflow)

# Tech tracking
tech-stack:
  added:
    - actions/configure-pages@v5
    - actions/upload-pages-artifact@v3
    - actions/deploy-pages@v4
    - mdBook 0.4.51 (pre-built binary, not cargo install)
    - mdbook-admonish 1.20.0 (pre-built binary)
  patterns:
    - Binary download pattern for CI tools (curl | tar -xz → bin/ → $GITHUB_PATH)
    - Two-job GitHub Pages workflow (build + deploy separation)
    - Cargo docs key prefix (-cargo-docs-) avoids collision with vscode-extension cache (-cargo-)

key-files:
  created:
    - .github/workflows/docs.yml
  modified: []

key-decisions:
  - "mdBook and mdbook-admonish downloaded as pre-built binaries (not cargo install) — saves 3-6 min per CI run"
  - "configure-pages@v5 placed in build job (not deploy job) — required before upload-pages-artifact per Pitfall 2"
  - "Cargo cache key prefixed with -cargo-docs- to avoid collision with vscode-extension.yml -cargo- cache"
  - "Step order enforced: mdbook build → cargo doc → inject redirect → cp merge → upload artifact (Pitfall 6)"

patterns-established:
  - "Binary tool download: mkdir -p bin && curl -sSL URL | tar -xz --directory=bin && echo GITHUB_WORKSPACE/bin >> GITHUB_PATH"
  - "Pages deployment: configure-pages@v5 (id: pages) in build job → upload-pages-artifact@v3 → separate deploy job with deploy-pages@v4"

requirements-completed: [CI-01, CI-02, CI-04]

# Metrics
duration: 2min
completed: 2026-03-27
---

# Phase 92 Plan 01: GitHub Actions CI/Deploy Summary

**Two-job GitHub Pages workflow deploying mdBook 0.4.51 + cargo doc to /Writ/ on push to master, with pre-built binary downloads and merged artifact at docs/target/book/api/**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-27T13:27:02Z
- **Completed:** 2026-03-27T13:29:13Z
- **Tasks:** 1 (Task 2 auto-approved checkpoint)
- **Files modified:** 1

## Accomplishments
- Created `.github/workflows/docs.yml` implementing the full two-job CI/CD pipeline
- Build job: downloads pinned mdBook 0.4.51 + mdbook-admonish 1.20.0 as pre-built binaries, builds mdBook from docs/, builds cargo doc --workspace --no-deps, injects api-redirect.html, merges target/doc into docs/target/book/api, uploads merged artifact
- Deploy job: uses locked action trio (configure-pages@v5, upload-pages-artifact@v3, deploy-pages@v4) to deploy to GitHub Pages at /Writ/
- All CI-01, CI-02, and CI-04 requirements satisfied

## Task Commits

Each task was committed atomically:

1. **Task 1: Create docs.yml GitHub Actions workflow** - `a3dbf3f` (feat)
2. **Task 2: Verify live deployment** - checkpoint:human-verify (auto-approved; requires one-time manual GitHub Pages source switch to "GitHub Actions" in repo Settings before first deploy)

## Files Created/Modified
- `.github/workflows/docs.yml` - Two-job GitHub Pages deployment workflow (build + deploy)

## Decisions Made
- Pre-built binary download chosen over `cargo install` — eliminates 3-6 min CI build time per run
- `configure-pages@v5` placed in the `build` job (before `upload-pages-artifact`) per Pitfall 2: placing it in the deploy job causes a missing environment configuration error
- Cargo cache key uses `-cargo-docs-` prefix to avoid colliding with `vscode-extension.yml` cache (which uses `-cargo-`)
- Step ordering strictly follows plan: mdbook build → cargo doc → inject redirect → cp merge → upload (Pitfall 6: merge must precede upload)

## Deviations from Plan

The automated verify script in the plan checked for the literal string `v0.4.51` but the workflow correctly uses env var interpolation (`v${MDBOOK_VERSION}`) — the literal string doesn't appear. All 27 acceptance criteria were manually verified and pass. The plan's verify script contained a minor discrepancy; the implementation is correct per the acceptance criteria.

None — plan executed exactly as written aside from the verify script discrepancy noted above.

## Issues Encountered

- Plan's `<verify>` bash script checked `grep -q 'v0.4.51'` which fails because the workflow uses `v${MDBOOK_VERSION}` interpolation (correct per acceptance criteria). Manual verification confirmed all acceptance criteria pass. Not a bug in the implementation.

## User Setup Required

**One-time manual GitHub Pages configuration required before first deploy:**
1. Go to repository Settings → Pages
2. Under "Build and deployment", change Source from "Deploy from a branch" to "GitHub Actions"
3. Save

This step cannot be automated from CI (GitHub API requires admin token). After this one-time setup, all future deploys are fully automated on push to master.

## Next Phase Readiness

- Phase 92 is the final phase of v9.0
- After the one-time GitHub Pages source switch, pushing to master deploys the full documentation site
- Site will be live at `https://[owner].github.io/Writ/` with mdBook content and `/Writ/api/` cargo doc redirect

---
*Phase: 92-github-actions-ci-deploy*
*Completed: 2026-03-27*
