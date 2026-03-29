---
phase: 122
slug: cross-module-type-resolution
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 122 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust integration tests) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p writ-compiler -- xmod 2>&1 \| head -30` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler -- xmod 2>&1 | head -30`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 122-01-01 | 01 | 1 | XMOD-01 | integration | `cargo test -p writ-compiler -- xmod` | ✅ | ⬜ pending |
| 122-01-02 | 01 | 1 | XMOD-02 | integration | `cargo test -p writ-compiler -- xmod_error` | ✅ | ⬜ pending |
| 122-02-01 | 02 | 2 | XMOD-03 | integration | `cargo test -p writ-compiler -- virtual_module` | ✅ | ⬜ pending |
| 122-02-02 | 02 | 2 | XMOD-04 | integration | `cargo test -p writ-compiler -- coll_with_library_separate_modules` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec documentation accuracy | XMOD-05 | Prose review | Read spec section and verify it matches implementation |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
