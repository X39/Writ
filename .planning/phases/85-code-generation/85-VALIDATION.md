---
phase: 85
slug: code-generation
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-24
---

# Phase 85 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness |
| **Config file** | none — `cargo test` |
| **Quick run command** | `cargo test -p writ-compiler --test emit_body_tests -- contract` |
| **Full suite command** | `cargo test -p writ-compiler` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler --test emit_body_tests -- contract`
- **After every plan wave:** Run `cargo test -p writ-compiler`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 85-01-01 | 01 | 1 | EMIT-01 | unit | `cargo test -p writ-compiler --test emit_body_tests -- test_contract_receiver_emits_call_virt` | ❌ W0 | ⬜ pending |
| 85-01-01 | 01 | 1 | EMIT-02 | unit | `cargo test -p writ-compiler --test emit_body_tests -- test_contract_receiver_call_virt_correct_idx_and_slot` | ❌ W0 | ⬜ pending |
| 85-01-02 | 01 | 1 | EMIT-04 | unit | `cargo test -p writ-compiler --test typecheck_tests -- test_contract_receiver_repro_script` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-compiler/tests/emit_body_tests.rs` — tests for EMIT-01 and EMIT-02 (contract receiver CALL_VIRT emission)
- [ ] `writ-compiler/tests/typecheck_tests.rs` — test for EMIT-04 (end-to-end repro script)
- [ ] `contract_method_slot_by_name` method on `ModuleBuilder` in `writ-compiler/src/emit/module_builder.rs`

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-24
