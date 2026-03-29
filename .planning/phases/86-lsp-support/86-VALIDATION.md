---
phase: 86
slug: lsp-support
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-24
---

# Phase 86 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in + `tokio::test` for async LSP tests |
| **Config file** | `writ-lsp/Cargo.toml` — `[[test]] name = "test_protocol"` |
| **Quick run command** | `cargo test -p writ-lsp` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-lsp`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 86-01-01 | 01 | 1 | LSP-01 | unit + integration | `cargo test -p writ-lsp test_dot_completions_contract` | ❌ W0 | ⬜ pending |
| 86-01-01 | 01 | 1 | LSP-02 | unit | `cargo test -p writ-lsp test_hover_contract` | ❌ W0 | ⬜ pending |
| 86-01-02 | 01 | 1 | LSP-03 | integration | `cargo test -p writ-lsp test_diagnostics_contract_valid` | ❌ W0 | ⬜ pending |
| 86-01-02 | 01 | 1 | LSP-03 | integration | `cargo test -p writ-lsp test_diagnostics_contract_invalid` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-lsp/src/queries/completion.rs` — add `test_dot_completions_contract` to existing `#[cfg(test)]` module
- [ ] `writ-lsp/src/queries/hover.rs` — add `test_hover_contract_def` to existing `#[cfg(test)]` module
- [ ] `writ-lsp/tests/test_protocol.rs` — add `test_diagnostics_contract_valid_no_errors` and `test_diagnostics_contract_invalid_produces_error`

*All gap items are new test functions in existing files — no new files or config needed.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-24
