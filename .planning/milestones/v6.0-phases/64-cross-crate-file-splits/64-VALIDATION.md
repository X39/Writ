---
phase: 64
slug: cross-crate-file-splits
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 64 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p writ-parser -p writ-lsp -p writ-dap -p writ-runtime -p writ-cli` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate_being_split>`
- **After every plan wave:** Run `cargo test --workspace && cargo clippy --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 64-01-01 | 01 | 1 | SPLIT-01 | integration | `cargo test -p writ-parser` | ✅ `writ-parser/tests/parser_tests.rs` | ⬜ pending |
| 64-02-01 | 02 | 1 | SPLIT-02 | integration | `cargo test -p writ-lsp` | ✅ `writ-lsp/tests/test_hover_protocol.rs` | ⬜ pending |
| 64-03-01 | 03 | 2 | SPLIT-06 | unit+integration | `cargo test -p writ-runtime` | ✅ `writ-runtime/tests/vm_tests.rs` + inline | ⬜ pending |
| 64-03-02 | 03 | 2 | SPLIT-07 | integration | `cargo test -p writ-dap` | ✅ `writ-dap/tests/test_initialize_sequence.rs` | ⬜ pending |
| 64-03-03 | 03 | 2 | SPLIT-14 | e2e | `cargo test -p writ-cli` | ✅ `writ-cli/tests/cli_integration.rs` | ⬜ pending |
| 64-04-01 | 04 | 3 | SPLIT-12 | n/a (doc) | `cargo build -p writ-lsp` | ✅ inline tests | ⬜ pending |
| 64-04-02 | 04 | 3 | SPLIT-13 | n/a (doc) | `cargo build -p writ-lsp` | ✅ no separate test file | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new test files need to be created for this structural refactoring phase.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
