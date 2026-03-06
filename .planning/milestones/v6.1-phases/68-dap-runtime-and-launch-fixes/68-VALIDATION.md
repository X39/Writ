---
phase: 68
slug: dap-runtime-and-launch-fixes
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 68 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test (`cargo test`) |
| **Config file** | `Cargo.toml` workspace |
| **Quick run command** | `cargo test -p writ-dap 2>&1` |
| **Full suite command** | `cargo test --workspace 2>&1` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-dap 2>&1`
- **After every plan wave:** Run `cargo test --workspace 2>&1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 68-01-01 | 01 | 1 | DAP-01 | unit | `cargo test -p writ-compiler encode_switch` | ❌ W0 | ⬜ pending |
| 68-01-02 | 01 | 1 | DAP-01 | integration | `cargo test -p writ-dap test_quest_system` | ✅ | ⬜ pending |
| 68-02-01 | 02 | 1 | DAP-02 | integration | `cargo test -p writ-dap test_multi_file` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Unit test for SWITCH byte-offset encoding round-trip in `writ-compiler` — emit enum match, serialize, round-trip through `decode_and_reindex`, verify no decode error
- [ ] Integration test for multi-file DAP launch — create temp writ.toml project, call compile_and_load with directory path, verify module loads

*Existing test infrastructure covers all other phase requirements.*

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
