---
phase: 68-dap-runtime-and-launch-fixes
plan: 02
subsystem: dap
tags: [dap, debug-adapter, multi-file, writ-compiler, project-mode, source-tracking]

# Dependency graph
requires:
  - phase: 68-dap-runtime-and-launch-fixes
    provides: DAP research and context for launch pipeline extensions
provides:
  - compile_and_load_project() function in writ-dap/src/launch.rs for multi-file project compilation
  - Mode detection in handle_launch dispatching single-file vs project mode
  - DapServer.source_paths Vec<(FileId, String)> replacing source_path Option<String>
  - Integration tests verifying multi-file project compilation through DAP
affects: [69-dialogue-function-golden-tests, future-dap-phases]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DAP launch mode detection: path.is_dir() || ends_with(writ.toml) = project mode"
    - "source_paths Vec<(FileId, String)> for multi-file source attribution in DapServer"
    - "compile_and_load_project delegates to writ_compiler::config API (load_config + discover_source_files)"

key-files:
  created: []
  modified:
    - writ-dap/src/launch.rs
    - writ-dap/src/server/mod.rs
    - writ-dap/src/server/handlers.rs
    - writ-dap/src/server/inspection.rs
    - writ-dap/tests/test_compile_and_load.rs

key-decisions:
  - "Use source_paths: Vec<(FileId, String)> to track all project source files, replacing source_path: Option<String>"
  - "Mode detection in handle_launch: path.is_dir() || program_path.ends_with('writ.toml') triggers project mode"
  - "Stack frame source attribution uses source_paths.first() as fallback for all frames (per-frame FileId tracking deferred)"

patterns-established:
  - "Project mode: resolve project_root from either directory path or writ.toml parent, then call compile_and_load_project"
  - "Single-file mode: call compile_and_load, wrap result in vec![(FileId(0), path)] to normalize to source_paths"

requirements-completed: [DAP-02]

# Metrics
duration: 15min
completed: 2026-03-18
---

# Phase 68 Plan 02: Multi-file writ.toml project launch for DAP Summary

**DAP launch pipeline extended with mode detection, compile_and_load_project via writ_compiler::config, and DapServer source_paths Vec replacing single source_path Option**

## Performance

- **Duration:** 15 min
- **Started:** 2026-03-18T17:20:00Z
- **Completed:** 2026-03-18T17:34:43Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments
- Added `compile_and_load_project()` to `writ-dap/src/launch.rs` using `writ_compiler::config::load_config` and `discover_source_files` to find and compile all `.writ` files in a project
- Replaced `source_path: Option<String>` with `source_paths: Vec<(writ_diagnostics::FileId, String)>` in `DapServer`, enabling multi-file source tracking
- Extended `handle_launch` in `handlers.rs` with mode detection: directory or `writ.toml` path triggers project mode, `.writ` path uses existing single-file mode
- Updated `build_stack_frames` in `inspection.rs` to use `source_paths.first()` for frame source attribution
- Added 3 integration tests for `compile_and_load_project` covering multi-file compilation, missing `writ.toml` error, and empty source directory error — all passing

## Task Commits

Each task was committed atomically:

1. **Task 1: Add compile_and_load_project to launch.rs and update DapServer struct** - `6375655` (feat)
2. **Task 2: Update handlers.rs and inspection.rs for multi-file source tracking** - `2c1d45b` (feat)
3. **Task 3: Add integration tests for multi-file project launch** - `d442883` (test)

**Plan metadata:** (docs commit below)

## Files Created/Modified
- `writ-dap/src/launch.rs` - Added `compile_and_load_project()` function using project config APIs
- `writ-dap/src/server/mod.rs` - Replaced `source_path: Option<String>` with `source_paths: Vec<(FileId, String)>`; added `writ_diagnostics` import
- `writ-dap/src/server/handlers.rs` - Added mode detection in `handle_launch`, dispatch to `compile_and_load_project`, updated breakpoint source event reference
- `writ-dap/src/server/inspection.rs` - Updated `build_stack_frames` to use `source_paths.first()` with comment explaining deferred per-frame attribution
- `writ-dap/tests/test_compile_and_load.rs` - Added 3 integration tests for project mode compilation; added `use writ_dap::launch::compile_and_load_project` and `use writ_module::heap::read_string`

## Decisions Made
- `source_paths: Vec<(FileId, String)>` rather than `HashMap<FileId, String>` — iteration order and index-based lookup both matter; Vec is simpler and sufficient for Phase 68 scale
- Per-frame source file attribution (mapping each StackFrame to the correct FileId based on SourceSpan) is deferred: current module format stores SourceSpan without FileId, so all frames use `source_paths.first()` as fallback
- Single-file mode normalizes to the same `source_paths` Vec format (`vec![(FileId(0), path)]`) so all downstream code operates uniformly

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None. The implementation followed the plan interfaces directly. `writ_diagnostics` was already a dependency of `writ-dap` so no Cargo.toml changes were needed. `writ_module` was already accessible for tests as a transitive dependency.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- DAP now supports writ.toml project directories as launch targets in addition to single `.writ` files
- `DapServer.source_paths` provides foundation for future per-frame source attribution once SourceSpan gains FileId
- Phase 69 (Dialogue/Function Golden Tests) can proceed independently

---
*Phase: 68-dap-runtime-and-launch-fixes*
*Completed: 2026-03-18*
