---
phase: 31-test-harness-and-functions-golden-test
plan: 01
subsystem: testing
tags: [golden-tests, writ-golden, similar, tempfile, disassembler, round-trip]

# Dependency graph
requires:
  - phase: 30-critical-bug-fixes
    provides: stable compile pipeline (parse->lower->resolve->typecheck->emit_bodies) and Module::from_bytes round-trip
provides:
  - writ-golden workspace crate with compile-disassemble-compare harness
  - BLESS=1 bless workflow to lock/update .expected golden files
  - Unified diff (--- expected / +++ actual) on golden file mismatch
  - run_golden_test(name) and compile_and_disassemble(src) for phases 32-36
affects: [32-structs-golden, 33-enums-golden, 34-entities-golden, 35-dialogue-golden, 36-closures-golden]

# Tech tracking
tech-stack:
  added: [similar v2.7.0 (diff library), tempfile v3 (temp dir for bless test)]
  patterns:
    - Round-trip isolation: compile -> Vec<u8> -> Module::from_bytes -> disassemble (never share in-memory module)
    - 16MB stack thread for compile pipeline (deep AST recursion safety)
    - BLESS=1 env var pattern for updating golden expected files

key-files:
  created:
    - writ-golden/Cargo.toml
    - writ-golden/tests/golden_tests.rs
  modified:
    - Cargo.toml (added writ-golden to workspace members)
    - Cargo.lock (similar, tempfile resolved)

key-decisions:
  - "Round-trip isolation: compile_and_disassemble goes through Module::from_bytes, not in-memory compiler state"
  - "16MB stack thread in compile_and_disassemble matches writ-cli cmd_compile pattern"
  - "bless_golden() exposed as pub(crate) for testability with temp dirs instead of env var manipulation"
  - "similar crate (already in Cargo.lock) used for TextDiff::from_lines unified diff"
  - "tempfile crate added as dev-dependency for test_bless_writes_file isolation"

patterns-established:
  - "Golden test pattern: .writ source + .expected IL text in writ-golden/tests/golden/"
  - "BLESS=1 cargo test -p writ-golden to update all expected files at once"
  - "run_golden_test(name) as the single entry point for each named golden fixture"

requirements-completed: [GOLD-01]

# Metrics
duration: 8min
completed: 2026-03-04
---

# Phase 31 Plan 01: Test Harness and Functions Golden Test Summary

**writ-golden workspace crate with 16MB-stack compile->bytes->Module::from_bytes->disassemble round-trip harness, BLESS=1 bless workflow, and unified diff on mismatch**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-04T00:00:00Z
- **Completed:** 2026-03-04T00:08:00Z
- **Tasks:** 1 of 1
- **Files modified:** 4

## Accomplishments

- Created `writ-golden` workspace crate registered in root `Cargo.toml`
- Implemented `compile_and_disassemble()` with 16MB stack thread and full round-trip isolation via `Module::from_bytes`
- Implemented `run_golden_test()` with BLESS=1 bless workflow and unified diff panic on mismatch
- Exposed `bless_golden()` helper for testable bless path without env var manipulation
- All 3 scaffold tests pass: `test_harness_pass`, `test_harness_fail_shows_diff`, `test_bless_writes_file`

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-golden crate with compile-disassemble-compare harness** - `5d029ef` (feat)

**Plan metadata:** (created below)

## Files Created/Modified

- `writ-golden/Cargo.toml` - Package manifest with writ-compiler, writ-assembler, writ-module, writ-diagnostics, writ-parser, similar, and tempfile dependencies
- `writ-golden/tests/golden_tests.rs` - Full harness: compile_and_disassemble, run_golden_test, bless_golden, and 3 scaffold tests
- `Cargo.toml` - Added "writ-golden" to workspace members array
- `Cargo.lock` - Resolved similar v2.7.0 and tempfile v3 (both were already in lock file)

## Decisions Made

- `bless_golden(name, actual, golden_dir)` exposed as `pub(crate)` function instead of inlining bless logic in `run_golden_test` — allows `test_bless_writes_file` to verify the bless path with a temp dir without touching env vars (env var mutation is not thread-safe in multi-threaded test runners)
- `similar` crate already present in Cargo.lock (pulled in by another dep) — used `TextDiff::from_lines` for line-level unified diff
- `tempfile` added as `[dev-dependencies]` only (not needed at runtime)

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Golden test harness is complete and all scaffold tests pass
- `writ-golden/tests/golden/` directory exists, ready to receive `.writ` + `.expected` file pairs
- Phase 31 plan 02 can add function IL golden fixtures (`fn_basic_call`, `fn_typed_params`, `fn_recursion`) using `run_golden_test(name)` as the entry point

---
*Phase: 31-test-harness-and-functions-golden-test*
*Completed: 2026-03-04*
