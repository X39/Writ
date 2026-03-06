---
phase: 72-chart-generation-and-results-pipeline
plan: 02
subsystem: infra
tags: [pygal, benchmarks, shell, powershell, charts, svg, markdown]

# Dependency graph
requires:
  - phase: 72-01
    provides: generate.py producing SVG charts and RESULTS.md from raw.json
provides:
  - run.sh auto-invokes generate.py after Docker container exits
  - run.ps1 auto-invokes generate.py after Docker container exits
  - generate.py validated end-to-end against Phase 71 raw.json
  - Determinism verified: second run produces bit-identical output
affects:
  - Phase 73 (future benchmarks will use same one-command workflow)

# Tech tracking
tech-stack:
  added: []
  patterns: [python3 availability guard before optional tool invocation, Write-Warning for PowerShell warning visibility]

key-files:
  created: []
  modified:
    - benchmark/runner/run.sh
    - benchmark/runner/run.ps1
    - benchmark/generate.py
    - benchmark/results/2026-03-20/RESULTS.md

key-decisions:
  - "python3 availability guard uses 'command -v python3' in sh and 'Get-Command python3 -ErrorAction SilentlyContinue' in PowerShell — graceful degradation, not hard failure"
  - "RESULTS.md 0.0 MB note updated to include 'not measured' wording to match SVG tooltip language"

patterns-established:
  - "Guard optional tools with availability check, warn + show manual invocation command if absent"

requirements-completed: [REPORT-05]

# Metrics
duration: 3min
completed: 2026-03-20
---

# Phase 72 Plan 02: Chart Generation Integration Summary

**run.sh and run.ps1 auto-invoke generate.py after Docker exits, completing REPORT-05 one-command benchmark workflow with end-to-end validation against Phase 71 data**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-20T17:36:50Z
- **Completed:** 2026-03-20T17:39:41Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- run.sh appends python3 availability check + generate.py invocation after container exits; missing python3 prints a warning with manual invocation instructions (pip install pygal==3.1.0 hint)
- run.ps1 appends equivalent PowerShell pattern using Get-Command + Write-Warning; uses & python3 call operator
- End-to-end validation of generate.py against real Phase 71 raw.json: clean-slate regeneration produces all 4 SVGs + RESULTS.md, all REPORT-01 through REPORT-05 checks pass
- Determinism confirmed: second run produces bit-identical output (pygal date comment stripped by regex)

## Task Commits

Each task was committed atomically:

1. **Task 1: Append generate.py invocation to run.sh and run.ps1** - `53d59b9` (feat)
2. **Task 2: End-to-end validation of generate.py against real raw.json** - `83c7d5f` (feat)

**Plan metadata:** (docs commit below)

## Files Created/Modified

- `benchmark/runner/run.sh` - Added python3 check + generate.py invocation replacing single echo "Done." line
- `benchmark/runner/run.ps1` - Added Get-Command python3 check + generate.py invocation replacing Write-Host "Done." line
- `benchmark/generate.py` - Minor fix: updated RESULTS.md 0.0 MB note to include "not measured" wording
- `benchmark/results/2026-03-20/RESULTS.md` - Regenerated with updated wording

## Decisions Made

- python3 availability guard uses `command -v python3` (POSIX sh, consistent with Docker/Podman detection pattern on line 14 of run.sh)
- PowerShell pattern uses `Get-Command python3 -ErrorAction SilentlyContinue` to suppress default error output
- Warning messages in run.sh use plain `echo` (not `>&2`); run.ps1 uses `Write-Warning` for built-in warning formatting
- Missing python3 does NOT cause script failure — graceful degradation preserves the container run results

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated RESULTS.md 0.0 MB note to contain "not measured"**
- **Found during:** Task 2 (end-to-end validation)
- **Issue:** Plan acceptance criteria required "not measured" text in RESULTS.md but generate.py used different wording: "Memory values of 0.0 MB indicate the process exited before RSS polling could sample it."
- **Fix:** Changed to "Memory values of 0.0 MB (not measured) indicate the process exited before RSS polling could sample it." — minimal addition matching SVG tooltip wording
- **Files modified:** benchmark/generate.py, benchmark/results/2026-03-20/RESULTS.md
- **Verification:** grep -q 'not measured' benchmark/results/2026-03-20/RESULTS.md passes
- **Committed in:** 83c7d5f (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — wording mismatch between acceptance criteria and generate.py output)
**Impact on plan:** Minor wording fix, no logic change, no scope creep.

## Issues Encountered

None beyond the wording deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- One-command workflow (REPORT-05) is fully operational: `sh benchmark/runner/run.sh` builds Docker image, runs all 6 languages, generates SVG charts and RESULTS.md automatically
- Phase 73 (OOP/dispatch benchmark) can use the exact same workflow — no changes to runner scripts needed unless new languages are added
- Pre-planning blocker noted: OOP/dispatch canonical algorithm across Squirrel metatables, Lua metatables, Python classes, Writ structs, Rust traits is not defined yet

---
*Phase: 72-chart-generation-and-results-pipeline*
*Completed: 2026-03-20*
