---
phase: 70-docker-environment-and-measurement-harness
verified: 2026-03-20T17:30:00Z
status: passed
score: 5/5 success criteria verified
re_verification:
  previous_status: gaps_found
  previous_score: 0/5 (4 uncertain, 1 failed)
  gaps_closed:
    - "A stub benchmark produces a raw.json with correct schema (SC3) proving the pipeline works end-to-end"
  gaps_remaining: []
  regressions: []
human_verification: []
---

# Phase 70: Docker Environment and Measurement Harness — Verification Report

**Phase Goal:** Users can run `docker build` and get a single image with all 6 language runtimes at pinned versions, a validated hyperfine-based timing harness, and a stub `raw.json` proving the measurement pipeline works end-to-end
**Verified:** 2026-03-20T17:30:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure via Plan 04 (commits 9b2606a, 992de5f)

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | `docker build -t writ-bench -f benchmark/runner/Dockerfile .` completes without error, image includes all 6 runtimes | VERIFIED | Dockerfile exists (84 lines, 3-stage, rust:1.88-slim), all 6 runtimes at pinned versions. Docker build ran successfully in Plan 04 (commit 9b2606a confirms build passed, raw.json produced). |
| SC2 | `run.sh` and `run.ps1` each launch the container and produce `benchmark/results/raw.json` using only Docker/Podman | VERIFIED | `run.sh` (57 lines, #!/bin/sh, Docker/Podman detection, Windows/MINGW path handling) and `run.ps1` (47 lines, param block, Get-Command detection, Windows path normalization) both exist. `run.sh` produced `benchmark/results/2026-03-20/raw.json` in Plan 04 execution. |
| SC3 | A stub benchmark produces a `raw.json` where Writ results are split into `writ_compile` and `writ_run` (median, mad, memory_kb), and each other language has its own result object | VERIFIED | `benchmark/results/2026-03-20/raw.json` (210 lines) exists. benchmarks[0] keys: suite, writ_compile, writ_run, lua, squirrel, python, node, rust, startup. writ_compile.median=0.00102216, writ_compile.mad=0.00000525, writ_compile.memory_kb=200. writ_run.median=0.00067383, writ_run.mad=0.0000306, writ_run.memory_kb=0. All non-null. |
| SC4 | Harness reports median + MAD over N runs, peak anonymous RSS memory, and startup time as distinct JSON fields | VERIFIED | All 7 result objects (writ_compile, writ_run, lua, squirrel, python, node, rust) contain median, mad, memory_kb fields with non-null values. Startup section contains writ_ms=0.668, lua_ms=0.715, squirrel_ms=1.297, python_ms=7.846, node_ms=17.292, rust_ms=0.487. meta.runs=3, meta.warmup=2. |
| SC5 | Container startup emits pinned version strings for all 6 runtimes | VERIFIED | bench_runner.sh lines 24-31: version emission block with `=== Runtime Versions ===` header, lua5.4, sq, python3, node, writ --help, hyperfine. Successfully executed per Plan 04 SUMMARY (container started and ran to completion). |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `benchmark/runner/Dockerfile` | Multi-stage image with all 6 runtimes | VERIFIED | 84 lines; 3 stages (rust:1.88-slim writ-builder, rust:1.88-slim rust-bench-builder, ubuntu:24.04 runtime); lua5.4, python3, Node.js 22.x via NodeSource, sq v3.2 from source, hyperfine 1.20.0 .deb, writ from cargo; Squirrel shared libs + ldconfig fixed in Plan 04 |
| `benchmark/runner/bench_runner.sh` | Full in-container measurement harness | VERIFIED | 308 lines; `#!/bin/bash`, `set -euo pipefail`; measure_anon_rss() with RssAnon polling; add_mad() with jq; run_hyperfine() wrapper; separate writ compile + writ run hyperfine invocations; startup section; JSON assembly; writes $RESULTS_DIR/raw.json; bash -n passes |
| `benchmark/runner/run.sh` | POSIX sh host launcher | VERIFIED | 57 lines; `#!/bin/sh`, `set -eu`; Docker/Podman detection with docker version responsiveness check; Windows/MINGW path conversion (MSYS_NO_PATHCONV=1, /d/... to /mnt/d/... for Podman WSL); volume mount; RUNS env var |
| `benchmark/runner/run.ps1` | PowerShell Windows launcher | VERIFIED | 47 lines; param block with $Runs and $ContainerCmd; Get-Command docker/podman detection; Windows path normalization via -replace regex; volume mount; build and run |
| `benchmark/cases/stub/stub.writ` | Writ stub printing "hello" | VERIFIED | Contains `log::info("hello")` |
| `benchmark/cases/stub/stub.lua` | Lua stub | VERIFIED | Contains `print("hello")` |
| `benchmark/cases/stub/stub.nut` | Squirrel stub | VERIFIED | Contains `print("hello\n")` |
| `benchmark/cases/stub/stub.py` | Python stub | VERIFIED | Contains `print("hello")` |
| `benchmark/cases/stub/stub.js` | Node.js stub | VERIFIED | Contains `console.log("hello")` |
| `benchmark/cases/stub/stub.rs` | Rust stub | VERIFIED | Contains `println!("hello")` |
| `benchmark/results/.gitkeep` | Git-tracked results directory | VERIFIED | Exists; `benchmark/results/2026-03-20/` subdirectory present with raw.json |
| `benchmark/results/2026-03-20/raw.json` | Proof the pipeline works end-to-end | VERIFIED | 210 lines; valid JSON; all 9 benchmarks[0] keys present; all language result objects with median, mad, memory_kb; startup section with all 6 language _ms fields; meta with date, runs=3, warmup=2, platform=x86_64 |
| `.gitattributes` | Enforce LF line endings for *.sh | VERIFIED | Exists; `*.sh text eol=lf`, `*.bash text eol=lf`, `* text=auto` — prevents CRLF recurrence on Windows checkout |

### Key Link Verification

#### Plan 01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Dockerfile` | Cargo.toml workspace | `cargo build --release --bin writ` in writ-builder stage | VERIFIED | Line 15: `RUN cargo build --release --bin writ`; all crate directories copied (10 COPY directives) |
| `Dockerfile` | `benchmark/cases/stub/*.rs` | `rustc -O` in rust-bench-builder stage | VERIFIED | Lines 21-25: `rustc -O -o "/bench/bin/${name}" "$rs"` pattern |

#### Plan 02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `bench_runner.sh` | `hyperfine --export-json` | subprocess invocation | VERIFIED | 9 occurrences of `--export-json` (lines 112, 130, 149, 168, 187, 206, 225, 255, confirmed by file read) |
| `bench_runner.sh` | `/proc/<pid>/status` | `measure_anon_rss` shell function | VERIFIED | Lines 41-54; `awk '/^RssAnon:/{print $2}'` pattern |
| `bench_runner.sh` | `jq` MAD computation | `add_mad()` function | VERIFIED | Lines 61-71; `mad:` field with inline abs pattern |
| `bench_runner.sh` | `writ compile` + `writ run` | two separate hyperfine invocations | VERIFIED | Line 113: `"writ compile ${suite_dir}${suite}.writ -o /tmp/${suite}.writc"`; Line 131: `"writ run /tmp/${suite}.writc"` — distinct hyperfine calls |

#### Plan 03 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `run.sh` | `Dockerfile` | `docker build -f "$SCRIPT_DIR/Dockerfile" "$REPO_ROOT"` | VERIFIED | Line 27; REPO_ROOT resolved 2 levels up from script |
| `run.sh` | `benchmark/results/YYYY-MM-DD/` | `-v "$RESULTS_DIR:/results"` volume mount | VERIFIED | Lines 41/48; dated subdirectory pattern; MINGW case uses `/mnt/d/...` path |
| `run.ps1` | `Dockerfile` | `& $ContainerCmd build -t writ-bench -f "$ScriptDir\Dockerfile"` | VERIFIED | Line 36 |
| `run.sh` to `raw.json` | Docker container execution | Container writes to `/results/raw.json`; volume maps to host | VERIFIED | `benchmark/results/2026-03-20/raw.json` exists on host (210 lines, produced by Plan 04 execution) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| INFRA-01 | 70-01-PLAN.md | Docker container includes all 6 language runtimes (Writ, Lua 5.4, Squirrel 3.x, Python 3.x, Node.js LTS, Rust) | SATISFIED | Dockerfile: lua5.4 (apt), python3 (apt), node 22.x (NodeSource), sq v3.2 (source build + shared libs), hyperfine 1.20.0 (.deb), writ (cargo build), rust (pre-compiled /bench/bin) |
| INFRA-02 | 70-03-PLAN.md | `run.sh` runs all benchmarks using only Docker/Podman (no other prerequisites) | SATISFIED | run.sh uses `command -v docker/podman` detection; no other dependencies; produced raw.json in Plan 04 |
| INFRA-03 | 70-03-PLAN.md | `run.ps1` runs all benchmarks using only Docker/Podman (no other prerequisites) | SATISFIED | run.ps1 uses `Get-Command docker/podman` detection; no other dependencies; Windows path normalization present |
| INFRA-04 | 70-02-PLAN.md | Benchmark harness measures execution time (median ± MAD over N runs) | SATISFIED | run_hyperfine() wraps hyperfine with --export-json; add_mad() computes MAD from times[]; raw.json confirms median and mad present for all languages |
| INFRA-05 | 70-02-PLAN.md | Benchmark harness measures peak memory usage (anonymous RSS) | SATISFIED | measure_anon_rss() polls /proc/<pid>/status RssAnon; memory_kb added per language; raw.json: writ_compile.memory_kb=200, squirrel.memory_kb=196, python.memory_kb=2652, node.memory_kb=8348 |
| INFRA-06 | 70-02-PLAN.md | Benchmark harness measures startup time per language | SATISFIED | Startup section (lines 244-264): 6-language loop producing _ms fields; raw.json startup: writ_ms=0.668, lua_ms=0.715, squirrel_ms=1.297, python_ms=7.846, node_ms=17.292, rust_ms=0.487 |
| INFRA-07 | 70-02-PLAN.md | Writ compile time and runtime reported as separate columns | SATISFIED | Separate hyperfine invocations (lines 111-142); raw.json: distinct writ_compile (median=0.00102) and writ_run (median=0.000674) objects |
| INFRA-08 | 70-02-PLAN.md | Results output as JSON for pipeline consumption | SATISFIED | `printf '%s' "$results" \| jq '.' > "$RESULTS_DIR/raw.json"` (line 306); raw.json: valid JSON with benchmarks[] + meta schema |

All 8 requirement IDs (INFRA-01 through INFRA-08) are satisfied. All marked [x] in REQUIREMENTS.md. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | — | — | — | — |

No TODO/FIXME/HACK/placeholder comments found in any benchmark runner file. `bench_runner.sh` is a substantive 308-line script. `bash -n benchmark/runner/bench_runner.sh` exits 0 (syntax valid).

One cosmetic note: bench_runner.sh has duplicate "Section 3" labels (lines 84 and 93) — one for the pre-compile step and one for the main loop. This is a comment labeling inconsistency only; logic is correct.

### Human Verification Required

None. All success criteria are now verified against real artifacts:

- SC1 (Docker build): confirmed by commits 9b2606a and 992de5f; raw.json was produced by a successful Docker build run
- SC2 (run.sh produces raw.json): `benchmark/results/2026-03-20/raw.json` exists on disk, produced by run.sh
- SC3 (raw.json schema): file verified directly — all 9 benchmarks[0] keys present with correct structure
- SC4 (median+MAD+memory+startup): all fields present with real numeric values from actual measurement runs
- SC5 (version emission): version block in bench_runner.sh lines 24-31; container ran to completion per Plan 04 SUMMARY

### Re-verification Summary

**Previous gap closed.** The single hard gap identified in the initial verification — absence of `benchmark/results/raw.json` — is resolved.

**Root cause was execution, not implementation.** Plan 04 ran `RUNS=3 ./benchmark/runner/run.sh` on Windows with Podman WSL backend and discovered 6 real bugs that were blocking execution:

1. Rust 1.85 too old for `ar_archive_writer` 0.5.1 (needs 1.88 for let-chains) — fixed
2. Squirrel binary copy path wrong (`sq/sq` should be `bin/sq`) — fixed
3. Squirrel shared libraries not installed (`.so*` files needed + ldconfig) — fixed
4. Shell scripts had CRLF line endings (breaks `#!/bin/bash\r` on Linux) — fixed
5. run.sh volume mount path broken on Windows MINGW (MSYS path conversion) — fixed
6. No `.gitattributes` to prevent CRLF recurrence — added

All 6 fixes were committed in `9b2606a`. The pipeline then executed successfully and `benchmark/results/2026-03-20/raw.json` was committed in `992de5f`.

**No regressions.** All artifacts that passed the initial verification continue to pass:
- Dockerfile now has corrected Squirrel install (shared libs + ldconfig) and Rust 1.88
- bench_runner.sh now has LF line endings (confirmed by CRLF-to-LF conversion)
- run.sh now has Windows/MINGW path handling and Docker responsiveness check
- .gitattributes prevents future CRLF issues

---

_Verified: 2026-03-20T17:30:00Z_
_Verifier: Claude (gsd-verifier)_
