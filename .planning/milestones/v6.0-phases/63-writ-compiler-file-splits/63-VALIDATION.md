---
phase: 63
slug: writ-compiler-file-splits
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 63 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test runner + insta 1.x (snapshot tests) |
| **Config file** | `writ-compiler/Cargo.toml` (dev-dependencies: insta) |
| **Quick run command** | `cargo test -p writ-compiler` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 63-01-01 | 01 | 1 | SPLIT-03 | regression | `cargo test -p writ-compiler typecheck` | ✅ | ⬜ pending |
| 63-02-01 | 02 | 1 | SPLIT-04 | regression | `cargo test -p writ-compiler emit` | ✅ | ⬜ pending |
| 63-03-01 | 03 | 1 | SPLIT-05 | regression | `cargo test -p writ-compiler emit_body` | ✅ | ⬜ pending |
| 63-04-01 | 04 | 1 | SPLIT-09 | regression | `cargo test --workspace` | ✅ | ⬜ pending |
| 63-05-01 | 05 | 2 | SPLIT-08, SPLIT-10, SPLIT-11 | build+regression | `cargo clippy -p writ-compiler && cargo test -p writ-compiler` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new test files or frameworks needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| No `pub use *` glob re-exports | Success Criterion 4 | Grep-based check | `grep -r "pub use.*\*" writ-compiler/src/` should return empty |
| Each new submodule has single responsibility | Success Criterion 2 | Code review | Verify each new file's content matches its name/purpose |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
