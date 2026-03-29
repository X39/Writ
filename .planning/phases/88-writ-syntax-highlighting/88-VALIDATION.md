---
phase: 88
slug: writ-syntax-highlighting
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 88 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdbook build + grep HTML inspection |
| **Config file** | docs/book.toml |
| **Quick run command** | `cd docs && mdbook build 2>&1` |
| **Full suite command** | `cd docs && mdbook build 2>&1 && grep -c "hljs-keyword" target/book/language-ref/syntax.html` |
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
| 88-01-01 | 01 | 1 | INFRA-04 | cli | `test -f docs/theme/highlight.js && grep -c "registerLanguage.*writ" docs/theme/highlight.js` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `docs/theme/highlight.js` — custom highlight.js with Writ language definition

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Keywords render in distinct colors | INFRA-04 | Visual rendering check | Run `mdbook serve docs/`, open a chapter with Writ code, verify keyword coloring |
| Format strings highlighted as strings | INFRA-04 | Visual rendering check | Check `$"..."` blocks render with string color |
| Comments highlighted distinctly | INFRA-04 | Visual rendering check | Check `//` and `/* */` blocks render as comments |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
