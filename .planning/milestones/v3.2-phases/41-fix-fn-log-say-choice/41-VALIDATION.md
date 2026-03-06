---
phase: 41
slug: fix-fn-log-say-choice
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 41 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none |
| **Quick run command** | `cargo test -p writ-golden -- test_fn_log_say_choice` |
| **Full suite command** | `cargo test -p writ-golden` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-golden -- test_fn_log_say_choice`
- **After every plan wave:** Run `cargo test -p writ-golden`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 41-01-01 | 01 | 0 | BUG-01 | unit | `cargo test -p writ-golden -- test_harness_bom_strip` | ❌ W0 | ⬜ pending |
| 41-01-02 | 01 | 1 | BUG-01 | golden | `cargo test -p writ-golden -- test_fn_log_say_choice` | ❌ W0 | ⬜ pending |
| 41-01-03 | 01 | 1 | BUG-01 | golden | `cargo test -p writ-golden -- test_fn_log_say_choice` | ❌ W0 | ⬜ pending |
| 41-01-04 | 01 | 1 | BUG-01 | golden | `BLESS=1 cargo test -p writ-golden -- test_fn_log_say_choice` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `test_fn_log_say_choice` function in `writ-golden/tests/golden_tests.rs` — calls `run_golden_test("fn_log_say_choice")` for BUG-01 main behavior
- [ ] `test_harness_bom_strip` unit test in `writ-golden/tests/golden_tests.rs` — verifies BOM-stripping in read path
- [ ] `fn_log_say_choice.writil` empty/placeholder file — required by `run_golden_test` before BLESS=1 fills it

*Existing infrastructure: `cargo test` harness exists; `compile_and_disassemble` function exists.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| UTF-8 encoding of blessed .writil | BUG-01 SC-3 | File encoding can't be asserted by string comparison | Run `file fn_log_say_choice.writil` or `xxd fn_log_say_choice.writil \| head -1` after bless — confirm no FF FE BOM |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
