---
phase: 45-writ-toml-project-file-compilation
plan: 02
subsystem: cli
tags: [cli, build, writ-build, profiles, debug-info, multi-file, spec]

# Dependency graph
requires:
  - "45-01 (ProfileConfig/ProfilesConfig structs, emit_debug_info parameter)"
provides:
  - "writ build subcommand: multi-file project compilation from writ.toml"
  - "run_pipeline() shared helper extracted from cmd_compile"
  - "Profile-aware output path: {output_base}/{profile}/{name}.writc"
  - "cmd_compile directory-input helpful error pointing to writ build"
  - "cmd_new Next steps updated to say writ build"
  - "Spec section 2.7 Build Profiles with [profile.debug]/[profile.release] docs"
affects:
  - "End users: writ build is now the recommended compile path for projects"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "run_pipeline() shared helper pattern: 5-stage pipeline extracted, callers own thread spawning"
    - "16MB-stack thread spawn in cmd_build for deep AST recursion safety"
    - "Box::leak() per-file for 'static source strings in multi-file pipeline"

key-files:
  created: []
  modified:
    - "writ-cli/src/main.rs"
    - "language-spec/spec/03_2_project_configuration_writ_toml.md"

key-decisions:
  - "run_pipeline() is synchronous — thread spawning stays in cmd_compile and cmd_build; callers own the 16MB stack thread"
  - "module_name parameter on run_pipeline is reserved (_module_name) — find_module_name() in collect.rs is sufficient for v3.2"
  - "output_base strips trailing slash from config.compiler.output (via as_deref().unwrap_or(build)) — avoids double-slash paths"

patterns-established:
  - "Multi-file pipeline: iterate discovered files, Box::leak each, push (FileId, display_path, src) tuples, pass Vec to run_pipeline"

requirements-completed: [TOOL-01, TOOL-02]

# Metrics
duration: 3min
completed: 2026-03-06
---

# Phase 45 Plan 02: writ build Subcommand and Spec Amendment Summary

**writ build subcommand for multi-file project compilation with profile-aware DebugLocal gating, shared run_pipeline() helper, and spec section 2.7 documenting [profile.debug]/[profile.release]**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T22:01:49Z
- **Completed:** 2026-03-06T22:04:55Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `Build` variant to the `Commands` enum with `path`, `--release`, `--debug`, and `--name` flags (clap-managed conflicts_with ensuring mutual exclusion)
- Extracted shared `run_pipeline()` helper from `cmd_compile` — the 5-stage parse/lower/resolve/typecheck/emit pipeline now lives in one place; thread spawning remains in each subcommand
- Implemented `cmd_build`: loads `writ.toml` via `load_config`, discovers `.writ` files via `discover_source_files`, spawns a 16MB-stack thread, calls `run_pipeline` with profile-selected `emit_debug_info`, writes output to `{output_base}/{profile}/{name}.writc` with `create_dir_all`
- Refactored `cmd_compile` to use `run_pipeline`; added directory-input detection returning helpful error "Use `writ build`"
- Updated `cmd_new` Next steps message: "writ compile sources/main.writ" -> "writ build"
- Added `[profile.debug]` and `[profile.release]` to the section 2.3 Optional Fields example TOML block
- Added section 2.7 Build Profiles documenting `debug_info` field, both profile defaults, output path pattern, and future extensibility note

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement writ build subcommand with pipeline refactor** - `1fd3973` (feat)
2. **Task 2: Spec amendment for profile sections** - `5d5691b` (feat)

## Files Created/Modified

- `writ-cli/src/main.rs` - Added Build subcommand, run_pipeline() helper, cmd_build(), refactored cmd_compile(), updated cmd_new() Next steps
- `language-spec/spec/03_2_project_configuration_writ_toml.md` - Added profile sections to 2.3 example TOML, added section 2.7 Build Profiles

## Decisions Made

- `run_pipeline()` is synchronous — it does not spawn its own thread. Thread spawning (with 16MB stack) is the responsibility of `cmd_compile` and `cmd_build`. This keeps the helper simple and avoids nested thread spawning.
- The `module_name` parameter is prefixed `_module_name` (reserved) — `find_module_name()` in collect.rs already derives the module name from namespace declarations in source, which is sufficient for v3.2. The output filename is separately controlled by `project.name` / `--name`.
- Output base uses `config.compiler.output.as_deref().unwrap_or("build")` — the trailing-slash from writ.toml `output = "build/"` is not doubled because `Path::join` handles it correctly on both platforms.

## Deviations from Plan

None — plan executed exactly as written.

## Verification Results

Integration smoke test passed end-to-end:
- `writ new test45` creates project with "Run 'writ build' to compile" in Next steps
- `writ build` (debug): discovers `sources/main.writ`, produces `build/debug/test45.writc` (492 bytes with debug info)
- `writ build --release`: produces `build/release/test45.writc` (486 bytes, smaller due to stripped DebugLocal)
- `writ compile sources/main.writ`: produces `sources/main.writc` unchanged
- `writ compile .`: exits 1 with "is a directory. Use `writ build`"
- All workspace tests: 100% pass (no regressions)

---
*Phase: 45-writ-toml-project-file-compilation*
*Completed: 2026-03-06*
