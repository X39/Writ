---
phase: 111
slug: assembler-completeness
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-28
---

# Phase 111 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p writ-assembler` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-assembler`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 111-01-01 | 01 | 1 | ASM-01 | build | `cargo build -p writ-assembler` | N/A | ⬜ pending |
| 111-01-02 | 01 | 1 | ASM-01, ASM-02 | unit | `cargo test -p writ-assembler` | ✅ | ⬜ pending |
| 111-01-03 | 01 | 1 | ASM-01, ASM-02 | integration | `cargo test -p writ-assembler round_trip` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Round-trip test for directive-complete module (ASM-01)
- [ ] Register type blob offset assertion test (ASM-02)

*Existing framework infrastructure covers all needs — only new test cases required.*

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
