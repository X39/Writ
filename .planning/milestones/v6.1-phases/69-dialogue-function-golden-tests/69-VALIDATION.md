---
phase: 69
slug: dialogue-function-golden-tests
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 69 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + `cargo test` |
| **Config file** | `Cargo.toml` (workspace) |
| **Quick run command** | `cargo test -p writ-golden -- dlg_` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-golden -- dlg_`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 69-01-01 | 01 | 1 | GOLD-01 | golden | `cargo test -p writ-golden -- test_dlg_fn_mix` | ❌ W0 | ⬜ pending |
| 69-01-02 | 01 | 1 | GOLD-02 | golden | `cargo test -p writ-golden -- test_dlg_quest_pattern` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-golden/tests/golden/dlg_fn_mix.writ` — source file for GOLD-01
- [ ] `writ-golden/tests/golden/dlg_quest_pattern.writ` — source file for GOLD-02
- [ ] `writ-golden/tests/golden/dlg_fn_mix.writil` — blessed snapshot (BLESS=1)
- [ ] `writ-golden/tests/golden/dlg_quest_pattern.writil` — blessed snapshot (BLESS=1)
- [ ] Test registration (Section L) in `writ-golden/tests/golden_tests.rs`

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
