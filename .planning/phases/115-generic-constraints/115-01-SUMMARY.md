---
phase: 115-generic-constraints
plan: "01"
subsystem: writ-compiler/check
tags: [type-checker, generics, bounds, diagnostics, tdd]
dependency_graph:
  requires: []
  provides: [generic-bound-enforcement, unsatisfied-bound-secondary-label]
  affects: [writ-compiler/src/check/env.rs, writ-compiler/src/check/error.rs, writ-compiler/src/check/check_expr.rs]
tech_stack:
  added: []
  patterns: [secondary-label-diagnostics, span-threading-through-FnSig]
key_files:
  created: []
  modified:
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/error.rs
    - writ-compiler/src/check/check_expr.rs
    - writ-compiler/tests/typecheck_tests.rs
decisions:
  - "Used EqBound/OrdBound contract names in tests to avoid shadowing prelude Eq contract"
  - "Used 'new Foo { x: 1 }' struct construction syntax (not 'Foo(x: 1)') in test fixtures"
  - "bound_decl_spans parallel to bounds (per-generic-param span, not per-bound) for simplicity"
  - "Contract methods in build_contract_methods use bound_decl_spans: Vec::new() (no generic params on contract methods)"
metrics:
  duration: "~11 minutes"
  completed: "2026-03-29"
  tasks_completed: 2
  files_modified: 4
---

# Phase 115 Plan 01: Generic Constraint Bound Enforcement Summary

Wire the type checker to enforce generic contract bounds at call sites with multi-span diagnostics: `FnSig` now carries `bound_decl_spans` and `fn_file`; `UnsatisfiedBound` errors include a secondary label pointing to the bound declaration; all 6 generic constraint tests pass.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Write failing and passing typecheck tests for generic bounds (TDD RED/GREEN) | 556899f | writ-compiler/tests/typecheck_tests.rs |
| 2 | Add bound declaration spans to FnSig and UnsatisfiedBound, wire secondary label | 778ffd2 | env.rs, error.rs, check_expr.rs |

## What Was Built

**Task 1 (TDD tests):** Added 6 tests to `typecheck_tests.rs`:
- `generic_bound_not_satisfied_emits_e0103` — struct without impl emits E0103
- `generic_single_bound_satisfied` — struct with impl produces no error
- `generic_multi_bound_both_satisfied` — both contracts implemented, no error
- `generic_multi_bound_missing_one_emits_e0103` — only one of two contracts implemented, E0103
- `generic_bound_error_has_secondary_label` — E0103 has secondary label (was RED state, now GREEN)
- `generic_bound_error_has_help_suggestion` — E0103 help text includes "consider adding `impl"

**Task 2 (Implementation):**
- `FnSig` struct: added `bound_decl_spans: Vec<SimpleSpan>` (parallel to `bounds`) and `fn_file: FileId`
- `build_fn_sig` and `build_fn_sig_from_ast_sig`: populate `bound_decl_spans` from `fn_decl.generics.iter().map(|gp| gp.span)` and `fn_file: entry.file_id`
- `build_contract_methods` and `build_impl_entry`: updated all 4 FnSig constructors
- `UnsatisfiedBound` error variant: added `bound_decl_span: SimpleSpan` and `bound_decl_file: FileId`
- `From<TypeError> for Diagnostic` arm: added `.with_secondary(bound_decl_file, bound_decl_span, ...)` call
- `check_contract_bounds`: threads `sig.bound_decl_spans[i]` and `sig.fn_file` into `UnsatisfiedBound`

## Deviations from Plan

**[Rule 1 - Bug] Test fixture syntax corrections**
- **Found during:** Task 1
- **Issue:** Plan used `other: self` as parameter type in contract methods (not valid syntax — `self` is a keyword, not a type). Also `Foo(x: 1)` struct construction is not valid (must be `new Foo { x: 1 }`). Also `Eq`/`Ord` are prelude names that cannot be shadowed (E0002).
- **Fix:** Changed contract methods to use `fn equals() -> bool` (no params), renamed contracts to `EqBound`/`OrdBound`, changed construction to `new Foo { x: 1 }` syntax
- **Files modified:** writ-compiler/tests/typecheck_tests.rs
- **Commit:** 556899f

## Test Results

- `cargo test -p writ-compiler -- generic`: 9 passed, 0 failed (includes all 6 new tests)
- `cargo test -p writ-compiler`: 67 passed, 0 failed (no regressions)

## Requirements Satisfied

- GEN-01: `generic_single_bound_satisfied` passes — single bound is enforced
- GEN-02: `generic_multi_bound_both_satisfied` passes — multi-bounds work
- GEN-03: `generic_bound_not_satisfied_emits_e0103` and `generic_multi_bound_missing_one_emits_e0103` pass — violations detected
- GEN-05: `generic_bound_error_has_secondary_label` passes — error points to both call site and bound declaration
- GEN-06: `generic_bound_error_has_help_suggestion` passes — help text includes "consider adding `impl`"

## Known Stubs

None — all plan goals achieved.

## Self-Check: PASSED

- writ-compiler/src/check/env.rs: FOUND
- writ-compiler/src/check/error.rs: FOUND
- writ-compiler/src/check/check_expr.rs: FOUND
- writ-compiler/tests/typecheck_tests.rs: FOUND
- .planning/phases/115-generic-constraints/115-01-SUMMARY.md: FOUND
- Commit 556899f: FOUND
- Commit 778ffd2: FOUND
