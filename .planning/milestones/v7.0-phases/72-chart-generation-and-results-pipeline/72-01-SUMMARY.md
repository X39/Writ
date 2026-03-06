---
phase: 72-chart-generation-and-results-pipeline
plan: 01
subsystem: benchmarking
tags: [pygal, python, svg, charts, benchmark, reporting]

# Dependency graph
requires:
  - phase: 70-docker-environment-and-measurement-harness
    provides: raw.json benchmark output format and dated results directory structure
  - phase: 71-compute-benchmarks-mvp
    provides: actual raw.json data in benchmark/results/2026-03-20/ for testing

provides:
  - benchmark/generate.py — standalone Python script producing 4 SVG charts + RESULTS.md from raw.json
  - exec_{suite}_all.svg — all-languages log-scale execution time bar chart per benchmark suite
  - exec_{suite}_interp.svg — interpreted-only linear-scale execution time chart per benchmark suite
  - memory.svg — grouped memory usage comparison chart across all suites
  - startup.svg — startup time comparison chart per language
  - RESULTS.md — markdown table with Language/Benchmark/Median/Compile/Memory/Ratio-to-Rust columns
  - Deterministic SVG output via no_prefix=True + date comment stripping (bit-identical re-runs)

affects: [phase-73-oop-dispatch-benchmark, phase-74-ci-workflow]

# Tech tracking
tech-stack:
  added: [pygal==3.1.0]
  patterns:
    - pygal Bar chart with disable_xml_declaration=True + no_prefix=True for deterministic SVG
    - Date comment stripping via _DATE_COMMENT_RE regex after chart.render() for bit-identical output
    - bytes.decode('utf-8') required before regex in pygal 3.1.0 (render() returns bytes)
    - ALL_LANGS canonical order constant ensures consistent color-to-language mapping across all charts
    - lang_memory_mb() returns 0.0 with tooltip label for short-lived processes (polling limitation)

key-files:
  created:
    - benchmark/generate.py
    - benchmark/results/2026-03-20/exec_stub_all.svg
    - benchmark/results/2026-03-20/exec_stub_interp.svg
    - benchmark/results/2026-03-20/memory.svg
    - benchmark/results/2026-03-20/startup.svg
    - benchmark/results/2026-03-20/RESULTS.md
  modified: []

key-decisions:
  - "Writ bar uses writ_run.memory_kb for memory chart (runtime process, not compiler process); compiler memory is separate concern"
  - "Startup chart uses benchmarks[0]['startup'] — startup keys are writ_ms/lua_ms/squirrel_ms/python_ms/node_ms/rust_ms, already in ms"
  - "memory_kb=0 rendered as bar with tooltip 'not measured (process too fast)' — valid data, documented limitation"
  - "generate.py writes output files to raw_path.parent (same directory as raw.json) — forward-compatible with any dated subdirectory"

patterns-established:
  - "Pattern: render_svg() with bytes.decode + date comment regex strip — apply to all pygal chart writes"
  - "Pattern: ALL_LANGS constant with canonical order — always add series in this order for color consistency"

requirements-completed: [REPORT-01, REPORT-02, REPORT-03, REPORT-04]

# Metrics
duration: 2min
completed: 2026-03-20
---

# Phase 72 Plan 01: Chart Generation and Results Pipeline Summary

**pygal-based SVG chart generator (benchmark/generate.py) producing 4 charts and RESULTS.md from raw.json with bit-identical deterministic output**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-20T17:32:22Z
- **Completed:** 2026-03-20T17:34:32Z
- **Tasks:** 2 (Task 1: create generate.py + outputs; Task 2: determinism verification)
- **Files modified:** 6

## Accomplishments

- Created `benchmark/generate.py` (~230 lines): reads raw.json, produces 2 exec SVGs per benchmark suite + memory.svg + startup.svg + RESULTS.md
- Writ bars show combined compile+run time with per-bar tooltip breakdown (compile: Xms, run: Xms)
- Determinism verified: bit-identical output on consecutive runs (no UUID patterns, date comment stripped)
- RESULTS.md table with all 6 languages, Writ compile column, ratio-to-Rust column (x1.0x for Rust, calculated for others)

## Task Commits

1. **Task 1: Create benchmark/generate.py with all chart generation and RESULTS.md** - `f9ab5fd` (feat)
2. **Task 2: Verify deterministic output** - No commit needed (validation only, no code changes — Task 1 files already deterministic)

## Files Created/Modified

- `benchmark/generate.py` — standalone Python 3.10+ script; CLI: `python3 benchmark/generate.py <path-to-raw.json>`
- `benchmark/results/2026-03-20/exec_stub_all.svg` — all-languages log-scale execution time chart
- `benchmark/results/2026-03-20/exec_stub_interp.svg` — interpreted-only linear-scale execution time chart
- `benchmark/results/2026-03-20/memory.svg` — grouped memory usage chart across all suites
- `benchmark/results/2026-03-20/startup.svg` — startup time comparison chart
- `benchmark/results/2026-03-20/RESULTS.md` — markdown results table

## Decisions Made

- Used `writ_run.memory_kb` for Writ's memory bar (runtime process, not compiler) — consistent with all other languages showing runtime memory
- `benchmarks[0]['startup']` used for startup chart — startup keys are language-level constants, same across all suites
- `memory_kb=0` displayed as `{'value': 0, 'label': 'not measured (process too fast)'}` — honest representation of polling limitation
- Output directory always derives from `raw_path.parent` — works for any dated subdirectory without hardcoding

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - pygal 3.1.0 installed and working as expected. The `chart.render()` returning bytes (not str) in pygal 3.1.0 was documented in RESEARCH.md and handled correctly with `.decode('utf-8')` before the regex sub.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- generate.py is ready for Phase 73 use — when Phase 73 adds fib/sieve benchmark data to raw.json, generate.py will automatically produce `exec_fib_all.svg`, `exec_fib_interp.svg`, `exec_sieve_all.svg`, `exec_sieve_interp.svg` in addition to the existing stub charts
- run.sh and run.ps1 integration (auto-invoke generate.py after container exits) is deferred to Phase 72 Plan 02
- Phase 74 CI workflow can reference generate.py directly

## Self-Check: PASSED

- benchmark/generate.py: FOUND
- benchmark/results/2026-03-20/exec_stub_all.svg: FOUND
- benchmark/results/2026-03-20/RESULTS.md: FOUND
- .planning/phases/72-chart-generation-and-results-pipeline/72-01-SUMMARY.md: FOUND
- Commit f9ab5fd: FOUND

---
*Phase: 72-chart-generation-and-results-pipeline*
*Completed: 2026-03-20*
