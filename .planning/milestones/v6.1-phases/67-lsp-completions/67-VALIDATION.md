---
phase: 67
slug: lsp-completions
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 67 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`#[test]`) + `#[tokio::test]` for async protocol tests |
| **Config file** | `Cargo.toml` workspace |
| **Quick run command** | `cargo test -p writ-lsp -- completion` |
| **Full suite command** | `cargo test -p writ-lsp` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-lsp -- completion`
- **After every plan wave:** Run `cargo test -p writ-lsp`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 67-01-01 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp -- test_dot_completion` | ❌ W0 | ⬜ pending |
| 67-01-02 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp -- test_dot_completion_array` | ❌ W0 | ⬜ pending |
| 67-01-03 | 01 | 1 | LSP-02 | unit | `cargo test -p writ-lsp -- test_namespace_completions_log` | ❌ W0 | ⬜ pending |
| 67-01-04 | 01 | 1 | LSP-02 | unit | `cargo test -p writ-lsp -- test_namespace_completions_option` | ❌ W0 | ⬜ pending |
| 67-01-05 | 01 | 1 | LSP-02 | unit | `cargo test -p writ-lsp -- test_namespace_completions_enum` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-lsp/src/queries/completion.rs` — add `test_dot_completion_receiver_found` (diagnostic unit test for `expr_at_offset` with receiver)
- [ ] `writ-lsp/src/queries/completion.rs` — add `test_namespace_completions_log`, `test_namespace_completions_option`, `test_namespace_completions_enum` (after `build_namespace_completions` is written)
- [ ] `writ-lsp/src/analysis_host.rs` — add integration test for dot-completion via `analyze_standalone` + `expr_at_offset` chain

*Existing infrastructure covers test framework setup. New test functions need stubs in Wave 0.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VS Code dot-completion popup | LSP-01 | Requires VS Code UI interaction | Type `p.` after declaring `let p: Point = ...;` — verify field list appears |
| VS Code `log::` completion popup | LSP-02 | Requires VS Code UI interaction | Type `log::` — verify 5 log levels appear |
| VS Code `Option::` completion popup | LSP-02 | Requires VS Code UI interaction | Type `Option::` — verify Some/None appear |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
