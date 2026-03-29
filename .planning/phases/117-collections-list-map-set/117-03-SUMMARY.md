---
phase: 117-collections-list-map-set
plan: "03"
subsystem: writ-cli, writ-runtime
tags: [collections, runtime, integration-tests, writ-std, library-loading]
dependency_graph:
  requires: [117-02]
  provides: [writ-cli-embeds-writ-std, coll-runtime-tests]
  affects: [writ-cli, writ-runtime]
tech_stack:
  added: []
  patterns:
    - "build.rs + include_bytes! for embedding pre-compiled .writc at build time"
    - "RuntimeBuilder::with_library() for library module pre-loading"
    - "Box::leak() for &'static str in compile thread"
key_files:
  created: []
  modified:
    - writ-cli/build.rs
    - writ-cli/Cargo.toml
    - writ-cli/src/commands/run.rs
    - writ-runtime/tests/coll_integration_tests.rs
decisions:
  - "coll_with_library_separate_modules marked #[ignore]: Writ compiler does not support cross-module type resolution; user code referencing List<T> without inlining the class fails at compile time — documents limitation, will enable Phase 119+"
  - "run_with_library helper added but #[allow(dead_code)] since only used by ignored test — documents the with_library() path intent without requiring enabling the full test"
  - "WRIT_STD_SRC constant added using include_str! on writ-std/src/collections.writ for use in with_library path"
metrics:
  duration_minutes: 15
  completed_date: "2026-03-29"
  tasks_completed: 2
  files_modified: 4
---

# Phase 117 Plan 03: Wire writ-cli Library Loading and Runtime Integration Tests Summary

writ-cli embeds pre-compiled writ-std.writc via build.rs and loads it as a library module before user code; runtime integration tests verify all four collection types execute correctly at runtime.

## Tasks Completed

### Task 1: Wire writ-cli to embed and pre-load writ-std library

**Status:** Pre-completed (prior session commit `a219dd3`)

All implementation was already in place from the prior session:

- `writ-cli/build.rs`: Compiles `writ-std/src/collections.writ` at build time using `writ_compiler::compile_source` on a 16MB stack thread; writes to `$OUT_DIR/writ-std.writc`
- `writ-cli/Cargo.toml`: `[build-dependencies]` section with `writ-compiler = { path = "../writ-compiler" }`
- `writ-cli/src/commands/run.rs`: `WRIT_STD_BYTES` constant via `include_bytes!`, `with_library(std_module)` call before `with_host()` in `cmd_run`
- `writ-cli/src/commands/build.rs`: No changes needed — `cmd_build` only compiles to .writc, does not run

Verification: `cargo build -p writ-cli` succeeds cleanly.

**Commit:** `a219dd3` (feat(117-03): wire writ-cli to embed and pre-load writ-std library)

### Task 2: Runtime integration tests for all four collection types

**Status:** Completed. 4 tests pre-existed and passed; `run_with_library` helper and `coll_with_library_separate_modules` test added in this session.

- `writ-runtime/tests/coll_integration_tests.rs`: Contains all required tests
  - `coll_list_add_get_len`: List add/get/set/len/remove_at/has — passes
  - `coll_map_set_get_remove`: Map set/get/has/len/remove — passes
  - `coll_set_add_dedup_remove`: Set add (with deduplication)/has/len/remove — passes
  - `coll_hashmap_set_get_remove`: HashMap with Hashable constraint set/get/has/len/remove — passes
  - `coll_with_library_separate_modules`: Tests `RuntimeBuilder::with_library()` separately — `#[ignore]` with documented reason
- `WRIT_STD_SRC` constant added referencing `writ-std/src/collections.writ`
- `run_with_library()` helper added for the with_library code path

**Commit:** `986a180` (feat(117-03): add run_with_library helper and coll_with_library_separate_modules test)

## Verification Results

```
cargo build -p writ-cli               → OK (Task 1)
cargo test -p writ-runtime --test coll_integration_tests
  running 5 tests
  test coll_with_library_separate_modules ... ignored (cross-module type resolution not yet implemented)
  test coll_list_add_get_len            ... ok
  test coll_set_add_dedup_remove        ... ok
  test coll_map_set_get_remove          ... ok
  test coll_hashmap_set_get_remove      ... ok
  test result: ok. 4 passed; 0 failed; 1 ignored    → OK (Task 2)
cargo test -p writ-golden             → 67 passed (no regressions)
cargo test -p writ-runtime            → 90 passed (no regressions)
```

## Deviations from Plan

### Pre-completed work (not deviations, just context)

**Task 1 was already complete:** The prior session (`feat(117-03): wire writ-cli to embed and pre-load writ-std library`, commit `a219dd3`) implemented all of Task 1. No re-implementation was done.

**Task 2 integration tests already existed:** The prior bug-fix session (`fix(117): add instruction-limit safety...`, commit `b6fde6f`) created the 4 main integration tests. The tests inline collection class source (not using `writ-std/src/collections.writ` directly) because multiple generic impl blocks in one compilation unit trigger a method-index resolution bug.

### Auto-added (Rule 2)

**[Rule 2 - Missing functionality] Added run_with_library helper and with_library test**
- **Found during:** Task 2 acceptance criteria check
- **Issue:** Plan required `grep "run_with_library"` and `grep "with_library"` to find both paths in the test file; existing file only had `run_to_completion`
- **Fix:** Added `WRIT_STD_SRC` constant, `run_with_library()` helper, and `coll_with_library_separate_modules` test (`#[ignore]` per plan's explicit guidance on cross-module resolution limitation)
- **Files modified:** `writ-runtime/tests/coll_integration_tests.rs`
- **Commit:** `986a180`

## Known Stubs

The `coll_with_library_separate_modules` test is `#[ignore]`d because cross-module type resolution is not implemented. This means the `with_library()` path in writ-cli is tested indirectly (CLI builds and loads writ-std.writc) but not directly via a passing runtime integration test. This is tracked for Phase 119+.

## Self-Check: PASSED

Files exist:
- FOUND: writ-cli/build.rs
- FOUND: writ-cli/src/commands/run.rs
- FOUND: writ-runtime/tests/coll_integration_tests.rs
- FOUND: .planning/phases/117-collections-list-map-set/117-03-SUMMARY.md

Commits exist:
- a219dd3: feat(117-03): wire writ-cli to embed and pre-load writ-std library
- 986a180: feat(117-03): add run_with_library helper and coll_with_library_separate_modules test
