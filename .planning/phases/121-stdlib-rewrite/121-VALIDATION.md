---
phase: 121
slug: stdlib-rewrite
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 121 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust integration tests) + insta (golden snapshots) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p writ-golden -- --ignored 2>&1 | head -50` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-golden -- --ignored 2>&1 | head -50`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 121-01-01 | 01 | 1 | STD-01 | golden | `BLESS=1 cargo test -p writ-golden test_coll_list` | ✅ | ⬜ pending |
| 121-01-02 | 01 | 1 | STD-02 | golden | `BLESS=1 cargo test -p writ-golden test_coll_map` | ✅ | ⬜ pending |
| 121-01-03 | 01 | 1 | STD-03 | integration | `cargo test -p writ-runtime -- collections` | ✅ | ⬜ pending |
| 121-01-04 | 01 | 1 | STD-04 | compilation | `cargo build -p writ-cli` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements — stdlib source files, golden tests, and integration tests already exist.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| No `add`/`remove_at`/`insert` calls in stdlib source | STD-01, STD-02 | Requires grep verification | `grep -n "\.add\|\.remove_at\|\.insert" writ-std/src/collections.writ` should return no matches |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
