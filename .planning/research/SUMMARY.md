# Project Research Summary

**Project:** Writ v7.0 — Cross-Language Benchmark Suite
**Domain:** Docker-containerized benchmark harness comparing Writ against Lua, Squirrel, Python, Node.js, and native Rust
**Researched:** 2026-03-20
**Confidence:** HIGH

## Executive Summary

This milestone adds a cross-language benchmark suite to an already-mature 9-crate Rust workspace (v6.1, 74,997 LOC). The existing `writ-cli`, `writ-compiler`, and `writ-runtime` crates are subjects under test — nothing in the core toolchain changes. The new infrastructure consists of benchmark source programs in six languages, a Docker-based execution harness, a Python chart generator, and a GitHub Actions CI workflow. The recommended approach follows established prior art (kostya/benchmarks, drujensen/fib, Are-We-Fast-Yet): Docker for reproducibility, hyperfine for timing, JSON as the canonical results format, and SVG charts generated on the host after the container exits. The architecture is a clean one-way pipeline: source files enter the Docker container, `raw.json` exits via a volume mount, and `generate.py` on the host produces SVG and markdown.

The most critical design constraint is measurement fairness. Writ has an explicit AOT compile step that other interpreted languages do not; this must be reported as a separate `compile_ms` metric so Writ `execution_ms` is directly comparable to Lua/Python/Node execution time. Node.js requires warmup before TurboFan JIT activates; without it, Node.js appears 5-10x slower than steady-state. Squirrel is not in Ubuntu apt repositories and must be built from source in Docker — its inclusion carries high implementation risk and must be validated early. Statistical rigor (median over mean, minimum 30 iterations, inter-quartile range reported alongside median) is non-negotiable for publishable results that can withstand external scrutiny.

The implementation risk is concentrated in three areas: Squirrel build reliability in CI, per-language measurement methodology correctness (warmup, startup separation, anonymous RSS for memory), and CI runner variance making committed numbers misleading. All three are avoidable by front-loading the Docker environment and harness design phases before writing any benchmark programs. A note on a specific gap: STACK.md and ARCHITECTURE.md give conflicting information on whether `squirrel3` is in Ubuntu 24.04 apt — STACK.md says it must be built from source, ARCHITECTURE.md lists `apt-get install squirrel3`. This must be validated with a single `docker run` test before Phase 2 planning.

---

## Key Findings

### Recommended Stack

The benchmark suite requires no changes to existing workspace crates. New infrastructure lives in `benchmark/` (top-level directory). The harness is implemented as a shell script (`bench_runner.sh`) inside the Docker container using `hyperfine` for timing and `jq` for JSON assembly, with chart generation handled by a Python script (`generate.py`) using `pygal` on the host. Criterion is explicitly ruled out: it cannot time external processes and is an in-process Rust-only tool. A `writ-bench` Rust crate is optional; the shell-based approach is fully sufficient and avoids adding a new crate.

**Core technologies:**
- `hyperfine` 1.18.0 — subprocess timing with JSON export; installed from GitHub Releases `.deb` (NOT in Ubuntu apt)
- `ubuntu:24.04` multi-stage Docker — reproducible runtime environment; matches GitHub Actions `ubuntu-latest` since January 2025
- `pygal` (Python, pip) — headless SVG bar chart generation; no system GUI dependencies; pure Python
- `jq` (apt) — JSON assembly inside the shell runner script
- `squirrel3` — must be built from source via CMake OR may be in Ubuntu 24.04 apt (validate before Phase 2)
- Rust 1.85+ stable — needed in Docker builder stage only; final image does not need the Rust toolchain
- `actions/checkout@v4` + `actions/upload-artifact@v4` — standard CI artifact pipeline
- `plotters` 0.3.7 + `plotters-svg` 0.3.7 — alternative to pygal if chart generation moves into a Rust binary

**What NOT to add:** `criterion`, `mlua`, `docker-compose`, Python `matplotlib`, `benchmark-action` on every PR, `squirrel-rs` FFI bindings.

### Expected Features

**Must have (P1 — table stakes):**
- Docker image with all 6 language runtimes at pinned versions — without this, nothing is reproducible
- Compute benchmarks: fib(40) recursive + prime sieve (N=1,000,000) in all 6 languages — minimum viable comparison
- Runner script producing structured JSON output — hub for all downstream reporting
- Writ compile time reported separately from execution time (`compile_ms` distinct from `execution_ms`) — fairness requirement unique to Writ
- Markdown table reporter (language, median_ms, memory_mb, relative-to-Rust ratio) — primary publishable artifact
- SVG bar chart for compute category on log scale — visual proof for README; log scale required because Rust/interpreter gap spans 2-3 orders of magnitude
- Methodology README disclosing hardware, language versions, run count, what is and is not measured
- Statistical rigor: median + IQR (or MAD), minimum 30 iterations, outlier treatment

**Should have (P2 — add after compute pipeline validated):**
- String processing category (concat loop, word count)
- Data structures category (linked list 100K nodes, hash map 1M insert/lookup)
- OOP/dispatch category (virtual method chain, closure/callback chain) — hardest to keep fair; add last
- Memory measurement via anonymous RSS (not total RSS) — important for game context; requires correct cgroup v2 handling
- GitHub Actions CI workflow with `workflow_dispatch` + weekly schedule
- Matrix multiply benchmark (500x500)

**Defer (v2+):**
- CI baseline comparison with regression alerts — only useful after historical baseline exists
- Per-run archival and trend tracking dashboards
- Squirrel OOP variants — Squirrel OOP model complexity is high
- Live web dashboard — maintenance burden, out of scope

### Architecture Approach

The system follows a clean separation: the Docker container handles all benchmark execution and produces `raw.json` via a volume-mounted results directory; chart generation and markdown table production run on the host after the container exits. This makes chart regeneration independent of benchmark re-runs and keeps the container image small (no Python charting dependencies inside). The multi-stage Docker build is critical: Rust benchmark binaries are pre-compiled in a builder stage so the runtime measurement excludes compilation overhead. Writ is measured in two separate hyperfine passes: `writ compile` and `writ run`.

**Major components:**
1. `benchmark/cases/<suite>/` — six source files per benchmark suite (fib.writ, fib.lua, fib.nut, fib.py, fib.js, fib.rs); flat layout for direct visual comparison
2. `benchmark/runner/Dockerfile` — multi-stage: writ-builder stage (Rust), rust-bench-builder stage (pre-compiled Rust benchmarks), final ubuntu:24.04 runtime image with hyperfine + language runtimes
3. `benchmark/runner/bench_runner.sh` — runs inside container; calls hyperfine per language per suite; assembles raw.json via jq; handles Writ two-step compile+run measurement
4. `benchmark/runner/run.sh` + `run.ps1` — host-side launchers; detect docker/podman; mount results volume; call generate.py after container exits
5. `benchmark/chart_gen/generate.py` — reads raw.json; produces per-suite SVG charts (log scale, Y=0 baseline) + RESULTS.md markdown table
6. `benchmark/results/YYYY-MM-DD/` — dated output directories; raw.json + charts/ + RESULTS.md; git-tracked
7. `.github/workflows/benchmark.yml` — `workflow_dispatch` + weekly schedule; artifact upload; optional manual result commit

**Key architectural patterns:**
- Multi-stage Docker: Rust binaries pre-compiled in builder stage so runtime measurement is execution-only
- Two-step Writ measurement: `writ compile` timed separately; only `writ_run_ms` in execution comparison chart
- JSON as canonical format: SVG and markdown are derived outputs regenerable from any historical raw.json
- Host-side chart generation: separates measurement environment from visualization tooling
- Dated subdirectories for results: prevents overwrite, enables historical diff

### Critical Pitfalls

1. **Non-equivalent workloads across languages** — define a paradigm-neutral canonical algorithm spec (exact data structure, exact operation count, expected output checksum) before writing any language implementations. OOP benchmarks must allocate the same number of objects in each language; verify with allocation counters. This is the highest-cost pitfall to fix after the fact.

2. **Writ compile time mixed into execution time** — always run `writ compile` as a pre-measurement step and report its time as `compile_ms`; the execution comparison chart must show only `writ_run_ms`. Must be a first-class constraint in the harness design, not an afterthought.

3. **Node.js JIT warmup missing** — run at minimum 200-500 warmup iterations before the timed measurement window for Node.js; never use `--jitless`; verify TurboFan is active for the timed portion. Without warmup, Node.js appears 5-10x slower than realistic steady-state.

4. **Squirrel not buildable in Docker CI** — validate the Squirrel CMake build (or apt availability) in Docker before committing to it as a benchmark target. If not reproducible, drop from CI and document as local-only rather than producing silent zero results.

5. **Memory metric includes OS page cache** — read anonymous RSS (`VmRSS - RssFile - RssShmem`) not total RSS; on cgroup v2 use `memory.stat` → `anon`; use a shared shell function for all languages so measurement method is identical across the comparison.

6. **GitHub Actions timing variance (10-30%)** — do not commit CI runner numbers as authoritative results; use CI only for regression detection with a minimum 15% threshold; generate publishable numbers locally in Docker on a stable machine.

7. **Chart Y-axis truncation** — always set Y-axis minimum to 0 for bar charts; use log scale for charts including Rust native (label it explicitly); generate a second linear-scale chart showing only interpreted/VM languages so the interpreted-language comparison is legible.

---

## Implications for Roadmap

The dependency graph dictates a strict phase ordering: Docker environment and measurement methodology must be locked before benchmark programs are written; JSON pipeline must be proven before charts are generated; charts must be validated before CI is wired up. FEATURES.md identifies a clear MVP (compute category only) that proves the pipeline end-to-end before expanding to harder categories. The pitfalls research reinforces this conservative ordering — every major pitfall has its root cause in a later phase assuming earlier phases were correct.

### Phase 1: Algorithm Specifications and Benchmark Design
**Rationale:** Research (Are-We-Fast-Yet methodology) shows that algorithm specification must precede any language implementation. Locking the canonical algorithm spec — exact parameters, expected output checksum, operation counts — prevents the most expensive pitfall: discovering non-equivalent implementations after all six language versions are written.
**Delivers:** Written spec document for each benchmark category (compute, string, data structures, OOP/dispatch) covering exact algorithm, parameters, expected output, object allocation count (for OOP parity). No code written yet. Writ compile/run separation policy documented here.
**Addresses:** Compute-heavy MVP (fib(40), prime sieve); algorithm specs for remaining categories
**Avoids:** Pitfall 1 (non-equivalent workloads — prevented by locking spec before implementation); Pitfall 2 (compile/run separation is specified here, not discovered later)
**Research flag:** Standard — algorithm selection and parameterization is exhaustively documented in prior art (AWFY, kostya/benchmarks, drujensen/fib). No additional research phase needed.

### Phase 2: Docker Environment and Measurement Harness
**Rationale:** Docker environment and measurement methodology are the foundation everything else depends on. Squirrel build risk must be confronted here, not discovered after benchmark programs exist. The measurement protocol (warmup, startup separation, anonymous RSS, iteration count) must be locked before any benchmark programs are written against it, because the protocol determines how each language's benchmark script must be structured.
**Delivers:** Working multi-stage Dockerfile with all 6 language runtimes at pinned versions; `bench_runner.sh` with correct per-language measurement protocol (Node.js warmup, Writ two-step, Python self-timing); validated Squirrel build or explicit fallback; version-check assertions at container startup; raw.json schema defined and validated with a stub benchmark
**Uses:** `ubuntu:24.04`, `hyperfine` 1.18.0 (from GitHub Releases `.deb`), `jq`, multi-stage Docker build, Squirrel 3.2
**Implements:** Docker container component, bench_runner.sh component
**Avoids:** Pitfall 3 (LuaJIT/PUC Lua conflation — version assertions in container startup); Pitfall 4 (Squirrel build); Pitfall 5 (memory metric — anonymous RSS function shared across all languages); Pitfall 6 (Python startup separation); Pitfall 2 (Node.js warmup protocol)
**Research flag:** Squirrel availability needs early validation — run `docker run ubuntu:24.04 apt-cache show squirrel3` before planning Phase 2 in detail. STACK.md and ARCHITECTURE.md conflict on this point.

### Phase 3: Benchmark Programs — Compute Category (MVP)
**Rationale:** Implement only the compute category (fib + prime sieve) across all six languages first. This validates the end-to-end pipeline with the simplest, most well-understood algorithms before tackling harder categories. Output checksum verification confirms algorithmic equivalence before any timing begins.
**Delivers:** `benchmark/cases/fib/` and `benchmark/cases/sieve/` with six source files each; output checksums verified across all six languages; `writ compile` + `writ run` both succeed; raw.json produced with correct `compile_ms` and `execution_ms` separation; statistical rigor validated (N>=30, median + IQR)
**Addresses:** P1 compute benchmarks; Writ compile-time separation; statistical rigor
**Avoids:** Pitfall 1 (output checksums verified before timing begins); Pitfall 10 (insufficient iterations — adaptive harness ensures N>=30)
**Research flag:** Standard — fib(40) and prime sieve are the most benchmarked algorithms in existence. No research needed.

### Phase 4: Chart Generation and Results Pipeline
**Rationale:** With compute benchmarks producing raw.json, build the full reporting pipeline before expanding to more benchmark categories. Locking chart configuration (Y-axis-zero policy, log scale, units) in version-controlled code prevents the chart Y-axis truncation pitfall and ensures all future categories automatically get correct charts without per-category manual configuration.
**Delivers:** `generate.py` producing per-suite SVG charts (log scale, Y=0 baseline enforced, units labeled) + linear-scale interpreted-languages-only chart + RESULTS.md markdown table; `run.sh` + `run.ps1` host launchers; validated end-to-end: one command from repo root produces committed results in `benchmark/results/YYYY-MM-DD/`
**Uses:** `pygal` (headless SVG, pip), Python 3.12, dated subdirectory results structure
**Implements:** chart_gen component, host runner scripts, results/ directory structure
**Avoids:** Pitfall 9 (Y-axis truncation — enforced in code with automated SVG assertion); anti-pattern of flat results directory (dated subdirectories used)
**Research flag:** Standard — pygal SVG generation is well-documented. No research phase needed.

### Phase 5: Benchmark Programs — Remaining Categories
**Rationale:** Expand from compute-only MVP to full four-category suite after the pipeline is proven. String processing is added first (medium complexity), then data structures (moderate — Squirrel requires hand-rolled structures), then OOP/dispatch last (highest fairness risk). Each category is gated by the same parity verification used in Phase 3 before any timing.
**Delivers:** `string_processing/`, `data_structures/`, `dispatch/` cases in all six languages; expanded raw.json with all four suites; updated charts and RESULTS.md; memory measurement added (anonymous RSS via shared shell function)
**Addresses:** P2 features: string processing, data structures, OOP/dispatch, memory measurement, matrix multiply
**Avoids:** Pitfall 1 (parity checklist from "Looks Done But Isn't" applied per category); Pitfall 5 (memory measurement with correct cgroup v2 anonymous RSS)
**Research flag:** OOP/dispatch category needs a targeted research spike before implementation — Squirrel OOP (metatables), Lua OOP (metatables), Python (native classes), Writ (struct + entity model), Rust (traits), Node.js (class syntax) all have different dispatch overhead profiles. Need to confirm canonical equivalence before writing code.

### Phase 6: GitHub Actions CI Workflow
**Rationale:** CI is added last, after the pipeline is locally validated and results are trusted. Adding CI before results are stable creates noise and burns CI minutes unnecessarily. The variance policy (15% regression threshold, no auto-commit of authoritative numbers from shared runners) must be encoded in the workflow design.
**Delivers:** `.github/workflows/benchmark.yml` with `workflow_dispatch` + weekly `schedule` trigger; artifact upload of raw.json + SVG charts; conditional result commit only on manual dispatch; CI variance documented in results README; regression threshold set to 15%
**Uses:** `actions/checkout@v4`, `actions/upload-artifact@v4`, `actions/setup-python@v5`, Docker (pre-installed on ubuntu-24.04 runners)
**Avoids:** Pitfall 7 (CI runner variance — 15% threshold, no auto-commit of authoritative numbers); anti-pattern of running benchmarks on every push
**Research flag:** Standard — GitHub Actions patterns are well-documented. No research phase needed.

### Phase Ordering Rationale

- Algorithm spec precedes code (Phase 1 before Phase 3) because fixing non-equivalent implementations after all six language versions are written is the highest-cost recovery in the pitfalls list.
- Docker and harness precede benchmark programs (Phase 2 before Phase 3) because the measurement protocol determines how benchmark programs must be structured (self-timing approach, warmup protocol, output format).
- MVP compute category precedes expanded categories (Phase 3 before Phase 5) because it validates the end-to-end pipeline with the least implementation risk before tackling Squirrel OOP or hash map fairness across six languages.
- Chart pipeline comes before expanded categories (Phase 4 before Phase 5) because chart configuration must be version-controlled code — not a one-off session — to guarantee consistent presentation for all future categories.
- CI comes last (Phase 6) because it is only useful after local results are trusted and the pipeline is stable. Adding CI earlier creates noise before the signal exists.

### Research Flags

Phases needing deeper investigation before or during planning:

- **Phase 2 (pre-planning):** Squirrel `squirrel3` apt availability in Ubuntu 24.04. Run `docker run ubuntu:24.04 apt-cache show squirrel3` immediately. If available: Phase 2 Dockerfile is simpler. If not: plan for a 3-5 minute CMake build layer and verify it on arm64 if needed.
- **Phase 5 (OOP/dispatch):** Canonical OOP benchmark implementation across six languages. Squirrel metatables, Lua metatables, Python native classes, Writ struct model, and Rust traits all have meaningfully different dispatch overhead profiles. Needs a targeted research spike to confirm canonical equivalence before any OOP benchmark code is written.

Phases with standard patterns (skip research-phase):
- **Phase 1:** Algorithm selection and parameterization is exhaustively documented in AWFY, kostya, drujensen/fib.
- **Phase 3:** fib(40) and prime sieve are the most benchmarked algorithms in existence.
- **Phase 4:** pygal SVG generation and markdown table generation are straightforward.
- **Phase 6:** GitHub Actions patterns are well-documented official documentation.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All core technologies verified against official docs and repos. One gap: Squirrel apt availability conflicts between research files — needs a one-line validation before Phase 2. Node.js LTS version has a minor conflict (STACK.md says 20 LTS, FEATURES.md says 22 LTS; use 22 to avoid immediate EOL). |
| Features | HIGH | Prior art is abundant and consistent across five reference suites. MVP definition (compute category first) is well-supported. Feature priority matrix is clear with explicit P1/P2/P3 assignments. |
| Architecture | HIGH | Component boundaries and data flow are fully specified with working code examples in all four research files. Multi-stage Docker pattern, two-step Writ measurement, and host-side chart generation are all validated against official docs and reference suites. |
| Pitfalls | HIGH | All 10 pitfalls are grounded in peer-reviewed literature (2025-2026), official V8/Docker/cgroup documentation, and direct measurement studies. Prevention strategies are specific, actionable, and include concrete "warning signs" for early detection. |

**Overall confidence:** HIGH

### Gaps to Address

- **Squirrel apt availability conflict:** STACK.md states Squirrel must be built from source; ARCHITECTURE.md lists `apt-get install squirrel3`. Validate with `docker run ubuntu:24.04 apt-cache show squirrel3` before Phase 2 planning. The answer determines Dockerfile complexity.
- **Node.js LTS version:** STACK.md recommends Node.js 20 LTS (EOL April 2026); FEATURES.md mentions Node.js 22 LTS. Use Node.js 22 LTS to avoid an EOL migration in the near term.
- **Writ benchmark program syntax:** Benchmark authors writing `.writ` files must be familiar with the current Writ language feature set (no generics, explicit `self`, `new Type {}` construction syntax, entity-component model). Validate each `.writ` file with `writ compile` on the current build before the Docker image is finalized.
- **Memory measurement on non-Linux hosts:** The anonymous RSS approach is Linux-only. Local development workflows on macOS/Windows will report `0` for memory. Document this clearly so developers do not file bugs about missing memory values in local non-Docker runs.
- **OOP/dispatch canonical algorithm:** The exact dispatch pattern to use across Squirrel metatables, Lua metatables, Python classes, and Writ structs is not resolved in the current research. This must be resolved (via a research spike in Phase 5) before any OOP benchmark code is written.

---

## Sources

### Primary (HIGH confidence)
- `github.com/kostya/benchmarks` — methodology, Docker approach, median+MAD reporting, memory measurement, idiomatic implementation requirement
- `github.com/smarr/are-we-fast-yet` + ACM DLS paper — paradigm-neutral algorithm design, cross-language fairness methodology (peer-reviewed)
- `github.com/drujensen/fib` — compile-time vs runtime separation pattern, 5-run average, Docker approach; widely cited
- `github.com/bdrung/startup-time` — startup time measurement, 1000-run averaging methodology
- `v8.dev/blog/maglev` — V8 JIT tier pipeline (Ignition → Sparkplug → Maglev → TurboFan), official V8 blog
- `docs.docker.com/engine/containers/runmetrics/` — cgroup v1/v2 memory accounting, anonymous RSS vs total RSS
- `runs-on.com/benchmarks/github-actions-cpu-performance/` — direct measurement of 20%+ CPU variance on GitHub Actions shared runners
- `arxiv:2501.12878` — statistical methods for reliable benchmarks, median over mean (IEEE/ACM 2025, peer-reviewed)
- `arxiv:2511.03533` — process isolation in benchmarking, Docker measurement variance (IEEE/ACM 2025, peer-reviewed)
- `github.com/sharkdp/hyperfine` — JSON export schema, v1.18.0 installation, timing methodology
- `github.com/plotters-rs/plotters` — 0.3.7 SVGBackend API, September 2024 release confirmed
- `packages.ubuntu.com/lua5.4` — Lua 5.4 availability in Ubuntu 24.04 confirmed
- `github.com/albertodemichelis/squirrel` — official Squirrel repo, v3.2 tag, CMake build required, last commit February 2026
- `github.com/actions/runner-images/issues/10636` — ubuntu-latest = ubuntu-24.04 confirmed January 2025

### Secondary (MEDIUM confidence)
- `crates.io/crates/procfs` 0.17 — VmRSS and peak RSS in Status struct (alternative to shell-based memory measurement)
- `github.com/benchmark-action/github-action-benchmark` — CI regression detection patterns
- `pypi.org/project/pygal` — headless SVG generation, no system GUI dependency confirmed
- `codspeed.io/blog/benchmarks-in-ci-without-noise` — 15% regression threshold for sub-1% false positive rate on shared runners
- `nodesource.com/blog/State-of-Nodejs-Performance-2024` — Node.js v22 Maglev tier behavior
- `github.com/RafaelGSS/bench-node` — official Node.js benchmarking library with warmup handling

### Tertiary (LOW confidence — needs validation)
- Squirrel `squirrel3` package in Ubuntu 24.04 apt — conflicting signals between research files; validate with `docker run` before Phase 2 planning begins

---
*Research completed: 2026-03-20*
*Ready for roadmap: yes*
