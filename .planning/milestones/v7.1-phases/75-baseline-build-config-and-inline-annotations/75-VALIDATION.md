---
phase: 75
slug: baseline-build-config-and-inline-annotations
status: finalized
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-22
---

# Phase 75 — Validation Strategy

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
| 75-01-01 | 01 | 1 | BUILD-01 | build | `cargo build --release 2>&1` | ✅ | ✅ green |
| 75-01-02 | 01 | 1 | BUILD-02 | build | `cargo build --release 2>&1` | ✅ | ✅ green |
| 75-01-03 | 01 | 1 | BUILD-03 | build | `cargo build --release 2>&1` | ✅ | ✅ green |
| 75-02-01 | 02 | 1 | BUILD-04 | grep | `grep "FxHashMap" writ-runtime/src/scheduler.rs` | ✅ | ✅ green |
| 75-02-02 | 02 | 1 | BUILD-05 | grep | `grep "FxHashMap" writ-runtime/src/dispatch.rs` | ✅ | ✅ green |
| 75-03-01 | 03 | 2 | INLINE-01 | grep | `grep "#\[inline\]" writ-runtime/src/helpers.rs` | ✅ | ✅ green |
| 75-03-02 | 03 | 2 | INLINE-02 | grep | `grep "#\[inline\]" writ-runtime/src/arith.rs` | ✅ | ✅ green |
| 75-03-03 | 03 | 2 | INLINE-03 | grep | `grep -c "#\[inline\]" writ-runtime/src/mod.rs` | ✅ | ✅ green |
| 75-03-04 | 03 | 2 | INLINE-04 | grep | `grep -v "#\[inline\]" writ-runtime/src/mod.rs \| grep "fn execute_one"` | ✅ | ✅ green |
| 75-04-01 | 04 | 3 | VERIFY-01 | benchmark | `cargo run --release -- benchmark/cases/fib/fib40.writ` | ✅ | ✅ green |
| 75-04-02 | 04 | 3 | VERIFY-02 | test | `cargo test --release 2>&1` | ✅ | ✅ green |
| 75-04-03 | 04 | 3 | VERIFY-03 | build | `cargo build --release 2>&1 \| grep -c warning` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. cargo test and cargo build are already configured.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| cargo-bloat unwind table check | BUILD-01 | Requires cargo-bloat install | `cargo bloat --release --filter unwind` — verify no unwind tables |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-22
