---
phase: 62
slug: clippy-warning-elimination
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 62 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + cargo clippy |
| **Config file** | Cargo.toml (workspace root) |
| **Quick run command** | `cargo clippy --workspace 2>&1 | grep -c "warning\|error"` |
| **Full suite command** | `cargo clippy --workspace 2>&1 && cargo test --workspace` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo clippy --workspace 2>&1 | grep -c "warning\|error"`
- **After every plan wave:** Run `cargo clippy --workspace 2>&1 && cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 62-01-01 | 01 | 1 | WARN-01 | lint | `cargo clippy --fix --workspace --allow-dirty 2>&1` | ✅ | ⬜ pending |
| 62-01-02 | 01 | 1 | WARN-01 | lint | `cargo clippy --workspace 2>&1 \| grep "never_loop"` | ✅ | ⬜ pending |
| 62-02-01 | 02 | 2 | WARN-02 | lint+test | `cargo clippy --workspace 2>&1 && cargo test --workspace` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `#[allow(...)]` suppressions justified | WARN-02 | Requires human judgment on justification quality | Review each `#[allow]` added in diff — must have adjacent comment explaining why |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
