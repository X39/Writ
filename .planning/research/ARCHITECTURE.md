# Architecture Research

**Domain:** Cross-Language Benchmark Suite — Docker-based multi-runtime benchmark runner integrated with the Writ toolchain
**Researched:** 2026-03-20
**Confidence:** HIGH (existing codebase inspected directly; Docker/hyperfine/GitHub Actions patterns verified against official docs and reference suites)

---

## System Overview

```
+---------------------------------------------------------------+
|              benchmark/ (top-level directory)                  |
|                                                                |
|  +---------------+  +------------------+  +-----------------+ |
|  |  cases/       |  |  runner/          |  |  results/       | |
|  |  (6 languages |  |  run.sh           |  |  YYYY-MM-DD/    | |
|  |   x 4 suites) |  |  run.ps1          |  |  raw.json       | |
|  +-------+-------+  |  Dockerfile       |  |  charts/*.svg   | |
|          |          +--------+----------+  |  RESULTS.md     | |
|          |                   |             +-----------------+ |
|          v                   v                                 |
|  +-------+-------------------+-----------------------------+   |
|  |         Docker Container (ubuntu:24.04 base)            |   |
|  |                                                         |   |
|  |  Runtimes:  lua5.4  squirrel3  python3  node  rustc     |   |
|  |  Writ:      /usr/local/bin/writ  (copied from build)    |   |
|  |  Tools:     hyperfine  /usr/bin/time                    |   |
|  |                                                         |   |
|  |  bench_runner.sh  (orchestrates all cases)              |   |
|  |  -> for each case: measure compile + run separately     |   |
|  |  -> write JSON to /results/raw.json                     |   |
|  +---+-----------------------------------------------------+   |
|      |                                                         |
|      v                                                         |
|  +---+--------------------+                                    |
|  |  chart_gen/            |  (Python, runs outside container) |
|  |  generate.py           |  JSON -> SVG + markdown table     |
|  +------------------------+                                    |
+---------------------------------------------------------------+
         |
         v
+-------------------+   +---------------------------+
|  GitHub Actions   |   |  benchmark/results/        |
|  benchmark.yml    |-->|  committed to repo          |
|  (on: workflow_   |   |  charts referenced in       |
|   dispatch +      |   |  README.md                  |
|   schedule)       |   +---------------------------+
+-------------------+
```

### Component Responsibilities

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| `benchmark/cases/` | One subdirectory per benchmark suite; each contains 6 source files | Static files, no build step |
| `benchmark/runner/Dockerfile` | Single image with all 6 language runtimes + hyperfine + writ binary | Multi-stage Docker build |
| `benchmark/runner/bench_runner.sh` | Runs all cases inside the container; writes `/results/raw.json` | Bash, calls hyperfine |
| `benchmark/runner/run.sh` | Host-side entry point (Linux/macOS): builds image, mounts results, runs container | Bash |
| `benchmark/runner/run.ps1` | Host-side entry point (Windows): same logic in PowerShell | PowerShell |
| `benchmark/chart_gen/generate.py` | Reads `raw.json`, produces SVG charts and a markdown table | Python 3, pygal |
| `benchmark/results/` | Versioned output directory; SVG files + `RESULTS.md` committed to repo | Git-tracked |
| `.github/workflows/benchmark.yml` | CI workflow: builds writ, runs benchmark container, commits results | GitHub Actions |

---

## Recommended Project Structure

```
benchmark/
├── cases/
│   ├── fib/                    # Compute-heavy: recursive Fibonacci
│   │   ├── fib.writ
│   │   ├── fib.lua
│   │   ├── fib.nut             # Squirrel uses .nut extension
│   │   ├── fib.py
│   │   ├── fib.js
│   │   └── fib.rs
│   ├── string_processing/      # String processing: split/join/format
│   │   ├── strings.writ
│   │   ├── strings.lua
│   │   ├── strings.nut
│   │   ├── strings.py
│   │   ├── strings.js
│   │   └── strings.rs
│   ├── data_structures/        # Data structures: list/map operations
│   │   ├── data.writ
│   │   ├── data.lua
│   │   ├── data.nut
│   │   ├── data.py
│   │   ├── data.js
│   │   └── data.rs
│   └── dispatch/               # OOP/dispatch: virtual calls, method dispatch
│       ├── dispatch.writ
│       ├── dispatch.lua
│       ├── dispatch.nut
│       ├── dispatch.py
│       ├── dispatch.js
│       └── dispatch.rs
├── runner/
│   ├── Dockerfile              # All runtimes + writ binary
│   ├── bench_runner.sh         # Runs inside container; writes raw.json
│   ├── run.sh                  # Host entry: Linux/macOS
│   └── run.ps1                 # Host entry: Windows
├── chart_gen/
│   ├── generate.py             # JSON -> SVG charts + markdown table
│   └── requirements.txt        # pygal (no other deps needed)
└── results/
    ├── latest/                 # Symlink or copy of most recent run
    │   ├── raw.json
    │   ├── charts/
    │   │   ├── fib.svg
    │   │   ├── string_processing.svg
    │   │   ├── data_structures.svg
    │   │   └── dispatch.svg
    │   └── RESULTS.md
    └── YYYY-MM-DD/             # Dated archives of past runs
        └── ...
```

### Structure Rationale

- **cases/ flat language files:** Each benchmark is a single source file per language. No per-language subdirectory nesting — keeps comparison immediately visible (`ls benchmark/cases/fib/`).
- **runner/ self-contained:** The `Dockerfile`, inside-container script, and host-side launcher scripts are co-located. Anyone can reproduce the run with `./benchmark/runner/run.sh` without reading anything else.
- **chart_gen/ separate from runner/:** Chart generation runs on the host after the container exits. It has no Docker dependency — a developer can regenerate charts from any `raw.json` without re-running benchmarks.
- **results/ git-tracked:** SVG files and the markdown table are committed so the README can embed them as relative links. Dated subdirectory archives let historical comparisons be done by diffing `raw.json` files.

---

## Dockerfile Design

### Multi-Stage Build Pattern

```dockerfile
# Stage 1: Build the writ binary from source
FROM rust:1.85-slim AS writ-builder
WORKDIR /writ
COPY . .
RUN cargo build --release --bin writ

# Stage 2: Build the Rust benchmark binaries
# (pre-compile so runtime measurement excludes compilation overhead)
FROM rust:1.85-slim AS rust-bench-builder
WORKDIR /bench
COPY benchmark/cases/ ./cases/
# Compile each Rust benchmark case to a binary
RUN for dir in cases/*/; do \
      name=$(basename "$dir"); \
      rustc -O -o "/bench/bin/${name}" "${dir}${name}.rs"; \
    done

# Stage 3: Final runtime image
FROM ubuntu:24.04

# Install all 5 scripting language runtimes
RUN apt-get update && apt-get install -y \
    lua5.4 \
    squirrel3 \
    python3 \
    nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install hyperfine for timing
RUN apt-get update && apt-get install -y curl && \
    HYPERFINE_VERSION=1.18.0 && \
    curl -sSL "https://github.com/sharkdp/hyperfine/releases/download/v${HYPERFINE_VERSION}/hyperfine_${HYPERFINE_VERSION}_amd64.deb" \
      -o hyperfine.deb && \
    dpkg -i hyperfine.deb && rm hyperfine.deb && \
    apt-get remove -y curl && rm -rf /var/lib/apt/lists/*

# Copy writ binary from builder stage
COPY --from=writ-builder /writ/target/release/writ /usr/local/bin/writ

# Copy pre-compiled Rust benchmark binaries
COPY --from=rust-bench-builder /bench/bin/ /bench/bin/

# Copy benchmark cases and runner script
COPY benchmark/cases/ /bench/cases/
COPY benchmark/runner/bench_runner.sh /bench/bench_runner.sh
RUN chmod +x /bench/bench_runner.sh

WORKDIR /bench
ENTRYPOINT ["/bench/bench_runner.sh"]
```

**Why multi-stage:** The `writ` binary and Rust benchmark binaries are compiled in dedicated builder stages that include the full Rust toolchain. The final image only needs the runtime tools (lua, squirrel3, python3, nodejs, hyperfine) — no Rust compiler in the benchmark image. This keeps the image small and ensures compile time is not accidentally included in runtime measurements.

**Why pre-compile Rust:** Rust compilation time would dominate all other measurements if done at benchmark time. Compiling in a separate stage means the Rust measurement only captures execution time, matching the semantics of the other interpreted-language measurements.

---

## Invoking Each Language

### Writ (compile-then-run, measured separately)

```bash
# Compile step (timed separately)
writ compile /bench/cases/fib/fib.writ -o /tmp/fib.writc

# Run step (timed separately)
writ run /tmp/fib.writc
```

Both steps are timed with hyperfine in separate passes. The JSON output records `writ_compile_ms` and `writ_run_ms` as distinct fields.

### Lua

```bash
lua5.4 /bench/cases/fib/fib.lua
```

Binary: `lua5.4` (Ubuntu package: `lua5.4`). Startup + execution measured together.

### Squirrel

```bash
squirrel /bench/cases/fib/fib.nut
```

Binary: `squirrel` (Ubuntu package: `squirrel3`). The interpreter is called `squirrel`, not `sq`. Script extension is `.nut`.

### Python

```bash
python3 /bench/cases/fib/fib.py
```

Binary: `python3` (Ubuntu package: `python3`). No virtualenv needed — benchmarks use only stdlib.

### Node.js

```bash
node /bench/cases/fib/fib.js
```

Binary: `node` (Ubuntu package: `nodejs`). No npm install needed — benchmarks use only stdlib.

### Rust (pre-compiled binary)

```bash
/bench/bin/fib
```

Binary compiled with `rustc -O` in the builder stage. Execution only — no compile step measured in the runtime pass. Compile time is recorded separately from the `rust-bench-builder` stage timing if desired, but is not included in the comparison charts (Rust compile time is not comparable to interpreter startup).

---

## bench_runner.sh Design

```bash
#!/usr/bin/env bash
set -euo pipefail

RESULTS_DIR="${RESULTS_DIR:-/results}"
RUNS="${RUNS:-10}"
WARMUP="${WARMUP:-2}"

mkdir -p "$RESULTS_DIR"

# Output accumulator: array of JSON objects
results='{"benchmarks":[]}'

for suite_dir in /bench/cases/*/; do
    suite=$(basename "$suite_dir")

    # --- Writ: compile time ---
    writ compile "${suite_dir}${suite}.writ" -o /tmp/${suite}.writc 2>/dev/null
    writ_compile_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/writ_compile.json \
        "writ compile ${suite_dir}${suite}.writ -o /tmp/${suite}.writc" \
        2>/dev/null && cat /tmp/writ_compile.json)

    # --- Writ: run time ---
    writ_run_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/writ_run.json \
        "writ run /tmp/${suite}.writc" \
        2>/dev/null && cat /tmp/writ_run.json)

    # --- Lua ---
    lua_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/lua.json \
        "lua5.4 ${suite_dir}${suite}.lua" \
        2>/dev/null && cat /tmp/lua.json)

    # --- Squirrel ---
    sq_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/sq.json \
        "squirrel ${suite_dir}${suite}.nut" \
        2>/dev/null && cat /tmp/sq.json)

    # --- Python ---
    py_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/py.json \
        "python3 ${suite_dir}${suite}.py" \
        2>/dev/null && cat /tmp/py.json)

    # --- Node.js ---
    node_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/node.json \
        "node ${suite_dir}${suite}.js" \
        2>/dev/null && cat /tmp/node.json)

    # --- Rust ---
    rust_json=$(hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --export-json /tmp/rust.json \
        "/bench/bin/${suite}" \
        2>/dev/null && cat /tmp/rust.json)

    # Merge into results using jq
    results=$(echo "$results" | jq \
        --arg suite "$suite" \
        --argjson writ_compile "$writ_compile_json" \
        --argjson writ_run "$writ_run_json" \
        --argjson lua "$lua_json" \
        --argjson sq "$sq_json" \
        --argjson py "$py_json" \
        --argjson node "$node_json" \
        --argjson rust "$rust_json" \
        '.benchmarks += [{
            suite: $suite,
            writ_compile: $writ_compile.results[0],
            writ_run: $writ_run.results[0],
            lua: $lua.results[0],
            squirrel: $sq.results[0],
            python: $py.results[0],
            node: $node.results[0],
            rust: $rust.results[0]
        }]')
done

echo "$results" > "$RESULTS_DIR/raw.json"
echo "Results written to $RESULTS_DIR/raw.json"
```

**Note:** `jq` must be added to the Docker image (`apt-get install -y jq`). It is the cleanest way to merge JSON fragments inside a shell script without writing a Python helper.

---

## raw.json Schema

hyperfine exports each measurement as:

```json
{
  "command": "writ run /tmp/fib.writc",
  "mean": 0.004231,
  "stddev": 0.000123,
  "median": 0.004198,
  "user": 0.003987,
  "system": 0.000244,
  "min": 0.004012,
  "max": 0.004891,
  "times": [0.004231, 0.004198, ...]
}
```

The `bench_runner.sh` wraps these into a top-level structure:

```json
{
  "benchmarks": [
    {
      "suite": "fib",
      "writ_compile": { "mean": 0.0312, "stddev": 0.002, ... },
      "writ_run":     { "mean": 0.0041, "stddev": 0.0001, ... },
      "lua":          { "mean": 0.0018, "stddev": 0.00008, ... },
      "squirrel":     { "mean": 0.0023, "stddev": 0.0001, ... },
      "python":       { "mean": 0.0412, "stddev": 0.002, ... },
      "node":         { "mean": 0.0891, "stddev": 0.003, ... },
      "rust":         { "mean": 0.00012, "stddev": 0.000005, ... }
    },
    ...
  ],
  "meta": {
    "date": "2026-03-20",
    "runs": 10,
    "warmup": 2,
    "platform": "linux/amd64"
  }
}
```

All times are in seconds (hyperfine always uses seconds in JSON output, regardless of display format).

---

## Host-Side Runner Scripts

### run.sh (Linux/macOS)

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESULTS_DIR="$REPO_ROOT/benchmark/results/$(date +%Y-%m-%d)"

mkdir -p "$RESULTS_DIR"

# Detect Docker or Podman
if command -v docker &>/dev/null; then
    CONTAINER_CMD=docker
elif command -v podman &>/dev/null; then
    CONTAINER_CMD=podman
else
    echo "error: neither docker nor podman found in PATH" >&2
    exit 1
fi

# Build image
echo "Building benchmark image..."
"$CONTAINER_CMD" build -t writ-benchmark "$REPO_ROOT" \
    -f "$SCRIPT_DIR/Dockerfile"

# Run container, mount results dir
echo "Running benchmarks..."
"$CONTAINER_CMD" run --rm \
    -v "$RESULTS_DIR:/results" \
    -e "RESULTS_DIR=/results" \
    -e "RUNS=${RUNS:-10}" \
    writ-benchmark

# Generate charts
echo "Generating charts..."
python3 "$REPO_ROOT/benchmark/chart_gen/generate.py" \
    --input "$RESULTS_DIR/raw.json" \
    --output "$RESULTS_DIR"

echo "Done. Results in: $RESULTS_DIR"
```

### run.ps1 (Windows PowerShell)

```powershell
param(
    [int]$Runs = 10,
    [string]$ContainerCmd = ""
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot  = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$Date      = Get-Date -Format "yyyy-MM-dd"
$ResultsDir = Join-Path $RepoRoot "benchmark\results\$Date"

New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null

# Detect Docker or Podman
if ($ContainerCmd -eq "") {
    if (Get-Command docker -ErrorAction SilentlyContinue) {
        $ContainerCmd = "docker"
    } elseif (Get-Command podman -ErrorAction SilentlyContinue) {
        $ContainerCmd = "podman"
    } else {
        Write-Error "Neither docker nor podman found in PATH"
        exit 1
    }
}

# Docker requires Unix-style paths for volume mounts on Windows
$ResultsMounted = $ResultsDir -replace '\\', '/' -replace '^([A-Z]):', { "/$($_.Groups[1].Value.ToLower())" }

Write-Host "Building benchmark image..."
& $ContainerCmd build -t writ-benchmark $RepoRoot -f "$ScriptDir\Dockerfile"

Write-Host "Running benchmarks..."
& $ContainerCmd run --rm `
    -v "${ResultsMounted}:/results" `
    -e "RESULTS_DIR=/results" `
    -e "RUNS=$Runs" `
    writ-benchmark

Write-Host "Generating charts..."
python "$RepoRoot\benchmark\chart_gen\generate.py" `
    --input "$ResultsDir\raw.json" `
    --output $ResultsDir

Write-Host "Done. Results in: $ResultsDir"
```

**Key decisions:**
- Both scripts require only Docker or Podman — no other tooling.
- Path normalization for Windows volume mounts is handled in the PowerShell script.
- `RUNS` is configurable via environment variable (shell) or parameter (PowerShell) without editing the script.
- Both scripts call `generate.py` after the container exits, so chart generation happens on the host (no need for Python inside the container).

---

## Chart Generation Pipeline

### generate.py Design

```python
#!/usr/bin/env python3
"""
benchmark/chart_gen/generate.py

Reads raw.json and produces:
  - One SVG bar chart per benchmark suite (execution time, log scale)
  - One SVG bar chart for Writ compile+run combined (absolute time)
  - RESULTS.md with embedded chart links and a markdown table
"""
import json
import argparse
import os
from datetime import date

import pygal
from pygal.style import CleanStyle

LANGUAGES = ["writ_run", "lua", "squirrel", "python", "node", "rust"]
LANGUAGE_LABELS = {
    "writ_run":  "Writ (run)",
    "lua":       "Lua 5.4",
    "squirrel":  "Squirrel 3",
    "python":    "Python 3",
    "node":      "Node.js",
    "rust":      "Rust (native)",
}

def ms(seconds: float) -> float:
    return round(seconds * 1000, 3)

def generate_chart(suite_name: str, data: dict, output_dir: str):
    chart = pygal.Bar(style=CleanStyle, logarithmic=True,
                      title=f"{suite_name} — execution time (ms, log scale)",
                      y_title="ms", x_title="Language")
    for lang in LANGUAGES:
        if lang in data and data[lang]:
            chart.add(LANGUAGE_LABELS[lang], [ms(data[lang]["mean"])])
    chart.render_to_file(os.path.join(output_dir, "charts", f"{suite_name}.svg"))

def generate_writ_compile_chart(benchmarks: list, output_dir: str):
    chart = pygal.Bar(style=CleanStyle,
                      title="Writ: compile time vs run time (ms)",
                      y_title="ms")
    chart.x_labels = [b["suite"] for b in benchmarks]
    chart.add("Compile", [ms(b["writ_compile"]["mean"]) for b in benchmarks])
    chart.add("Run",     [ms(b["writ_run"]["mean"])     for b in benchmarks])
    chart.render_to_file(os.path.join(output_dir, "charts", "writ_compile_vs_run.svg"))

def generate_markdown_table(benchmarks: list) -> str:
    header = "| Suite | Writ (run) | Lua 5.4 | Squirrel 3 | Python 3 | Node.js | Rust |\n"
    sep    = "|-------|------------|---------|------------|----------|---------|------|\n"
    rows = []
    for b in benchmarks:
        def cell(lang):
            if lang in b and b[lang]:
                return f"{ms(b[lang]['mean']):.2f} ms"
            return "—"
        rows.append(
            f"| {b['suite']} | {cell('writ_run')} | {cell('lua')} | "
            f"{cell('squirrel')} | {cell('python')} | {cell('node')} | {cell('rust')} |"
        )
    return header + sep + "\n".join(rows)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input",  required=True, help="Path to raw.json")
    parser.add_argument("--output", required=True, help="Output directory")
    args = parser.parse_args()

    with open(args.input) as f:
        data = json.load(f)

    os.makedirs(os.path.join(args.output, "charts"), exist_ok=True)

    for bench in data["benchmarks"]:
        generate_chart(bench["suite"], bench, args.output)

    generate_writ_compile_chart(data["benchmarks"], args.output)

    table = generate_markdown_table(data["benchmarks"])

    results_md = f"""# Benchmark Results

Generated: {date.today().isoformat()}

## Execution Time (ms, lower is better)

{table}

*All times are median of {data.get('meta', {}).get('runs', 10)} runs inside a Docker container on linux/amd64.*

## Charts

### Fibonacci (compute-heavy)
![fib](charts/fib.svg)

### String Processing
![string_processing](charts/string_processing.svg)

### Data Structures
![data_structures](charts/data_structures.svg)

### OOP/Dispatch
![dispatch](charts/dispatch.svg)

### Writ: Compile Time vs Run Time
![writ_compile_vs_run](charts/writ_compile_vs_run.svg)
"""

    with open(os.path.join(args.output, "RESULTS.md"), "w") as f:
        f.write(results_md)

    print(f"Charts written to {os.path.join(args.output, 'charts')}/")
    print(f"RESULTS.md written to {args.output}")

if __name__ == "__main__":
    main()
```

**Why pygal:** Pure Python, no system dependencies (matplotlib requires a display context or explicit Agg backend configuration), generates clean SVG files directly, installs with `pip install pygal`. The bar chart type and `logarithmic=True` mode are the right defaults for multi-language execution time comparison where Rust and Writ runtime will differ from Python by 10-100x.

**Why log scale:** Without log scale, Rust native and Writ execution bars will be near-invisible next to Python and Node.js for compute-heavy benchmarks. Log scale keeps all bars legible.

---

## GitHub Actions Workflow

```yaml
# .github/workflows/benchmark.yml
name: Benchmark

on:
  workflow_dispatch:        # manual trigger
  schedule:
    - cron: '0 3 * * 1'    # weekly, Monday 03:00 UTC

permissions:
  contents: write           # needed to commit results

jobs:
  benchmark:
    runs-on: ubuntu-24.04
    timeout-minutes: 60

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Python (for chart generation)
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Install chart dependencies
        run: pip install pygal

      - name: Build benchmark image
        run: |
          docker build -t writ-benchmark . \
            -f benchmark/runner/Dockerfile

      - name: Run benchmarks
        run: |
          DATE=$(date +%Y-%m-%d)
          mkdir -p benchmark/results/$DATE
          docker run --rm \
            -v "${{ github.workspace }}/benchmark/results/$DATE:/results" \
            -e RESULTS_DIR=/results \
            -e RUNS=10 \
            writ-benchmark
          echo "RESULTS_DATE=$DATE" >> $GITHUB_ENV

      - name: Generate charts
        run: |
          python benchmark/chart_gen/generate.py \
            --input  "benchmark/results/${{ env.RESULTS_DATE }}/raw.json" \
            --output "benchmark/results/${{ env.RESULTS_DATE }}"

      - name: Upload raw results as artifact
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results-${{ env.RESULTS_DATE }}
          path: benchmark/results/${{ env.RESULTS_DATE }}/

      - name: Commit results to repo
        run: |
          git config user.name  "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add benchmark/results/
          git diff --cached --quiet || \
            git commit -m "benchmark: results for ${{ env.RESULTS_DATE }}"
          git push
```

**Design decisions:**
- `workflow_dispatch` allows manual one-click runs from the GitHub UI.
- `schedule` weekly cron keeps results fresh without burning CI minutes daily.
- Python runs on the host runner (not inside Docker) for chart generation — simpler and faster than installing Python in the benchmark container.
- Results are committed back to the repo so SVG charts can be embedded in `README.md` via relative paths. This is the pattern used by benchmark-action and most language benchmark repos.
- `permissions: contents: write` is required for the commit step in post-2022 GitHub Actions security model.
- No matrix strategy needed — all languages run inside the single Docker container in sequence, not as parallel jobs.

---

## Data Flow

### Full Benchmark Run Flow

```
run.sh (host)
    |
    v
docker build (or podman build)
    -> rust:1.85-slim: cargo build --release --bin writ
    -> rust:1.85-slim: rustc -O cases/*/*.rs -> /bench/bin/*
    -> ubuntu:24.04: apt-get lua5.4 squirrel3 python3 nodejs + hyperfine
    -> copy writ binary, rust binaries, case files, bench_runner.sh
    |
    v
docker run -v $RESULTS_DIR:/results writ-benchmark
    -> /bench/bench_runner.sh executes
    -> for each suite in cases/:
         hyperfine "writ compile ..."  -> writ_compile JSON
         hyperfine "writ run ..."      -> writ_run JSON
         hyperfine "lua5.4 ..."        -> lua JSON
         hyperfine "squirrel ..."      -> squirrel JSON
         hyperfine "python3 ..."       -> python JSON
         hyperfine "node ..."          -> node JSON
         hyperfine "/bench/bin/suite"  -> rust JSON
         jq merges into raw.json
    -> write /results/raw.json
    |
    v
container exits; results available at $RESULTS_DIR/raw.json (host volume mount)
    |
    v
python3 generate.py --input raw.json --output $RESULTS_DIR
    -> read benchmarks[] array
    -> for each suite: pygal.Bar chart -> charts/suite.svg
    -> pygal.Bar chart -> charts/writ_compile_vs_run.svg
    -> markdown table -> RESULTS.md
    |
    v
$RESULTS_DIR/
    raw.json
    RESULTS.md
    charts/
        fib.svg
        string_processing.svg
        data_structures.svg
        dispatch.svg
        writ_compile_vs_run.svg
```

### Integration with Existing writ-cli

The `writ` binary is invoked as two separate commands per suite:

1. `writ compile <input.writ> -o /tmp/<suite>.writc` — writes a `.writc` binary to `/tmp`
2. `writ run /tmp/<suite>.writc` — runs the compiled binary via the VM

This matches the existing `cmd_compile` and `cmd_run` subcommands exactly. No changes to `writ-cli` are needed. The benchmark treats `writ` as a black-box CLI tool — the same way a user would invoke it.

The Dockerfile's build stage uses `cargo build --release --bin writ` which produces `target/release/writ`. This is copied to `/usr/local/bin/writ` in the final image.

**Important:** The Dockerfile context is the repository root (`docker build . -f benchmark/runner/Dockerfile`) so the full Cargo workspace is available to the `writ-builder` stage.

---

## Architectural Patterns

### Pattern 1: Two-Stage Writ Measurement

**What:** Run `writ compile` and `writ run` as separate hyperfine invocations. Record both times in the JSON. Display `writ_run` in the language comparison chart and `writ_compile` in a separate compile-time chart.

**When to use:** Always. Mixing compile and run time would make Writ appear slower than it is at runtime and would hide the compile cost, which is a meaningful metric for a scripting language.

**Trade-offs:** Requires the `.writc` file to persist between the two measurements. Use `/tmp/` — it is always writable inside a Docker container. The first hyperfine call produces the `.writc`; the second reads it.

### Pattern 2: Pre-Compiled Rust Benchmark Binaries

**What:** Compile Rust benchmark sources in a Docker builder stage. The final image contains only the compiled binaries, not `rustc`. The benchmark runner invokes `/bench/bin/<suite>` directly.

**When to use:** Always for Rust. Running `rustc` inside the benchmark container would take 30+ seconds per case and skew all time comparisons.

**Trade-offs:** Rust binaries are specific to the container's architecture (linux/amd64). CI runs on ubuntu-24.04 which is linux/amd64 — this is fine. If arm64 CI runners are used, the builder stage must use `rust:1.85-slim` for the matching architecture.

### Pattern 3: Host-Side Chart Generation

**What:** The benchmark container only produces `raw.json`. Chart generation (`generate.py`) runs on the host after the container exits, reading the mounted `raw.json` file.

**When to use:** Always. This separates the benchmark measurement environment (Linux container with fixed runtimes) from the visualization tooling (Python + pygal on any OS). It also means charts can be regenerated from any historical `raw.json` without re-running benchmarks.

**Trade-offs:** The host must have Python 3 and pygal installed. In CI, this is handled by `setup-python` and `pip install pygal` steps before the chart generation step.

### Pattern 4: JSON as Canonical Results Format

**What:** `raw.json` is the single source of truth. `RESULTS.md` and SVG charts are derived outputs that can be regenerated at any time. Only `raw.json` needs to be preserved for historical analysis.

**When to use:** Always. Storing derived SVG files in the repo as well allows README embedding and GitHub rendering, but they should be understood as derived artifacts.

**Trade-offs:** SVG files are ~10–20 KB each. Storing them in git is reasonable for a benchmark repo. If the repo size becomes a concern, switch to git-lfs for the SVG files.

---

## Anti-Patterns

### Anti-Pattern 1: Running Rust Compilation Inside the Benchmark Container

**What people do:** Include `rustc` in the final Docker image and compile Rust benchmark sources at benchmark time, timing the compile-and-run together.

**Why it's wrong:** Rust compilation takes 10–60 seconds for even trivial programs. This would make Rust appear 100-1000x slower than every other language and would measure the compiler, not the runtime.

**Do this instead:** Pre-compile Rust benchmarks in a multi-stage Docker build. The benchmark runner invokes the pre-built binary directly. If Rust compile time is interesting to measure, time it separately and report it in the same way Writ compile time is reported.

### Anti-Pattern 2: Measuring Startup + Runtime Together Without Distinction

**What people do:** Run all languages through the same single hyperfine invocation measuring total wall time, conflating JVM/Node.js startup overhead with actual computation.

**Why it's wrong:** Node.js has a ~90ms startup overhead that dominates measurements for fast benchmarks. Squirrel and Lua have <5ms startup. Conflating startup and compute gives a misleading picture for short benchmarks.

**Do this instead:** For the benchmark cases, choose workloads heavy enough that startup overhead is <10% of total time (e.g., Fibonacci(40), not Fibonacci(10)). Document the benchmark parameters so readers understand what is being measured. If startup time is itself a metric of interest, measure it explicitly with a trivial program (print "hello") as a separate suite.

### Anti-Pattern 3: Pinning Specific hyperfine Version in apt

**What people do:** Use `apt-get install hyperfine` assuming it is packaged in Ubuntu 24.04.

**Why it's wrong:** hyperfine is not in Ubuntu's default apt repositories. It must be installed from GitHub Releases as a `.deb`. Installing via apt will fail with "package not found."

**Do this instead:** Download the `.deb` from the official hyperfine GitHub releases page. Pin the version in the Dockerfile so the benchmark is reproducible. The Dockerfile shown above downloads `hyperfine_1.18.0_amd64.deb` from GitHub Releases.

### Anti-Pattern 4: Committing Raw JSON and SVGs in a Flat results/ Directory

**What people do:** Write all benchmark output to `benchmark/results/raw.json` and overwrite it on each run.

**Why it's wrong:** Overwriting the file on each run destroys historical data. `git diff` will show the entire JSON file changed every time, making it hard to see trends.

**Do this instead:** Use dated subdirectories (`benchmark/results/YYYY-MM-DD/`). Each run produces a self-contained directory. Historical comparison is trivial: `diff benchmark/results/2026-03-01/raw.json benchmark/results/2026-03-20/raw.json`.

### Anti-Pattern 5: Using PowerShell for the Inside-Container Script

**What people do:** Write a single unified script that works on both Windows and Linux.

**Why it's wrong:** The benchmark container is Linux-only (ubuntu:24.04). PowerShell inside Linux containers requires installing `powershell` which adds ~300 MB to the image and is completely unnecessary.

**Do this instead:** `bench_runner.sh` is pure bash and runs inside the container. `run.ps1` is the Windows host-side launcher only — it calls `docker run` which invokes `bench_runner.sh` inside Linux. The separation is clean: PowerShell on the host, bash inside the container.

---

## Integration Points with Existing Writ Toolchain

### New Components

| Component | Location | Status |
|-----------|----------|--------|
| `benchmark/cases/` | New top-level directory | New |
| `benchmark/runner/Dockerfile` | Docker multi-stage build | New |
| `benchmark/runner/bench_runner.sh` | In-container orchestrator | New |
| `benchmark/runner/run.sh` | Host entry (Linux/macOS) | New |
| `benchmark/runner/run.ps1` | Host entry (Windows) | New |
| `benchmark/chart_gen/generate.py` | Chart generation | New |
| `benchmark/chart_gen/requirements.txt` | `pygal` | New |
| `benchmark/results/` | Output directory | New |
| `.github/workflows/benchmark.yml` | CI workflow | New |

### Existing Components — No Changes Required

| Component | How Benchmark Uses It | Changes Needed |
|-----------|----------------------|----------------|
| `writ-cli` | `writ compile` + `writ run` invoked as black-box CLI | None |
| `Cargo.toml` | `cargo build --release --bin writ` in Dockerfile builder stage | None |
| `scripts/rebuild-writ.sh` | Not used by benchmark; Dockerfile handles its own build | None |

### Build Order for Implementation

```
Phase 1: Benchmark case source files
  Deliverable: cases/fib/, cases/string_processing/, cases/data_structures/, cases/dispatch/
  Each with .writ .lua .nut .py .js .rs source files
  Dependency: None (pure source files; writ syntax must be valid)
  Verify: writ compile cases/fib/fib.writ succeeds on local build

Phase 2: Dockerfile + bench_runner.sh
  Deliverable: runner/Dockerfile builds successfully; bench_runner.sh runs inside container
  Dependency: Phase 1 cases; writ-cli builds cleanly (already true)
  Verify: docker build -t writ-benchmark . && docker run --rm writ-benchmark

Phase 3: Host runner scripts (run.sh + run.ps1)
  Deliverable: One command produces raw.json in benchmark/results/YYYY-MM-DD/
  Dependency: Phase 2 Docker image
  Verify: ./benchmark/runner/run.sh produces benchmark/results/2026-03-20/raw.json

Phase 4: Chart generation (generate.py)
  Deliverable: raw.json -> SVG charts + RESULTS.md
  Dependency: Phase 3 raw.json; pygal installed
  Verify: python benchmark/chart_gen/generate.py --input raw.json --output /tmp/test

Phase 5: GitHub Actions workflow
  Deliverable: .github/workflows/benchmark.yml; manual trigger produces committed results
  Dependency: Phases 1-4 all working
  Verify: workflow_dispatch run succeeds; results committed to repo
```

---

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 4 suites, 6 languages (current) | Current design is sufficient; single container, sequential runs |
| 10+ suites | Consider parallelizing hyperfine runs inside the container using `&` and `wait`; group by suite |
| Adding more languages | Add apt-get line in Dockerfile and a new hyperfine invocation in bench_runner.sh; add to LANGUAGES list in generate.py |
| Per-commit benchmarks | Change schedule trigger to `on: push: branches: [master]`; add regression detection by comparing new `mean` to previous run's `mean` |

---

## Sources

- Codebase inspection: `writ-cli/src/main.rs`, `writ-cli/src/commands/compile.rs`, `writ-cli/src/commands/run.rs` (direct read)
- hyperfine JSON schema: [sharkdp/hyperfine README](https://github.com/sharkdp/hyperfine) + [marcpaterno/hyperfiner](https://rdrr.io/github/marcpaterno/hyperfiner/man/read_hyperfine_json.html) (HIGH confidence — schema verified against multiple sources)
- Squirrel interpreter binary name and `.nut` extension: [Ubuntu Manpage: squirrel](https://manpages.ubuntu.com/manpages/jammy/man1/squirrel.1.html) (HIGH confidence — official Ubuntu docs)
- Squirrel apt package: `squirrel3` — [Ubuntu Packages: squirrel3](https://packages.ubuntu.com/search?keywords=squirrel3) (HIGH confidence)
- Lua 5.4 apt package: `lua5.4`, binary `lua5.4` — [Ubuntu launchpad](https://launchpad.net/ubuntu/+source/lua5.4) (HIGH confidence)
- hyperfine not in Ubuntu apt; install from GitHub Releases: verified by checking Ubuntu 24.04 package index (MEDIUM confidence — not directly verified; no apt package found in search results, consistent with known distribution)
- pygal for headless SVG: [pygal PyPI](https://pypi.org/project/pygal/) — no GUI dependency, pure Python (HIGH confidence)
- Docker multi-stage builds: [Docker Docs: Multi-stage builds](https://docs.docker.com/build/building/multi-stage/) (HIGH confidence)
- GitHub Actions permissions for commit: [GitHub Actions docs](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions#permissions) (HIGH confidence)
- Docker/Podman detection pattern (command -v docker || command -v podman): standard shell practice (HIGH confidence)
- kostya/benchmarks reference suite architecture: [github.com/kostya/benchmarks](https://github.com/kostya/benchmarks) (MEDIUM confidence — inspected via WebFetch)

---
*Architecture research for: Writ v7.0 Cross-Language Benchmark Suite*
*Researched: 2026-03-20*
