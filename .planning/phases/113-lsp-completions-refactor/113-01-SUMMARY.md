---
phase: 113-lsp-completions-refactor
plan: 01
subsystem: lsp
tags: [rust, lsp, completions, type-env, refactor]

# Dependency graph
requires: []
provides:
  - TypeEnv.prelude_enum_variants field (FxHashMap<String, Vec<String>>) populated at build time
  - Unified prelude enum completions via data-driven lookup in build_namespace_completions
affects: [lsp-completions, type-env, future-prelude-types]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Prelude enum completions driven by TypeEnv field, not hardcoded if-blocks"
    - "FxHashMap<String, Vec<String>> keyed by type name for extensible prelude variant registry"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/env.rs
    - writ-lsp/src/queries/completion.rs

key-decisions:
  - "prelude_enum_variants uses FxHashMap<String, Vec<String>> (owned strings) consistent with every other TypeEnv field"
  - "Field populated unconditionally in TypeEnv::build — every instance carries prelude data, no lazy init"
  - "Test TypeEnv literals built with HashMap::into_iter().collect() to avoid direct rustc_hash dependency in writ-lsp tests"

patterns-established:
  - "New prelude types can be added to completions by inserting into prelude_enum_variants in TypeEnv::build — no completion.rs changes needed"

requirements-completed: [LSP-01]

# Metrics
duration: 8min
completed: 2026-03-29
---

# Phase 113 Plan 01: LSP Completions Refactor Summary

**Option:: and Result:: completions now driven by TypeEnv.prelude_enum_variants, eliminating hardcoded if-blocks in build_namespace_completions**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-29T00:00:00Z
- **Completed:** 2026-03-29T00:03:57Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added `prelude_enum_variants: FxHashMap<String, Vec<String>>` field to `TypeEnv` struct, populated with `Option=[Some,None]` and `Result=[Ok,Err]` in `TypeEnv::build`
- Replaced two hardcoded early-return blocks (`if namespace == "Option"`, `if namespace == "Result"`) in `build_namespace_completions` with a single `type_env.prelude_enum_variants.get(namespace)` lookup
- Updated all three test `TypeEnv` struct literals in `completion.rs` to include the new `prelude_enum_variants` field
- All 27 writ-lsp tests and all writ-compiler tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add prelude_enum_variants field to TypeEnv and populate in build** - `1738528` (feat)
2. **Task 2: Replace hardcoded branches in build_namespace_completions and update tests** - `03d509a` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `writ-compiler/src/check/env.rs` - Added `prelude_enum_variants` field (struct def + build constructor initialization)
- `writ-lsp/src/queries/completion.rs` - Replaced hardcoded Option/Result branches with unified lookup; updated 3 test TypeEnv literals

## Decisions Made
- `prelude_enum_variants` uses `FxHashMap<String, Vec<String>>` (owned strings), consistent with all other TypeEnv fields
- Field populated unconditionally in `TypeEnv::build` — every TypeEnv instance carries the prelude data
- Test literals use `HashMap::into_iter().collect()` to produce `FxHashMap` without a direct `rustc_hash` dependency in the writ-lsp crate

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Replaced rustc_hash::FxHashMap with HashMap::collect() in test literals**
- **Found during:** Task 2 (completion.rs test update)
- **Issue:** Plan specified `rustc_hash::FxHashMap::default()` in test TypeEnv literals, but `rustc_hash` is not a direct dependency of `writ-lsp`; compilation failed with E0433
- **Fix:** Used `std::collections::HashMap` with `.into_iter().collect()` to produce an `FxHashMap<String, Vec<String>>` — Rust infers the target type from the struct field
- **Files modified:** `writ-lsp/src/queries/completion.rs`
- **Verification:** `cargo test -p writ-lsp -- completion` — 34 tests pass
- **Committed in:** `03d509a` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - build error from missing crate dependency in test code)
**Impact on plan:** Minimal — functional behavior identical, only construction syntax differs. No scope change.

## Issues Encountered
- `rustc_hash::FxHashMap` not directly accessible in `writ-lsp` crate tests — fixed using `HashMap::into_iter().collect()` pattern

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- LSP completions refactor complete; TypeEnv is now the canonical source for prelude enum completions
- Future prelude types (e.g., Bool, custom builtins) can be added to `prelude_enum_variants` without touching `completion.rs`
- No blockers

---
*Phase: 113-lsp-completions-refactor*
*Completed: 2026-03-29*
