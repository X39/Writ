---
phase: 87
slug: mdbook-scaffold
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 87 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdbook build (CLI validation) |
| **Config file** | docs/book.toml |
| **Quick run command** | `cd docs && mdbook build 2>&1` |
| **Full suite command** | `cd docs && mdbook build 2>&1 && grep -l "base href" target/book/index.html` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd docs && mdbook build 2>&1`
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 87-01-01 | 01 | 1 | INFRA-01 | cli | `test -f docs/book.toml` | ❌ W0 | ⬜ pending |
| 87-01-02 | 01 | 1 | INFRA-02 | cli | `test -f docs/src/SUMMARY.md` | ❌ W0 | ⬜ pending |
| 87-01-03 | 01 | 1 | INFRA-03 | cli | `cd docs && mdbook build 2>&1` | ❌ W0 | ⬜ pending |
| 87-01-04 | 01 | 1 | LANG-02 | cli | `head -1 docs/src/language-ref/*.md \| grep -v "^# 1\."` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `docs/book.toml` — mdBook configuration
- [ ] `docs/src/SUMMARY.md` — chapter navigation
- [ ] `mdbook` CLI installed (cargo install mdbook --version 0.4.51)

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Sidebar shows distinct chapter titles | INFRA-02 | Visual check in browser | Run `mdbook serve`, verify sidebar titles differ |
| Admonish callout boxes render styled | INFRA-03 | Visual rendering | Check note/warning/tip blocks in browser |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
