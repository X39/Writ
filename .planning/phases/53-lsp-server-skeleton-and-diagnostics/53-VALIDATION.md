---
phase: 53
slug: lsp-server-skeleton-and-diagnostics
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-14
updated: 2026-03-16
---

# Phase 53 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `cargo test` |
| **Config file** | None required (Cargo workspace) |
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
| 53-01-01 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp --lib -- convert::tests::test_severity` | ✅ green |
| 53-01-02 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp --lib -- convert::tests::test_offset` | ✅ green |
| 53-01-03 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp --lib -- convert::tests::test_writ_diag_to_lsp` | ✅ green |
| 53-01-04 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp --lib -- convert::tests::test_parse_error_to_diag` | ✅ green |
| 53-01-05 | 01 | 1 | LSP-08 | unit | `cargo test -p writ-lsp --lib -- analysis_host::tests::test_analyze_project_with_toml` | ✅ green |
| 53-02-01 | 02 | 1 | EXT-01 | smoke | `node -e "const g=require('./writ-vscode/syntaxes/writ.tmLanguage.json');console.assert(g.scopeName==='source.writ');console.assert(g.repository.comments);console.assert(g.repository.strings);console.assert(g.repository.keywords)"` | ✅ green |
| 53-02-02 | 02 | 1 | EXT-02 | smoke | `node -e "const p=require('./writ-vscode/package.json');console.assert(p.contributes.languages[0].id==='writ');console.assert(p.contributes.grammars[0].scopeName==='source.writ');console.assert(p.contributes.languages[0].extensions[0]==='.writ')"` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Test Coverage Summary

| Module | Tests | Coverage Area |
|--------|-------|---------------|
| `convert.rs` | 16 | offset_to_position (6), span_to_range (2), severity (3), writ_diag_to_lsp (4), parse_error_to_diag (1) |
| `analysis_host.rs` | 8 | standalone valid/parse/type/cascade/sources (5), project missing-toml (2), project with-toml (1) |
| `queries.rs` | 19 | position_to_byte_offset (4), expr_at_offset (3), hover (2), completions (3), references (1), semantic_tokens (4), signature_help (1), dot_completions (1) |
| **Total** | **43** | |

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TextMate grammar produces correct highlighting in VS Code | EXT-01 | Visual verification — programmatic grammar validation only checks structure, not visual correctness | Open a .writ file in VS Code with the extension installed; verify keywords, strings, numbers, dialogue blocks, entity names use distinct colors |
| VS Code auto-activates on .writ file open | EXT-02 | Requires VS Code runtime | Open a .writ file; verify language mode shows "Writ" in status bar without manual selection |
| Squiggle appears within 1 second | LSP-01 | Timing-sensitive | Save a .writ file with a type error; observe squiggle appearance timing |
| Squiggle disappears after fix | LSP-01 | End-to-end behavior | Fix the error, save; observe squiggle removal |

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
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |
