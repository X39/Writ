---
phase: 45
slug: writ-toml-project-file-compilation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 45 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust's built-in `#[test]` via cargo test |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p writ-compiler --lib config` |
| **Full suite command** | `cargo test -p writ-cli && cargo test -p writ-compiler --lib config` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler --lib config`
- **After every plan wave:** Run `cargo test -p writ-cli && cargo test -p writ-compiler --lib config`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 45-01-01 | 01 | 1 | TOOL-01 | unit | `cargo test -p writ-compiler --lib config` | ✅ | ⬜ pending |
| 45-01-02 | 01 | 1 | TOOL-02 | unit | `cargo test -p writ-compiler --lib config` | ❌ W0 | ⬜ pending |
| 45-02-01 | 02 | 2 | TOOL-01 | integration | `cargo test -p writ-cli` | ❌ W0 | ⬜ pending |
| 45-02-02 | 02 | 2 | TOOL-02 | integration | `cargo test -p writ-cli` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-compiler/src/config.rs` — add ProfileConfig struct and deserialization tests
- [ ] `writ-cli/src/main.rs` or `writ-cli/tests/` — integration tests for `writ build` subcommand

*Existing config tests cover parse_basic_config, scaffold_toml_round_trips, default_sources, discover_writ_files, missing_toml_error.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Scaffold `writ new` text update | TOOL-01 | String output comparison | Run `writ new test-proj`, verify "Next steps" says `writ build` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
