---
phase: 89
slug: language-reference-and-il-spec
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 89 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdbook build + HTML link verification |
| **Config file** | docs/book.toml |
| **Quick run command** | `cd docs && mdbook build 2>&1` |
| **Full suite command** | `cd docs && mdbook build 2>&1 && find target/book -name "*.html" -path "*/language-ref/*" \| wc -l` |
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
| 89-01-01 | 01 | 1 | LANG-01, IL-01 | cli | `cd docs && mdbook build 2>&1` | ✅ | ⬜ pending |
| 89-01-02 | 01 | 1 | LANG-03 | cli | `grep -c "](../" language-spec/spec/*.md` | ❌ W0 | ⬜ pending |
| 89-01-03 | 01 | 1 | IL-02 | cli | `cd docs && mdbook build 2>&1` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| All chapters browsable in sidebar | LANG-01, IL-01 | Visual navigation | Run `mdbook serve docs/`, verify all chapters load |
| Tables render correctly | IL-02 | Visual rendering | Check IL spec chapters for table formatting |
| Cross-references resolve | LANG-03 | Link navigation | Click cross-reference links, verify they navigate correctly |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
