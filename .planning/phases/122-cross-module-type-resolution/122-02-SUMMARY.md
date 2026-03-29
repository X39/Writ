---
phase: 122-cross-module-type-resolution
plan: "02"
subsystem: compiler/config,compiler/check,writ-cli,writ-runtime/tests
tags: [xmod, config, dependencies, virtual-module, integration-tests, cli-wiring]
dependency_graph:
  requires: [inject_module_types, inject_library_sigs, compile_with_libraries]
  provides: [writ-toml-dependencies, cli-library-loading, virtual-module-injection, xmod-integration-tests]
  affects: [writ-cli/commands/build, writ-compiler/config, writ-compiler/check/library_sigs, writ-runtime/tests, writ-compiler/tests]
tech_stack:
  added: []
  patterns:
    - "DependencyConfig enum (Path/Detailed) with #[serde(untagged)] for backward-compatible string shorthand"
    - "Virtual module injected at CLI level (writ-cli depends on both writ-compiler and writ-runtime; writ-compiler cannot)"
    - "lib_module_storage owns Module values; lib_refs borrows them — avoids move-into-closure issue"
    - "Top-level fn sig injection: rebuild type_method_ranges/impl_method_ranges in inject_library_sigs to detect non-owned methods"
key_files:
  created:
    - (none — all modifications)
  modified:
    - writ-compiler/src/config.rs
    - writ-cli/src/commands/build.rs
    - writ-compiler/src/check/library_sigs.rs
    - writ-runtime/tests/coll_integration_tests.rs
    - writ-compiler/tests/xmod_tests.rs
decisions:
  - "Virtual module injection at CLI (cmd_build) level — writ-runtime cannot be a writ-compiler dependency"
  - "lib_module_storage.push(virtual_module) before spawn closure — Module moved into storage before thread captures it"
  - "Top-level fn sigs injected into type_env.fn_sigs by rebuild of ownership ranges in inject_library_sigs (Rule 1 auto-fix)"
  - "run_with_library uses two compile(WRIT_STD_SRC) calls — first for library module in compile_with_libraries, second for runtime with_library (can't share Module across both)"
metrics:
  duration: "~7 minutes"
  completed: "2026-03-30T00:XX:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_created: 0
  files_modified: 5
---

# Phase 122 Plan 02: CLI Wiring, Config, Virtual Module Injection, Integration Tests Summary

**One-liner:** Wired writ.toml [dependencies] config parsing, CLI library loading, virtual module injection into cmd_build, and added comprehensive xmod integration tests with a bug fix for missing top-level function sig injection.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Config + CLI wiring + virtual module injection | 6c49272 | config.rs, build.rs |
| 2 | Integration tests + un-ignore coll_with_library_separate_modules | 9b18d47 | library_sigs.rs, coll_integration_tests.rs, xmod_tests.rs |

## What Was Built

### Task 1: Config + CLI Wiring + Virtual Module Injection

**`writ-compiler/src/config.rs`** — `DependencyConfig` enum added:
- `DependencyConfig::Path(String)` — short form: `writ-std = "path/to/file.writc"`
- `DependencyConfig::Detailed { path: String }` — long form with `[dependencies.name]` table
- `WritConfig.dependencies: HashMap<String, DependencyConfig>` with `#[serde(default)]`
- Three new unit tests: `parse_dependencies_config`, `parse_detailed_dependency_config`, `dependencies_default_empty`

**`writ-cli/src/commands/build.rs`** — Library loading before compile thread:
- Iterates `config.dependencies`, reads each `.writc` file, decodes with `Module::from_bytes`
- Pushes `writ_runtime::virtual_module::build_writ_runtime_module()` to give prelude contracts (Iterable, Iterator, Add, Eq, etc.) real DefId entries via `inject_module_types`
- Passes `lib_refs` slice to `run_pipeline`

### Task 2: Integration Tests + coll Test Un-ignore

**`writ-compiler/src/check/library_sigs.rs`** — Bug fix: top-level function sig injection:
- Added injection of top-level function signatures into `type_env.fn_sigs`
- Rebuilds `type_method_ranges` and `impl_method_ranges` within `inject_library_sigs` to detect non-owned (top-level) methods — same logic as `inject_module_types`
- Looks up DefId for each top-level method name in DefMap and inserts FnSig

**`writ-runtime/tests/coll_integration_tests.rs`** — Un-ignore `coll_with_library_separate_modules`:
- Removed `#[ignore]` attribute — test now runs as a normal test
- Updated `run_with_library` to use `compile_with_libraries` (spawned on 16MB stack thread)
- Two `compile(WRIT_STD_SRC)` calls: first for `compile_with_libraries` library arg, second for `RuntimeBuilder::with_library` (Module can't be shared across both)

**`writ-compiler/tests/xmod_tests.rs`** — Comprehensive XMOD-06 integration tests:
- `xmod_smoke_method_call` upgraded from accepting Ok-or-Err to asserting Ok
- `xmod_field_access`: struct field access on library type (validates `struct_fields` injection)
- `xmod_multiple_libraries`: two libraries simultaneously (validates no DefId collisions)
- `xmod_type_not_found_error`: clean error for unknown type reference (not a panic)
- `xmod_top_level_function_call`: library top-level fn callable from user code (validates `fn_sigs` injection)
- `xmod_class_method_call`: class method call on library type (validates `impl_index` injection)

## Test Results

- All 8 xmod tests pass (`cargo test -p writ-compiler -- xmod`)
- `coll_with_library_separate_modules` passes un-ignored
- Full workspace: 0 failures across all crates (`cargo test --workspace`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing top-level function signature injection in inject_library_sigs**
- **Found during:** Task 2 execution — `xmod_top_level_function_call` failed with `"undefined variable 'add_ints'"`
- **Issue:** `inject_library_sigs` populated struct fields, impl methods, and contract methods but did NOT inject top-level function signatures into `type_env.fn_sigs`. The DefMap entry was present (from `inject_module_types`) but typecheck couldn't find the function's parameter/return types.
- **Fix:** Added top-level function detection and `FnSig` injection into `type_env.fn_sigs` in `inject_library_sigs`, mirroring the ownership-range detection from `inject_module_types`
- **Files modified:** `writ-compiler/src/check/library_sigs.rs`
- **Commit:** 9b18d47

**2. [Rule 3 - Blocking] Virtual module function path required sub-module access**
- **Found during:** Task 1 build attempt — `writ_runtime::build_writ_runtime_module()` not found at crate root
- **Fix:** Used `writ_runtime::virtual_module::build_writ_runtime_module()` (function is in `virtual_module` submodule, not re-exported at root)
- **Files modified:** `writ-cli/src/commands/build.rs`
- **Commit:** Included in 6c49272

### cmd_run not modified

The plan mentioned updating `cmd_run` with "the same pattern." However, `cmd_run` executes a pre-compiled `.writc` binary — it has no compile step and no `run_pipeline` call. Library loading is a compile-time concern only. `cmd_run` was correctly left unmodified.

## Known Stubs

None — all test cases exercise real injection paths with actual assertions (no `let _ = result` stubs remaining except in the type-not-found test which validates Err).

## Self-Check: PASSED
