---
phase: 76
slug: zero-allocation-call-convention
status: finalized
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-22
---

# Phase 76 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test --release -p writ-runtime 2>&1` |
| **Full suite command** | `cargo test --release 2>&1` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p writ-runtime 2>&1`
- **After every plan wave:** Run `cargo test --release 2>&1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 76-01-01 | 01 | 1 | CALL-01 | grep+test | `grep -c "Vec::with_capacity" writ-runtime/src/dispatch/calls.rs` | ✅ | ✅ green |
| 76-01-02 | 01 | 1 | CALL-02,03 | test | `cargo test --release -p writ-runtime 2>&1` | ✅ | ✅ green |
| 76-01-03 | 01 | 1 | CALL-04,05 | test | `cargo test --release -p writ-runtime -- tail_call 2>&1` | ✅ | ✅ green |
| 76-02-01 | 02 | 2 | VERIFY-01 | benchmark | `cargo run --release -- benchmark/cases/fib/fib40.writ` | ✅ | ✅ green |
| 76-02-02 | 02 | 2 | VERIFY-02,03 | test | `cargo test --release 2>&1` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] New test: `tail_call_passes_multiple_args` — exercises tail-call with argc >= 2
- [x] New test: `call_indirect_passes_args` — exercises indirect call with arguments

Evidence: `tail_call_passes_multiple_args` confirmed at vm_tests.rs line 946; `call_indirect_passes_args` confirmed at vm_tests.rs line 1575 (both from VERIFICATION.md).

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-22
