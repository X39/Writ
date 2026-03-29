---
phase: 101
slug: writ-module-typeof-instruction-and-format-version-bump
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-28
---

# Phase 101 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p writ-module -p writ-assembler --lib 2>&1` |
| **Full suite command** | `cargo test -p writ-module -p writ-assembler 2>&1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-module -p writ-assembler --lib`
- **After every plan wave:** Run `cargo test -p writ-module -p writ-assembler`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 101-01-01 | 01 | 1 | SPEC-05 | unit | `cargo test -p writ-module typeof` | ✅ | ⬜ pending |
| 101-01-02 | 01 | 1 | SPEC-06 | unit | `cargo test -p writ-module unsupported_version` | ✅ | ⬜ pending |
| 101-01-03 | 01 | 1 | SPEC-05 | unit | `cargo test -p writ-assembler typeof` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. cargo test is already configured for both crates.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
