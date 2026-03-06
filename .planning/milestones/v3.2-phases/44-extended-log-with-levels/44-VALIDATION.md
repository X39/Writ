---
phase: 44
slug: extended-log-with-levels
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 44 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + cargo test |
| **Config file** | `Cargo.toml` (workspace) |
| **Quick run command** | `cargo test -p writ-compiler -- log 2>&1` |
| **Full suite command** | `cargo test --workspace 2>&1` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler && cargo test -p writ-golden`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 44-01-01 | 01 | 1 | TOOL-03 | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ✅ `writ-compiler/tests/typecheck_tests.rs` | ⬜ pending |
| 44-01-02 | 01 | 1 | TOOL-03 | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ W0: add log level tests | ⬜ pending |
| 44-01-03 | 01 | 1 | TOOL-03 | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ W0: add negative test | ⬜ pending |
| 44-01-04 | 01 | 1 | TOOL-03 | golden | `cargo test -p writ-golden -- fn_log_say_choice 2>&1` | ✅ (after re-bless) | ⬜ pending |
| 44-01-05 | 01 | 1 | TOOL-03 | unit | `cargo test -p writ-cli -- on_log 2>&1` | ❌ W0: add CliHost test | ⬜ pending |
| 44-01-06 | 01 | 1 | TOOL-03 | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ W0: root-qualified test | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_namespace_compiles` covering all 5 levels
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_bare_fails` (negative: `log("msg")` without extern decl produces error)
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_root_qualified` (`::log::debug(msg)` succeeds)
- [ ] `writ-cli/src/cli_host.rs` (tests section) — add `test_on_log_debug_prefix` verifying `[DEBUG]` format

*Existing infrastructure covers golden test needs (fn_log_say_choice).*

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
