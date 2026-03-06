# Phase 74: CI Workflow - Context

**Gathered:** 2026-03-20
**Status:** Ready for planning

<domain>
## Phase Boundary

GitHub Actions workflow file (`.github/workflows/benchmark.yml`) that runs the existing Docker-based benchmark suite on manual dispatch and weekly schedule, then uploads results (raw.json, SVG charts, RESULTS.md) as downloadable CI artifacts. No changes to the benchmark harness, Docker image, or chart generation pipeline — this phase wires existing infrastructure into CI.

</domain>

<decisions>
## Implementation Decisions

### Workflow Triggers
- `workflow_dispatch` with an optional `runs` input parameter (default: 10) — lets users configure benchmark iterations from the Actions UI
- `schedule` cron: `0 6 * * 1` — weekly on Monday at 6:00 UTC (start of week, low CI contention)
- Both triggers satisfy CI-01 and CI-02

### Runner and Docker Strategy
- `runs-on: ubuntu-latest` — Docker is pre-installed on GitHub Actions Ubuntu runners, no Docker-in-Docker setup needed
- Build step: `docker build -t writ-bench -f benchmark/runner/Dockerfile .` (same as run.sh)
- Run step: `docker run --rm -v $RESULTS_DIR:/results -e RESULTS_DIR=/results -e RUNS=${{ inputs.runs || 10 }} writ-bench`
- Don't reuse run.sh directly in CI — replicate the core docker commands inline for clarity and to avoid shell portability issues with MINGW path conversion logic

### Chart Generation in CI
- Install `pygal` via `pip install pygal==3.1.0` on the runner (not inside Docker)
- Run `python3 benchmark/generate.py $RESULTS_DIR/raw.json` after the Docker container finishes
- Ubuntu runner has Python 3 pre-installed — no setup-python action needed for this

### Artifact Upload
- Upload the entire `benchmark/results/YYYY-MM-DD/` directory as a single artifact named `benchmark-results-YYYY-MM-DD`
- Use `actions/upload-artifact@v4` (consistent with existing vscode-extension.yml pattern)
- Artifact contains: `raw.json`, per-benchmark SVG charts, memory/startup SVG charts, `RESULTS.md`
- No artifact retention override — use GitHub's default (90 days)

### Claude's Discretion
- Exact step names and ordering within the workflow
- Whether to add a concurrency group to prevent parallel benchmark runs
- Cache strategy for Docker layers (if any)
- Whether to echo version info or summary to the Actions job summary

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — CI-01 (workflow_dispatch), CI-02 (weekly schedule), CI-03 (artifact upload)
- `.planning/ROADMAP.md` §Phase 74 — Success criteria (3 items)

### Existing CI patterns
- `.github/workflows/rust.yml` — Simple push/PR workflow pattern (actions/checkout@v4)
- `.github/workflows/vscode-extension.yml` — Artifact upload pattern (actions/upload-artifact@v4, cargo cache)

### Benchmark infrastructure (consumed, not modified)
- `benchmark/runner/Dockerfile` — Docker image definition (context is repo root)
- `benchmark/runner/bench_runner.sh` — Container entrypoint that runs all benchmarks
- `benchmark/runner/run.sh` — Host runner script (reference for docker build/run commands, NOT used directly in CI)
- `benchmark/generate.py` — Chart generation script (invoked after Docker run)

### Prior phase context
- `.planning/phases/70-docker-environment-and-measurement-harness/70-CONTEXT.md` — Docker architecture decisions, JSON schema, measurement approach

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `actions/checkout@v4` — used in both existing workflows
- `actions/upload-artifact@v4` — used in vscode-extension.yml for VSIX upload
- `actions/cache@v4` — used in vscode-extension.yml for cargo cache (may be useful for Docker layer caching)

### Established Patterns
- Workflows live in `.github/workflows/` with descriptive YAML filenames
- `ubuntu-latest` runner for all CI jobs
- Simple single-job structure in rust.yml; multi-job with dependencies in vscode-extension.yml

### Integration Points
- New file: `.github/workflows/benchmark.yml` (no conflicts with existing rust.yml and vscode-extension.yml)
- Docker build context: repo root (same as run.sh)
- Results directory: `benchmark/results/YYYY-MM-DD/` (same as run.sh)

</code_context>

<specifics>
## Specific Ideas

- STATE.md decision: "CI numbers are not authoritative — 15% regression threshold; publishable numbers from local Docker runs on stable machine" — CI is for convenience/automation, not authoritative performance numbers
- Don't commit results from CI back to the repo — artifacts only (avoids noisy auto-commits from scheduled runs)
- The workflow should work identically whether triggered manually or by schedule

</specifics>

<deferred>
## Deferred Ideas

- CI-04 (regression detection with configurable threshold) — listed in REQUIREMENTS.md as v7.1+ future requirement
- REPORT-06 (historical trend charts) — v7.1+ future requirement
- Auto-commit results from CI — deliberately excluded; CI artifacts are ephemeral, local runs produce committed results

</deferred>

---

*Phase: 74-ci-workflow*
*Context gathered: 2026-03-20*
