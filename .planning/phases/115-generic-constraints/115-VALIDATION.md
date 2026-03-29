---
phase: 115
slug: generic-constraints
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 115 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p writ-compiler --lib` |
| **Full suite command** | `cargo test -p writ-compiler && cargo test -p writ-golden` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler --lib`
- **After every plan wave:** Run `cargo test -p writ-compiler && cargo test -p writ-golden`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 115-01-01 | 01 | 1 | GEN-01 | golden | `cargo test -p writ-golden` | ❌ W0 | ⬜ pending |
| 115-01-02 | 01 | 1 | GEN-02 | golden | `cargo test -p writ-golden` | ❌ W0 | ⬜ pending |
| 115-01-03 | 01 | 1 | GEN-03 | unit | `cargo test -p writ-compiler --lib` | ❌ W0 | ⬜ pending |
| 115-01-04 | 01 | 1 | GEN-04 | unit | `cargo test -p writ-compiler --lib` | ❌ W0 | ⬜ pending |
| 115-01-05 | 01 | 1 | GEN-05 | golden | `cargo test -p writ-golden` | ❌ W0 | ⬜ pending |
| 115-01-06 | 01 | 1 | GEN-06 | golden | `cargo test -p writ-golden` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Golden test `.writ` file for generic constraint success case (GEN-01)
- [ ] Golden test `.writ` file for multi-bound syntax (GEN-02)
- [ ] Golden test `.writ` file for constraint violation error (GEN-03)
- [ ] Unit test for GenericConstraint table emission (GEN-04)

*Existing test infrastructure covers framework and fixtures.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| IL binary inspection | GEN-04 | Binary table requires hex dump | Compile test file, verify GenericConstraint table rows in IL output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
