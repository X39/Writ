---
phase: 72
slug: chart-generation-and-results-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-20
---

# Phase 72 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Python 3.10+ with pygal 3.1.0 (script validation via CLI) |
| **Config file** | none — generate.py is standalone |
| **Quick run command** | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json` |
| **Full suite command** | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && diff <(python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json) benchmark/results/2026-03-20/` |
| **Estimated runtime** | ~2 seconds |

---

## Sampling Rate

- **After every task commit:** Run `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json`
- **After every plan wave:** Verify all output files exist and re-run produces identical output
- **Before `/gsd:verify-work`:** Full determinism check must pass
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 72-01-01 | 01 | 1 | REPORT-01 | integration | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && ls benchmark/results/2026-03-20/*exec*.svg` | ❌ W0 | ⬜ pending |
| 72-01-02 | 01 | 1 | REPORT-02 | integration | `ls benchmark/results/2026-03-20/*memory*.svg` | ❌ W0 | ⬜ pending |
| 72-01-03 | 01 | 1 | REPORT-03 | integration | `ls benchmark/results/2026-03-20/*startup*.svg` | ❌ W0 | ⬜ pending |
| 72-01-04 | 01 | 1 | REPORT-04 | integration | `grep 'Ratio to Rust' benchmark/results/2026-03-20/RESULTS.md` | ❌ W0 | ⬜ pending |
| 72-01-05 | 01 | 1 | REPORT-05 | integration | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json` (determinism re-run) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `benchmark/generate.py` — main script (created by plan tasks)
- [ ] `pip install pygal` — pygal 3.1.0 must be available on host

*Existing benchmark/results/2026-03-20/raw.json provides test input data.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| SVG renders correctly in browser | REPORT-01 | Visual check | Open any .svg file in browser, verify bars/labels/tooltips display |
| RESULTS.md renders in GitHub | REPORT-04 | GitHub rendering | Preview RESULTS.md in GitHub or VS Code markdown preview |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
