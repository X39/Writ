# Phase 70: Docker Environment and Measurement Harness - Research

**Researched:** 2026-03-20
**Domain:** Docker multi-stage builds, hyperfine CLI, shell benchmarking harness, memory measurement via /proc
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Docker Image Architecture:**
- Multi-stage Dockerfile: Stage 1 builds `writ` binary from Rust source, Stage 2 builds Rust benchmark stubs, Stage 3 is the runtime image (ubuntu:24.04)
- Final image contains only runtime interpreters + hyperfine + writ binary — no Rust compiler in the benchmark image
- Base image: `ubuntu:24.04`
- Docker context is repo root (`docker build -t writ-bench -f benchmark/runner/Dockerfile .`)

**Language Runtime Installation:**
- Lua 5.4: `apt-get install lua5.4` — binary is `lua5.4`
- Squirrel 3.2: Build from source (clone `albertodemichelis/squirrel` at pinned git tag `v3.2`, cmake build, copy `sq` binary)
- Python 3.12: `apt-get install python3`
- Node.js 22 LTS: Install via NodeSource PPA (`setup_22.x`)
- Rust benchmarks: Pre-compiled with `rustc -O` in a builder stage
- Writ: `cargo build --release --bin writ` in builder stage, copy to `/usr/local/bin/writ`

**Measurement Tool:**
- Use `hyperfine` CLI for all timing measurements — install from GitHub Releases `.deb`
- Use `jq` inside the container to merge per-language JSON results
- Writ measured in two separate hyperfine invocations: `writ compile` and `writ run`

**Memory Measurement:**
- Anonymous RSS only: `VmRSS - RssFile - RssShmem` from `/proc/<pid>/status`
- Shared shell function `measure_anon_rss()` used by all language measurements
- Reports `0` on non-Linux (documented)

**Startup Time:**
- Measured as wall-clock time for a no-op program (each language prints "hello") via hyperfine
- Reported as a distinct JSON field

**Runner Scripts:**
- `benchmark/runner/run.sh` (POSIX sh): builds Docker image, runs container with volume mount, detects Docker or Podman
- `benchmark/runner/run.ps1` (PowerShell): equivalent logic, handles Windows path normalization
- Both produce output in `benchmark/results/YYYY-MM-DD/` dated subdirectories
- `RUNS` configurable via env var / parameter (default: 10)

**Stub Benchmark:**
- Minimal hello-world per language to prove pipeline works end-to-end
- Produces valid `raw.json` with: `compile_ms` and `run_ms` for Writ, `execution_ms` for all others, plus `memory_kb`, `startup_ms`
- Lives in `benchmark/cases/stub/`

**JSON Output Schema:**
- Top-level: `{ "benchmarks": [...], "meta": { "date", "runs", "warmup", "platform" } }`
- Per benchmark: `{ "suite", "writ_compile": {...}, "writ_run": {...}, "lua": {...}, ... }`
- Per language result: hyperfine's native JSON format (mean, median, stddev, min, max, times[])
- All times in seconds (hyperfine default)

**Version Pinning:**
- Lua 5.4.x, Squirrel 3.2 (git tag), Python 3.12.x, Node.js 22.x LTS, hyperfine 1.18.0+

### Claude's Discretion

- Exact Dockerfile layer ordering and caching strategy
- Whether `bench_runner.sh` uses a case config file or hardcoded language list
- Temporary file management for `.writc` compilation artifacts
- Error handling for individual language failures (skip vs fail-fast)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFRA-01 | Docker container includes all 6 language runtimes (Writ, Lua 5.4, Squirrel 3.x, Python 3.x, Node.js LTS, Rust) | Multi-stage Dockerfile pattern; Squirrel source build procedure; NodeSource PPA for Node.js 22; hyperfine .deb install |
| INFRA-02 | `run.sh` script runs all benchmarks using only Docker/Podman (no other prerequisites) | POSIX sh pattern with `command -v docker \|\| command -v podman` detection; volume mount pattern |
| INFRA-03 | `run.ps1` script runs all benchmarks using only Docker/Podman (no other prerequisites) | PowerShell path normalization for Windows volume mounts; same detection logic |
| INFRA-04 | Benchmark harness measures execution time (median ± MAD over N runs) | hyperfine provides median and stddev natively; MAD must be computed from hyperfine's `times[]` array via jq |
| INFRA-05 | Benchmark harness measures peak memory usage (anonymous RSS) | hyperfine v1.20.0 does NOT measure memory; must use separate `/proc/<pid>/status` shell function around subprocess |
| INFRA-06 | Benchmark harness measures startup time per language | Separate hyperfine run against a no-op/hello-world per language; reported as distinct `startup_ms` field |
| INFRA-07 | Writ compile time and runtime reported as separate columns | Two separate hyperfine invocations; `writ_compile` and `writ_run` as distinct JSON keys |
| INFRA-08 | Results output as JSON for pipeline consumption | hyperfine `--export-json` produces per-language JSON; jq merges into unified `raw.json` |

</phase_requirements>

---

## Summary

Phase 70 creates the Docker infrastructure and measurement harness that all subsequent benchmark phases depend on. The work is greenfield — no existing `benchmark/` directory. The primary deliverables are: a multi-stage Dockerfile producing an ubuntu:24.04 image with all 6 language runtimes, a `bench_runner.sh` orchestration script, host-side launcher scripts (`run.sh` and `run.ps1`), and a stub benchmark case that produces a valid `raw.json` demonstrating the full pipeline works end-to-end.

The most important implementation details uncovered in this research phase are:

**Critical finding 1:** hyperfine v1.20.0 (current release) does NOT support memory measurement. The `memory_usage_byte` field exists only in an unmerged draft PR (#790, `new-metrics` branch). Memory measurement must be implemented via a separate `/proc/<pid>/status` reading shell function that wraps each language subprocess independently.

**Critical finding 2:** hyperfine does not compute MAD (Median Absolute Deviation) natively. The JSON output includes `times[]` (all individual run durations). MAD must be computed from this array using jq arithmetic after the hyperfine run, then injected into the per-language result object before merging into `raw.json`.

**Critical finding 3:** `squirrel3` is NOT available in Ubuntu 24.04 Noble apt repositories (confirmed absent — it skips from jammy/22.04 to oracular/24.10). Source build from `albertodemichelis/squirrel` at tag `v3.2` is the only option. The cmake build produces binary `sq` by default; `make install` copies it to `/usr/local/bin/`.

**Primary recommendation:** Use `bench_runner.sh` as a pure bash orchestration script with hyperfine for timing, a custom `measure_anon_rss()` shell function for memory, a `compute_mad()` jq function for MAD, and jq for JSON assembly. Memory measurement and MAD are additional computation layers on top of hyperfine's output, not capabilities of hyperfine itself.

---

## Standard Stack

### Core Tools in Docker Image

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| ubuntu:24.04 | 24.04 LTS | Base image | Matches GitHub Actions ubuntu-latest; apt provides Lua 5.4, Python 3.12 |
| hyperfine | 1.20.0 | Timing measurements | Provides median, stddev, min, max, times[] via --export-json; single .deb install |
| jq | 1.7.1 (apt) | JSON assembly | Standard tool; available in apt; handles per-language JSON merge and MAD computation |
| lua5.4 | 5.4.x (apt) | Lua runtime | Direct apt install: `apt-get install lua5.4` |
| python3 | 3.12.x (apt) | Python runtime | Default Python in Ubuntu 24.04 |
| nodejs | 22.x LTS | Node.js runtime | NodeSource PPA setup_22.x required (Ubuntu nodejs package is outdated) |
| cmake + build-essential + git | 3.28 (apt) | Squirrel build deps | Required for Squirrel source build |
| curl + ca-certificates | (apt) | Download tools | Needed for NodeSource PPA and hyperfine .deb download |

### Builder Stage Tools

| Tool | Version | Purpose | When Used |
|------|---------|---------|-----------|
| rust:1.85-slim | 1.85 | Build writ binary | Stage 1 — cargo build --release --bin writ |
| rust:1.85-slim | 1.85 | Build Rust benchmarks | Stage 2 — rustc -O for each .rs stub |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| hyperfine .deb from GitHub Releases | apt-get install hyperfine | hyperfine is NOT in Ubuntu 24.04 apt repos — must use GitHub Releases |
| jq for JSON merge + MAD | Python helper script | jq is available in apt and keeps the harness pure-shell; Python adds container size |
| /proc/<pid>/status for memory | /usr/bin/time -v | /usr/bin/time -v measures wall time + MaxRSS together; /proc lets us measure anonymous RSS specifically |
| NodeSource PPA setup_22.x | nvm inside Docker | nvm adds complexity; NodeSource PPA is the production Docker pattern |

**Installation (inside Dockerfile final stage):**
```bash
# apt packages
apt-get install -y lua5.4 python3 jq cmake build-essential git curl ca-certificates

# Node.js 22 LTS via NodeSource
curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs

# Squirrel 3.2 from source
git clone --depth 1 --branch v3.2 \
    https://github.com/albertodemichelis/squirrel.git /tmp/squirrel
cd /tmp/squirrel && mkdir build && cd build && cmake .. && make
cp /tmp/squirrel/build/sq/sq /usr/local/bin/sq
rm -rf /tmp/squirrel

# hyperfine from GitHub Releases
HYPERFINE_VERSION=1.20.0
curl -sSL "https://github.com/sharkdp/hyperfine/releases/download/v${HYPERFINE_VERSION}/hyperfine_${HYPERFINE_VERSION}_amd64.deb" \
    -o /tmp/hyperfine.deb
dpkg -i /tmp/hyperfine.deb && rm /tmp/hyperfine.deb
```

**Version verification (run before writing this table):**
```bash
# hyperfine latest release: v1.20.0 (November 18, 2025) — confirmed via GitHub Releases
# Squirrel v3.2 tag — confirmed on albertodemichelis/squirrel
# jq 1.7.x available in Ubuntu 24.04 noble apt (universe)
```

---

## Architecture Patterns

### Recommended Project Structure

```
benchmark/
├── cases/
│   └── stub/               # Phase 70 only — hello world per language
│       ├── stub.writ
│       ├── stub.lua
│       ├── stub.nut         # Squirrel extension
│       ├── stub.py
│       ├── stub.js
│       └── stub.rs
├── runner/
│   ├── Dockerfile           # Multi-stage: writ-builder + rust-bench-builder + runtime
│   ├── bench_runner.sh      # Runs inside container; writes /results/raw.json
│   ├── run.sh               # Host entry: Linux/macOS (POSIX sh)
│   └── run.ps1              # Host entry: Windows (PowerShell)
└── results/
    └── .gitkeep             # Directory committed to git; populated at runtime
```

### Pattern 1: Multi-Stage Dockerfile

**What:** Three distinct stages — writ-builder (cargo), rust-bench-builder (rustc), runtime (ubuntu:24.04).

**When to use:** Always. Separates build toolchains from the runtime image.

**Example:**
```dockerfile
# Stage 1: Build writ binary
FROM rust:1.85-slim AS writ-builder
WORKDIR /writ
COPY . .
RUN cargo build --release --bin writ

# Stage 2: Build Rust benchmark stubs
FROM rust:1.85-slim AS rust-bench-builder
WORKDIR /bench
COPY benchmark/cases/ ./cases/
RUN mkdir -p /bench/bin && \
    for rs in cases/stub/*.rs; do \
        name=$(basename "$rs" .rs); \
        rustc -O -o "/bench/bin/${name}" "$rs"; \
    done

# Stage 3: Runtime image
FROM ubuntu:24.04
# ... (see Standard Stack section for install commands)
COPY --from=writ-builder /writ/target/release/writ /usr/local/bin/writ
COPY --from=rust-bench-builder /bench/bin/ /bench/bin/
COPY benchmark/cases/ /bench/cases/
COPY benchmark/runner/bench_runner.sh /bench/bench_runner.sh
RUN chmod +x /bench/bench_runner.sh
WORKDIR /bench
ENTRYPOINT ["/bench/bench_runner.sh"]
```

### Pattern 2: hyperfine JSON Export + jq MAD Computation

**What:** hyperfine produces `times[]` array in JSON. jq computes median (already available as `median` field) and MAD from the raw times.

**Critical detail:** hyperfine v1.20.0 JSON output includes `median` directly as a field. It does NOT include MAD. MAD must be derived from `times[]`.

**Example — MAD computation via jq:**
```bash
# After hyperfine run:
# /tmp/writ_run.json contains: { "results": [{ "median": 0.0041, "times": [0.0039, 0.0041, ...] }] }

# Compute MAD using jq:
writ_run_json=$(cat /tmp/writ_run.json | jq '.results[0] + {
    mad: (
        (.results[0].times | sort) as $sorted |
        ($sorted | length) as $n |
        ($sorted[$n/2 | floor]) as $med |
        ([ $sorted[] | (. - $med | fabs) ] | sort) as $devs |
        $devs[$n/2 | floor]
    )
}')
```

**hyperfine v1.20.0 JSON output schema (verified from source):**
```json
{
  "results": [{
    "command": "writ run /tmp/stub.writc",
    "mean": 0.004231,
    "stddev": 0.000123,
    "median": 0.004198,
    "user": 0.003987,
    "system": 0.000244,
    "min": 0.004012,
    "max": 0.004891,
    "times": [0.004231, 0.004198, ...],
    "exit_codes": [0, 0, ...]
  }]
}
```

Note: `memory_usage_byte` is NOT in the v1.20.0 release output.

### Pattern 3: Anonymous RSS Memory Measurement

**What:** hyperfine does not measure memory. A wrapper shell function measures anonymous RSS by reading `/proc/<pid>/status` of the child process.

**Example:**
```bash
measure_anon_rss() {
    # $1 = command to run
    # Returns peak anonymous RSS in KB
    local peak_kb=0
    "$@" &
    local pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        if [ -f "/proc/$pid/status" ]; then
            local rss_anon
            rss_anon=$(awk '/^RssAnon:/{print $2}' /proc/$pid/status 2>/dev/null || echo 0)
            if [ "$rss_anon" -gt "$peak_kb" ] 2>/dev/null; then
                peak_kb=$rss_anon
            fi
        fi
    done
    wait "$pid"
    echo "$peak_kb"
}
```

**Fallback for /proc/pid/status without RssAnon (kernel < 4.5):**
```bash
# VmRSS - RssFile - RssShmem gives anonymous RSS
rss_anon=$(awk '
    /^VmRSS:/{rss=$2}
    /^RssFile:/{file=$2}
    /^RssShmem:/{shmem=$2}
    END{print rss-file-shmem}
' /proc/$pid/status 2>/dev/null || echo 0)
```

Ubuntu 24.04 runs kernel 6.x which provides `RssAnon` directly.

### Pattern 4: Startup Time as Separate Measurement

**What:** A no-op hello-world program per language measured with hyperfine; reported as `startup_ms` distinct from `execution_ms`.

**Stub implementations:**
- `stub.writ`: `fn main() { log::info("hello"); }`
- `stub.lua`: `print("hello")`
- `stub.nut`: `print("hello")`
- `stub.py`: `print("hello")`
- `stub.js`: `console.log("hello")`
- `stub.rs`: `fn main() { println!("hello"); }` (pre-compiled binary)

**Example:**
```bash
# Startup time = time to execute a minimal program (hello world)
hyperfine --runs "$RUNS" --warmup 2 \
    --export-json /tmp/startup_writ.json \
    "writ run /bench/bin/stub.writc"
```

### Pattern 5: Squirrel Source Build in Docker

**What:** Squirrel 3.2 is not in Ubuntu 24.04 apt. Build from source in the runtime stage (not a builder stage — squirrel is a runtime dependency, not a build tool).

**Build procedure:**
```dockerfile
# Inside the final runtime stage
RUN apt-get install -y cmake build-essential git && \
    git clone --depth 1 --branch v3.2 \
        https://github.com/albertodemichelis/squirrel.git /tmp/squirrel && \
    cd /tmp/squirrel && mkdir build && cd build && cmake .. -DCMAKE_BUILD_TYPE=Release && \
    make && cp sq/sq /usr/local/bin/sq && \
    cd / && rm -rf /tmp/squirrel
```

**Binary produced:** `sq` (default cmake output, confirmed from CMakeLists.txt)
**Script extension:** `.nut`
**Invocation:** `sq /bench/cases/stub/stub.nut`

**Version assertion at container startup:**
```bash
sq --version 2>&1 | grep -E "Squirrel" || { echo "ERROR: sq not found or wrong version" >&2; exit 1; }
```

### Pattern 6: Version String Emission at Container Startup

**What:** Container emits pinned version strings for all 6 runtimes before running benchmarks (success criterion #5).

**Example in bench_runner.sh:**
```bash
echo "=== Runtime Versions ==="
lua5.4 -v 2>&1 | head -1
sq --version 2>&1 | head -1
python3 --version
node --version
/usr/local/bin/writ --version 2>/dev/null || echo "writ (version unknown)"
echo "hyperfine $(hyperfine --version)"
echo "========================="
```

### Pattern 7: Windows Volume Mount Path Normalization

**What:** Docker on Windows requires Unix-style paths for -v flags. PowerShell must convert `C:\Users\...` to `/c/Users/...`.

**Example:**
```powershell
# Convert Windows path to Docker-compatible Unix path
$ResultsMounted = $ResultsDir `
    -replace '\\', '/' `
    -replace '^([A-Za-z]):', { "/$($_.Groups[1].Value.ToLower())" }
# C:\Users\dev\Writ\benchmark\results\2026-03-20 -> /c/Users/dev/Writ/benchmark/results/2026-03-20
```

### Anti-Patterns to Avoid

- **Including rustc in the final image:** Adds 1GB+ to image size, unnecessarily. Pre-compile Rust stubs in builder stage.
- **Using `squirrel3` via apt on ubuntu:24.04:** Package does not exist in Noble. Will fail with "package not found."
- **Using Ubuntu's `nodejs` apt package:** Severely outdated (Node.js 12.x in Noble universe). Always use NodeSource PPA for Node.js 22 LTS.
- **Using hyperfine `memory_usage_byte` field:** Does not exist in v1.20.0 released binary. Only in unmerged draft PR #790.
- **Measuring memory with `docker stats`:** Reports total RSS including OS page cache. Use `/proc/<pid>/status` for anonymous RSS.
- **Running the full measurement inside the builder stage:** The final image must not contain the Rust toolchain. All compilation happens in named builder stages.
- **Using `squirrel` as the binary name:** The cmake build output is `sq`, not `squirrel`. ARCHITECTURE.md incorrectly states the binary is called `squirrel` — the actual binary is `sq`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timing measurement with statistics | Custom time loop | hyperfine | handles warmup, provides median/stddev/min/max/times[], subprocess isolation |
| JSON merging | String concatenation | jq | handles quoting, nesting, type conversion correctly |
| Docker/Podman detection | Complex shell logic | `command -v docker \|\| command -v podman` | POSIX portable, zero deps |
| Squirrel installation | apt-based approach | Source build at v3.2 tag | squirrel3 absent from Ubuntu 24.04 Noble apt repos |
| Node.js installation | Ubuntu's nodejs package | NodeSource setup_22.x | Ubuntu nodejs is Node.js 12; NodeSource provides 22 LTS |

**Key insight:** The measurement harness complexity is in the memory measurement and MAD computation — both require custom shell logic because hyperfine v1.20.0 does not provide them. Everything else (timing, export, process management) is provided by hyperfine.

---

## Common Pitfalls

### Pitfall 1: squirrel3 Not in Ubuntu 24.04 Noble apt

**What goes wrong:** `apt-get install squirrel3` fails with "Unable to locate package squirrel3" on ubuntu:24.04.

**Why it happens:** The squirrel3 package was dropped from Ubuntu 24.04 Noble. It exists in jammy (22.04), oracular (24.10), and plucky (25.04) but NOT in noble (24.04).

**How to avoid:** Build from source using `git clone --depth 1 --branch v3.2 https://github.com/albertodemichelis/squirrel.git`. Pin to the `v3.2` git tag. Add `cmake build-essential git` to apt install.

**Warning signs:** Docker build fails at the squirrel install step with "Unable to locate package."

### Pitfall 2: hyperfine Does Not Measure Memory

**What goes wrong:** Planner assumes `hyperfine --export-json` produces `memory_usage_byte` field. It does not in v1.20.0. The ARCHITECTURE.md pre-existing research also assumed this. The `memory_usage_byte` struct field exists only in the unmerged draft PR #790 (`new-metrics` branch).

**Why it happens:** The `BenchmarkResult` struct in the source code already has the field in the `new-metrics` branch, but it is not in the v1.20.0 release binary.

**How to avoid:** Use a custom `measure_anon_rss()` shell function that polls `/proc/<pid>/status` while the benchmarked process runs. Run the language interpreter as a background job, poll memory in a tight loop, wait for completion, return the peak value.

**Warning signs:** `raw.json` has no `memory_kb` field or all values are 0.

### Pitfall 3: MAD Is Not Provided by hyperfine

**What goes wrong:** INFRA-04 requires "median ± MAD over N runs." hyperfine provides `median` and `stddev` but NOT MAD. If `bench_runner.sh` just passes through hyperfine's JSON, the `mad` field will be absent from `raw.json`.

**Why it happens:** MAD (Median Absolute Deviation) is a more robust dispersion metric than stddev but less commonly implemented.

**How to avoid:** Compute MAD from hyperfine's `times[]` array using jq: `(times | map(. - median | fabs) | sort)` then take the middle element. Add the computed `mad` field to the per-language JSON object before assembling `raw.json`.

**Warning signs:** `raw.json` has `stddev` per language entry but no `mad` field.

### Pitfall 4: Squirrel Binary Is `sq`, Not `squirrel`

**What goes wrong:** ARCHITECTURE.md (existing pre-phase research) states the binary is called `squirrel`. The actual cmake build output is `sq`. Running `squirrel stub.nut` will fail with "command not found."

**Why it happens:** The project directory contains both `/sq/` and `/squirrel/` subdirectories; the `/squirrel/` directory is the C++ library, not the interpreter. The interpreter binary from cmake is `sq`.

**How to avoid:** After cmake build, copy `build/sq/sq` to `/usr/local/bin/sq`. Use `sq` everywhere in bench_runner.sh.

**Warning signs:** Docker container startup fails with "sq: command not found" or bench_runner.sh exits with "squirrel: not found."

### Pitfall 5: Squirrel --version Exit Code

**What goes wrong:** Squirrel's `sq` binary may return a non-zero exit code for `--version` or `-v`. Shell scripts using `set -euo pipefail` will abort if the version check exits non-zero.

**How to avoid:** Use `sq --version 2>&1 | head -1 || true` for the version emission. Do not rely on exit code; check stdout/stderr content.

### Pitfall 6: NodeSource setup_22.x Requires Interactive Shell Context

**What goes wrong:** `curl ... | bash -` may fail inside Docker without the `-E` flag to preserve environment, or may require `apt-get` to be updated first.

**How to avoid:** Use the explicit GPG key import method that does not rely on a pipe to bash:
```dockerfile
RUN curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" \
    > /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && apt-get install -y nodejs
```

**Warning signs:** Docker build hangs at NodeSource setup step or exits with "gpg: keyserver receive failed."

### Pitfall 7: Docker Build Context Must Be Repo Root

**What goes wrong:** `docker build benchmark/runner/` uses `benchmark/runner/` as context. Stage 1 (`COPY . .`) will only copy the runner subdirectory, not the full Cargo workspace.

**Why it happens:** The CONTEXT.md decision is `docker build -t writ-bench -f benchmark/runner/Dockerfile .` — the context is `.` (repo root). This is non-standard (the Dockerfile is not in the root). The `-f` flag specifies Dockerfile path; the `.` is the context.

**How to avoid:** Always use `docker build -t writ-bench -f benchmark/runner/Dockerfile .` with the repo root as context. The runner scripts (`run.sh`, `run.ps1`) must use `$REPO_ROOT` as the Docker build context, not `$SCRIPT_DIR`.

**Warning signs:** Stage 1 `cargo build` fails with "error: could not find `Cargo.toml`."

### Pitfall 8: /proc/pid/status Memory Polling Race Condition

**What goes wrong:** The background process completes before the polling loop starts. Peak RSS is never captured. Returns 0.

**How to avoid:** Start the polling loop immediately after the `&` backgrounding. Use `kill -0 "$pid"` to check if the process is still running. For very short-lived processes (stub benchmarks), accept that memory measurement may return 0 for near-instant executables — document this in the output schema (`memory_kb: 0` means "too fast to measure").

---

## Code Examples

Verified patterns from official sources and codebase inspection:

### bench_runner.sh Core Loop

```bash
#!/bin/bash
set -euo pipefail

RESULTS_DIR="${RESULTS_DIR:-/results}"
RUNS="${RUNS:-10}"
WARMUP="${WARMUP:-2}"

# Memory measurement: anonymous RSS from /proc/<pid>/status
measure_anon_rss() {
    local peak_kb=0
    "$@" &
    local pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        local rss
        rss=$(awk '/^RssAnon:/{print $2}' "/proc/$pid/status" 2>/dev/null || echo 0)
        [ "$rss" -gt "$peak_kb" ] 2>/dev/null && peak_kb=$rss
    done
    wait "$pid" || true
    echo "$peak_kb"
}

# MAD computation: takes a hyperfine JSON result object and adds mad field
add_mad_to_result() {
    jq '.results[0] + {
        mad: (
            .results[0].times as $times |
            ($times | length) as $n |
            (.results[0].median) as $med |
            ($times | map(. - $med | if . < 0 then -. else . end) | sort) as $devs |
            $devs[($n / 2 | floor)]
        )
    }'
}

mkdir -p "$RESULTS_DIR"

# Emit version strings (success criterion #5)
echo "=== Runtime Versions ==="
lua5.4 -v 2>&1 | head -1
sq --version 2>&1 | head -1 || echo "sq (version check failed)"
python3 --version
node --version
writ --version 2>/dev/null || echo "writ binary present"
hyperfine --version
echo "========================="

results='{"benchmarks":[],"meta":{}}'

for suite_dir in /bench/cases/*/; do
    suite=$(basename "$suite_dir")

    # --- Writ compile ---
    # Pre-compile once for startup measurement
    writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}.writc" 2>/dev/null || true

    hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json "/tmp/writ_compile_raw.json" \
        "writ compile ${suite_dir}${suite}.writ -o /tmp/${suite}.writc" \
        2>/dev/null
    writ_compile_json=$(add_mad_to_result < /tmp/writ_compile_raw.json)

    # --- Writ run ---
    hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json "/tmp/writ_run_raw.json" \
        "writ run /tmp/${suite}.writc" \
        2>/dev/null
    writ_run_json=$(add_mad_to_result < /tmp/writ_run_raw.json)
    writ_mem_kb=$(measure_anon_rss writ run "/tmp/${suite}.writc")

    # --- Lua ---
    hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json "/tmp/lua_raw.json" \
        "lua5.4 ${suite_dir}${suite}.lua" \
        2>/dev/null
    lua_json=$(add_mad_to_result < /tmp/lua_raw.json)
    lua_mem_kb=$(measure_anon_rss lua5.4 "${suite_dir}${suite}.lua")

    # --- (similar for squirrel, python, node, rust) ---

    # Assemble suite result
    results=$(echo "$results" | jq \
        --arg suite "$suite" \
        --argjson writ_compile "$writ_compile_json" \
        --argjson writ_run "$writ_run_json" \
        --argjson writ_mem "$writ_mem_kb" \
        # ... other languages
        '.benchmarks += [{
            suite: $suite,
            writ_compile: $writ_compile,
            writ_run: ($writ_run + {memory_kb: $writ_mem}),
            # ... other languages
        }]')
done

# Inject meta
DATE=$(date +%Y-%m-%d)
PLATFORM=$(uname -m)
results=$(echo "$results" | jq \
    --arg date "$DATE" --arg runs "$RUNS" --arg warmup "$WARMUP" --arg platform "$PLATFORM" \
    '.meta = {date: $date, runs: ($runs | tonumber), warmup: ($warmup | tonumber), platform: $platform}')

echo "$results" > "$RESULTS_DIR/raw.json"
echo "Written: $RESULTS_DIR/raw.json"
```

### Writ CLI Invocation (verified from compile.rs and run.rs)

```bash
# Compile (produces .writc file, prints "Compiled: /tmp/stub.writc" to stderr)
writ compile /bench/cases/stub/stub.writ -o /tmp/stub.writc

# Run (entry defaults to "main"; no extra flags needed for stub benchmarks)
writ run /tmp/stub.writc

# Note: writ compile prints to stderr only ("Compiled: ...")
# Note: writ run exits 0 on success; exits 1 on error with "error: ..." on stderr
```

### run.sh (POSIX sh)

```sh
#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DATE=$(date +%Y-%m-%d)
RESULTS_DIR="$REPO_ROOT/benchmark/results/$DATE"

mkdir -p "$RESULTS_DIR"

if command -v docker > /dev/null 2>&1; then
    CONTAINER_CMD=docker
elif command -v podman > /dev/null 2>&1; then
    CONTAINER_CMD=podman
else
    echo "error: docker or podman required" >&2
    exit 1
fi

echo "Building benchmark image..."
"$CONTAINER_CMD" build -t writ-bench -f "$SCRIPT_DIR/Dockerfile" "$REPO_ROOT"

echo "Running benchmarks..."
"$CONTAINER_CMD" run --rm \
    -v "$RESULTS_DIR:/results" \
    -e "RESULTS_DIR=/results" \
    -e "RUNS=${RUNS:-10}" \
    writ-bench

echo "Done. Results: $RESULTS_DIR/raw.json"
```

### run.ps1 (PowerShell)

```powershell
param([int]$Runs = 10, [string]$ContainerCmd = "")
$ErrorActionPreference = "Stop"

$ScriptDir  = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot   = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$Date       = Get-Date -Format "yyyy-MM-dd"
$ResultsDir = Join-Path $RepoRoot "benchmark\results\$Date"

New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null

if ($ContainerCmd -eq "") {
    if (Get-Command docker -ErrorAction SilentlyContinue) { $ContainerCmd = "docker" }
    elseif (Get-Command podman -ErrorAction SilentlyContinue) { $ContainerCmd = "podman" }
    else { Write-Error "docker or podman required"; exit 1 }
}

# Normalize Windows path for Docker volume mount
$ResultsMounted = $ResultsDir -replace '\\', '/' `
    -replace '^([A-Za-z]):', { "/$($_.Groups[1].Value.ToLower())" }

Write-Host "Building benchmark image..."
& $ContainerCmd build -t writ-bench -f "$ScriptDir\Dockerfile" $RepoRoot

Write-Host "Running benchmarks..."
& $ContainerCmd run --rm `
    -v "${ResultsMounted}:/results" `
    -e "RESULTS_DIR=/results" `
    -e "RUNS=$Runs" `
    writ-bench

Write-Host "Done. Results: $ResultsDir\raw.json"
```

### raw.json Schema (Phase 70 stub output)

```json
{
  "benchmarks": [
    {
      "suite": "stub",
      "writ_compile": {
        "command": "writ compile /bench/cases/stub/stub.writ -o /tmp/stub.writc",
        "mean": 0.031, "stddev": 0.002, "median": 0.030, "mad": 0.001,
        "min": 0.028, "max": 0.035,
        "times": [0.031, 0.030, ...],
        "memory_kb": 0
      },
      "writ_run": {
        "command": "writ run /tmp/stub.writc",
        "mean": 0.004, "stddev": 0.0003, "median": 0.004, "mad": 0.0002,
        "min": 0.003, "max": 0.005,
        "times": [0.004, ...],
        "memory_kb": 1024
      },
      "lua": {
        "command": "lua5.4 /bench/cases/stub/stub.lua",
        "execution_ms": 0.002,
        "median": 0.002, "mad": 0.0001,
        "memory_kb": 512
      },
      "squirrel": { ... },
      "python":   { ... },
      "node":     { ... },
      "rust":     { ... },
      "startup": {
        "writ_ms":     4.2,
        "lua_ms":      1.8,
        "squirrel_ms": 2.1,
        "python_ms":   45.3,
        "node_ms":     87.6,
        "rust_ms":     0.8
      }
    }
  ],
  "meta": {
    "date": "2026-03-20",
    "runs": 10,
    "warmup": 2,
    "platform": "x86_64"
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| hyperfine via apt | hyperfine .deb from GitHub Releases | Always | hyperfine never packaged in Ubuntu default repos |
| Node.js 20 LTS (STACK.md recommendation) | Node.js 22 LTS (STATE.md locked decision) | STATE.md decision | Node 20 EOL April 2026; use 22 LTS |
| squirrel binary named `squirrel` (ARCHITECTURE.md) | Binary is `sq` | Verified from cmake output | bench_runner.sh must use `sq`, not `squirrel` |
| hyperfine memory_usage_byte (assumed available) | Must use /proc shell function | hyperfine PR #790 unmerged | Adds custom memory polling to harness |
| MAD from hyperfine (assumed available) | Must compute from times[] via jq | hyperfine never had MAD | Adds jq MAD computation to harness |

**Deprecated/outdated (in prior research documents):**
- STACK.md recommends Node.js 20 LTS — overridden by STATE.md locked decision (Node.js 22 LTS)
- ARCHITECTURE.md uses `squirrel` as binary name — incorrect; cmake produces `sq`
- ARCHITECTURE.md does not include memory measurement implementation — must use custom shell function
- ARCHITECTURE.md does not include MAD computation — must compute from times[] via jq
- STACK.md mentions `writ-bench` Rust crate — explicitly overridden in CONTEXT.md (Phase 70 uses hyperfine + bash, not Rust harness)

---

## Open Questions

1. **Does `writ` have a `--version` flag?**
   - What we know: `writ-cli/src/main.rs` uses clap with `#[command(name = "writ", about = "Writ IL toolchain")]` but no explicit `--version` flag visible in the parsed struct.
   - What's unclear: Whether clap auto-generates `--version` from `Cargo.toml` version field.
   - Recommendation: Test `writ --version` in the Dockerfile; if it fails, use `writ --help | head -1` or just omit from version emission. Fall back to `echo "writ (present)"`.

2. **Does the Squirrel cmake build require internet access at Docker build time?**
   - What we know: `git clone` requires network; Docker build has network access by default.
   - What's unclear: Whether GitHub rate limiting or firewall rules affect the clone during CI builds.
   - Recommendation: Use `--depth 1 --branch v3.2` for a shallow clone; if clone fails in CI, consider vendoring the squirrel source as a git submodule.

3. **What output does `writ run stub.writc` produce for the stub benchmark?**
   - What we know: `log::info("hello")` is available in Writ (from the inject_log_extern_defs pipeline). The CliHost processes log requests synchronously.
   - What's unclear: Exact stdout vs stderr behavior — does `log::info` write to stdout or stderr in CliHost?
   - Recommendation: Check `writ-cli/src/cli_host.rs` to confirm. For the stub, any output that exits 0 is sufficient.

4. **Does Squirrel `sq` accept standard input or require a file argument?**
   - What we know: `sq script.nut` runs a script file. Documentation shows file-based invocation.
   - What's unclear: Whether `sq --version` exits 0 or non-zero.
   - Recommendation: Use `sq --version 2>&1 | head -1 || true` with `|| true` to ignore exit code.

---

## Validation Architecture

`workflow.nyquist_validation` key is absent from `.planning/config.json` — treating as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | None (bash scripts and Docker) — no Rust test framework needed for Phase 70 |
| Config file | N/A |
| Quick run command | `bash -n benchmark/runner/bench_runner.sh` (syntax check) |
| Full suite command | `docker build -t writ-bench -f benchmark/runner/Dockerfile . && docker run --rm -v /tmp/bench-test:/results writ-bench` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-01 | All 6 runtimes in Docker image | smoke | `docker run --rm writ-bench lua5.4 -v && docker run --rm writ-bench sq --version && docker run --rm writ-bench python3 --version && docker run --rm writ-bench node --version && docker run --rm writ-bench writ --help` | ❌ Wave 0 |
| INFRA-02 | run.sh produces raw.json with Docker/Podman only | integration | `bash benchmark/runner/run.sh && test -f benchmark/results/*/raw.json` | ❌ Wave 0 |
| INFRA-03 | run.ps1 produces raw.json on Windows | manual-only | PowerShell on Windows required | N/A |
| INFRA-04 | raw.json has median and MAD fields per language | smoke | `jq '.benchmarks[0].writ_run.median, .benchmarks[0].writ_run.mad' benchmark/results/*/raw.json` | ❌ Wave 0 |
| INFRA-05 | raw.json has memory_kb field per language | smoke | `jq '.benchmarks[0].writ_run.memory_kb' benchmark/results/*/raw.json` | ❌ Wave 0 |
| INFRA-06 | raw.json has startup fields | smoke | `jq '.benchmarks[0].startup' benchmark/results/*/raw.json` | ❌ Wave 0 |
| INFRA-07 | writ_compile and writ_run are separate JSON keys | smoke | `jq 'has("writ_compile") and has("writ_run")' <<< $(jq '.benchmarks[0]' benchmark/results/*/raw.json)` | ❌ Wave 0 |
| INFRA-08 | raw.json is valid JSON with expected top-level keys | smoke | `jq '.benchmarks \| length, .meta.date' benchmark/results/*/raw.json` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `bash -n benchmark/runner/bench_runner.sh` (syntax only — Docker not available in all dev environments)
- **Per wave merge:** `docker build + docker run` full pipeline
- **Phase gate:** Full Docker build + run + raw.json validation before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `benchmark/runner/Dockerfile` — multi-stage build (does not exist yet)
- [ ] `benchmark/runner/bench_runner.sh` — orchestration script (does not exist yet)
- [ ] `benchmark/runner/run.sh` — host launcher (does not exist yet)
- [ ] `benchmark/runner/run.ps1` — Windows launcher (does not exist yet)
- [ ] `benchmark/cases/stub/stub.writ` — stub benchmark (does not exist yet)
- [ ] `benchmark/cases/stub/stub.lua` — stub benchmark (does not exist yet)
- [ ] `benchmark/cases/stub/stub.nut` — stub benchmark (does not exist yet)
- [ ] `benchmark/cases/stub/stub.py` — stub benchmark (does not exist yet)
- [ ] `benchmark/cases/stub/stub.js` — stub benchmark (does not exist yet)
- [ ] `benchmark/cases/stub/stub.rs` — stub benchmark (does not exist yet)
- [ ] `benchmark/results/.gitkeep` — ensure results/ tracked in git

---

## Sources

### Primary (HIGH confidence)

- `D:\dev\git\Writ\writ-cli\src\main.rs` — exact CLI subcommand signatures, `writ compile <input> -o <output>` and `writ run <input> --entry <name>` verified
- `D:\dev\git\Writ\writ-cli\src\commands\compile.rs` — compile output: writes `<input>.writc` by default, prints "Compiled: <path>" to stderr, exits 0
- `D:\dev\git\Writ\writ-cli\src\commands\run.rs` — run signature: `writ run <input> [--entry main]`, exits 0 on success
- `github.com/sharkdp/hyperfine/blob/v1.20.0/src/benchmark/benchmark_result.rs` — confirmed JSON fields: command, mean, stddev, median, user, system, min, max, times[], exit_codes (NO memory_usage_byte in v1.20.0 release)
- `github.com/sharkdp/hyperfine/pull/790` — memory_usage_byte is in unmerged DRAFT PR, not released
- `github.com/sharkdp/hyperfine/releases/tag/v1.20.0` — latest release v1.20.0, November 18, 2025
- `man7.org/linux/man-pages/man5/proc_pid_status.5.html` — VmRSS = RssAnon + RssFile + RssShmem; RssAnon is anonymous RSS; available since Linux kernel 4.5

### Secondary (MEDIUM confidence)

- Ubuntu packages.ubuntu.com squirrel3 search — squirrel3 absent from ubuntu 24.04 noble; present in oracular (24.10) and plucky (25.04) but NOT noble
- `github.com/albertodemichelis/squirrel/blob/master/COMPILE` — cmake build procedure; `make install` copies to `/usr/local/bin/sq`
- NodeSource installation guide — setup_22.x PPA method for Node.js 22 LTS on Ubuntu 24.04; GPG key import approach preferred over pipe-to-bash
- linuxcommandlibrary.com/man/hyperfine — confirmed: no `--enable-memory-measurement` flag in hyperfine man page

### Tertiary (LOW confidence — flag for validation)

- Wikipedia: Median absolute deviation — formula: median(|Xi - median(X)|); used as basis for jq implementation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — hyperfine version and JSON schema verified from source code; squirrel apt absence confirmed; NodeSource PPA method verified
- Architecture: HIGH — writ CLI signatures verified from source; Dockerfile pattern verified against CONTEXT.md locked decisions
- Pitfalls: HIGH — hyperfine no-memory-measurement and no-MAD verified directly from v1.20.0 source; squirrel3 apt absence confirmed from Ubuntu package search

**Research date:** 2026-03-20
**Valid until:** 2026-06-20 (stable infrastructure; hyperfine releases infrequently)

**Critical corrections to prior research documents:**
1. ARCHITECTURE.md uses `squirrel` as binary name — INCORRECT. cmake build produces `sq`.
2. ARCHITECTURE.md assumes hyperfine exports memory data — INCORRECT. Must use /proc shell function.
3. STACK.md recommends Node.js 20 LTS — OVERRIDDEN by STATE.md locked decision (Node.js 22 LTS).
4. Both prior docs omit MAD computation — must derive from hyperfine `times[]` via jq.
