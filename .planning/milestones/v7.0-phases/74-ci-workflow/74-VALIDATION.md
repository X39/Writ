---
phase: 74
slug: ci-workflow
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-20
---

# Phase 74 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | GitHub Actions YAML lint + manual trigger verification |
| **Config file** | `.github/workflows/benchmark.yml` |
| **Quick run command** | `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/benchmark.yml'))"` |
| **Full suite command** | `gh workflow run benchmark.yml` (requires push to remote) |
| **Estimated runtime** | ~2 seconds (YAML lint) |

---

## Sampling Rate

- **After every task commit:** Run YAML syntax validation
- **After every plan wave:** Validate workflow structure against CI-01/CI-02/CI-03
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 2 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 74-01-01 | 01 | 1 | CI-01 | structural | `grep 'workflow_dispatch' .github/workflows/benchmark.yml` | ❌ W0 | ⬜ pending |
| 74-01-02 | 01 | 1 | CI-02 | structural | `grep 'schedule' .github/workflows/benchmark.yml` | ❌ W0 | ⬜ pending |
| 74-01-03 | 01 | 1 | CI-03 | structural | `grep 'upload-artifact' .github/workflows/benchmark.yml` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `.github/workflows/benchmark.yml` — the single deliverable file for this phase

*Existing CI infrastructure (rust.yml, vscode-extension.yml) provides patterns but no test framework is needed for a workflow file.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| workflow_dispatch triggers from Actions UI | CI-01 | Requires GitHub remote + UI interaction | Push branch, go to Actions tab, click "Run workflow" |
| Weekly schedule fires automatically | CI-02 | Requires waiting for cron trigger | Verify cron expression; manual trigger proves the job works |
| Artifacts downloadable from run summary | CI-03 | Requires completed CI run | After manual trigger, check Artifacts section on run page |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 2s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
