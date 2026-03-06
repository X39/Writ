# Phase 70: Docker Environment and Measurement Harness - Context

**Gathered:** 2026-03-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Multi-stage Docker image with all 6 language runtimes (Writ, Lua 5.4, Squirrel 3.x, Python 3.x, Node.js 22 LTS, pre-compiled Rust), a hyperfine-based timing harness, host-side runner scripts (sh + PowerShell), and a stub benchmark producing `raw.json` proving the measurement pipeline works end-to-end. Benchmark programs themselves (fib, sieve, etc.) are Phase 71+. Chart generation is Phase 72.

</domain>

<decisions>
## Implementation Decisions

### Docker Image Architecture
- Multi-stage Dockerfile: Stage 1 builds `writ` binary from Rust source, Stage 2 builds Rust benchmark stubs, Stage 3 is the runtime image (ubuntu:24.04)
- Final image contains only runtime interpreters + hyperfine + writ binary — no Rust compiler in the benchmark image
- Base image: `ubuntu:24.04` (matches GitHub Actions ubuntu-latest and provides apt packages for Lua, Python, Node.js)
- Docker context is the repo root (`docker build -t writ-bench -f benchmark/runner/Dockerfile .`) so the full Cargo workspace is available to the builder stage

### Language Runtime Installation
- **Lua 5.4**: `apt-get install lua5.4` — binary is `lua5.4`
- **Squirrel 3.2**: Build from source (clone `albertodemichelis/squirrel` at pinned git tag `v3.2`, cmake build, copy `sq` binary). Squirrel is NOT reliably available via apt on Ubuntu 24.04 — must validate with `sq --version` assertion at container startup
- **Python 3.12**: `apt-get install python3` — default Python in Ubuntu 24.04
- **Node.js 22 LTS**: Install via NodeSource PPA (`setup_22.x`). Do NOT use Ubuntu's `nodejs` apt package (severely outdated)
- **Rust benchmarks**: Pre-compiled with `rustc -O` in a builder stage. Final image contains only the binary — no rustc
- **Writ**: `cargo build --release --bin writ` in builder stage, copy to `/usr/local/bin/writ`

### Measurement Tool
- Use `hyperfine` CLI for all timing measurements — install from GitHub Releases `.deb` (not in apt repos)
- hyperfine exports JSON natively with `--export-json`, providing mean, median, stddev, min, max, and all individual times
- Use `jq` inside the container to merge per-language JSON results into a single `raw.json`
- Writ is measured in two separate hyperfine invocations: `writ compile` and `writ run` (compile/run split)

### Memory Measurement
- Anonymous RSS only: `VmRSS - RssFile - RssShmem` from `/proc/<pid>/status`
- Shared shell function `measure_anon_rss()` used by all language measurements for consistency
- Reports `0` on non-Linux (documented) — Docker ensures Linux for authoritative runs

### Startup Time
- Measured as wall-clock time for a no-op program (each language prints "hello") via hyperfine
- Reported as a distinct JSON field, separate from execution time

### Runner Scripts
- `benchmark/runner/run.sh` (POSIX sh): builds Docker image, runs container with volume mount for results, detects Docker or Podman via `command -v`
- `benchmark/runner/run.ps1` (PowerShell): equivalent logic, handles Windows path normalization for Docker volume mounts
- Both scripts produce output in `benchmark/results/YYYY-MM-DD/` dated subdirectories
- `RUNS` configurable via env var / parameter (default: 10)

### Stub Benchmark
- Minimal hello-world per language (prints a known string) to prove the pipeline works end-to-end
- The stub produces a valid `raw.json` with: `compile_ms` and `run_ms` for Writ, `execution_ms` for all other languages, plus `memory_kb`, `startup_ms` fields
- Stub lives in `benchmark/cases/stub/` with one file per language

### JSON Output Schema
- Top-level: `{ "benchmarks": [...], "meta": { "date", "runs", "warmup", "platform" } }`
- Per benchmark: `{ "suite", "writ_compile": {...}, "writ_run": {...}, "lua": {...}, "squirrel": {...}, "python": {...}, "node": {...}, "rust": {...} }`
- Per language result: hyperfine's native JSON format (mean, median, stddev, min, max, times[])
- All times in seconds (hyperfine default)

### Version Pinning
- Container startup emits version strings for all 6 runtimes (success criterion #5)
- Versions: Lua 5.4.x, Squirrel 3.2 (git tag), Python 3.12.x, Node.js 22.x LTS, Rust stable (pinned via `rust-toolchain.toml`), hyperfine 1.18.0+

### Claude's Discretion
- Exact Dockerfile layer ordering and caching strategy
- Whether `bench_runner.sh` uses a case config file or hardcoded language list
- Temporary file management for `.writc` compilation artifacts
- Error handling for individual language failures (skip vs fail-fast)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project context
- `.planning/REQUIREMENTS.md` — INFRA-01 through INFRA-08 define all infrastructure requirements for this phase
- `.planning/ROADMAP.md` §Phase 70 — Success criteria (5 items) that must be TRUE
- `.planning/STATE.md` — Accumulated decisions section contains locked choices (Node.js 22, memory metric, etc.)

### Research
- `.planning/research/ARCHITECTURE.md` — Full system architecture, Dockerfile design, bench_runner.sh design, raw.json schema, data flow diagram
- `.planning/research/STACK.md` — Technology choices, version compatibility, installation methods, alternatives considered
- `.planning/research/PITFALLS.md` — 10 critical pitfalls including Squirrel build, Node.js warmup, memory measurement, CI variance

### Existing CLI (subject under test)
- `writ-cli/src/commands/compile.rs` — `writ compile` subcommand (invoked by benchmark harness)
- `writ-cli/src/commands/run.rs` — `writ run` subcommand (invoked by benchmark harness)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-cli` crate: `writ compile` and `writ run` subcommands are the black-box subject under test — no modifications needed
- `Cargo.toml` workspace: `cargo build --release --bin writ` is the build command for the Dockerfile builder stage
- Existing `serde` + `serde_json` in workspace (available if a Rust harness component is added later)

### Established Patterns
- CLI subcommands via `clap` derive API in `writ-cli/src/main.rs`
- `.writc` binary module format produced by `writ compile`, consumed by `writ run`
- No existing benchmark infrastructure — this is greenfield

### Integration Points
- `writ` binary invoked as subprocess by the benchmark harness (not linked in-process)
- `benchmark/` top-level directory is new — no conflicts with existing crate structure
- `.github/workflows/` may need to coexist with any existing CI workflows

</code_context>

<specifics>
## Specific Ideas

- Squirrel apt availability on Ubuntu 24.04 is flagged as a blocker in STATE.md — validate with `docker run ubuntu:24.04 apt-cache show squirrel3` during implementation
- ARCHITECTURE.md recommends `squirrel3` apt package, but STACK.md says it's NOT in apt — source build is the safe path
- STACK.md recommends a `writ-bench` Rust crate approach; ARCHITECTURE.md recommends hyperfine + bash. The roadmap explicitly says "hyperfine-based timing harness" — go with hyperfine + bash for Phase 70
- If a Rust harness is needed later (e.g., for memory polling via `procfs`), it can be added in Phase 71+

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 70-docker-environment-and-measurement-harness*
*Context gathered: 2026-03-20*
