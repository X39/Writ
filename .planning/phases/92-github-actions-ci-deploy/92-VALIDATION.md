---
phase: 92
slug: github-actions-ci-deploy
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-27
---

# Phase 92 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | GitHub Actions workflow YAML validation |
| **Config file** | .github/workflows/docs.yml |
| **Quick run command** | `python -c "import yaml; yaml.safe_load(open('.github/workflows/docs.yml'))" && echo "VALID"` |
| **Full suite command** | `grep -c "deploy-pages" .github/workflows/docs.yml && grep -c "mdbook-v0.4.51" .github/workflows/docs.yml` |
| **Estimated runtime** | ~1 second |

---

## Sampling Rate

- **After every task commit:** Run quick command
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 1 second

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 92-01-01 | 01 | 1 | CI-01, CI-02, CI-04 | cli | `test -f .github/workflows/docs.yml` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `.github/workflows/docs.yml` — CI/CD workflow file

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Site loads at github.io/Writ/ | CI-04 | Requires live deployment | Push to master, wait for workflow, check URL |
| /Writ/api/ resolves | CI-04 | Requires live deployment | Check /Writ/api/ URL after deploy |
| GitHub Pages source set to Actions | CI-01 | Repo settings change | Verify in Settings > Pages |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
