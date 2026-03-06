---
phase: 70-docker-environment-and-measurement-harness
plan: 01
subsystem: infra
tags: [docker, dockerfile, multi-stage-build, hyperfine, lua, squirrel, python, nodejs, rust, benchmark]

# Dependency graph
requires: []
provides:
  - Multi-stage Dockerfile (ubuntu:24.04 runtime image) with Writ, Lua 5.4, Squirrel 3.2, Python 3.12, Node.js 22 LTS, pre-compiled Rust, hyperfine 1.20.0, jq
  - Stub benchmark source files for all 6 languages (each prints "hello")
  - benchmark/results/.gitkeep git-tracked results directory
  - Placeholder bench_runner.sh entrypoint emitting version strings
affects: [70-02, 70-03, 71-fibonacci-benchmark, 72-chart-generation]

# Tech tracking
tech-stack:
  added: [hyperfine 1.20.0, lua5.4 (apt), squirrel 3.2 (source build), python3 (apt), nodejs 22.x (NodeSource PPA), jq (apt)]
  patterns:
    - Multi-stage Docker build: writ-builder (cargo) + rust-bench-builder (rustc) + ubuntu:24.04 runtime
    - NodeSource GPG key method for Node.js (not pipe-to-bash setup script)
    - Squirrel source build from v3.2 git tag (squirrel3 absent from Ubuntu 24.04 Noble apt)
    - hyperfine from GitHub Releases .deb (not in Ubuntu apt repos)
    - Squirrel binary is 'sq' (not 'squirrel' — cmake default output)

key-files:
  created:
    - benchmark/runner/Dockerfile
    - benchmark/runner/bench_runner.sh
    - benchmark/cases/stub/stub.writ
    - benchmark/cases/stub/stub.lua
    - benchmark/cases/stub/stub.nut
    - benchmark/cases/stub/stub.py
    - benchmark/cases/stub/stub.js
    - benchmark/cases/stub/stub.rs
    - benchmark/results/.gitkeep
  modified: []

key-decisions:
  - "Three-stage Dockerfile: writ-builder (cargo build --release --bin writ), rust-bench-builder (rustc -O per .rs stub), ubuntu:24.04 runtime"
  - "Squirrel binary named 'sq' not 'squirrel' — cmake default; ARCHITECTURE.md was incorrect"
  - "Node.js 22 LTS via NodeSource GPG key method (not pipe-to-bash) to avoid interactive shell context issues"
  - "hyperfine 1.20.0 installed from GitHub Releases .deb (not in Ubuntu 24.04 apt repos)"
  - "Build context is repo root: docker build -t writ-bench -f benchmark/runner/Dockerfile ."
  - "bench_runner.sh is stub/placeholder in Plan 01 — Plan 02 replaces it with full measurement harness"

patterns-established:
  - "Pattern: Docker build context = repo root; Dockerfile path specified via -f flag"
  - "Pattern: Squirrel version must be sourced from git tag v3.2, cmake, then cp sq/sq /usr/local/bin/sq"
  - "Pattern: All language runtimes emit version strings at container startup for validation"
  - "Pattern: Stub benchmarks per language each print 'hello' — minimal correctness probe"

requirements-completed: [INFRA-01]

# Metrics
duration: 2min
completed: 2026-03-20
---

# Phase 70 Plan 01: Docker Environment and Stub Benchmarks Summary

**Three-stage Dockerfile (ubuntu:24.04) with Writ, Lua 5.4, Squirrel 3.2 (source build), Python 3.12, Node.js 22 LTS (NodeSource), pre-compiled Rust, hyperfine 1.20.0, and 6-language stub benchmarks**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-20T13:48:54Z
- **Completed:** 2026-03-20T13:50:59Z
- **Tasks:** 2 of 2
- **Files modified:** 9

## Accomplishments

- Multi-stage Dockerfile with three distinct stages: writ-builder (cargo), rust-bench-builder (rustc -O), and ubuntu:24.04 runtime with all 6 language interpreters
- Stub benchmark source files for all 6 languages (Writ, Lua, Squirrel, Python, Node.js, Rust), each printing "hello" to prove runtime works
- Placeholder bench_runner.sh entrypoint emitting version strings for all 6 runtimes; Plan 02 will replace with full measurement harness

## Task Commits

Each task was committed atomically:

1. **Task 1: Create stub benchmark source files for all 6 languages** - `328c748` (feat)
2. **Task 2: Create multi-stage Dockerfile with all 6 language runtimes** - `67ccbd0` (feat)

## Files Created/Modified

- `benchmark/runner/Dockerfile` - Three-stage build: writ-builder, rust-bench-builder, ubuntu:24.04 runtime with all runtimes + hyperfine
- `benchmark/runner/bench_runner.sh` - Placeholder entrypoint emitting version strings for all 6 runtimes
- `benchmark/cases/stub/stub.writ` - Writ stub: `fn main() { log::info("hello"); }`
- `benchmark/cases/stub/stub.lua` - Lua stub: `print("hello")`
- `benchmark/cases/stub/stub.nut` - Squirrel stub: `print("hello\n");` (explicit newline required)
- `benchmark/cases/stub/stub.py` - Python stub: `print("hello")`
- `benchmark/cases/stub/stub.js` - Node.js stub: `console.log("hello");`
- `benchmark/cases/stub/stub.rs` - Rust stub: `fn main() { println!("hello"); }`
- `benchmark/results/.gitkeep` - Empty file to git-track the results directory

## Decisions Made

- Squirrel binary is `sq` (not `squirrel`): cmake build produces `sq` by default; ARCHITECTURE.md was incorrect. `bench_runner.sh` uses `sq` everywhere.
- Node.js 22 LTS installed via NodeSource GPG key method rather than pipe-to-bash setup script, avoiding interactive shell context issues in Docker.
- hyperfine 1.20.0 installed from GitHub Releases `.deb` — it is not available in Ubuntu 24.04 apt repos.
- Squirrel 3.2 built from source at git tag `v3.2` — `squirrel3` apt package is absent from Ubuntu 24.04 Noble repos.
- Build context is repo root (`docker build -t writ-bench -f benchmark/runner/Dockerfile .`) so the full Cargo workspace is available to writ-builder stage.
- `bench_runner.sh` is a stub in Plan 01 that only emits version strings; Plan 02 implements the full measurement harness.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Dockerfile and stub benchmarks ready for Plan 02 (measurement harness: bench_runner.sh, run.sh, run.ps1)
- Docker build has not been run (requires network access for NodeSource, GitHub Releases, and Squirrel git clone) — Plan 02 or CI validates the full build
- Squirrel `sq --version` exit code behavior unknown; bench_runner.sh uses `|| echo "sq (version info unavailable)"` guard

---
*Phase: 70-docker-environment-and-measurement-harness*
*Completed: 2026-03-20*

## Self-Check: PASSED

- FOUND: benchmark/runner/Dockerfile
- FOUND: benchmark/runner/bench_runner.sh
- FOUND: benchmark/cases/stub/stub.writ
- FOUND: benchmark/cases/stub/stub.lua
- FOUND: benchmark/cases/stub/stub.nut
- FOUND: benchmark/cases/stub/stub.py
- FOUND: benchmark/cases/stub/stub.js
- FOUND: benchmark/cases/stub/stub.rs
- FOUND: benchmark/results/.gitkeep
- FOUND: 70-01-SUMMARY.md
- FOUND: commit 328c748 (task 1)
- FOUND: commit 67ccbd0 (task 2)
