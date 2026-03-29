---
phase: 90
slug: getting-started-and-architecture
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 90 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | mdbook build + content grep |
| **Config file** | docs/book.toml |
| **Quick run command** | `cd docs && mdbook build 2>&1` |
| **Full suite command** | `cd docs && mdbook build 2>&1 && grep -l "cargo build" target/book/getting-started/installation.html` |
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
| 90-01-01 | 01 | 1 | START-01 | cli | `grep "cargo build" docs/src/getting-started/installation.md` | ❌ W0 | ⬜ pending |
| 90-01-02 | 01 | 1 | START-02 | cli | `grep "Hello" docs/src/getting-started/hello-world.md` | ❌ W0 | ⬜ pending |
| 90-01-03 | 01 | 1 | START-03 | cli | `grep "compile" docs/src/getting-started/cli-reference.md` | ❌ W0 | ⬜ pending |
| 90-01-04 | 01 | 1 | ARCH-01, ARCH-02, ARCH-03 | cli | `grep "parse" docs/src/architecture/pipeline.md` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `docs/src/getting-started/installation.md` — new prose chapter
- [ ] `docs/src/getting-started/hello-world.md` — new prose chapter
- [ ] `docs/src/getting-started/cli-reference.md` — new prose chapter
- [ ] `docs/src/architecture/pipeline.md` — new prose chapter
- [ ] `docs/src/architecture/crate-map.md` — new prose chapter

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Chapters appear in sidebar navigation | All | Visual check | Run `mdbook serve docs/`, verify new sections visible |
| Installation steps are followable | START-01 | Human comprehension | Read installation page, verify steps are clear |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
