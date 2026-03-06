---
phase: 70-docker-environment-and-measurement-harness
plan: 02
subsystem: infra
tags: [bash, hyperfine, jq, benchmark, docker, memory-measurement, MAD, startup-time, raw-json]

# Dependency graph
requires:
  - phase: 70-01
    provides: Placeholder bench_runner.sh, Dockerfile, stub benchmark source files for all 6 languages
provides:
  - Full in-container benchmark orchestration script (bench_runner.sh) with hyperfine timing, anonymous RSS memory measurement, MAD computation, and startup time measurement
  - raw.json schema implementation: benchmarks[].writ_compile, writ_run, lua, squirrel, python, node, rust, startup + meta
affects: [70-03, 71-fibonacci-benchmark, 72-chart-generation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "measure_anon_rss() shell function: background subprocess + /proc/<pid>/status RssAnon poll loop"
    - "add_mad() jq filter: MAD = median(|Xi - median(X)|) computed from hyperfine times[] array"
    - "run_hyperfine() wrapper: combines hyperfine --export-json with add_mad pipeline"
    - "Skip-on-failure error handling: hyperfine failure sets language result to null in JSON"
    - "Separate writ compile + writ run as two distinct hyperfine invocations (INFRA-07)"

key-files:
  created: []
  modified:
    - benchmark/runner/bench_runner.sh

key-decisions:
  - "add_mad() uses inline abs pattern (if . < 0 then -. else . end) instead of jq fabs — portable across all jq versions"
  - "measure_anon_rss redirects benchmarked command stdout/stderr to /dev/null so only peak_kb is echoed"
  - "Pre-compile stub.writc before loop so startup time measurement has a ready .writc file"
  - "Skip-on-failure: failed language measurements set to null in JSON; script continues rather than aborts"
  - "printf '%s' used instead of echo for jq piping to avoid interpretation of backslashes"
  - "Startup measurement reuses /bench/cases/stub/ files for all suites (not per-suite stub)"

patterns-established:
  - "Pattern: measure_anon_rss() function signature: takes command + args as $@; returns peak KB on stdout"
  - "Pattern: add_mad() reads hyperfine JSON from stdin; outputs results[0] + {mad:...} object"
  - "Pattern: per-language result = hyperfine JSON fields + mad + memory_kb (all in one jq object)"

requirements-completed: [INFRA-04, INFRA-05, INFRA-06, INFRA-07, INFRA-08]

# Metrics
duration: 2min
completed: 2026-03-20
---

# Phase 70 Plan 02: Benchmark Runner Measurement Harness Summary

**Full bench_runner.sh with hyperfine timing + /proc anonymous RSS memory polling + jq MAD computation + separate writ compile/run measurements + startup time + raw.json assembly**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-20T13:54:08Z
- **Completed:** 2026-03-20T13:56:15Z
- **Tasks:** 1 of 1
- **Files modified:** 1

## Accomplishments

- Replaced placeholder bench_runner.sh stub with a 200+ line full measurement harness covering all 6 language runtimes
- Implemented measure_anon_rss() function that polls /proc/<pid>/status RssAnon field in a tight loop while the benchmarked process runs in the background
- Implemented add_mad() jq filter that derives MAD (Median Absolute Deviation) from hyperfine's times[] array using inline abs pattern for maximum jq version compatibility
- Writ measured as two separate hyperfine invocations (writ compile and writ run), satisfying INFRA-07
- Startup time section measures minimal hello-world execution time per language via hyperfine, reporting as *_ms fields
- raw.json assembled with benchmarks[] array + meta object; each language result contains median, mad, and memory_kb fields
- Skip-on-failure error handling: if a language binary fails, its result is null in JSON and the script continues

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement bench_runner.sh with full measurement harness** - `56060d2` (feat)

## Files Created/Modified

- `benchmark/runner/bench_runner.sh` - Full measurement orchestration: version emission, measure_anon_rss(), add_mad(), run_hyperfine(), 6-language benchmark loop, startup time section, raw.json assembly

## Decisions Made

- Used `printf '%s' "$var" | jq` instead of `echo "$var" | jq` for all jq pipelines to avoid backslash interpretation edge cases
- `add_mad()` uses `if . < 0 then -. else . end` pattern for absolute value (no `fabs` dependency) — verified portable in jq 1.6+
- Pre-compile stub.writc at script top (before the loop) so startup measurement's `writ run /tmp/stub.writc` always has a .writc to run
- Memory measurement for writ compile uses a separate output path (`/tmp/${suite}_mem.writc`) so it does not corrupt the main compile artifact
- Startup measurement reuses /bench/cases/stub/ files for all suites rather than using per-suite stubs

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- bench_runner.sh is syntactically valid bash (verified with `bash -n`)
- Full end-to-end raw.json production requires Docker (validated in Plan 03 Task 2)
- Docker daemon was not running in the current dev environment; Docker build/run deferred to Plan 03

---
*Phase: 70-docker-environment-and-measurement-harness*
*Completed: 2026-03-20*

## Self-Check: PASSED

- FOUND: benchmark/runner/bench_runner.sh
- FOUND: 70-02-SUMMARY.md
- FOUND: commit 56060d2 (task 1)
