---
phase: 06-pipeline-integration-and-snapshot-testing
plan: 01
subsystem: testing
tags: [rust, insta, snapshot-testing, lowering, determinism]

# Dependency graph
requires:
  - phase: 05-entity-lowering
    provides: all lowering passes implemented (lower_fn, lower_dialogue, lower_entity, lower_operator_impls, expression helpers)
provides:
  - R15 acceptance criteria met: every lowering pass has snapshot coverage
  - integration snapshot exercising fn + operator impl + dlg + entity end-to-end
  - determinism assertion proving FNV-1a localization keys are not pointer-dependent
  - pass-through item coverage: struct, enum, contract, component, extern, const, global, namespace, using
affects: [v1.0-milestone-completion]

# Tech tracking
tech-stack:
  added: []
  patterns: [insta assert_debug_snapshot for all tests, INSTA_UPDATE=always for snapshot acceptance, no separate cargo insta accept step needed]

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__fn_basic_with_params_and_return.snap
    - writ-compiler/tests/snapshots/lowering_tests__integration_all_constructs.snap
    - writ-compiler/tests/snapshots/lowering_tests__passthrough_struct_and_enum.snap
    - writ-compiler/tests/snapshots/lowering_tests__passthrough_contract_and_component.snap
    - writ-compiler/tests/snapshots/lowering_tests__passthrough_extern_const_global.snap
    - writ-compiler/tests/snapshots/lowering_tests__passthrough_namespace_and_using.snap
  modified:
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "global mut required: parser grammar is 'global mut name: type = expr;' not 'global name: type = expr;'; test source corrected during execution"
  - "Determinism test uses direct lower() calls instead of lower_src helper to compare two independent run outputs"
  - "pass ordering doc comment verified accurate — all five expression helpers and three structural passes documented; no drift found"

patterns-established:
  - "Direct lower() calls for multi-run comparison tests that lower_src helper cannot support"
  - "Integration test exercises all major construct types in a single program to catch cross-pass interactions"

requirements-completed: [R15]

# Metrics
duration: 15min
completed: 2026-02-27
---

# Phase 6 Plan 01: Pipeline Integration and Snapshot Testing Summary

**7 new snapshot tests close R15 quality gate: lower_fn, all pass-through items, full fn+operator+dlg+entity integration, and FNV-1a determinism assertion; 69 compiler tests pass stably**

## Performance

- **Duration:** 15 min
- **Started:** 2026-02-27T09:10:50Z
- **Completed:** 2026-02-27T09:25:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Added `fn_basic_with_params_and_return` snapshot directly exercising `lower_fn` with typed params and return type
- Added 4 pass-through snapshot tests covering every previously untested pass: struct/enum, contract/component, extern/const/global, namespace/using
- Added `integration_all_constructs` snapshot that lowers a full Writ program (fn + operator impl + dlg + entity) through all passes in one assertion
- Added `localization_keys_are_deterministic` test that runs lowering twice on identical source and asserts `format!("{ast:?}")` equality
- Verified `lower/mod.rs` pass ordering doc comment is accurate — all five expression helpers and three structural passes listed with correct rationale
- All 177 parser tests and 69 compiler tests pass with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: lower_fn, pass-through items, integration snapshot** - `7b33e4c` (feat)
2. **Task 2: determinism test and pass ordering doc verification** - `dc7f525` (feat)

## Files Created/Modified
- `writ-compiler/tests/lowering_tests.rs` - Added 7 new snapshot tests in 3 new sections
- `writ-compiler/tests/snapshots/lowering_tests__fn_basic_with_params_and_return.snap` - lower_fn direct snapshot
- `writ-compiler/tests/snapshots/lowering_tests__integration_all_constructs.snap` - Full pipeline integration snapshot
- `writ-compiler/tests/snapshots/lowering_tests__passthrough_struct_and_enum.snap` - struct + enum pass-through
- `writ-compiler/tests/snapshots/lowering_tests__passthrough_contract_and_component.snap` - contract + component pass-through
- `writ-compiler/tests/snapshots/lowering_tests__passthrough_extern_const_global.snap` - extern + const + global pass-through
- `writ-compiler/tests/snapshots/lowering_tests__passthrough_namespace_and_using.snap` - namespace + using pass-through

## Decisions Made
- Corrected `global` syntax to `global mut`: the Writ parser grammar is `global mut name: type = expr;`; the plan source string used `global score: int = 0;` which produced a parse error; fixed inline (Rule 1 auto-fix)
- Determinism test calls `writ_parser::parse` and `lower()` directly rather than `lower_src` helper to obtain two independent Ast values for comparison
- Pass ordering doc comment in `lower/mod.rs` verified accurate; no code change made

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed global declaration syntax in passthrough_extern_const_global test**
- **Found during:** Task 1 (passthrough_extern_const_global test)
- **Issue:** Test source used `global score: int = 0;` but parser grammar requires `global mut name: type = expr;`; parse failed with "found 'Ident(\"score\")' at 64..69 expected 'KwMut'"
- **Fix:** Changed source to `global mut score: int = 0;`
- **Files modified:** writ-compiler/tests/lowering_tests.rs
- **Verification:** Test now parses and snapshot accepted; stable on second run
- **Committed in:** 7b33e4c (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — incorrect parser syntax in plan source)
**Impact on plan:** Single source string correction; no scope creep; all R15 criteria met as planned.

## Issues Encountered
None beyond the auto-fixed global syntax issue above.

## Next Phase Readiness
- R15 acceptance criteria fully closed: every lowering pass documented with snapshot, integration snapshot covers all construct types, determinism proven
- Milestone v1.0 (CST-to-AST Lowering Pipeline) is complete: all 6 phases executed, all 12 plans done (including gap-closure 05-03)
- No blockers for any future milestone work

---
*Phase: 06-pipeline-integration-and-snapshot-testing*
*Completed: 2026-02-27*
