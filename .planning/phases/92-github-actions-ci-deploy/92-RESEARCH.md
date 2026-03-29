# Phase 92: GitHub Actions CI/Deploy - Research

**Researched:** 2026-03-27
**Domain:** GitHub Actions, GitHub Pages, mdBook CI, cargo doc deployment
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- mdBook 0.4.51 pinned — download as pre-built binary, not via cargo install
- site-url = "/Writ/" — asset paths must resolve at /Writ/ path on GitHub Pages
- Deploy job uses actions/configure-pages@v5, upload-pages-artifact@v3, deploy-pages@v4
- cargo doc redirect template at docs/api-redirect.html (from Phase 91) must be copied into target/doc/index.html during build
- .nojekyll file required in the merged artifact

### Claude's Discretion
All other implementation choices — all implementation choices are at Claude's discretion for this pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CI-01 | GitHub Actions workflow builds mdBook and cargo doc on push to master | GitHub Actions starter-workflows/pages/mdbook.yml pattern; two-job structure with build + deploy |
| CI-02 | Built site deployed to gh-pages via official actions (configure-pages, upload-pages-artifact, deploy-pages) | Canonical permissions + environment pattern verified; action versions locked in CONTEXT.md |
| CI-04 | mdBook version pinned to 0.4.51 in CI to avoid preprocessor compatibility issues | Binary download URL confirmed; mdbook-admonish 1.20.0 binary also available |
</phase_requirements>

---

## Summary

Phase 92 creates `.github/workflows/docs.yml` — a GitHub Actions workflow that builds the mdBook site and cargo doc output on every push to master, merges them into a single directory artifact, and deploys to GitHub Pages at `/Writ/`.

The canonical pattern for GitHub Pages via Actions uses a two-job workflow: a `build` job that produces the site artifact and a `deploy` job that uses `actions/deploy-pages`. Permissions (`pages: write`, `id-token: write`), the `github-pages` environment, and concurrency group `"pages"` with `cancel-in-progress: false` are mandatory structural requirements from GitHub.

The key constraint is that both mdBook and mdbook-admonish must be downloaded as pre-built binaries — not built via `cargo install` — to keep the build job under 5 minutes. Both tools have x86_64-linux binaries available at their v0.4.51 and v1.20.0 release pages respectively. The cargo doc build uses `dtolnay/rust-toolchain@stable` (matching existing project workflows) with `actions/cache@v4` for the cargo registry and target directory.

**Primary recommendation:** Create a two-job `docs.yml` workflow. The `build` job downloads mdBook and mdbook-admonish binaries, builds the book from `docs/`, runs `cargo doc --workspace --no-deps`, merges `target/doc` into `docs/target/book/api/`, injects the redirect, then uploads the merged artifact. The `deploy` job uses the locked action trio from CONTEXT.md.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| actions/checkout | v4 | Repository checkout | Project standard (used in all 3 existing workflows) |
| dtolnay/rust-toolchain | stable | Install Rust for cargo doc | Project standard (used in vscode-extension.yml) |
| actions/cache | v4 | Cache cargo registry and target | Project standard (used in vscode-extension.yml) |
| actions/configure-pages | v5 | Configure GitHub Pages for Actions deployment | Locked in CONTEXT.md; current at v6 but v5 required |
| actions/upload-pages-artifact | v3 | Package and upload site artifact | Locked in CONTEXT.md; current at v4 but v3 required |
| actions/deploy-pages | v4 | Deploy artifact to GitHub Pages | Locked in CONTEXT.md; current at v5 but v4 required |
| mdBook | 0.4.51 | Build documentation site | Pinned — 0.5.x breaks mdbook-admonish (issue #233) |
| mdbook-admonish | 1.20.0 | Admonish preprocessor | Pinned to match project's book.toml |

### Binary Download URLs (verified)
| Tool | URL Pattern |
|------|-------------|
| mdBook 0.4.51 | `https://github.com/rust-lang/mdBook/releases/download/v0.4.51/mdbook-v0.4.51-x86_64-unknown-linux-gnu.tar.gz` |
| mdbook-admonish 1.20.0 | `https://github.com/tommilligan/mdbook-admonish/releases/download/v1.20.0/mdbook-admonish-v1.20.0-x86_64-unknown-linux-gnu.tar.gz` |

**Note on action version currency:** As of 2026-03-27 the current stable releases are configure-pages@v6, upload-pages-artifact@v4, deploy-pages@v5. CONTEXT.md locks v5/v3/v4 respectively — use those exact versions.

**Installation (binary download pattern):**
```bash
curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.51/mdbook-v0.4.51-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=bin
curl -sSL https://github.com/tommilligan/mdbook-admonish/releases/download/v1.20.0/mdbook-admonish-v1.20.0-x86_64-unknown-linux-gnu.tar.gz | tar -xz --directory=bin
```

**Version verification:** Confirmed against GitHub releases pages 2026-03-27.

---

## Architecture Patterns

### Recommended Workflow Structure
```
.github/workflows/
└── docs.yml          # New file — docs build + deploy
.github/workflows/
├── rust.yml          # Existing — cargo test
├── vscode-extension.yml  # Existing — VS Code extension
└── benchmark.yml     # Existing — benchmarks
```

### Pattern 1: Two-Job GitHub Pages Workflow
**What:** `build` job produces artifact; `deploy` job consumes it. Jobs are separate because deploy requires `environment: github-pages` which cannot run in the same job as artifact upload without the specific action sequence.
**When to use:** Always for GitHub Pages via Actions — GitHub's official pattern.
**Example:**
```yaml
# Source: https://github.com/actions/starter-workflows/blob/main/pages/mdbook.yml
name: Deploy docs to GitHub Pages

on:
  push:
    branches: [master]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # ... build steps ...
      - uses: actions/upload-pages-artifact@v3
        with:
          path: docs/target/book

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

### Pattern 2: Merge mdBook + cargo doc Into Single Directory
**What:** mdBook builds to `docs/target/book/`. cargo doc builds to `target/doc/`. CI copies `target/doc/` into `docs/target/book/api/`, then uploads `docs/target/book/` as the single Pages artifact.
**When to use:** When mdBook is the root site and rustdoc is served at a subpath.
**Example:**
```bash
# 1. Build mdBook (outputs to docs/target/book/)
cd docs && bin/mdbook build && cd ..

# 2. Build cargo doc
cargo doc --workspace --no-deps

# 3. Inject redirect (api-redirect.html -> target/doc/index.html)
cp docs/api-redirect.html target/doc/index.html

# 4. Merge: copy rustdoc output into mdBook output at /api/ subpath
cp -r target/doc docs/target/book/api
```

### Pattern 3: Binary Install for Fast CI
**What:** Download pre-built binaries for mdBook and mdbook-admonish instead of `cargo install` (which compiles from source and takes 2-5 minutes per tool).
**When to use:** Any CI that pins tool versions — which should be all CI.
**Example:**
```yaml
- name: Install tools
  run: |
    mkdir -p bin
    curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.4.51/mdbook-v0.4.51-x86_64-unknown-linux-gnu.tar.gz \
      | tar -xz --directory=bin
    curl -sSL https://github.com/tommilligan/mdbook-admonish/releases/download/v1.20.0/mdbook-admonish-v1.20.0-x86_64-unknown-linux-gnu.tar.gz \
      | tar -xz --directory=bin
    echo "$GITHUB_WORKSPACE/bin" >> $GITHUB_PATH
```

### Anti-Patterns to Avoid
- **cargo install for CI tools:** `cargo install mdbook mdbook-admonish` compiles from source — adds 3-6 minutes to build time and may pull a different version than pinned.
- **Single-job build+deploy:** Combining everything into one job breaks the GitHub Pages deployment contract. Deploy must use `actions/deploy-pages` in its own job with `environment: github-pages`.
- **Omitting `cancel-in-progress: false`:** Cancelling a Pages deployment mid-flight can leave the site in a broken state.
- **Missing `id: pages` on configure-pages step:** The `steps.pages.outputs.base_url` output is not critical for this project (mdBook handles base URL via book.toml), but the `id` is needed if you want to reference the output.
- **Uploading cargo doc directly without merging:** If you upload `target/doc/` as a separate artifact, there's no way to serve it from a subpath of the mdBook site with `upload-pages-artifact` (only one Pages artifact per workflow run).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Serving rustdoc at /api/ subpath | Custom redirect server, gh-pages branch manipulation | Directory merge in CI (`cp -r target/doc docs/target/book/api`) | Simple shell copy; no extra tooling needed |
| Symlink resolution in cargo doc output | Custom tar with dereference flags | `upload-pages-artifact@v3` already uses `--dereference --hard-dereference` | Built into the action |
| Pages deployment token auth | Custom GITHUB_TOKEN auth | `id-token: write` permission + `actions/deploy-pages` | GitHub OIDC handles this automatically |
| Root index.html for cargo doc | Automated cargo doc wrapper | `cp docs/api-redirect.html target/doc/index.html` before merge | Known cargo issue #1016; simple copy suffices |

---

## Common Pitfalls

### Pitfall 1: mdbook build working directory
**What goes wrong:** Running `mdbook build` from repo root instead of from `docs/` causes mdBook to look for `book.toml` in the wrong place — the build fails or uses wrong paths.
**Why it happens:** mdBook resolves `src`, `build-dir`, and preprocessor config relative to `book.toml` location. The `docs/book.toml` has `build-dir = "target/book"` which is `docs/target/book` only when run from inside `docs/`.
**How to avoid:** Either `cd docs && mdbook build` or `mdbook build --dest-dir` explicitly. Use `cd docs && bin/mdbook build` consistently.
**Warning signs:** `mdbook: error: error opening "src/SUMMARY.md"` or output landing in repo root `book/`.

### Pitfall 2: configure-pages step must be in the build job
**What goes wrong:** Placing `actions/configure-pages@v5` in the `deploy` job — not the `build` job — causes a missing environment configuration error at upload time.
**Why it happens:** `configure-pages` sets up the Pages environment context needed before `upload-pages-artifact` runs. It must precede the upload step in the same job.
**How to avoid:** Always put `configure-pages` → `build` → `upload-pages-artifact` in sequence within the `build` job.
**Warning signs:** `Error: The artifact was not found` from `deploy-pages`.

### Pitfall 3: cargo doc produces no root index.html
**What goes wrong:** Deploying `target/doc/` directly results in a 404 at `/Writ/api/` because cargo never generates `target/doc/index.html` for workspaces.
**Why it happens:** Known cargo issue #1016 — cargo doc for workspaces only generates per-crate index pages, not a workspace root index.
**How to avoid:** Always run `cp docs/api-redirect.html target/doc/index.html` before merging. This is the purpose of the Phase 91 template file.
**Warning signs:** `/Writ/api/` returns 404 instead of redirecting.

### Pitfall 4: .nojekyll does not need to be created manually
**What goes wrong:** Developers add an explicit `touch .nojekyll` step targeting the artifact directory, creating duplicate or misplaced files.
**Why it happens:** Documentation and older guides recommend creating `.nojekyll` manually. With `upload-pages-artifact@v3`, the tar command archives `.` (all files including dotfiles) with only `.git` and `.github` excluded — so mdBook's generated `.nojekyll` at `docs/target/book/.nojekyll` is automatically included.
**How to avoid:** Do not add a manual `touch .nojekyll` step. mdBook 0.4.x generates it automatically in its output directory.
**Warning signs:** If `.nojekyll` is missing from the deployed site, verify mdBook version (some very old versions didn't generate it).

### Pitfall 5: GitHub Pages source must be set to "GitHub Actions"
**What goes wrong:** Workflow runs successfully but site never deploys — deploy-pages returns `HttpError: Not Found` or "Branch 'gh-pages' not found".
**Why it happens:** GitHub Pages defaults to "Deploy from a branch". The `actions/deploy-pages` action requires the source to be manually switched to "GitHub Actions" in Repository Settings → Pages.
**How to avoid:** This is a one-time manual step that cannot be automated from CI. It must be done before the first workflow run is expected to deploy.
**Warning signs:** `deploy-pages` step fails with deployment API error even though the artifact uploaded successfully.

### Pitfall 6: Path in upload-pages-artifact must be the merged root
**What goes wrong:** Uploading `docs/target/book` before copying `target/doc` into it means the `/api/` subpath is absent from the deployed site.
**Why it happens:** Step ordering — the cp merge step must complete before the upload step runs.
**How to avoid:** Order strictly: (1) build mdBook, (2) build cargo doc, (3) inject redirect, (4) cp merge, (5) upload-pages-artifact.
**Warning signs:** `/Writ/api/` resolves to 404 on the live site.

---

## Code Examples

### Complete docs.yml Workflow Skeleton
```yaml
# Source: GitHub starter-workflows/pages/mdbook.yml + project requirements
name: Deploy docs to GitHub Pages

on:
  push:
    branches: [master]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    env:
      MDBOOK_VERSION: "0.4.51"
      MDBOOK_ADMONISH_VERSION: "1.20.0"
    steps:
      - uses: actions/checkout@v4

      - name: Install mdBook and mdbook-admonish
        run: |
          mkdir -p bin
          curl -sSL "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/mdbook-v${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
            | tar -xz --directory=bin
          curl -sSL "https://github.com/tommilligan/mdbook-admonish/releases/download/v${MDBOOK_ADMONISH_VERSION}/mdbook-admonish-v${MDBOOK_ADMONISH_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
            | tar -xz --directory=bin
          echo "$GITHUB_WORKSPACE/bin" >> $GITHUB_PATH

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-docs-${{ hashFiles('**/Cargo.lock') }}

      - name: Setup Pages
        id: pages
        uses: actions/configure-pages@v5

      - name: Build mdBook
        run: cd docs && mdbook build

      - name: Build cargo doc
        run: cargo doc --workspace --no-deps

      - name: Inject API redirect
        run: cp docs/api-redirect.html target/doc/index.html

      - name: Merge cargo doc into book output
        run: cp -r target/doc docs/target/book/api

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: docs/target/book

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

### Verifying the Site Structure Before Upload
```bash
# After merge step — expected directory layout:
# docs/target/book/
# ├── .nojekyll          (auto-generated by mdBook 0.4.51)
# ├── index.html         (mdBook home page)
# ├── api/               (merged cargo doc)
# │   ├── index.html     (redirect to writ_compiler/index.html)
# │   ├── writ_compiler/
# │   ├── writ_parser/
# │   └── ...
# ├── language-ref/
# ├── il-spec/
# └── ...
ls docs/target/book/
ls docs/target/book/api/
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Deploy from `gh-pages` branch | Deploy via Actions artifact | 2022 | Cleaner, no branch needed |
| `cargo install mdbook` in CI | Download pre-built binary | Always best practice | 3-5 min CI savings |
| `peaceiris/actions-gh-pages` | `actions/deploy-pages` (official) | 2022 | First-party action, more reliable |
| `actions-rs/toolchain` | `dtolnay/rust-toolchain` | 2023 (actions-rs deprecated Oct 2023) | dtolnay is the current community standard |

**Deprecated/outdated:**
- `actions-rs/toolchain`: deprecated October 2023 — project already uses `dtolnay/rust-toolchain` (confirmed in vscode-extension.yml)
- `actions/upload-artifact@v3`: deprecated January 2025 — project uses v4 correctly

---

## Open Questions

1. **mdbook-admonish preprocessor path in CI**
   - What we know: `mdbook-admonish` binary must be on `$PATH` when `mdbook build` runs; `book.toml` specifies `command = "mdbook-admonish"` in `[preprocessor.admonish]`
   - What's unclear: Whether the `bin/` directory approach is sufficient or if a specific working directory matters
   - Recommendation: Add `echo "$GITHUB_WORKSPACE/bin" >> $GITHUB_PATH` before the `cd docs && mdbook build` step; this is the standard PATH injection pattern for GitHub Actions

2. **Cargo doc build time on ubuntu-latest**
   - What we know: ubuntu-latest has 2 cores and 7 GB RAM; workspace has 10 crates; `cargo doc` compiles only doc targets (faster than full build)
   - What's unclear: First-run build time without cache warming
   - Recommendation: Include `actions/cache@v4` for cargo registry and target — after first run, incremental builds should stay well under 5 minutes

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| GitHub Actions runner (ubuntu-latest) | All steps | Yes (SaaS) | Ubuntu 22.04+ | — |
| mdBook 0.4.51 binary | Build mdBook | Downloaded in CI | 0.4.51 | — |
| mdbook-admonish 1.20.0 binary | mdbook preprocessor | Downloaded in CI | 1.20.0 | — |
| Rust stable (dtolnay/rust-toolchain) | cargo doc | Installed in CI | current stable | — |
| GitHub Pages (Actions source) | deploy-pages | Manual setup required | — | None — one-time manual step |

**Missing dependencies with no fallback:**
- GitHub Pages source must be manually switched to "GitHub Actions" in Repository Settings → Pages before the first deploy will succeed. This is a one-time human action that cannot be automated.

**Missing dependencies with fallback:**
- None.

---

## Validation Architecture

Nyquist validation is not applicable to this phase. The phase produces a YAML workflow file and the acceptance criteria are verified by the deployed site resolving correctly — this requires a live GitHub Actions run, not a local test suite.

Manual verification checklist (post-deploy):
- [ ] `docs.yml` workflow appears in GitHub Actions tab and runs on push to master
- [ ] Build job completes in under 5 minutes
- [ ] `https://[owner].github.io/Writ/` loads the home page with correct CSS and navigation
- [ ] `https://[owner].github.io/Writ/api/` redirects to `writ_compiler/index.html` (not 404)

---

## Sources

### Primary (HIGH confidence)
- `https://github.com/actions/starter-workflows/blob/main/pages/mdbook.yml` — canonical two-job mdBook deploy workflow structure
- `https://raw.githubusercontent.com/actions/upload-pages-artifact/v3/action.yml` — verified tar uses `--dereference --hard-dereference`, archives `.` (including dotfiles), only excludes `.git` and `.github`
- `https://github.com/rust-lang/mdBook/releases/expanded_assets/v0.4.51` — confirmed binary URL for mdBook 0.4.51 Linux x86_64
- `https://github.com/tommilligan/mdbook-admonish/releases/expanded_assets/v1.20.0` — confirmed binary URL for mdbook-admonish 1.20.0 Linux x86_64
- `D:/dev/git/Writ/.github/workflows/vscode-extension.yml` — project uses `dtolnay/rust-toolchain@stable` and `actions/cache@v4` pattern
- `D:/dev/git/Writ/docs/target/book/` — verified mdBook 0.4.51 generates `.nojekyll` automatically in output

### Secondary (MEDIUM confidence)
- `https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages` — confirms required permissions: `contents: read`, `pages: write`, `id-token: write`; confirms `environment: github-pages` and `needs: build` pattern
- `https://github.com/rust-lang/mdBook/wiki/Automated-Deployment:-GitHub-Actions` — confirms binary download approach; wiki recommends `--directory=./mdbook` flag pattern
- `https://rust-lang.github.io/mdBook/continuous-integration.html` — official mdBook CI docs confirm binary download is faster and does not require Rust install
- `https://github.com/actions/deploy-pages/releases` — confirms v4.0.5 is latest stable in v4 series (v5 released March 2026); v4 is locked per CONTEXT.md

### Tertiary (LOW confidence)
- `https://github.com/orgs/community/discussions/72823` — community discussion confirms `.lock` file and permission handling for cargo doc + Pages; exact solution referenced in issue #303 not fully retrieved

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — action versions locked by CONTEXT.md; binary URLs confirmed against GitHub releases; project workflow patterns confirmed from existing `.github/workflows/`
- Architecture: HIGH — starter-workflows official template verified; merge pattern derived from project structure analysis
- Pitfalls: HIGH for structural pitfalls (working directory, job structure, configure-pages placement); MEDIUM for timing estimates (cargo doc build time not directly measured)

**Research date:** 2026-03-27
**Valid until:** 2026-07-27 (stable GitHub Actions ecosystem; mdBook and mdbook-admonish version pins are hard-coded)
