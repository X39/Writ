---
phase: 84-type-system
plan: 02
subsystem: writ-compiler/check
tags: [type-system, contracts, method-resolution, TyKind]
dependency_graph:
  requires: [84-01]
  provides: [contract-method-resolution, return-position-contract-assignability]
  affects: [writ-compiler/check]
tech_stack:
  added: []
  patterns: [contract-method-dispatch, directional-assignability]
key_files:
  created: []
  modified:
    - writ-compiler/src/check/check_expr/access.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/tests/typecheck_tests.rs
decisions:
  - "Contract method resolution looks up contract_methods by DefId — no impl_index needed for the dispatch itself"
  - "Return-position contract assignability mirrors let-binding logic from Plan 01 (same directional pattern)"
  - "The two Plan 01 #[ignore] tests now pass without modification — method resolution covers both param and return positions"
metrics:
  duration: "10 minutes"
  completed: "2026-03-23T23:12:36Z"
  tasks_completed: 2
  files_modified: 3
  tests_added: 3
  tests_passing: 88
requirements_satisfied: [TYPE-05]
---

# Phase 84 Plan 02: Contract Method Resolution on Receivers Summary

Method calls on contract-typed receivers now resolve through `contract_methods` lookup; return-position contract assignability added to `check_stmt.rs`; all Plan 01 ignored tests un-ignored and passing.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add TyKind::Contract arm to check_member_access | 0ec6c80 | check_expr/access.rs |
| 2 | Tests for contract method resolution on receivers | cefc202 | typecheck_tests.rs, check_stmt.rs |

## What Was Built

**TyKind::Contract arm in check_member_access** (`access.rs` line 90):
- Looks up `ctx.type_env.contract_methods.get(&contract_def_id)` for the method by name
- Builds a `Func` type from `method_sig.params` and `method_sig.ret` (self param excluded since it's implicit)
- Returns `TypedExpr::Field { ty: fn_ty, ... }` on success — same pattern as Struct/Class method resolution
- Emits `TypeError::UnknownField` (E0106) with the contract name when method is not found

**Contract-assignability in return position** (`check_stmt.rs` AstStmt::Return branch):
- When `ret_ty` is `TyKind::Contract`, performs the same directional assignability check as let-bindings
- Checks concrete class/struct/entity implements the contract via `impl_index`
- Emits `TypeError::MissingContractImpl` (E0112) if not satisfied
- Falls through to normal `unify()` for non-contract return types

## Tests

Three new tests added to `typecheck_tests.rs`:
1. `test_contract_method_call_on_receiver` — `let m: Movable = new Cat{}; m.move_(); let s: int = m.speed();` — no errors
2. `test_contract_method_call_unknown_method` — `m.fly()` on a Movable — expects E0106
3. `test_contract_method_call_with_args` — `a.add(5)` on an Adder — no errors, args type-check

Two previously-ignored tests from Plan 01 un-ignored:
4. `test_contract_as_param_type` — `fn greet(s: Speakable) -> string { return s.speak(); }` — now passes
5. `test_contract_as_return_type` — `fn make_runnable() -> Runnable { return new Task_{}; }` — now passes

**Result: 88 tests pass, 0 ignored.**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Return-position contract assignability not handled**
- **Found during:** Task 2 — `test_contract_as_return_type` (un-ignored from Plan 01) failed with E0100 TypeMismatch
- **Issue:** `check_stmt.rs` AstStmt::Return used plain `unify()` which fails for concrete→contract assignments (same issue Plan 01 fixed for let-bindings)
- **Fix:** Added contract-assignability check in the Return branch mirroring the let-binding logic: when `ret_ty` is `TyKind::Contract`, check via `impl_index` instead of unification
- **Files modified:** `writ-compiler/src/check/check_stmt.rs`
- **Commit:** cefc202

## Decisions Made

1. **contract_methods lookup only** — contract method resolution doesn't need impl_index; it reads directly from the contract definition's method signatures. The actual dispatch (CALL_VIRT) is Phase 85's concern.
2. **Return-position contract assignability** — the same directional-assignability pattern from let-bindings applies to return statements. Both are now handled.
3. **check_bracket_access no change needed** — contract types fall through to the `_` wildcard arm which emits a TypeMismatch error. No explicit Contract arm needed there.
4. **call.rs no change needed** — call mechanics work through the TypedExpr::Field + TypedExpr::Call pattern already established.

## Known Stubs

None — all features of this plan are fully implemented. Phase 85 will handle CALL_VIRT emission for the runtime dispatch.

## Self-Check

### Files Exist Check
- writ-compiler/src/check/check_expr/access.rs — exists
- writ-compiler/src/check/check_stmt.rs — exists
- writ-compiler/tests/typecheck_tests.rs — exists

### Commits Exist Check
- 0ec6c80 — feat(84-02): add TyKind::Contract arm to check_member_access
- cefc202 — test(84-02): add contract method resolution tests and fix return-position contracts

## Self-Check: PASSED
