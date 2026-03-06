---
phase: 61
slug: signature-help-diagnostics-extension-polish
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 61 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`#[test]`) via `cargo test` |
| **Config file** | none (workspace Cargo.toml) |
| **Quick run command** | `cargo test -p writ-lsp` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-lsp`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 61-01-01 | 01 | 1 | LSP-07 | unit | `cargo test -p writ-lsp test_signature_help_incomplete_source` | ❌ W0 | ⬜ pending |
| 61-01-02 | 01 | 1 | LSP-07 | unit | `cargo test -p writ-lsp test_signature_help_active_param_incomplete` | ❌ W0 | ⬜ pending |
| 61-01-03 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp test_zero_width_span_expansion` | ❌ W0 | ⬜ pending |
| 61-01-04 | 01 | 1 | LSP-01 | unit | `cargo test -p writ-lsp test_entity_missing_brace_span` | ❌ W0 | ⬜ pending |
| 61-01-05 | 01 | 1 | DIFF-01 | manual | n/a | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-lsp/src/queries.rs` — add `test_signature_help_incomplete_source`: calls `build_signature_help` with source `"fn foo(a: int, b: int) {} fn main() { foo("` and verifies `Some` is returned
- [ ] `writ-lsp/src/queries.rs` — add `test_signature_help_active_param_incomplete`: same but with `"fn foo(a: int, b: int) {} fn main() { foo(1,"` and verifies `active_parameter == Some(1)`
- [ ] `writ-lsp/src/convert.rs` — add `test_zero_width_span_expansion`: creates a `SimpleSpan { start: 10, end: 10, .. }` parse error, verifies the resulting diagnostic range has `start != end`
- [ ] `writ-lsp/src/convert.rs` — add `test_entity_missing_brace_span`: parse entity source with missing brace, verify diagnostic span is non-zero-width

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Entity names visually distinct from struct names in default VS Code theme | DIFF-01 | Color perception is visual; cannot be asserted in tests | 1. Open a `.writ` file with both `entity` and `struct` declarations. 2. Verify entity names show a different color than struct names in Dark+ theme. 3. Check `package.json` diff shows `configurationDefaults` with distinct color codes. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
