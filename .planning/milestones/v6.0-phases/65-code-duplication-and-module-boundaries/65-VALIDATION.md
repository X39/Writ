---
phase: 65
slug: code-duplication-and-module-boundaries
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 65 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo check --workspace` |
| **Full suite command** | `cargo test --workspace --no-fail-fast` |
| **Estimated runtime** | ~45 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace`
- **After every plan wave:** Run `cargo test --workspace --no-fail-fast`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 45 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 65-01-01 | 01 | 1 | DUP-01, DUP-02 | unit | `cargo test -p writ-compiler --test lowering_tests` | ✅ | ⬜ pending |
| 65-02-01 | 02 | 1 | MOD-02 | static | `cargo check --workspace` | ✅ | ⬜ pending |
| 65-03-01 | 03 | 2 | MOD-03 | static | `cargo check --workspace` | ✅ | ⬜ pending |
| 65-04-01 | 04 | 2 | MOD-01 | manual | `cargo doc --workspace --no-deps 2>&1 | grep "warning"` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.*

- No new test files needed — lowering tests validate DUP-01/DUP-02
- `cargo check --workspace` validates MOD-02 and MOD-03
- `cargo doc` validates MOD-01 doc headers

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Module doc headers present and accurate | MOD-01 | Content quality requires human review | Read each lib.rs `//!` header; verify it describes the actual module hierarchy |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 45s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
