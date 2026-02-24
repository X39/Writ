---
phase: 03-operator-and-concurrency-lowering
plan: 02
subsystem: testing
tags: [rust, insta, snapshot-testing, operator-lowering, concurrency]

# Dependency graph
requires:
  - phase: 03-operator-and-concurrency-lowering-01
    provides: lower_operator_impls in writ-compiler/src/lower/operator.rs
  - phase: 02-foundational-expression-lowering-02
    provides: concurrency pass-through in lower_expr (Spawn/Join/Cancel/Defer/Detached)
provides:
  - "R6 operator desugaring snapshot tests (10 tests)"
  - "R7 concurrency pass-through snapshot tests (5 tests)"
  - "Accepted insta snapshot files for all 15 new tests"
affects: [future phases consuming lowered AST, CI quality gate]

# Tech tracking
tech-stack:
  added: []
  patterns: [insta assert_debug_snapshot for AST lowering tests]

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__operator_binary_add_desugars_to_add_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_binary_sub_desugars_to_sub_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_unary_neg_desugars_to_neg_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_unary_not_desugars_to_not_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_eq_desugars_with_derived_ne.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_ord_desugars_with_derived_gt.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_eq_and_ord_derives_all_four.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_index_desugars_to_index_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__operator_index_set_desugars_to_index_mut_contract.snap
    - writ-compiler/tests/snapshots/lowering_tests__impl_mixed_fn_and_op_members.snap
    - writ-compiler/tests/snapshots/lowering_tests__concurrency_spawn_passthrough.snap
    - writ-compiler/tests/snapshots/lowering_tests__concurrency_join_passthrough.snap
    - writ-compiler/tests/snapshots/lowering_tests__concurrency_cancel_passthrough.snap
    - writ-compiler/tests/snapshots/lowering_tests__concurrency_defer_passthrough.snap
    - writ-compiler/tests/snapshots/lowering_tests__concurrency_detached_spawn_passthrough.snap
  modified:
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "INSTA_UPDATE=always used to auto-accept snapshots in one step rather than two-step cargo insta accept"
  - "R6 and R7 tests appended to existing lowering_tests.rs rather than separate test files"
  - "concurrency tests use typed fn params (h: Handle) to avoid parser requiring type annotation"

patterns-established:
  - "Snapshot test pattern: lower_src() -> insta::assert_debug_snapshot!(ast)"
  - "operator-only impl blocks produce no base AstDecl::Impl (verified by snapshot)"
  - "Sub disambiguation by param count: 0 params = Neg, 1 param = Sub"

requirements-completed: [R6, R7]

# Metrics
duration: 8min
completed: 2026-02-26
---

# Phase 3 Plan 02: Operator and Concurrency Lowering Snapshot Tests Summary

**15 snapshot tests lock down R6 operator desugaring (binary, unary, derived, index) and R7 concurrency pass-through (spawn, join, cancel, defer, detached), bringing total to 29 tests all green**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-26T19:10:48Z
- **Completed:** 2026-02-26T19:18:00Z
- **Tasks:** 2
- **Files modified:** 16 (1 test file + 15 snapshot files)

## Accomplishments
- 10 R6 operator lowering snapshot tests covering all OpSymbol variants and derived operator generation
- 5 R7 concurrency pass-through snapshot tests confirming 1:1 AstExpr mapping
- All snapshots accepted (no pending) - zero AstImplMember::Op in any snapshot output
- Eq+Ord combined test confirms exactly 6 impl blocks (Eq, Ord, Ne, Gt, LtEq, GtEq)
- spawn detached nesting verified: Spawn { expr: Detached { expr: Call } }

## Task Commits

Each task was committed atomically:

1. **Task 1: Add R6 operator lowering snapshot tests** - `6cea89b` (test)
2. **Task 2: Add R7 concurrency pass-through snapshot tests** - `dab11c9` (test)

## Files Created/Modified
- `writ-compiler/tests/lowering_tests.rs` - Added 15 new snapshot tests (10 R6 + 5 R7)
- `writ-compiler/tests/snapshots/lowering_tests__operator_binary_add_desugars_to_add_contract.snap` - Add contract desugar
- `writ-compiler/tests/snapshots/lowering_tests__operator_binary_sub_desugars_to_sub_contract.snap` - Sub contract desugar
- `writ-compiler/tests/snapshots/lowering_tests__operator_unary_neg_desugars_to_neg_contract.snap` - Neg (not Sub) for 0 params
- `writ-compiler/tests/snapshots/lowering_tests__operator_unary_not_desugars_to_not_contract.snap` - Not contract
- `writ-compiler/tests/snapshots/lowering_tests__operator_eq_desugars_with_derived_ne.snap` - Eq + derived Ne
- `writ-compiler/tests/snapshots/lowering_tests__operator_ord_desugars_with_derived_gt.snap` - Ord + derived Gt
- `writ-compiler/tests/snapshots/lowering_tests__operator_eq_and_ord_derives_all_four.snap` - 6 impls: Eq, Ord, Ne, Gt, LtEq, GtEq
- `writ-compiler/tests/snapshots/lowering_tests__operator_index_desugars_to_index_contract.snap` - Index contract
- `writ-compiler/tests/snapshots/lowering_tests__operator_index_set_desugars_to_index_mut_contract.snap` - IndexMut contract
- `writ-compiler/tests/snapshots/lowering_tests__impl_mixed_fn_and_op_members.snap` - Base impl + contract impl
- `writ-compiler/tests/snapshots/lowering_tests__concurrency_spawn_passthrough.snap` - spawn -> AstExpr::Spawn
- `writ-compiler/tests/snapshots/lowering_tests__concurrency_join_passthrough.snap` - join -> AstExpr::Join
- `writ-compiler/tests/snapshots/lowering_tests__concurrency_cancel_passthrough.snap` - cancel -> AstExpr::Cancel
- `writ-compiler/tests/snapshots/lowering_tests__concurrency_defer_passthrough.snap` - defer -> AstExpr::Defer
- `writ-compiler/tests/snapshots/lowering_tests__concurrency_detached_spawn_passthrough.snap` - Spawn { Detached { Call } }

## Decisions Made
- Used `INSTA_UPDATE=always` to auto-accept in one step rather than two-step `cargo insta accept`
- Typed fn params used for concurrency tests (e.g., `fn f(h: Handle)`) so parser has type annotations without needing let bindings
- R6 and R7 tests appended to existing `lowering_tests.rs` to keep all lowering tests in one file

## Deviations from Plan
None - plan executed exactly as written. All source strings parsed on first attempt with no parser investigation needed.

## Issues Encountered
None - all 10 operator test sources parsed correctly on first run. All 5 concurrency test sources parsed on first run.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 3 is complete: R6 (operator desugaring) and R7 (concurrency pass-through) both implemented and snapshot-tested
- 29 total tests all green as quality gate for the lowering pipeline
- The lowering pipeline is ready for Phase 4 if planned

---
*Phase: 03-operator-and-concurrency-lowering*
*Completed: 2026-02-26*
