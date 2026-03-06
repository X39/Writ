---
phase: 42
slug: choiceoption-rename
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 42 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + insta snapshot testing |
| **Config file** | `writ-compiler/Cargo.toml`, `writ-runtime/Cargo.toml`, `writ-golden/Cargo.toml` |
| **Quick run command** | `cargo test -p writ-compiler 2>&1 \| tail -5` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -10` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler 2>&1 | tail -5`
- **After every plan wave:** Run `cargo test --workspace 2>&1 | tail -10`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 42-01-01 | 01 | 1 | LANG-01 | unit | `cargo test -p writ-compiler lowering_tests::dlg_choice_basic 2>&1 \| tail -3` | ✅ | ⬜ pending |
| 42-01-02 | 01 | 1 | LANG-01 | snapshot | `cargo insta test -p writ-compiler --accept 2>&1 \| tail -5` | ✅ | ⬜ pending |
| 42-01-03 | 01 | 2 | LANG-01 | integration | `cargo test --workspace 2>&1 \| tail -10` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

- `writ-compiler/tests/lowering_tests.rs` already contains `dlg_choice_basic`, `dlg_choice_label_key_emitted`, `dlg_choice_speaker_scope_isolation`
- `writ-compiler/tests/snapshots/` already has blessed snapshots for all choice tests
- `writ-golden/` already has the `fn_log_say_choice` golden test

*No new test infrastructure needed — snapshot blessing is the verification mechanism.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CALL_EXTERN references ChoiceOption ExternDef token | LANG-01 SC-3 | Requires inspecting emitted IL token value | Write a `.writ` file with `$ choice { "Good" { } "Bad" { } }`, compile via `writ compile`, disassemble and verify `CALL_EXTERN` references a token that resolves to `ChoiceOption` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
