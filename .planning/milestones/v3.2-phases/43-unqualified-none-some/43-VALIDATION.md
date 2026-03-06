---
phase: 43
slug: unqualified-none-some
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 43 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + `writ-compiler/tests/` integration tests |
| **Config file** | `Cargo.toml` workspace |
| **Quick run command** | `cargo test -p writ-compiler unqualified` |
| **Full suite command** | `cargo test -p writ-compiler && cargo test -p writ-golden` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-compiler unqualified`
- **After every plan wave:** Run `cargo test -p writ-compiler && cargo test -p writ-golden`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 43-01-01 | 01 | 0 | LANG-02-A,B,C,E,F,G,H,I | unit stubs | `cargo test -p writ-compiler` | ❌ W0 | ⬜ pending |
| 43-01-02 | 01 | 1 | LANG-02 | unit resolve | `cargo test -p writ-compiler unqualified` | ❌ W0 | ⬜ pending |
| 43-01-03 | 01 | 1 | LANG-02-A,B,H | unit typecheck | `cargo test -p writ-compiler unqualified` | ❌ W0 | ⬜ pending |
| 43-01-04 | 01 | 1 | LANG-02-E | unit typecheck | `cargo test -p writ-compiler bare_none` | ❌ W0 | ⬜ pending |
| 43-01-05 | 01 | 1 | LANG-02-F,G,I | unit resolve | `cargo test -p writ-compiler using_glob` | ❌ W0 | ⬜ pending |
| 43-01-06 | 01 | 1 | LANG-02-C | unit typecheck | `cargo test -p writ-compiler user_none_shadows` | ❌ W0 | ⬜ pending |
| 43-01-07 | 01 | 2 | LANG-02-D | golden | `cargo test -p writ-golden` | ✅ existing | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `writ-compiler/tests/typecheck_tests.rs` — add test stubs: `none_unqualified_with_annotation`, `some_unqualified_infers_type`, `none_some_in_pattern_position`, `user_none_shadows_builtin`, `bare_none_no_annotation_error`
- [ ] `writ-compiler/tests/resolve_tests.rs` — add test stubs: `using_enum_glob`, `using_glob_conflict_ambiguous`, `using_option_glob_redundant_no_error`

*LANG-02-D already covered by the existing `fn_optional` golden test — no new test needed for that case.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec subsection for `using Enum::*;` | LANG-02 | Doc review | Read `language-spec/spec/24_23_modules_namespaces.md` and confirm new subsection is present and correct |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
