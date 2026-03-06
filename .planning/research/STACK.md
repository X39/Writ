# Stack Research

**Domain:** Cross-language benchmark suite — Writ vs Lua, Squirrel, Python, Node.js, Rust (native)
**Researched:** 2026-03-20
**Confidence:** HIGH (Rust tooling), MEDIUM (Docker/CI patterns), MEDIUM (Squirrel embedding)

---

## Context

This is a SUBSEQUENT MILESTONE on an existing 9-crate Rust workspace (v6.1, 74,997 LOC). The question
is NOT "what stack do we need?" but "what NEW infrastructure and dependencies are required for
cross-language benchmarking?". Existing crates (`writ-compiler`, `writ-runtime`, `writ-cli`) are the
subjects under test — they are not changed here.

**Scope of new additions:**
- A new `writ-bench` Rust crate: benchmark harness, result collection, SVG/markdown output
- A `benchmark/` top-level directory: benchmark programs in each language + runner scripts
- A `benchmark/docker/` subdirectory: Dockerfile + container scripts
- A `.github/workflows/benchmark.yml` GitHub Actions CI workflow

---

## Already Validated (DO NOT RE-RESEARCH)

| Crate | Purpose | Benchmark relevance |
|-------|---------|---------------------|
| `writ-cli` | `writ compile` + `writ run` | Subject under test; invoked as a child process by the harness |
| `writ-runtime` | IL VM + scheduler | Rust-native benchmark variant runs runtime directly in-process |
| `writ-compiler` | Compile pipeline | Compile time reported separately from run time |
| `writ-module` | IL binary format | `.writc` output loaded by runtime benchmarks |

**Existing workspace dependencies (already available, do not re-add):**
`serde`, `serde_json` (1.0), `thiserror` (2.0), `clap`, `ariadne`, `byteorder`, `rustc-hash`

---

## Recommended Stack

### Core: `writ-bench` Rust Crate

This crate is the measurement harness and output generator. It does NOT use `cargo bench` / Criterion
because the benchmarks being measured are cross-language (separate processes) and the harness must
shell out to Lua, Python, Node, Squirrel interpreters. Criterion is designed for in-process Rust
microbenchmarks only — it cannot time external processes. The harness is a custom binary.

#### Measurement

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `std::process::Command` | stdlib | Spawn each language interpreter as a child process | Zero-dependency, correct stdio capture. No external crate needed. |
| `std::time::Instant` | stdlib | Wall-clock timing of each subprocess invocation | Nanosecond resolution, portable. Used for execution time and startup time. |
| `procfs` | 0.17 | Read peak RSS from `/proc/<pid>/status` on Linux | Cross-language memory measurement requires external observation. `procfs` exposes `VmPeak` (peak virtual) and `VmRSS` (current resident set) for any PID. Linux-only; Docker container runs Linux so this is fine. Provides `Process::new(pid)?.status()?.vm_rss_peak`. |
| `serde` + `serde_json` | 1.0 | Serialize measurement results to JSON for persistence | Already in workspace. Results written to `benchmark/results/YYYY-MM-DD.json` for CI artifact upload and chart input. |

**Memory measurement approach:** The harness spawns each interpreter as a child process, captures its
PID, and polls `/proc/<pid>/status` at 10 ms intervals while the child runs to capture peak RSS.
On non-Linux hosts (Windows, macOS), memory measurement falls back to `0` with a `#[cfg]` guard —
Docker ensures Linux in CI.

**Startup time:** Measured as wall-clock time for a no-op benchmark (empty main). Subtracted from
execution time in reporting.

#### SVG Chart Generation

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `plotters` | 0.3.7 | Generate SVG bar charts | Most mature Rust charting library with SVG backend. Actively maintained (0.3.7 released September 2024). Provides `SVGBackend`, `ChartBuilder`, `BarChart`-equivalent via `Histogram`. No system dependencies (pure Rust). Outputs standalone `.svg` files. |
| `plotters-svg` | 0.3.7 | SVG rendering backend for plotters | Required companion crate; `plotters` since 0.3 splits backends into separate crates. Pure Rust, no native dependencies. |

**Why plotters over alternatives:**
- `resvg`: SVG renderer (input SVG → rasterize), not a chart generator. Wrong tool.
- `poloto`: Simpler API but no bar chart support, limited styling.
- `charts-rs`: Newer, easier bar charts, but smaller ecosystem, less documentation.
- Hand-rolled SVG string generation: Viable for simple bar charts (SVG is XML). Acceptable fallback
  if plotters bar chart API proves cumbersome — SVG bar charts are ~50 lines of string formatting.

#### Markdown Table Generation

No crate needed. Markdown tables are plain text with `|` separators. The harness generates them
via `std::fmt::Write` directly. A template crate like `minijinja` would be over-engineering for
a fixed-schema results table.

---

### Docker Container

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Docker / Podman | latest (Docker 27.x / Podman 5.x) | Reproducible build + run environment | All interpreters pinned to known versions; eliminates "works on my machine" timing variance. Single Dockerfile supports both `docker build` and `podman build` (OCI-compatible). |
| `ubuntu:24.04` base image | 24.04 LTS | Base OS for all interpreters | Ubuntu 24.04 (Noble Numbat) is the current LTS. GitHub Actions `ubuntu-latest` now maps to ubuntu-24.04 (confirmed January 2025 rollout). Lua 5.4, Python 3.12, Node.js 20 LTS are all available as `apt` packages. Consistency between local Docker and CI. |

**Language interpreter versions in the container:**

| Language | Version | Installation method | Notes |
|----------|---------|---------------------|-------|
| Lua | 5.4.8 | `apt-get install lua5.4` | lua5.4 package available in Ubuntu 24.04. Current Lua release. |
| Squirrel | 3.2 | Build from source (github.com/albertodemichelis/squirrel) | Squirrel is NOT in apt repositories. Must clone + cmake build in Dockerfile. ~5 min Docker layer. Pin to git tag `v3.2`. |
| Python | 3.12 | `apt-get install python3.12` | Default Python in Ubuntu 24.04. |
| Node.js | 20.x LTS | `curl -fsSL https://deb.nodesource.com/setup_20.x | bash` | NodeSource PPA. Node 20 is the current LTS (EOL April 2026). Do NOT use Ubuntu's `nodejs` package — it is severely outdated. |
| Rust (native) | 1.87+ stable | `curl https://sh.rustup.rs | sh` | Rust stable toolchain needed to compile the Writ CLI and native Rust benchmark variants. Pin via `rust-toolchain.toml` in repo root. |

**Squirrel build concern:** Squirrel has no official Docker image and is not in Ubuntu package repositories.
The Dockerfile must build it from source. This is a one-time cost cached in the Docker layer. The
Squirrel command-line interpreter binary is `sq` after `cmake --build`.

---

### Runner Scripts

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `benchmark/run.sh` (POSIX sh) | sh (not bash) | Linux/macOS runner script | Invoke `docker build` then `docker run writ-bench`. POSIX sh for maximum portability — no bash-isms. |
| `benchmark/run.ps1` (PowerShell) | PowerShell 7.x | Windows runner script | PowerShell 7 is cross-platform and available on all GitHub Actions Windows runners. Equivalent logic to `run.sh`. Calls `docker` or `podman` depending on what's on PATH. |

Scripts are thin wrappers: they build the Docker image, run the container with `--volume` mounts for
results output, and print the path to generated files. No complex logic in scripts.

---

### GitHub Actions CI

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `actions/checkout` | v4 | Checkout repository | Standard. v4 is the current stable version. |
| `actions/upload-artifact` | v4 | Upload benchmark result JSON + SVG files | Standard. v4 uses immutable artifacts (no overwrite). |
| Docker (pre-installed) | included in ubuntu-24.04 runner | Container build + run in CI | GitHub Actions ubuntu-24.04 runners include Docker. No additional setup step needed. |
| `benchmark-action/github-action-benchmark` | v1 | Optional: track performance over time, alert on regression | Consumes JSON result files, stores history in `gh-pages` branch, posts PR comments on regression. Use in a scheduled weekly run, not every commit. |

**Workflow triggers:**
- `workflow_dispatch` (manual) — primary trigger; benchmark runs are expensive
- `schedule: cron: '0 2 * * 0'` (weekly Sunday 02:00 UTC) — baseline tracking
- NOT triggered on every push/PR — execution cost is too high

**Artifact strategy:** Upload `benchmark/results/YYYY-MM-DD.json` and the SVG charts as workflow
artifacts. Do NOT commit results to `master` on every run — commit only on manual dispatch with
explicit `--commit-results` flag in the runner script. This avoids polluting git history.

---

### Supporting Libraries (NEW — add to `writ-bench/Cargo.toml`)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `plotters` | 0.3.7 | SVG bar chart generation | Always — the chart generator |
| `plotters-svg` | 0.3.7 | SVG backend for plotters | Always — companion to plotters |
| `procfs` | 0.17 | Peak RSS memory measurement via `/proc` | Linux only (Docker container). Gated with `#[cfg(target_os = "linux")]`. |
| `serde` | 1.0 | Serialize/deserialize benchmark results | Derive `Serialize`/`Deserialize` on result structs |
| `serde_json` | 1.0 | Write results to JSON files | Already in workspace; add to `writ-bench` explicitly |
| `thiserror` | 2.0 | Error types in harness | Already in workspace |
| `clap` | 4.x | CLI flags for harness binary (`--languages`, `--output-dir`, `--commit-results`) | Already in workspace |

**NOT adding `criterion`:** Criterion is a Rust in-process microbenchmark framework. It cannot time
external processes and is not suitable for cross-language comparison. The `writ-bench` crate is a
custom harness binary, not a `[[bench]]` target.

---

## Integration Points with Existing Crates

### How `writ-bench` Exercises the Writ Toolchain

```
writ-bench binary
    ↓  subprocess
writ compile benchmark/writ/fib.writ  → benchmark/writ/fib.writc
    (timing: compile_time_ms)
    ↓  subprocess
writ run benchmark/writ/fib.writc
    (timing: exec_time_ms, memory: peak_rss_kb)
```

The harness invokes `writ` CLI as a subprocess (not in-process). This is intentional:
- Measures real user experience including process startup overhead
- Compile time and run time are reported separately (Writ design constraint from PROJECT.md)
- The CLI binary path is configurable; CI builds it with `cargo build --release` first

**Rust-native variant:** For the native Rust benchmark, the harness compiles and runs a standalone
`benchmark/rust/fib.rs` via `rustc` subprocess, or links a pre-compiled `writ-bench-native` binary.
This represents the Rust "ceiling" — the best case performance target.

### New Crate: `writ-bench`

Added to `Cargo.toml` workspace `members`:
```toml
[workspace]
resolver = "3"
members = [
    "writ-assembler", "writ-cli", "writ-compiler", "writ-dap",
    "writ-diagnostics", "writ-golden", "writ-lsp", "writ-module",
    "writ-parser", "writ-runtime",
    "writ-bench",   # NEW
]
```

`writ-bench` has NO dependencies on other workspace crates. It is a standalone harness that invokes
`writ` as a subprocess. This keeps the benchmarking infrastructure independent of compiler internals
and ensures the benchmark measures the same binary a user would run.

---

## Installation

### Rust

```toml
# writ-bench/Cargo.toml
[package]
name    = "writ-bench"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "writ-bench"
path = "src/main.rs"

[dependencies]
plotters     = { version = "0.3.7", default-features = false, features = ["svg_backend"] }
plotters-svg = "0.3.7"
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
thiserror    = "2"
clap         = { version = "4", features = ["derive"] }

[target.'cfg(target_os = "linux")'.dependencies]
procfs = { version = "0.17", default-features = false }
```

### Docker

```dockerfile
# benchmark/docker/Dockerfile
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    lua5.4 \
    python3.12 \
    cmake cmake-extras build-essential git \
    curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Node.js 20 LTS via NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Squirrel 3.2 from source
RUN git clone --depth 1 --branch v3.2 \
        https://github.com/albertodemichelis/squirrel.git /tmp/squirrel \
    && cmake -S /tmp/squirrel -B /tmp/squirrel/build -DCMAKE_BUILD_TYPE=Release \
    && cmake --build /tmp/squirrel/build \
    && cp /tmp/squirrel/build/sq/sq /usr/local/bin/sq \
    && rm -rf /tmp/squirrel

# Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace
COPY . .

RUN cargo build --release -p writ-cli

ENTRYPOINT ["cargo", "run", "--release", "-p", "writ-bench", "--"]
```

### Runner Scripts

```sh
#!/bin/sh
# benchmark/run.sh
set -e
CONTAINER_TOOL=${CONTAINER_TOOL:-docker}
$CONTAINER_TOOL build -t writ-bench -f benchmark/docker/Dockerfile .
$CONTAINER_TOOL run --rm -v "$(pwd)/benchmark/results:/workspace/benchmark/results" \
    writ-bench "$@"
```

```powershell
# benchmark/run.ps1
param([string]$ContainerTool = "docker")
& $ContainerTool build -t writ-bench -f benchmark/docker/Dockerfile .
& $ContainerTool run --rm -v "${PWD}/benchmark/results:/workspace/benchmark/results" `
    writ-bench @args
```

### GitHub Actions Workflow Skeleton

```yaml
# .github/workflows/benchmark.yml
name: Benchmark Suite
on:
  workflow_dispatch:
  schedule:
    - cron: '0 2 * * 0'

jobs:
  benchmark:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - name: Build benchmark container
        run: docker build -t writ-bench -f benchmark/docker/Dockerfile .
      - name: Run benchmarks
        run: |
          mkdir -p benchmark/results
          docker run --rm \
            -v "${{ github.workspace }}/benchmark/results:/workspace/benchmark/results" \
            writ-bench --output-dir /workspace/benchmark/results
      - uses: actions/upload-artifact@v4
        with:
          name: benchmark-results-${{ github.run_id }}
          path: benchmark/results/
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Custom harness binary (`writ-bench`) | `criterion` + `[[bench]]` targets | Only if all benchmarks are in-process Rust. Criterion cannot time external processes — wrong tool for cross-language work. |
| `plotters` 0.3.7 + `plotters-svg` | Hand-rolled SVG string generation | SVG bar charts are simple enough that ~80 lines of `write!` macros would work. Use if plotters API proves difficult for the specific bar chart layout needed. |
| `plotters` 0.3.7 | `charts-rs` | `charts-rs` has a simpler bar chart API, but is less mature (smaller community, less documentation). Use if plotters bar charts require too much boilerplate. |
| `procfs` for memory measurement | `psutil` (Python) / shell `ps` in container | `procfs` is native Rust and works in-process. Polling with `ps` requires spawning a shell subprocess while the benchmarked process is running — adds jitter. |
| `ubuntu:24.04` base image | `debian:bookworm-slim` | Debian slim is smaller but requires more manual apt source configuration for Node.js. Ubuntu 24.04 matches the GitHub Actions runner OS exactly — less divergence. |
| Docker/Podman single Dockerfile | `docker-compose` with separate services | Separate services add complexity for a sequential benchmark runner. One container + one entrypoint is simpler and sufficient. |
| `workflow_dispatch` + weekly `schedule` trigger | Trigger on every push | Benchmark runs take 10-30 minutes. Running on every push consumes GitHub Actions minutes unnecessarily. Manual dispatch is the primary trigger. |
| Squirrel 3.2 built from source | Squirrel as an embedded Rust crate via `squirrel-rs` | `squirrel-rs` (github.com/cyderize/squirrel-rs) provides Rust FFI bindings, but the project has low activity. Building the `sq` CLI from source and running it as a subprocess is simpler and more consistent with how Lua/Python/Node are measured. |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `criterion` | In-process Rust-only microbenchmark framework. Cannot time external processes. Adding it would measure Rust overhead of calling subprocess, not the actual language performance. | Custom `writ-bench` binary with `std::time::Instant` and `std::process::Command` |
| `hyperfine` (the CLI tool) | `hyperfine` v1.20.0 is excellent for manual cross-language benchmarking from a terminal, but it cannot be embedded in a Rust binary to collect structured results for SVG/markdown output. | `std::process::Command` + `std::time::Instant` in `writ-bench` |
| `mlua` (Lua embedding crate) | Embeds Lua into the Rust process. This would measure Lua performance through Rust FFI overhead, NOT as a standalone language. The benchmark must run Lua as an independent process to be a fair comparison. | Invoke `lua5.4` as a subprocess via `std::process::Command` |
| `docker-compose` | Adds a YAML dependency for what is ultimately a sequential task. One Dockerfile + one entrypoint is simpler. | Single `Dockerfile` with an entrypoint that runs the harness |
| `benchmark-action/github-action-benchmark` in every PR | Expensive (minutes per run) and adds flakiness if runners have load variance. | Use only in weekly scheduled workflow; not in PR checks |
| Separate `writ-bench-results` git branch for auto-commit | Auto-committing to a results branch from CI makes git history confusing and requires write tokens. | Upload results as GitHub Actions artifacts; commit manually when publishing |
| Python `matplotlib` for chart generation | Introduces a Python dependency in the Rust build pipeline. All chart generation should be in `writ-bench` Rust binary. | `plotters` + `plotters-svg` |

---

## Stack Patterns by Variant

**If running benchmarks locally without Docker:**
- Invoke `writ-bench --no-docker` (flag to skip container, use system interpreters)
- Harness detects available interpreters via `which lua5.4 lua lua5.3` etc.
- Memory measurement only available on Linux; skipped gracefully on macOS/Windows
- This mode is for development; Docker mode is canonical for published results

**If Squirrel source build fails in Docker:**
- Fall back to Squirrel 3.1 from SourceForge if the git tag is unavailable
- Or skip Squirrel from that benchmark run and mark results as `N/A`
- The harness result schema must support `null` for missing language entries

**If GitHub Actions free minutes are a concern:**
- Use `workflow_dispatch` only (disable the weekly schedule)
- Cache the Docker image layer between runs using `actions/cache` with the Dockerfile hash as cache key
- Build the container once as a separate job, push to GitHub Container Registry (ghcr.io), pull in benchmark jobs

---

## Version Compatibility

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `plotters 0.3.7` | `plotters-svg 0.3.7` | Must match major.minor | Both in `plotters-rs` org; keep versions in sync |
| `plotters 0.3.7` | Rust 2021 edition | Compatible | Rust 2024 edition (workspace) is also compatible |
| `procfs 0.17` | Linux kernel 4.x+ | Compatible | Ubuntu 24.04 runs kernel 6.x; fully supported |
| `serde 1.0` + `serde_json 1.0` | Already workspace versions | No conflict | `writ-bench` declares same versions as workspace |
| `clap 4.x` | Already workspace version | No conflict | Already used by `writ-cli` |
| `ubuntu:24.04` Docker base | GitHub Actions `ubuntu-latest` | Match — ubuntu-latest = ubuntu-24.04 since Jan 2025 | Ensures container and CI runner OS match |
| Node.js 20 LTS | EOL April 2026 | Fine for v7.0 milestone | Upgrade to Node 22 LTS after April 2026 |
| Squirrel 3.2 | cmake 3.16+ | `cmake` in Ubuntu 24.04 is 3.28+ | Compatible |

---

## Sources

- `docs.rs/plotters/latest` — version 0.3.7 confirmed, SVGBackend API verified (HIGH confidence, official docs)
- `github.com/plotters-rs/plotters` — 0.3.7 released September 8, 2024, active maintenance confirmed (HIGH confidence, official repo)
- `crates.io/crates/criterion` — 0.8.2 latest (criterion-rs org fork, released February 2026); Criterion is in-process only, confirmed inapplicable for cross-language subprocess timing (HIGH confidence)
- `github.com/criterion-rs/criterion.rs` — 0.8.2 latest release February 4, 2026 (HIGH confidence, official repo)
- `crates.io/crates/mlua` — 0.11.6 confirmed; ruled out because it embeds Lua in-process (HIGH confidence)
- `crates.io/crates/procfs` — 0.17 for `/proc` memory stats; `VmRSS` and peak RSS confirmed in `Status` struct (MEDIUM confidence, docs.rs)
- `github.com/sharkdp/hyperfine/releases` — v1.20.0 released November 18, 2025; ruled out for embedded use (HIGH confidence, official repo)
- `github.com/kostya/benchmarks` — methodology review: RSS measurement, Docker containers, median ± MAD reporting (MEDIUM confidence, community project)
- `github.com/khvzak/script-bench-rs` — uses Criterion for in-process embedding benchmarks; confirmed inapplicable for subprocess approach (MEDIUM confidence, community project)
- `packages.ubuntu.com/lua5.4` — Lua 5.4 available in Ubuntu 24.04 repositories (HIGH confidence, official Ubuntu package search)
- `lua.org/versions.html` — Lua 5.4.8 current release, June 2025 (HIGH confidence, official site)
- `github.com/albertodemichelis/squirrel` — official Squirrel repository; v3.2 tag; cmake build required (HIGH confidence, official repo)
- `github.com/actions/runner-images/issues/10636` — ubuntu-latest = ubuntu-24.04 confirmed January 2025 (HIGH confidence, official GitHub Actions repo)
- `crates.io/crates/serde_json` — 1.0.149 latest as of January 2026; using SemVer `"1"` range (HIGH confidence)
- WebSearch: GitHub Actions benchmark action patterns, artifact upload for results (MEDIUM confidence)

---

*Stack research for: Writ v7.0 cross-language benchmark suite*
*Researched: 2026-03-20*
