---
phase: 70
slug: docker-environment-and-measurement-harness
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-20
---

# Phase 70 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash + Docker (integration tests via shell scripts) |
| **Config file** | none — infrastructure created in this phase |
| **Quick run command** | `docker build -t writ-bench -f benchmark/runner/Dockerfile .` |
| **Full suite command** | `bash benchmark/runner/run.sh` |
| **Estimated runtime** | ~120 seconds (Docker build + stub benchmark) |

---

## Sampling Rate

- **After every task commit:** Run `docker build -t writ-bench -f benchmark/runner/Dockerfile .`
- **After every plan wave:** Run `bash benchmark/runner/run.sh`
- **Before `/gsd:verify-work`:** Full suite must produce valid `raw.json`
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 70-01-01 | 01 | 1 | INFRA-01 | integration | `docker build -t writ-bench -f benchmark/runner/Dockerfile .` | ❌ W0 | ⬜ pending |
| 70-01-02 | 01 | 1 | INFRA-01 | integration | `docker run --rm writ-bench --versions` | ❌ W0 | ⬜ pending |
| 70-02-01 | 02 | 1 | INFRA-04 | integration | `bash benchmark/runner/run.sh` | ❌ W0 | ⬜ pending |
| 70-02-02 | 02 | 1 | INFRA-05 | integration | `grep memory_kb benchmark/results/raw.json` | ❌ W0 | ⬜ pending |
| 70-02-03 | 02 | 1 | INFRA-06 | integration | `grep startup_ms benchmark/results/raw.json` | ❌ W0 | ⬜ pending |
| 70-02-04 | 02 | 1 | INFRA-07 | integration | `grep compile_ms benchmark/results/raw.json` | ❌ W0 | ⬜ pending |
| 70-03-01 | 03 | 1 | INFRA-02 | integration | `bash benchmark/runner/run.sh` | ❌ W0 | ⬜ pending |
| 70-03-02 | 03 | 1 | INFRA-03 | integration | `pwsh benchmark/runner/run.ps1` | ❌ W0 | ⬜ pending |
| 70-03-03 | 03 | 1 | INFRA-08 | integration | `python3 -c "import json; json.load(open('benchmark/results/raw.json'))"` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `benchmark/runner/Dockerfile` — multi-stage Docker image with all 6 runtimes
- [ ] `benchmark/runner/bench_runner.sh` — in-container orchestration script
- [ ] `benchmark/runner/run.sh` — host-side Linux/macOS launcher
- [ ] `benchmark/runner/run.ps1` — host-side Windows launcher
- [ ] `benchmark/cases/stub/` — stub benchmark files for all 6 languages

*All infrastructure is new — no existing test framework to leverage.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Docker image builds on CI runner | INFRA-01 | Requires GitHub Actions environment | Push to branch, trigger workflow_dispatch |
| PowerShell script works on Windows | INFRA-03 | Requires Windows Docker Desktop | Run `.\benchmark\runner\run.ps1` on Windows host |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
