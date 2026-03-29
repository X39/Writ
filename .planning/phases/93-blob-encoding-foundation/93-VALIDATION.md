---
phase: 93
slug: blob-encoding-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 93 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p writ-module --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-module --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 93-01-01 | 01 | 1 | BLOB-03 | unit | `cargo test -p writ-module attr` | ❌ W0 | ⬜ pending |
| 93-01-02 | 01 | 1 | BLOB-01, BLOB-02 | unit | `cargo test -p writ-module attr::tests::round_trip` | ❌ W0 | ⬜ pending |
| 93-02-01 | 02 | 2 | BLOB-01 | integration | `cargo test -p writ-compiler emit` | ✅ | ⬜ pending |
| 93-02-02 | 02 | 2 | BLOB-02 | integration | `cargo test --workspace` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-module/src/attr.rs` — AttrValue enum, ATTR_TAG_* constants, encode/decode functions
- [ ] `writ-module/src/attr.rs` (tests module) — round-trip test for string, int, bool, named args

*Existing test infrastructure (cargo test) covers the framework requirement.*

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
