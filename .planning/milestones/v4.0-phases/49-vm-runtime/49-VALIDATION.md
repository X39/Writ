---
phase: 49
slug: vm-runtime
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-12
---

# Phase 49 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test framework (cargo test) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p writ-runtime` |
| **Full suite command** | `cargo test -p writ-runtime` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-runtime`
- **After every plan wave:** Run `cargo test -p writ-runtime`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 49-01-01 | 01 | 1 | VM-01 | unit | `cargo test -p writ-runtime inline_struct_no_heap_alloc` | Wave 0 | pending |
| 49-01-02 | 01 | 1 | VM-02 | unit | `cargo test -p writ-runtime mov_inline_struct_independent_copy` | Wave 0 | pending |
| 49-01-03 | 01 | 1 | VM-03 | unit | `cargo test -p writ-runtime new_kind_dispatch` | Wave 0 | pending |
| 49-01-04 | 01 | 1 | VM-04 | unit | `cargo test -p writ-runtime gc_traces_inline_struct_ref_fields` | Wave 0 | pending |
| 49-01-05 | 01 | 1 | VM-05 | unit | `cargo test -p writ-runtime box_unbox_inline_struct` | Wave 0 | pending |
| 49-01-06 | 01 | 1 | VM-06 | unit | `cargo test -p writ-runtime class_field_access_regression` | Wave 0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] New tests in `writ-runtime/src/` test modules covering VM-01 through VM-06
- [ ] Framework already installed — no setup needed

*Existing infrastructure covers framework requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
