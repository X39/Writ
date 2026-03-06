# Phase 74: CI Workflow - Research

**Researched:** 2026-03-20
**Domain:** GitHub Actions — workflow triggers, Docker build/run in CI, artifact upload
**Confidence:** HIGH

## Summary

Phase 74 adds a single new file: `.github/workflows/benchmark.yml`. It wires the existing Docker-based benchmark suite (Dockerfile, bench_runner.sh) and Python chart generator (generate.py) into GitHub Actions. No changes to benchmark infrastructure — purely CI orchestration.

The workflow must handle two triggers: `workflow_dispatch` (manual, with `runs` input) and a weekly `schedule` cron. It must build the Docker image, run the container with a volume-mounted results directory, run `generate.py` on the host, and upload the entire dated results directory as a single artifact using `actions/upload-artifact@v4`.

All three decisions are locked in CONTEXT.md. The only areas of planner discretion are step naming/ordering, whether to add a concurrency guard, Docker layer caching, and whether to echo a job summary.

**Primary recommendation:** Write a single-job workflow file following the established pattern from `vscode-extension.yml` (actions/checkout@v4, actions/upload-artifact@v4, ubuntu-latest), with inline Docker commands derived directly from `run.sh` (not invoking `run.sh` itself), and `pip install pygal==3.1.0` before running `generate.py`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Workflow Triggers:**
- `workflow_dispatch` with an optional `runs` input parameter (default: 10) — lets users configure benchmark iterations from the Actions UI
- `schedule` cron: `0 6 * * 1` — weekly on Monday at 6:00 UTC (start of week, low CI contention)
- Both triggers satisfy CI-01 and CI-02

**Runner and Docker Strategy:**
- `runs-on: ubuntu-latest` — Docker is pre-installed on GitHub Actions Ubuntu runners, no Docker-in-Docker setup needed
- Build step: `docker build -t writ-bench -f benchmark/runner/Dockerfile .` (same as run.sh)
- Run step: `docker run --rm -v $RESULTS_DIR:/results -e RESULTS_DIR=/results -e RUNS=${{ inputs.runs || 10 }} writ-bench`
- Don't reuse run.sh directly in CI — replicate the core docker commands inline for clarity and to avoid shell portability issues with MINGW path conversion logic

**Chart Generation in CI:**
- Install `pygal` via `pip install pygal==3.1.0` on the runner (not inside Docker)
- Run `python3 benchmark/generate.py $RESULTS_DIR/raw.json` after the Docker container finishes
- Ubuntu runner has Python 3 pre-installed — no setup-python action needed for this

**Artifact Upload:**
- Upload the entire `benchmark/results/YYYY-MM-DD/` directory as a single artifact named `benchmark-results-YYYY-MM-DD`
- Use `actions/upload-artifact@v4` (consistent with existing vscode-extension.yml pattern)
- Artifact contains: `raw.json`, per-benchmark SVG charts, memory/startup SVG charts, `RESULTS.md`
- No artifact retention override — use GitHub's default (90 days)

### Claude's Discretion
- Exact step names and ordering within the workflow
- Whether to add a concurrency group to prevent parallel benchmark runs
- Cache strategy for Docker layers (if any)
- Whether to echo version info or summary to the Actions job summary

### Deferred Ideas (OUT OF SCOPE)
- CI-04 (regression detection with configurable threshold) — listed in REQUIREMENTS.md as v7.1+ future requirement
- REPORT-06 (historical trend charts) — v7.1+ future requirement
- Auto-commit results from CI — deliberately excluded; CI artifacts are ephemeral, local runs produce committed results
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CI-01 | GitHub Actions workflow runs benchmarks on `workflow_dispatch` trigger | `on.workflow_dispatch` with `inputs.runs` (default 10); confirmed in GitHub Actions docs |
| CI-02 | GitHub Actions workflow runs benchmarks on weekly schedule | `on.schedule` cron `0 6 * * 1` (Monday 06:00 UTC); confirmed in GitHub Actions docs |
| CI-03 | CI results uploaded as artifacts | `actions/upload-artifact@v4` with dated artifact name; pattern already in vscode-extension.yml |
</phase_requirements>

## Standard Stack

### Core
| Library/Action | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| actions/checkout | v4 | Checkout repo for Docker build context | Used in both existing workflows; current stable |
| actions/upload-artifact | v4 | Upload results directory as downloadable artifact | Used in vscode-extension.yml for VSIX; v4 is current stable |
| docker (pre-installed) | 24.x+ | Build and run benchmark container | Pre-installed on ubuntu-latest GitHub-hosted runners; no setup action needed |
| python3 (pre-installed) | 3.x | Run generate.py for chart generation | Pre-installed on ubuntu-latest; no setup-python action needed |
| pygal | 3.1.0 | Python chart library for generate.py | Pinned version matching existing generate.py dependency |

### Supporting (Discretionary)
| Action | Version | Purpose | When to Use |
|--------|---------|---------|-------------|
| actions/cache | v4 | Cache Docker build layers | Useful but optional — build cost for this Dockerfile is high (Rust compile, Squirrel cmake) so cache helps; already used in vscode-extension.yml for cargo |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline docker commands | run.sh | run.sh contains MINGW/Windows path conversion logic irrelevant to Linux CI; inline is cleaner |
| pip install pygal==3.1.0 | setup-python + requirements.txt | No requirements.txt exists; single-package install is simpler |
| ubuntu-latest | ubuntu-22.04 (pinned) | Pinned avoids drift but ubuntu-latest matches existing workflows |

**Installation (workflow-level):**
```bash
pip install pygal==3.1.0
```

No npm or Rust toolchain setup needed — everything runs inside Docker.

## Architecture Patterns

### Recommended Workflow Structure
```
.github/workflows/
├── rust.yml              # existing — push/PR, cargo build+test
├── vscode-extension.yml  # existing — push/tag, VSIX build+publish
└── benchmark.yml         # NEW — manual dispatch + weekly schedule, benchmark + artifact
```

### Pattern 1: Dual-Trigger Workflow
**What:** A single workflow file with both `workflow_dispatch` (with typed inputs) and `schedule` triggers. When triggered by schedule, `inputs.runs` is not set; use `${{ inputs.runs || 10 }}` to fall back to the default.
**When to use:** Whenever a workflow must be runnable both manually and automatically.
**Example:**
```yaml
# Source: GitHub Actions official docs
on:
  workflow_dispatch:
    inputs:
      runs:
        description: 'Number of benchmark iterations'
        required: false
        default: '10'
        type: string
  schedule:
    - cron: '0 6 * * 1'
```

**Key pitfall:** `inputs.runs` is a string type in the workflow YAML even though it's a number. Pass it to the container with `-e RUNS=${{ inputs.runs || 10 }}`. The bench_runner.sh reads `RUNS="${RUNS:-10}"` which handles both cases.

### Pattern 2: Docker Build + Volume-Mounted Run (CI-safe)
**What:** Run Docker without Docker-in-Docker. GitHub Actions ubuntu-latest runners have Docker installed. Build the image, then run with a host volume mount to capture results.
**When to use:** Any Docker-based workflow on ubuntu-latest.
**Example:**
```yaml
- name: Build benchmark image
  run: docker build -t writ-bench -f benchmark/runner/Dockerfile .

- name: Run benchmarks
  run: |
    RESULTS_DIR="$GITHUB_WORKSPACE/benchmark/results/$(date +%Y-%m-%d)"
    mkdir -p "$RESULTS_DIR"
    docker run --rm \
      -v "$RESULTS_DIR:/results" \
      -e RESULTS_DIR=/results \
      -e RUNS=${{ inputs.runs || 10 }} \
      writ-bench
    echo "RESULTS_DIR=$RESULTS_DIR" >> $GITHUB_ENV
```

**Key note:** On Linux CI (no MINGW), no path conversion is needed. The `//results` double-slash workaround from run.sh is Windows-only — use single `/results` in CI.

### Pattern 3: Artifact Upload with Dynamic Name
**What:** Upload a dated directory as a single artifact with a date-stamped name.
**When to use:** When artifact names must encode when they were produced.
**Example:**
```yaml
# Source: vscode-extension.yml pattern + official upload-artifact@v4 docs
- name: Upload benchmark artifacts
  uses: actions/upload-artifact@v4
  with:
    name: benchmark-results-${{ env.DATE }}
    path: benchmark/results/${{ env.DATE }}/
```

The `DATE` env var is set earlier in the job:
```yaml
- name: Set date
  run: echo "DATE=$(date +%Y-%m-%d)" >> $GITHUB_ENV
```

### Pattern 4: Concurrency Guard (Discretionary)
**What:** Prevent two benchmark runs from executing simultaneously on the same branch (e.g., if a scheduled run is still going when a manual dispatch fires).
**When to use:** Recommended for long-running jobs — this Docker build + benchmark run is 10-30+ minutes.
**Example:**
```yaml
concurrency:
  group: benchmark-${{ github.ref }}
  cancel-in-progress: false
```

`cancel-in-progress: false` means a second run waits rather than canceling the in-flight run. This is safer than `true` for benchmarks (partial runs produce no artifact).

### Anti-Patterns to Avoid
- **Calling run.sh from CI:** Contains MINGW path conversion logic that is harmless on Linux but adds unnecessary complexity and coupling. Replicate the three core docker commands inline.
- **Using `workflow_dispatch` inputs without fallback:** `${{ inputs.runs }}` is empty when triggered by schedule — always use `${{ inputs.runs || 10 }}`.
- **Uploading only raw.json:** The decision is to upload the entire `benchmark/results/YYYY-MM-DD/` directory (raw.json + all SVGs + RESULTS.md) as a single artifact.
- **Installing python/pip via setup-python action:** Ubuntu-latest already has Python 3 and pip; `pip install pygal==3.1.0` is sufficient.
- **Committing results from CI:** Explicitly excluded. Results go to artifacts only, not git commits.
- **Using `//results` double-slash in docker run volume:** This is the MINGW workaround from run.sh. On Linux CI, use single `/results`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Date formatting | Custom date logic | `$(date +%Y-%m-%d)` in bash | Built-in, reliable |
| Artifact name with date | Manual string concat | `${{ env.DATE }}` (set via GITHUB_ENV) | Clean, readable, works across steps |
| Docker availability check | Docker/Podman detection (from run.sh) | None — ubuntu-latest always has Docker | The run.sh check is for local portability; CI always uses docker |
| `runs` input fallback | Complex conditional | `${{ inputs.runs || 10 }}` | GitHub Actions expression handles this in one token |

**Key insight:** The benchmark infrastructure is already built. This phase is pure orchestration — the right approach minimizes custom logic and uses battle-tested actions.

## Common Pitfalls

### Pitfall 1: inputs.runs is Empty on Schedule Trigger
**What goes wrong:** `${{ inputs.runs }}` evaluates to empty string when triggered by `schedule`. Passing an empty `RUNS=` to the container causes bench_runner.sh's `RUNS="${RUNS:-10}"` to see an empty value (not unset), so it does NOT fall back to 10 — it gets `RUNS=""`.
**Why it happens:** `workflow_dispatch.inputs` are only populated when the workflow is triggered by `workflow_dispatch`, not `schedule`.
**How to avoid:** Use the expression `${{ inputs.runs || 10 }}` everywhere `inputs.runs` is referenced. This yields `10` when inputs.runs is empty/unset.
**Warning signs:** Benchmark output shows `RUNS=0` or hyperfine errors about iteration count.

### Pitfall 2: RESULTS_DIR Not Available Across Steps
**What goes wrong:** Setting `RESULTS_DIR` as a shell variable in one `run:` step makes it unavailable to subsequent steps (each step runs in a new shell).
**Why it happens:** Each `run:` step is an isolated bash subprocess.
**How to avoid:** Write to `$GITHUB_ENV` using `echo "RESULTS_DIR=..." >> $GITHUB_ENV`. This makes the variable available to all subsequent steps in the job as `${{ env.RESULTS_DIR }}`.
**Warning signs:** "No such file or directory" errors in the upload step because the path resolves to empty.

### Pitfall 3: Docker Build Context Must Be Repo Root
**What goes wrong:** If `docker build` is run from a subdirectory, the COPY instructions in the Dockerfile (e.g., `COPY Cargo.toml`, `COPY writ-assembler/`) fail because they reference paths relative to the build context.
**Why it happens:** Dockerfile uses repo-root-relative paths for COPY.
**How to avoid:** Run `docker build -t writ-bench -f benchmark/runner/Dockerfile .` from `$GITHUB_WORKSPACE` (the default working directory in GitHub Actions after checkout). Do NOT use `working-directory:` to change into a subdirectory.
**Warning signs:** Docker build errors like `COPY failed: file not found in build context`.

### Pitfall 4: Artifact Upload Path Must Include Trailing Slash or Glob
**What goes wrong:** `actions/upload-artifact@v4` with `path: benchmark/results/2026-03-20` uploads the directory but may behave differently depending on whether a trailing slash is included.
**Why it happens:** v4 behavior: without trailing slash, the directory name itself is included in the artifact; with trailing slash, only the contents are included.
**How to avoid:** Use `path: benchmark/results/${{ env.DATE }}/` with trailing slash to upload the directory's contents, or omit the slash to include the directory itself. Either works — just be consistent. The decision specifies uploading the entire directory, so `benchmark/results/${{ env.DATE }}/` is correct.
**Warning signs:** Artifacts with an extra nesting level in the download.

### Pitfall 5: Benchmark Docker Build Time on GitHub-Hosted Runners
**What goes wrong:** The Dockerfile compiles Writ from source (`cargo build --release`) and builds Squirrel from source (cmake). This is a 5-15 minute build without caching.
**Why it happens:** No layer caching by default in GitHub Actions.
**How to avoid (discretionary):** Add Docker layer caching via `actions/cache@v4` or GitHub's built-in `cache-from` buildx option. However, multi-stage builds with Rust compilation have limited cache effectiveness because the Cargo.lock change invalidates cargo layers. Acceptable to skip cache for v7.0; add if build time is excessive.
**Warning signs:** Every workflow run takes 15+ minutes just on the Docker build step.

### Pitfall 6: generate.py Fails if raw.json Not Present
**What goes wrong:** If the Docker container fails (non-zero exit), the `run: python3 benchmark/generate.py ...` step still executes and crashes with "Error: raw.json does not exist."
**Why it happens:** By default, steps continue after a failed step unless `set -e` stops the job, but the Docker run error might be caught by `set -e` in the workflow.
**How to avoid:** With `set -euo pipefail` semantics, a Docker container non-zero exit will stop the job before reaching generate.py. No special handling needed — GitHub Actions stops on non-zero exit codes by default for `run:` steps. The bench_runner.sh also uses `set -euo pipefail` internally.
**Warning signs:** N/A — this is a non-issue given default workflow behavior. Mention for clarity.

## Code Examples

Verified patterns from existing workflows and official docs:

### Complete Workflow Skeleton
```yaml
# Source: based on vscode-extension.yml pattern + GitHub Actions docs
name: Benchmarks

on:
  workflow_dispatch:
    inputs:
      runs:
        description: 'Benchmark iterations (default: 10)'
        required: false
        default: '10'
        type: string
  schedule:
    - cron: '0 6 * * 1'   # Monday 06:00 UTC

concurrency:
  group: benchmark-${{ github.ref }}
  cancel-in-progress: false

jobs:
  benchmark:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Set date and results directory
        run: |
          echo "DATE=$(date +%Y-%m-%d)" >> $GITHUB_ENV
          echo "RESULTS_DIR=$GITHUB_WORKSPACE/benchmark/results/$(date +%Y-%m-%d)" >> $GITHUB_ENV

      - name: Create results directory
        run: mkdir -p "${{ env.RESULTS_DIR }}"

      - name: Build benchmark image
        run: docker build -t writ-bench -f benchmark/runner/Dockerfile .

      - name: Run benchmarks
        run: |
          docker run --rm \
            -v "${{ env.RESULTS_DIR }}:/results" \
            -e RESULTS_DIR=/results \
            -e RUNS=${{ inputs.runs || 10 }} \
            writ-bench

      - name: Install pygal
        run: pip install pygal==3.1.0

      - name: Generate charts and RESULTS.md
        run: python3 benchmark/generate.py "${{ env.RESULTS_DIR }}/raw.json"

      - name: Upload benchmark artifacts
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results-${{ env.DATE }}
          path: ${{ env.RESULTS_DIR }}/
```

### GITHUB_ENV Pattern (Multi-Step Variable Sharing)
```yaml
# Source: GitHub Actions docs — sharing data between steps
- name: Set date
  run: echo "DATE=$(date +%Y-%m-%d)" >> $GITHUB_ENV

# Later steps access as: ${{ env.DATE }}
```

### inputs Fallback for Schedule Compatibility
```yaml
# Source: GitHub Actions docs — workflow_dispatch + schedule dual trigger
-e RUNS=${{ inputs.runs || 10 }}
```

### Existing Artifact Upload Pattern (from vscode-extension.yml)
```yaml
# Source: .github/workflows/vscode-extension.yml
- name: Upload VSIX artifact
  uses: actions/upload-artifact@v4
  with:
    name: writ-vsix
    path: writ-vscode/*.vsix
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| upload-artifact@v2/v3 | upload-artifact@v4 | 2023-2024 | v4 is faster, required for Node.js 20; v2/v3 deprecated Nov 2024 |
| docker/setup-buildx-action for caching | actions/cache@v4 + --cache-from | ongoing | Both approaches valid; inline cache-from simpler for single-machine builds |
| `github.event.inputs.X` | `inputs.X` context | 2021+ | `inputs` context preferred; preserves boolean types; both work for strings |

**Deprecated/outdated:**
- `actions/upload-artifact@v2` and `@v3`: Deprecated November 2024; use v4.
- `github.event.inputs` (for accessing dispatch inputs): Still works but `inputs` context is preferred.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Manual inspection — GitHub Actions workflows are validated by triggering them |
| Config file | `.github/workflows/benchmark.yml` (new file) |
| Quick run command | `act -W .github/workflows/benchmark.yml -e .planning/phases/74-ci-workflow/act-event.json` (if `act` installed) |
| Full suite command | Trigger via GitHub Actions UI (workflow_dispatch) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CI-01 | workflow_dispatch trigger appears in Actions UI and runs successfully | manual | Trigger via GitHub Actions UI | ❌ Wave 0 (new file) |
| CI-02 | Weekly schedule cron executes automatically on Monday | manual | Observe scheduled run OR use `act --schedule` | ❌ Wave 0 (new file) |
| CI-03 | Artifacts appear in run summary after job completes | manual | Check Actions run summary for artifact download link | ❌ Wave 0 (new file) |

**Note:** GitHub Actions workflows cannot be unit-tested locally without `act`. All three requirements are manual-only because they depend on the GitHub Actions runtime environment (runner, artifact storage, schedule system). Local validation is limited to YAML syntax checking (`yamllint`) and reviewing the workflow structure.

### Sampling Rate
- **Per task commit:** `yamllint .github/workflows/benchmark.yml` (syntax check, if yamllint installed)
- **Per wave merge:** Trigger workflow_dispatch from GitHub Actions UI and verify artifact appears
- **Phase gate:** All three success criteria confirmed by manual Actions run before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `.github/workflows/benchmark.yml` — the entire deliverable (new file; covers CI-01, CI-02, CI-03)
- [ ] No test infrastructure gaps (benchmark infrastructure already exists; no Rust tests needed for this phase)

## Open Questions

1. **Docker layer caching for the Rust compile step**
   - What we know: The Dockerfile Stage 1 does `cargo build --release` which is 5-10 minutes cold. GitHub Actions ubuntu-latest ephemeral runners have no persistent Docker layer cache.
   - What's unclear: Whether to add `actions/cache@v4` for Docker layers (complex setup) or accept the build cost. This is Claude's discretion.
   - Recommendation: Skip caching for v7.0. The Dockerfile is already multi-stage; if build time becomes a problem, add GitHub's `buildx` cache-to/cache-from in a follow-up. The benchmark run itself takes longer than the build for high RUNS counts.

2. **Job summary / echo output**
   - What we know: GitHub Actions supports writing to `$GITHUB_STEP_SUMMARY` for a formatted markdown summary on the run page.
   - What's unclear: Whether to echo RESULTS.md content into the job summary for immediate visibility without downloading the artifact.
   - Recommendation: Include a step that appends `RESULTS.md` content to `$GITHUB_STEP_SUMMARY`. This is low-effort, high-value, and fits within Claude's discretion.

## Sources

### Primary (HIGH confidence)
- `.github/workflows/vscode-extension.yml` — established artifact upload pattern, actions/checkout@v4, actions/upload-artifact@v4 usage
- `.github/workflows/rust.yml` — established ubuntu-latest runner pattern
- `benchmark/runner/run.sh` — authoritative docker build/run commands (adapted for CI, minus Windows path logic)
- `benchmark/runner/bench_runner.sh` — RUNS env var handling, RESULTS_DIR convention
- `benchmark/generate.py` — pygal==3.1.0 requirement confirmed in docstring
- GitHub Actions official docs (workflow-syntax) — workflow_dispatch inputs, schedule cron, GITHUB_ENV pattern, concurrency groups

### Secondary (MEDIUM confidence)
- GitHub Actions community discussions — confirmed `inputs.runs || 10` fallback pattern for dual-trigger workflows
- `benchmark/runner/Dockerfile` — confirms build context must be repo root (COPY paths are repo-root-relative)

### Tertiary (LOW confidence)
- None required — all critical patterns confirmed from first-party sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all actions are already in use in this repo; pygal version confirmed in generate.py
- Architecture: HIGH — workflow structure derived directly from existing repo patterns and locked CONTEXT.md decisions
- Pitfalls: HIGH (inputs fallback, GITHUB_ENV) / MEDIUM (Docker caching tradeoffs) — primary source verification

**Research date:** 2026-03-20
**Valid until:** 2026-04-20 (GitHub Actions APIs are stable; actions versions rarely break)
