---
phase: 117
slug: collections-list-map-set
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 117 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p writ-compiler --lib && cargo test -p writ-runtime --lib` |
| **Full suite command** | `cargo test -p writ-compiler && cargo test -p writ-runtime && cargo test -p writ-golden` |
| **Estimated runtime** | ~45 seconds |

---

## Sampling Rate

- **After every task commit:** Run relevant crate tests
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 45 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 117-01-01 | 01 | 1 | COLL-06 | unit | `cargo test -p writ-runtime gc_struct_array` | ❌ W0 | ⬜ pending |
| 117-01-02 | 01 | 1 | COLL-06 | golden | `cargo test -p writ-golden generic_inherent` | ❌ W0 | ⬜ pending |
| 117-02-01 | 02 | 2 | COLL-01..05 | golden | `cargo test -p writ-golden coll_` | ❌ W0 | ⬜ pending |
| 117-03-01 | 03 | 3 | COLL-06 | integration | `cargo test -p writ-runtime coll_integration` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] GC struct-array transitivity test
- [ ] Generic inherent impl golden test
- [ ] Collection golden test stubs

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 45s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
