---
phase: 120
slug: array-semantics-correction
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 120 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test --workspace -- --test-threads=1 2>&1 \| tail -20` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -40` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace -- --test-threads=1 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test --workspace 2>&1 | tail -40`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 120-01-01 | 01 | 1 | ARR-01 | compile-error | `cargo test --package writ-golden` | existing | pending |
| 120-01-02 | 01 | 1 | ARR-02 | unit+integration | `cargo test --package writ-runtime array_resize` | new | pending |
| 120-01-03 | 01 | 1 | ARR-03 | unit+integration | `cargo test --package writ-runtime array_copy` | new | pending |
| 120-01-04 | 01 | 1 | ARR-04 | compile-error+golden | `cargo test --package writ-golden` | existing | pending |
| 120-01-05 | 01 | 1 | ARR-05 | compile-error | `cargo test --package writ-golden` | existing | pending |
| 120-01-06 | 01 | 1 | ARR-06 | manual-spec-review | spec file read | existing | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] New runtime tests for resize/copy in `writ-runtime/tests/vm_tests.rs`
- [ ] New golden test cases for removed methods (compiler error tests)
- [ ] Golden test updates for resize/copy emission

*Existing cargo test infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec wording accuracy | ARR-06 | Prose review — not automatable | Read language-spec/spec/07_6_primitive_types.md §1.6.1-1.6.3 and verify "allocation-explicit" wording, resize/copy documented, growth methods removed |
| IL spec table accuracy | ARR-06 | Table layout review | Read language-spec/spec/57_3_9_arrays.md and verify opcode table matches D-03 from CONTEXT.md |

*All code behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
