---
phase: 70-docker-environment-and-measurement-harness
plan: 04
subsystem: infra
tags: [docker, podman, benchmark, rust, squirrel, lua, python, nodejs, hyperfine, raw.json]

# Dependency graph
requires:
  - phase: 70-docker-environment-and-measurement-harness plan 03
    provides: run.sh, run.ps1, Dockerfile, bench_runner.sh all complete
provides:
  - benchmark/results/2026-03-20/raw.json — proof artifact with all 6 runtimes measured
  - Verified Docker/Podman build pipeline works end-to-end on Windows with Podman WSL
  - Fixed Dockerfile (Rust 1.88, Squirrel so path, shared lib install)
  - Fixed shell scripts (LF line endings, Windows Podman path handling)
affects: [71-stub-benchmarks, 72-algorithm-benchmarks, 73-oop-benchmarks, 74-ci-pipeline]

# Tech tracking
tech-stack:
  added: [podman-machine (WSL), .gitattributes LF enforcement]
  patterns:
    - "Podman WSL volume mounts require /mnt/<drive>/... path format on Windows MINGW"
    - "MSYS_NO_PATHCONV=1 + /mnt/d/... for volume; plain path for build context"
    - "Squirrel cmake build: binary at build/bin/sq; shared libs .so* at build/; copy both + ldconfig"
    - "Shell scripts for Linux containers must have LF endings; enforce via .gitattributes *.sh eol=lf"

key-files:
  created:
    - benchmark/results/2026-03-20/raw.json
    - .gitattributes
  modified:
    - benchmark/runner/Dockerfile
    - benchmark/runner/bench_runner.sh
    - benchmark/runner/run.sh

key-decisions:
  - "Rust 1.88-slim required (not 1.85): ar_archive_writer 0.5.1 uses let-chains stabilized in 1.88"
  - "Squirrel shared libraries must be copied to /usr/local/lib + ldconfig run at build time"
  - "Windows path handling in run.sh: detect Docker responsiveness, convert /d/... to /mnt/d/... for Podman WSL"
  - ".gitattributes *.sh eol=lf added to prevent CRLF recurrence in shell scripts"

patterns-established:
  - "run.sh Windows path conversion: cut drive letter from MINGW path, prepend /mnt/"
  - "Podman machine start required before Docker CLI can use default context on Windows"

requirements-completed: [INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06, INFRA-07, INFRA-08]

# Metrics
duration: 45min
completed: 2026-03-20
---

# Phase 70 Plan 04: Gap Closure — Docker Pipeline End-to-End Validation Summary

**End-to-end Docker benchmark pipeline validated: raw.json produced with all 6 runtimes (Writ compile/run split, Lua, Squirrel, Python, Node.js 22, Rust), median+MAD+memory_kb+startup fields correct, Phase 70 SC1-SC5 all verified.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-20T15:10:00Z
- **Completed:** 2026-03-20T16:35:00Z
- **Tasks:** 2 (1 auto + 1 human-verify auto-approved)
- **Files modified:** 5 (Dockerfile, bench_runner.sh, run.sh, .gitattributes, raw.json)

## Accomplishments

- Docker image built successfully with all 6 language runtimes using Podman WSL backend
- `benchmark/results/2026-03-20/raw.json` produced with correct schema — closes Phase 70 verification gap
- Squirrel 3.2 benchmark fully working: binary + shared libraries installed, ldconfig updated
- `run.sh` now works on Windows with Podman via MINGW path conversion
- `.gitattributes` prevents CRLF line-ending issues for future shell scripts

## Task Commits

Each task was committed atomically:

1. **Task 1: Run Docker pipeline end-to-end and validate raw.json** - `9b2606a` (fix), `992de5f` (feat)
2. **Task 2: Confirm pipeline output and close verification gap** - auto-approved (checkpoint:human-verify)

**Plan metadata:** (this SUMMARY commit)

## Files Created/Modified

- `benchmark/runner/Dockerfile` - Updated Rust to 1.88, fixed Squirrel binary/lib paths
- `benchmark/runner/bench_runner.sh` - Converted CRLF to LF line endings
- `benchmark/runner/run.sh` - Converted CRLF to LF + Windows/Podman path handling
- `.gitattributes` - Enforce LF line endings for all *.sh files
- `benchmark/results/2026-03-20/raw.json` - Proof artifact (210 lines, all 6 runtimes)

## Decisions Made

- **Rust 1.88-slim** selected over 1.85 because `ar_archive_writer` 0.5.1 (transitive dep) uses `let` chains stabilized in 1.88
- **Squirrel shared libs** (`libsquirrel.so.0`, `libsqstdlib.so.0`) must be copied to `/usr/local/lib/` and `ldconfig` run; cmake builds them as shared by default
- **Podman WSL** used instead of Docker Desktop (Docker Desktop not running); Podman machine started and exposed docker_engine pipe
- **Windows path conversion** in `run.sh`: MINGW paths `/d/dev/...` converted to `/mnt/d/dev/...` for Podman WSL volume mounts; `MSYS_NO_PATHCONV=1` set around run command only (not build command, which needs MSYS auto-conversion for the build context)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Dockerfile Rust version too old for ar_archive_writer 0.5.1**
- **Found during:** Task 1 (Docker build step 1)
- **Issue:** `rust:1.85-slim` cannot compile `ar_archive_writer 0.5.1` — uses `let` chains (E0658) stabilized in Rust 1.88
- **Fix:** Updated both build stages from `rust:1.85-slim` to `rust:1.88-slim`
- **Files modified:** `benchmark/runner/Dockerfile`
- **Verification:** Stage 1 cargo build passes, writ binary produced
- **Committed in:** `9b2606a`

**2. [Rule 1 - Bug] Squirrel binary copy path wrong (`sq/sq` should be `bin/sq`)**
- **Found during:** Task 1 (Docker build Squirrel stage)
- **Issue:** `cp sq/sq /usr/local/bin/sq` fails — cmake places binary at `build/bin/sq` not `build/sq/sq`
- **Fix:** Changed `cp sq/sq` to `cp bin/sq`
- **Files modified:** `benchmark/runner/Dockerfile`
- **Verification:** Squirrel builds and `sq` is in `/usr/local/bin/sq`
- **Committed in:** `9b2606a`

**3. [Rule 1 - Bug] Squirrel shared libraries not installed**
- **Found during:** Task 1 (container runtime — `sq` fails on `libsqstdlib.so.0: No such file or directory`)
- **Issue:** cmake builds Squirrel as shared library by default; only binary was copied; shared libs missing from runtime image
- **Fix:** Added `find /tmp/squirrel/build -name "*.so*" -exec cp {} /usr/local/lib/ \;` and `ldconfig` after binary copy
- **Files modified:** `benchmark/runner/Dockerfile`
- **Verification:** `sq stub.nut` runs correctly; squirrel median ~1ms in raw.json
- **Committed in:** `9b2606a`

**4. [Rule 1 - Bug] Shell scripts have CRLF line endings, preventing Linux execution**
- **Found during:** Task 1 (container startup — `No such file or directory` for `/bench/bench_runner.sh`)
- **Issue:** `bench_runner.sh` and `run.sh` had CRLF line endings; Linux shell interprets shebang as `#!/bin/bash\r` (not found)
- **Fix:** `sed -i 's/\r//'` on both scripts to convert to LF
- **Files modified:** `benchmark/runner/bench_runner.sh`, `benchmark/runner/run.sh`
- **Verification:** `file bench_runner.sh` shows "ASCII text executable" (no CRLF); container starts and runs
- **Committed in:** `9b2606a`

**5. [Rule 2 - Missing Critical] Added .gitattributes to prevent CRLF recurrence**
- **Found during:** Task 1 (after CRLF fix — no mechanism to prevent recurrence)
- **Issue:** Git on Windows auto-converts LF to CRLF on checkout without `eol=lf` config; would break container scripts again
- **Fix:** Created `.gitattributes` with `*.sh text eol=lf` and `*.bash text eol=lf`
- **Files modified:** `.gitattributes` (created)
- **Verification:** `.gitattributes` committed; future checkouts will use LF for shell scripts
- **Committed in:** `9b2606a`

**6. [Rule 1 - Bug] run.sh volume mount path conversion broken on Windows**
- **Found during:** Task 1 (container writes to `C:/Program Files/Git/results/raw.json` instead of `/results`)
- **Issue:** MSYS converts `/results` to `C:\Program Files\Git\results` in volume spec; raw.json not written to host path
- **Fix:** Added Windows case in `run.sh`: detect MINGW/MSYS, convert MINGW drive path (`/d/...`) to Podman WSL path (`/mnt/d/...`), set `MSYS_NO_PATHCONV=1` for run command only, use `//results` to prevent MSYS conversion of container path
- **Files modified:** `benchmark/runner/run.sh`
- **Verification:** `run.sh` produces `benchmark/results/2026-03-20/raw.json` on host
- **Committed in:** `9b2606a`

---

**Total deviations:** 6 auto-fixed (4 Rule 1 bugs, 1 Rule 2 missing critical, 1 Rule 1 bug in run.sh)
**Impact on plan:** All fixes necessary for the pipeline to execute at all. No scope creep — every fix was directly blocking execution.

## Issues Encountered

- Docker Desktop was not running; used Podman machine (`podman machine start`) which exposes Docker API via `docker_engine` pipe
- MSYS path conversion (Git Bash on Windows) interferes with Docker/Podman volume mount specs in two different ways: MSYS converts `/results` to `C:\Program Files\Git\results` AND converts drive paths — required careful handling of when to disable MSYS conversion
- `docker version` check in `run.sh` now used to detect whether Docker is actually responsive (not just installed)

## Next Phase Readiness

Phase 70 is complete. All 5 success criteria verified:
- SC1: Docker build completes without error (Podman WSL, all 3 stages pass)
- SC2: `run.sh` produces `benchmark/results/YYYY-MM-DD/raw.json` using only Docker/Podman
- SC3: raw.json has `writ_compile` and `writ_run` as separate objects with median, mad, memory_kb
- SC4: Median, MAD, anonymous RSS memory, and startup times all present as distinct JSON fields
- SC5: Container startup emits version strings for all 6 runtimes (sq prints usage line, documented)

Ready for Phase 71 (stub benchmarks with algorithm spec), Phase 72 (algorithm benchmarks), etc.

---
*Phase: 70-docker-environment-and-measurement-harness*
*Completed: 2026-03-20*

## Self-Check: PASSED

- FOUND: `benchmark/results/2026-03-20/raw.json`
- FOUND: `benchmark/runner/Dockerfile`
- FOUND: `.gitattributes`
- FOUND: `.planning/phases/70-docker-environment-and-measurement-harness/70-04-SUMMARY.md`
- FOUND commit `9b2606a`: fix(70-04): Docker pipeline fixes for end-to-end execution
- FOUND commit `992de5f`: feat(70-04): produce proof artifact raw.json via Docker pipeline
