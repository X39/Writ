---
phase: 54
slug: lsp-navigation-and-completions
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-14
updated: 2026-03-16
---

# Phase 54 — Validation Strategy

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

- **After every task commit:** Run `cargo test -p writ-lsp`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | Status |
|---------|------|------|-------------|-----------|-------------------|--------|
| 54-01-01 | 01 | 1 | LSP-02 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_identifier_completions_has_keywords` | ✅ green |
| 54-01-02 | 01 | 1 | LSP-03 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_dot_completions_struct_fields` | ✅ green |
| 54-01-03 | 01 | 1 | LSP-04 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_hover_text_var` | ✅ green |
| 54-01-04 | 01 | 1 | LSP-04 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_hover_text_fn_call` | ✅ green |
| 54-01-05 | 01 | 1 | LSP-05 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_goto_def_resolves_fn` | ✅ green |
| 54-01-06 | 01 | 1 | LSP-05 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_goto_def_builtin_returns_none` | ✅ green |
| 54-01-07 | 01 | 1 | LSP-06 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_collect_references_finds_uses` | ✅ green |
| 54-01-08 | 01 | 1 | LSP-07 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_signature_help_finds_param` | ✅ green |
| 54-01-09 | 01 | 1 | DIFF-01 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_semantic_tokens_entity` | ✅ green |
| 54-01-10 | 01 | 1 | DIFF-02 | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_dot_completions_entity_components` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Test Coverage Summary

| Module | Tests | Coverage Area |
|--------|-------|---------------|
| `queries.rs` | 21 | position_to_byte_offset (4), expr_at_offset (3), goto-def (2), hover (2), references (1), completions (3), dot-completions (2), signature_help (1), semantic_tokens (4) |
| `convert.rs` | 16 | offset_to_position (6), span_to_range (2), severity (3), writ_diag_to_lsp (4), parse_error_to_diag (1) |
| `analysis_host.rs` | 8 | standalone (5), project missing-toml (2), project with-toml (1) |
| **Total** | **45** | |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Semantic highlighting visual distinction | DIFF-01 | Visual confirmation needed | Open a .writ file in VS Code, verify entity names, component types, dialogue speakers, keywords are visually distinct |

---

## Validation Sign-Off

- [x] All tasks have automated verify commands
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] All requirements have at least one automated test
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** complete

---

## Validation Audit 2026-03-16

| Metric | Count |
|--------|-------|
| Gaps found | 2 |
| Resolved | 2 |
| Escalated | 0 |
