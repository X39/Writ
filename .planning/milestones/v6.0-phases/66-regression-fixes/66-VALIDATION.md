---
phase: 66
slug: regression-fixes
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 66 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + insta 1.x |
| **Config file** | writ-compiler/Cargo.toml (dev-dep: insta) |
| **Quick run command** | `cargo clippy --workspace && cargo test -p writ-compiler --test lowering_tests && cargo test -p writ-compiler --test emit_tests` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo clippy --workspace`
- **After every plan wave:** Run `cargo test -p writ-compiler --test lowering_tests && cargo test -p writ-compiler --test emit_tests`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 66-01-01 | 01 | 1 | WARN-02 | lint | `cargo clippy --workspace` | N/A | ⬜ pending |
| 66-01-02 | 01 | 1 | WARN-02 | snapshot | `cargo test -p writ-compiler --test lowering_tests` | ✅ | ⬜ pending |
| 66-01-03 | 01 | 1 | WARN-02 | integration | `cargo test -p writ-compiler --test emit_tests` | ✅ | ⬜ pending |
| 66-01-04 | 01 | 1 | WARN-02 | artifact | `git status writ-compiler/tests/snapshots/` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new test files, fixtures, or framework installs needed.

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
