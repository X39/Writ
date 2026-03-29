---
phase: 80
slug: clone-cleanup-micro-opts
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-22
---

# Phase 80 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test --release -p writ-runtime` |
| **Full suite command** | `cargo test --release` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --release -p writ-runtime`
- **After every plan wave:** Run `cargo test --release`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 80-01-01 | 01 | 1 | VERIFY-01 | unit+grep | `cargo test --release -p writ-runtime && grep -rn '\.clone()' writ-runtime/src/dispatch/` | ✅ | ⬜ pending |
| 80-01-02 | 01 | 1 | VERIFY-02 | unit | `cargo test --release -p writ-runtime` | ✅ | ⬜ pending |
| 80-01-03 | 01 | 1 | VERIFY-03 | build | `cargo build --release 2>&1 | grep warning` | ✅ | ⬜ pending |
| 80-01-04 | 01 | 1 | VERIFY-04 | benchmark | `cargo run --release -- run benchmark/cases/fibonacci/fib.writ` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| fib(40) timing | VERIFY-04 | Performance measurement requires manual observation | Run fib(40) in release mode, record wall-clock time |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
