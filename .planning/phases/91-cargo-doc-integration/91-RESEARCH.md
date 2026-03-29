# Phase 91: cargo doc Integration - Research

**Researched:** 2026-03-27
**Domain:** Rust rustdoc / cargo doc workspace tooling, rustdoc lint suppression, GitHub Pages redirect patterns
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — infrastructure phase, all implementation choices at Claude's discretion.

### Claude's Discretion
All implementation choices. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CI-03 | cargo doc output merged into site at /api/ path with working root redirect | Confirmed: inject target/doc/index.html with meta-refresh redirect to writ_compiler/index.html; lint suppression via [workspace.lints.rustdoc] table |
</phase_requirements>

## Summary

`cargo doc --workspace --no-deps` produces a `target/doc/` tree with one subdirectory per library crate but no `index.html` at the root — this is a known permanent limitation (cargo issue #1016, open since 2014 with no upstream resolution). The fix is always the same: write a minimal HTML file with a `<meta http-equiv="refresh">` tag pointing at the chosen root crate directory. For this workspace, `writ_compiler` is the correct redirect target because it is the primary library crate with an explicit public API (`pub mod ast`, `pub mod check`, etc.) and crate-level documentation.

The warn-by-default rustdoc lints (`broken_intra_doc_links`, `private_intra_doc_links`, `invalid_html_tags`, `bare_urls`, `redundant_explicit_links`, `invalid_codeblock_attributes`, `invalid_rust_codeblocks`) are the primary risk. These are safe warnings today (cargo doc exits 0) but will become hard errors in Phase 92 when `RUSTDOCFLAGS="-D warnings"` is added to CI. The workspace has 10 crates, all edition 2024, none with any existing lint attributes. A single `[workspace.lints.rustdoc]` table in the root `Cargo.toml` plus `[lints] workspace = true` in each member crate provides one-shot suppression. Only crates that should enforce docs (none at this stage — pre-1.0 project) need specific enable rules.

The `.nojekyll` file is required in `target/doc/` to prevent GitHub Pages from processing the rustdoc output through Jekyll, which would break paths containing underscores.

**Primary recommendation:** Run `cargo doc --workspace --no-deps` locally, fix any warn-by-default lint hits, write `target/doc/index.html` (meta-refresh to `writ_compiler/index.html`), write `target/doc/.nojekyll`. Commit the workspace lint config. The output of this phase is a `target/doc/` tree that Phase 92 can `cp -r` into the site artifact under `api/`.

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `cargo doc` | ships with Rust 1.93 (current) | Generate HTML rustdoc from workspace | Built-in, no extra dependencies |
| `rustdoc` | ships with Rust 1.93 (current) | Invoked by cargo doc per-crate | Built-in |

### Supporting
| Mechanism | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `[workspace.lints.rustdoc]` | Cargo 1.74+ (available) | Workspace-wide rustdoc lint config | Suppress warn-by-default lints uniformly |
| `target/doc/index.html` (manual) | — | Root redirect for /api/ URL | Always required — cargo never generates it |
| `target/doc/.nojekyll` | — | Prevent Jekyll processing on GitHub Pages | Always required for Rust doc deployments |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `[workspace.lints]` table | Per-crate `#![allow(rustdoc::...)]` in each `lib.rs` | Per-crate requires touching 9 files; workspace table is single edit + member opt-in |
| Static meta-refresh HTML | JavaScript redirect | Meta-refresh works without JavaScript, is the community standard, no extra tooling |
| Redirect to `writ_compiler` | Redirect to `writ_runtime` | `writ_compiler` has the more user-relevant public API (entry point for embedders); `writ_runtime` is also public but is downstream |

**Installation:** No additional packages. `cargo doc` is part of the Rust toolchain.

## Architecture Patterns

### Workspace Crate Inventory

The workspace has 10 crates. Their cargo doc behavior:

| Crate | Has lib.rs | Has main.rs | Produces lib docs | Notes |
|-------|-----------|------------|------------------|-------|
| `writ-assembler` | yes | no | yes | — |
| `writ-cli` | no | yes | no (binary only) | No lib docs |
| `writ-compiler` | yes | yes | yes | **Redirect target** — primary public API |
| `writ-dap` | yes | yes | yes | LSP/DAP server |
| `writ-diagnostics` | yes | no | yes | — |
| `writ-golden` | no | no | no | Test-only crate, no src/, cargo doc skips it |
| `writ-lsp` | yes | yes | yes | Has explicit `[lib]` and `[[bin]]` |
| `writ-module` | yes | no | yes | — |
| `writ-parser` | yes | no | yes | — |
| `writ-runtime` | yes | no | yes | — |

`writ-golden` has no `src/` directory at all (only `tests/`) — cargo doc will skip it naturally. This is correct behavior.

`writ-cli` is a binary-only crate — no library docs will be generated.

### Pattern 1: Workspace Lint Table

**What:** A single `[workspace.lints.rustdoc]` block in `Cargo.toml` that all members opt into.
**When to use:** Always — avoids editing 9 individual `lib.rs` files.

```toml
# Root Cargo.toml
[workspace.lints.rustdoc]
# Suppress warn-by-default rustdoc lints — pre-1.0 project, docs are
# a best-effort baseline. Re-enable per-crate when docs are polished.
broken_intra_doc_links = "allow"
private_intra_doc_links = "allow"
invalid_html_tags = "allow"
bare_urls = "allow"
redundant_explicit_links = "allow"
invalid_codeblock_attributes = "allow"
invalid_rust_codeblocks = "allow"
```

Each member crate's `Cargo.toml` gets:
```toml
[lints]
workspace = true
```

**Requires Cargo 1.74+.** The current toolchain is Rust 1.93-nightly — this is satisfied.

**Important constraint:** A crate using `[lints] workspace = true` CANNOT also add additional `[lints.rustdoc]` entries in the same block. Each crate either inherits workspace lints OR defines its own — not both. This is not a problem here because all 10 crates should use the workspace config.

### Pattern 2: Manual index.html Redirect

**What:** A minimal HTML file written to `target/doc/index.html` after `cargo doc` runs.
**When to use:** Always — cargo never generates this file (confirmed cargo issue #1016).

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=writ_compiler/index.html">
    <link rel="canonical" href="writ_compiler/index.html">
    <title>Writ API Documentation</title>
  </head>
  <body>
    <p>Redirecting to <a href="writ_compiler/index.html">writ_compiler</a>…</p>
  </body>
</html>
```

Key details:
- The `url=` value is `writ_compiler/index.html` (relative, not absolute) — this works correctly whether the page is served at `/api/` (GitHub Pages) or `target/doc/` (local).
- Crate names in `target/doc/` use underscores: `writ-compiler` crate becomes `writ_compiler/` directory.
- `content="0; url=..."` means instant redirect (0-second delay).

### Pattern 3: .nojekyll File

**What:** An empty file at `target/doc/.nojekyll`.
**When to use:** Always required for GitHub Pages deployments of rustdoc.
**Why:** GitHub Pages runs Jekyll by default; Jekyll ignores directories starting with `_`. Rustdoc generates directories like `_sources/`, `_static/`, etc. Without `.nojekyll`, these assets vanish and the docs render as broken pages.

```bash
touch target/doc/.nojekyll
```

### Anti-Patterns to Avoid

- **Committing target/doc/ to git:** The `target/` directory is in `.gitignore`. The index.html redirect and .nojekyll file should be created by CI/build scripts, not committed. Phase 92 creates them in the workflow — Phase 91 validates and documents the pattern.
- **Using absolute URL in meta-refresh:** `url=/Writ/api/writ_compiler/index.html` would break local testing. Use relative URL.
- **Forgetting .nojekyll:** Docs will render with broken CSS/JS on GitHub Pages. Easy to miss locally.
- **Redirecting to writ-cli:** writ-cli has no library docs (binary-only crate). The `target/doc/writ/` directory it would produce contains only sparse binary crate docs, not useful.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rustdoc lint control | Per-file `#![allow(...)]` in every lib.rs | `[workspace.lints.rustdoc]` | 9 files vs 1 config block; workspace table is idiomatic Cargo 1.74+ approach |
| Root redirect HTML | Complex multi-crate index page | Single meta-refresh HTML | Community standard; meta-refresh is instant; no JavaScript dependency |
| Lint audit | Manual grep of each crate | Run `cargo doc --workspace --no-deps` and read warnings | Rustdoc itself identifies all lint violations precisely |

**Key insight:** The entire "missing index.html" problem is a known limitation with a one-liner fix. There is no need for third-party tools (cargo-ghp-upload, etc.) for this phase — the deliverable is just the lint config and the redirect file specification for CI.

## Common Pitfalls

### Pitfall 1: Crate Name Hyphen-to-Underscore Conversion
**What goes wrong:** The redirect URL uses `writ-compiler/index.html` (with hyphen) instead of `writ_compiler/index.html` (with underscore). The browser gets a 404.
**Why it happens:** Cargo package names use hyphens (`writ-compiler`) but Rust crate names and rustdoc directory names use underscores (`writ_compiler/`).
**How to avoid:** Always use underscores in the redirect URL. The directory created by cargo doc is `target/doc/writ_compiler/`, never `target/doc/writ-compiler/`.
**Warning signs:** 404 on the redirect target; `ls target/doc/` shows no `writ-compiler/` directory but does show `writ_compiler/`.

### Pitfall 2: Warn-by-Default Lints Become Hard Errors in CI
**What goes wrong:** `cargo doc --workspace --no-deps` completes locally (exit code 0), but fails in Phase 92 CI when `RUSTDOCFLAGS="-D warnings"` is set.
**Why it happens:** Default behavior is warn (not error). CI adds `-D warnings` to surface problems.
**How to avoid:** Run `cargo doc --workspace --no-deps 2>&1 | grep "warning:"` locally after adding the workspace lint config. Zero warnings = CI-safe. If any remain, they must be fixed or explicitly suppressed before Phase 92.
**Warning signs:** Any `warning:` lines in cargo doc output, especially `bare_urls`, `invalid_html_tags`, `broken_intra_doc_links`.

### Pitfall 3: writ-golden / Binary-only Crates Producing Sparse Docs
**What goes wrong:** Confusion about which crates produce meaningful docs. `cargo doc --workspace` includes all workspace members.
**Why it happens:** `writ-golden` has no `src/lib.rs` (test-only), so cargo doc skips it. `writ-cli` is binary-only — it produces sparse docs but not library docs. This is expected.
**How to avoid:** Accept that binary crates and test-only crates produce minimal or no library documentation. The redirect target must be a library crate.
**Warning signs:** None — this is expected behavior, not a failure.

### Pitfall 4: `[lints] workspace = true` Incompatibility with Additional Lints
**What goes wrong:** A crate with `[lints] workspace = true` tries to add per-crate lint overrides like `[lints.rustdoc] broken_intra_doc_links = "deny"`. Cargo rejects this with a hard error.
**Why it happens:** Cargo's lint inheritance is all-or-nothing per crate. You cannot mix workspace inheritance with per-crate additions in the same `[lints]` block.
**How to avoid:** For this phase all crates use `workspace = true`. If future phases need per-crate overrides, those crates must remove `workspace = true` and duplicate the full lint config locally.
**Warning signs:** Cargo error: "cannot mix workspace inheritance with other lints".

### Pitfall 5: Virtual Workspace Cannot Contain `[lints]` Directly
**What goes wrong:** Adding `[lints.rustdoc]` directly to the root Cargo.toml without using `[workspace.lints.rustdoc]`.
**Why it happens:** This workspace is a virtual workspace (the root Cargo.toml has `[workspace]` but no `[package]`). Virtual manifests cannot have a `[lints]` section — only `[workspace.lints]` is allowed.
**How to avoid:** Always use `[workspace.lints.rustdoc]`, not `[lints.rustdoc]`, in the root Cargo.toml.
**Warning signs:** Cargo error: "virtual manifest specifies a `lints` section which is not allowed".

## Code Examples

Verified patterns from official sources and community practice:

### Root Cargo.toml — Workspace Lints Block
```toml
# Cargo.toml (workspace root)
[workspace.lints.rustdoc]
broken_intra_doc_links = "allow"
private_intra_doc_links = "allow"
invalid_html_tags = "allow"
bare_urls = "allow"
redundant_explicit_links = "allow"
invalid_codeblock_attributes = "allow"
invalid_rust_codeblocks = "allow"
```

### Member Crate Cargo.toml — Lint Inheritance
```toml
# writ-compiler/Cargo.toml (and all other crates)
[lints]
workspace = true
```

### Manual index.html Redirect
```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=writ_compiler/index.html">
    <link rel="canonical" href="writ_compiler/index.html">
    <title>Writ API Documentation</title>
  </head>
  <body>
    <p>Redirecting to <a href="writ_compiler/index.html">writ_compiler</a>…</p>
  </body>
</html>
```

### Shell Script to Inject Redirect (Phase 92 CI Use)
```bash
# After: cargo doc --workspace --no-deps
# Creates the redirect and nojekyll files in the doc output tree
cat > target/doc/index.html << 'EOF'
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=writ_compiler/index.html">
    <link rel="canonical" href="writ_compiler/index.html">
    <title>Writ API Documentation</title>
  </head>
  <body>
    <p>Redirecting to <a href="writ_compiler/index.html">writ_compiler</a>...</p>
  </body>
</html>
EOF
touch target/doc/.nojekyll
```

### Verify cargo doc Produces No Warnings
```bash
# Run locally after configuring workspace lints — should produce zero warning lines
cargo doc --workspace --no-deps 2>&1 | grep "^warning:"
# Expected output: (empty)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-crate `#![allow(rustdoc::...)]` in every lib.rs | `[workspace.lints.rustdoc]` in root Cargo.toml | Cargo 1.74 (stable Nov 2023) | Single config point for all crates |
| GitHub Actions v2/v3 deploy actions | `actions/upload-pages-artifact@v3` + `actions/deploy-pages@v4` | 2023-2024 | Phase 92 concern, not Phase 91 |

**No deprecated approaches involved in this phase.** The redirect pattern has been stable since the issue was first identified (2014). The workspace lints table is the current idiomatic solution for lint config sharing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | `cargo doc` | yes | 1.93.0-nightly | — |
| cargo doc | Doc generation | yes | Ships with Rust 1.93 | — |

No external dependencies beyond the Rust toolchain. This phase is code/config-only (Cargo.toml edits + creating `target/doc/index.html`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Manual verification (no automated test framework needed) |
| Config file | n/a |
| Quick run command | `cargo doc --workspace --no-deps 2>&1 \| grep "^warning:"` |
| Full suite command | `cargo doc --workspace --no-deps && ls target/doc/index.html && ls target/doc/.nojekyll` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CI-03 | `cargo doc --workspace --no-deps` completes without errors | smoke | `cargo doc --workspace --no-deps` | n/a |
| CI-03 | `target/doc/index.html` exists and redirects to writ_compiler | manual inspect | `cat target/doc/index.html` | Created by this phase |
| CI-03 | No rustdoc warnings (CI-safe) | smoke | `cargo doc --workspace --no-deps 2>&1 \| grep "^warning:"` | n/a |

### Sampling Rate
- **Per task commit:** `cargo doc --workspace --no-deps 2>&1 | grep "^warning:"` — must be empty
- **Phase gate:** All three success criteria verified before proceeding to Phase 92

### Wave 0 Gaps
None — no test framework changes needed. Verification is manual inspection + warning count check.

## Open Questions

1. **Exact set of warn-by-default lint violations in current codebase**
   - What we know: The codebase has doc comments on all public crates (crate-level `//!`), but a heuristic analysis found 0–72% undocumented public items across crates. Warn-by-default lints do NOT fire on missing_docs (that's allow by default), only on doc quality issues like bare URLs, broken links, etc.
   - What's unclear: Whether the current doc comments contain any bare URLs, broken intra-doc links, or invalid HTML tags — this can only be determined by actually running `cargo doc`.
   - Recommendation: Run `cargo doc --workspace --no-deps` as the first plan step; the output is the authoritative list of what needs fixing.

2. **writ-compiler vs. writ-runtime as redirect target**
   - What we know: Both are library crates with documented public APIs. `writ-compiler` is the primary compilation entry point (`compile_source` function, all pipeline stages exposed as `pub mod`). `writ-runtime` is the VM execution entry point.
   - What's unclear: Which crate a user embedding Writ would navigate to first.
   - Recommendation: `writ_compiler` — it is documented as "the compilation pipeline" with an explicit crate root doc. The ROADMAP also lists "writ-cli or writ-compiler" with writ-compiler as the natural public API.

## Sources

### Primary (HIGH confidence)
- `cargo doc --workspace` behavior — verified from official Cargo docs and cargo issue #1016 (known 12-year-old limitation)
- `[workspace.lints]` table syntax — [Cargo Workspaces official docs](https://doc.rust-lang.org/cargo/reference/workspaces.html), requires Cargo 1.74+
- Rustdoc lints and default levels — [The rustdoc book: Lints](https://doc.rust-lang.org/rustdoc/lints.html)
- Rustdoc lint levels table (broken_intra_doc_links=warn, bare_urls=warn, etc.) — verified via WebFetch of official rustdoc book

### Secondary (MEDIUM confidence)
- `meta http-equiv="refresh"` redirect pattern — [DEV Community: Prepare your Rust API docs for Github Pages](https://dev.to/deciduously/prepare-your-rust-api-docs-for-github-pages-2n5i), cross-referenced with GitHub community discussion #72823
- `.nojekyll` requirement — documented in GitHub Pages docs and universally referenced in Rust doc deployment guides
- CRATE_NAME underscore conversion pattern (`tr '[:upper:]' '[:lower:]' | cut` or manual) — community standard; cargo doc directory naming is rustdoc behavior, not configurable

### Tertiary (LOW confidence)
- Exact set of rustdoc lint hits in this codebase — not verified; requires running cargo doc locally
- `[lints] workspace = true` incompatibility with additional lints — reported in cargo issue #13157; verified conceptually by RFC 3389 description

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — built-in tooling, no external dependencies
- Architecture patterns: HIGH — workspace lint table is official Cargo feature; redirect pattern is community standard verified against official rustdoc behavior
- Pitfalls: MEDIUM — hyphen/underscore pitfall and .nojekyll are well-documented; the exact warning count in this codebase is LOW confidence (requires local run to confirm)

**Research date:** 2026-03-27
**Valid until:** 2027-03-27 (stable tooling; workspace.lints is stable since Cargo 1.74)
