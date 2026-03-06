---
phase: 58
slug: dialogue-speaker-semantic-tokens
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 58 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`#[test]`) |
| **Config file** | none — workspace-level `cargo test` |
| **Quick run command** | `cargo test -p writ-lsp --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-lsp --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 58-01-01 | 01 | 1 | DIFF-01 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_semantic_tokens_dialogue_speaker` | ❌ W0 | ⬜ pending |
| 58-01-02 | 01 | 1 | DIFF-01 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_semantic_tokens_includes_dialogue_speaker` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `collect_dialogue_speaker_tokens` function in `writ-lsp/src/queries.rs`
- [ ] `collect_speaker_tokens_in_dlg_body` helper in `queries.rs`
- [ ] `collect_dlg_if_else_speakers` helper in `queries.rs`
- [ ] Extend `collect_semantic_tokens` to call and merge speaker tokens
- [ ] Remove `#[allow(dead_code)]` from `TOKEN_TYPE_DIALOGUE_SPEAKER`
- [ ] Two new tests in existing `#[cfg(test)]` block in `queries.rs`

*No new files required — all changes to existing `writ-lsp/src/queries.rs`.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
