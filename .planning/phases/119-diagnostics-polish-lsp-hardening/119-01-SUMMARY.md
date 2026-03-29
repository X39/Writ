---
phase: 119-diagnostics-polish-lsp-hardening
plan: "01"
subsystem: writ-diagnostics, writ-cli
tags: [diagnostics, cli, hardening, warnings]
depends_on: []
provides: [ariadne-cross-file-guard, deny-warnings-flag]
affects: [writ-diagnostics, writ-cli]
tech_stack:
  added: []
  patterns: [HashSet guard for ariadne sources, deny_warnings pipeline parameter]
key_files:
  created: []
  modified:
    - writ-diagnostics/src/render.rs
    - writ-cli/src/pipeline.rs
    - writ-cli/src/main.rs
    - writ-cli/src/commands/compile.rs
    - writ-cli/src/commands/build.rs
decisions:
  - "Build known_file_ids HashSet from sources slice at top of render_diagnostics — O(n) upfront cost prevents ariadne panic on absent FileId"
  - "deny_warnings check inserted after each stage (resolve, typecheck) that renders diagnostics — user sees which warnings triggered the failure before the error message"
  - "Pipeline unit tests in pipeline.rs #[cfg(test)] use Box::leak (acceptable leak in test context) to satisfy run_pipeline's 'static lifetime requirement"
  - "W0004 namespace/path mismatch used as deny-warnings test trigger — reliable, no imports needed, always emitted when namespace does not match file path"
metrics:
  duration: "~25 minutes"
  completed: "2026-03-29"
  tasks: 2
  files_modified: 5
---

# Phase 119 Plan 01: Diagnostics Polish — Guard ariadne Rendering and --deny-warnings Flag Summary

Guard ariadne rendering against cross-file secondary label panics (DIAG-01) via a `known_file_ids` filter, and add `--deny-warnings` CLI flag to compile/build subcommands that fails the pipeline when any warning is present (DIAG-03).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Guard ariadne sources slice and verify DIAG-01/DIAG-02 | 105dd80 | writ-diagnostics/src/render.rs |
| 2 | Add --deny-warnings flag to CLI build and compile subcommands | 5cae70a | writ-cli/src/pipeline.rs, writ-cli/src/main.rs, writ-cli/src/commands/compile.rs, writ-cli/src/commands/build.rs |

## What Was Built

### Task 1: DIAG-01 Guard (render.rs)

At the top of `render_diagnostics`, a `HashSet<FileId>` is built from the `sources` slice. Secondary labels whose `file_id` is not in the set are silently skipped before the ariadne label construction loop. This prevents the ariadne `sources()` cache from panicking when:
- A `UnsatisfiedBound` diagnostic attaches a secondary label pointing to `FileId(u32::MAX)` (synthetic built-in types)
- A cross-file project renders diagnostics for only a subset of files

Two new tests were added:
- `render_diagnostics_cross_file_guard`: absent FileId(99) secondary label is skipped; primary label still rendered
- `render_diagnostics_sentinel_file_id_guard`: FileId(u32::MAX) secondary label does not panic

The existing DIAG-01/DIAG-02 tests (`generic_bound_error_has_secondary_label`, `generic_bound_error_has_help_suggestion`) continue to pass — these validate secondary labels for files that ARE in the sources slice, and the guard correctly allows them through.

### Task 2: DIAG-03 --deny-warnings (pipeline.rs, main.rs, compile.rs, build.rs)

`run_pipeline` gains a `deny_warnings: bool` parameter. After the resolve stage and after the typecheck stage, if `deny_warnings` is true and any `Severity::Warning` diagnostic was emitted, the pipeline returns `Err("compilation failed: warnings treated as errors (--deny-warnings)")`. Diagnostics are rendered to stderr before this check, so the user sees which specific warnings caused the failure.

`Commands::Build` and `Commands::Compile` in `main.rs` each gain `#[arg(long)] deny_warnings: bool`. The value is threaded through `cmd_build` / `cmd_compile` to `run_pipeline`.

Pipeline unit tests validate both cases:
- `deny_warnings_fails_on_warning`: W0004 namespace mismatch with `deny_warnings=true` → Err containing "warnings treated as errors"
- `deny_warnings_false_allows_warning`: W0004 with `deny_warnings=false` → Ok

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all functionality is fully wired.

## Self-Check: PASSED

- writ-diagnostics/src/render.rs: exists, contains `known_file_ids`, `render_diagnostics_cross_file_guard`, `render_diagnostics_sentinel_file_id_guard`
- writ-cli/src/pipeline.rs: exists, contains `deny_warnings`, `warnings treated as errors`
- writ-cli/src/main.rs: exists, contains `deny_warnings` for both Build and Compile
- writ-cli/src/commands/compile.rs: exists, contains `deny_warnings`
- writ-cli/src/commands/build.rs: exists, contains `deny_warnings`
- Commits 105dd80 and 5cae70a both exist in git log
- All tests pass: writ-diagnostics (4), writ-compiler generic_bound_error (2), writ-cli (23 total)
