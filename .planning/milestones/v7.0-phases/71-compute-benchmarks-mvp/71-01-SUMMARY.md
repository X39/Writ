---
phase: 71-compute-benchmarks-mvp
plan: 01
subsystem: benchmarks
tags: [fibonacci, benchmark, writ, lua, squirrel, python, nodejs, rust]

requires:
  - phase: 70-docker-environment-and-measurement-harness
    provides: bench_runner.sh auto-discovery from /bench/cases/*/ and bench/cases/stub/ reference pattern

provides:
  - Fibonacci naive recursive benchmark (fib(40)=102334155) in all 6 benchmark languages
  - benchmark/cases/fib/ directory with 6 source files ready for bench_runner.sh auto-discovery

affects:
  - 71-compute-benchmarks-mvp (plan 02 — Dockerfile Rust compilation + Docker validation)
  - 72+ future benchmark suites (pattern established for case directory structure)

tech-stack:
  added: []
  patterns:
    - "Benchmark case dir = suite name = filename prefix (fib/fib.writ, fib/fib.lua, ...)"
    - "Writ output via log::info produces [INFO] <value> on stderr"
    - "Squirrel print() requires explicit newline: print(fib(40) + \"\\n\")"
    - "Python recursion limit set to 10000 as safety measure (fib(40) depth=40)"
    - "Rust fib uses u64 (fib(40)=102334155 fits, no overflow risk)"

key-files:
  created:
    - benchmark/cases/fib/fib.writ
    - benchmark/cases/fib/fib.lua
    - benchmark/cases/fib/fib.nut
    - benchmark/cases/fib/fib.py
    - benchmark/cases/fib/fib.js
    - benchmark/cases/fib/fib.rs
  modified: []

key-decisions:
  - "fib(40) naive recursive: no memoization, stresses function call overhead and integer arithmetic"
  - "Writ uses log::info($\"{result}\") format string to convert int to string for output"
  - "Lua uses local function (idiomatic; avoids global namespace pollution)"
  - "Rust uses u64 not i64 (fib values are non-negative, u64 is conventional for such benchmarks)"

patterns-established:
  - "Benchmark case: one directory per suite, filenames match directory name, 6 language files per suite"
  - "Verification: run each language locally where available, note Docker-only for unavailable runtimes"

requirements-completed: [BENCH-01, BENCH-08]

duration: 10min
completed: 2026-03-20
---

# Phase 71 Plan 01: Fibonacci Benchmark Source Files Summary

**Naive recursive fib(40)=102334155 implemented in all 6 languages (Writ/Lua/Squirrel/Python/Node.js/Rust) with cross-language output verified**

## Performance

- **Duration:** ~10 min (build time dominated by Cargo release compilation)
- **Started:** 2026-03-20T16:17:23Z
- **Completed:** 2026-03-20T16:27:23Z
- **Tasks:** 1 of 1
- **Files modified:** 6 created

## Accomplishments

- Created all 6 Fibonacci source files in `benchmark/cases/fib/` matching bench_runner.sh auto-discovery pattern
- Verified Writ produces `[INFO] 102334155` on stderr (compile + run both succeed)
- Verified Python, Node.js, and Rust each produce `102334155` on stdout locally
- Lua and Squirrel not available locally — flagged for Docker validation only

## Task Commits

1. **Task 1: Create Fibonacci benchmark source files for all 6 languages** - `4621c65` (feat)

**Plan metadata:** (this commit)

## Files Created/Modified

- `benchmark/cases/fib/fib.writ` - Writ naive recursive fib, log::info($"{result}") output
- `benchmark/cases/fib/fib.lua` - Lua local function, print(fib(40))
- `benchmark/cases/fib/fib.nut` - Squirrel function, explicit "\n" in print
- `benchmark/cases/fib/fib.py` - Python def fib, sys.setrecursionlimit(10000)
- `benchmark/cases/fib/fib.js` - Node.js function, console.log(fib(40))
- `benchmark/cases/fib/fib.rs` - Rust fn fib(n: u64), println!

## Decisions Made

- Writ output uses `log::info($"{result}")` — the only stdout/stderr mechanism available in Writ main programs
- Python `sys.setrecursionlimit(10000)` added as safety measure even though fib(40) only reaches depth 40
- Rust uses `u64` (unsigned) rather than `i64` since Fibonacci values are non-negative
- Lua uses `local function` (idiomatic scoping, prevents global leakage)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Writ fib(40) naive recursive is slow in the interpreter (~3 minutes on release build locally). This is expected — the benchmark exists to measure interpreter performance, and slowness is the data point. No fix applied.
- Lua (`lua5.4`) and Squirrel (`sq`) not installed locally on the development machine. Both are available in the Docker container. Output equivalence will be confirmed in plan 02 Docker validation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All 6 fib source files exist and pass bench_runner.sh naming conventions
- Plan 02 will add fib.rs to Dockerfile wildcard build loop and run full Docker E2E validation
- Lua and Squirrel output verification pending Docker run in plan 02

---
*Phase: 71-compute-benchmarks-mvp*
*Completed: 2026-03-20*
