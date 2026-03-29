---
phase: 109-type-12-closure-capture-fix
plan: "01"
subsystem: writ-compiler
tags: [closure, capture, lambda, type-checker, codegen]
dependency_graph:
  requires: []
  provides: [TYPE-12-closure-capture]
  affects: [writ-golden]
tech_stack:
  added: []
  patterns: [post-hoc-capture-analysis, collect-var-refs-walker, lambda-node-collection]
key_files:
  created:
    - writ-golden/tests/golden/closure_capture.writ
    - writ-golden/tests/golden/closure_capture.writil
  modified:
    - writ-compiler/src/check/check_expr/lambda.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-golden/tests/golden_tests.rs
    - writ-golden/tests/golden/fn_log_say_choice.writil
decisions:
  - "Used post-hoc capture analysis (walk typed body after pop_scope) rather than scope-depth tracking to keep LocalEnv simple"
  - "Collect whole Lambda node (not just body) in collect_lambda_exprs_from_* so params are accessible during body emission"
  - "Nested lambdas are excluded from outer capture walks; each lambda's captures are handled when check_lambda is called recursively"
  - "Void-return lambda bodies now emit RetVoid instead of Ret r_src (correctness fix discovered during Task 2)"
metrics:
  duration: "~25 minutes"
  completed: "2026-03-28"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 4
  files_created: 2
---

# Phase 109 Plan 01: TYPE-12 Closure Capture Fix Summary

End-to-end closure capture: type checker now populates `Vec<Capture>` from outer-scope variable references, and the lambda body emitter loads captured variables from the capture struct via `GET_FIELD`.

## What Was Built

### Task 1: Capture Analysis in Type Checker (`check_expr/lambda.rs`)

Replaced the hardcoded `let captures = Vec::new()` with a post-hoc analysis pass:

1. After type-checking the lambda body and calling `pop_scope()` (which removes the lambda's params from `local_env`), walk the typed body using a new `collect_var_refs` function.
2. For each `TypedExpr::Var { name, .. }` found: skip if it's a lambda param (via `param_set`), skip if already seen (dedup), look up in `ctx.local_env` — if found, add a `Capture { name, ty, mode: ByValue }`.
3. Variables not in `local_env` (globals, consts, functions) are correctly excluded.
4. Nested lambdas are not walked — inner captures are handled when `check_lambda` is called recursively.

Added `collect_var_refs` and `collect_var_refs_stmt` helper functions covering all `TypedExpr` and `TypedStmt` variants.

### Task 2: Capture-Aware Lambda Body Emission (`emit/body/mod.rs`)

Two changes to the lambda emission infrastructure:

**Collect whole Lambda nodes:** Renamed `collect_lambda_bodies_from_*` to `collect_lambda_exprs_from_*` and changed the Lambda arm to push `expr` (the whole Lambda node) rather than `body.as_ref()`. This gives the emission loop access to params, captures, and body.

**Capture loading:** In the emission loop, for each lambda with non-empty `captures_info`:
- Allocate r0 as the capture struct reference (the delegate target passed by the VM)
- For each captured variable, emit `GET_FIELD r_cap, r0, field_idx` and insert into `emitter.locals`
- Register lambda params after captures so captures get lower register indices

**Void-return fix:** Lambda bodies now emit `RetVoid` when the body type is `Ty(4)` (void), rather than always emitting `Ret { r_src }`. This fixed a latent bug in void-return lambdas (previously emitting `RET r0` which is undefined behavior).

### Task 2: Golden Test (`closure_capture.writ`)

```writ
pub fn main() -> int {
    let x: int = 42;
    let f: fn() -> int = fn() -> int { x };
    f()
}
```

The blessed `.writil` snapshot confirms the full capture pipeline:
- `__closure_0` TypeDef with field `x`
- `GET_FIELD r1, r0, 0` in `__invoke_0` body (load capture from struct)
- `NEW r2, ...` + `SET_FIELD r2, 0, r0` in `main` (allocate + populate capture struct)
- `NEW_DELEGATE r1, ..., r2` (bind capture struct to delegate)
- `CALL_INDIRECT r3, r1, r4, 0` (invoke the delegate)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed void-return lambda emitting `RET r_src` instead of `RET_VOID`**
- **Found during:** Task 2 — `test_fn_log_say_choice` golden test failed after emission changes
- **Issue:** Previously the lambda body loop always emitted `Ret { r_src: r }`. When the lambda returns void (`fn() -> void { ... }`), this emits `RET r0` (some stale/wrong register) instead of `RET_VOID`
- **Fix:** Added `if lambda_body.ty() == Ty(4) { emit RetVoid } else { emit Ret { r_src } }`
- **Files modified:** `writ-compiler/src/emit/body/mod.rs`
- **Snapshot:** `writ-golden/tests/golden/fn_log_say_choice.writil` re-blessed

## Test Results

- `cargo test -p writ-compiler`: 95 passed, 0 failed
- `cargo test -p writ-golden`: 55 passed, 0 failed (including new `test_closure_capture`)
- `cargo test -p writ-runtime`: 90 passed, 0 failed

## Commits

- `4d5d13d`: feat(109-01): implement closure capture analysis in type checker
- `d0be7d1`: feat(109-01): implement capture-aware lambda body emission and golden test

## Self-Check: PASSED

- `writ-compiler/src/check/check_expr/lambda.rs`: modified (collect_var_refs added)
- `writ-compiler/src/emit/body/mod.rs`: modified (collect_lambda_exprs, capture loading)
- `writ-golden/tests/golden/closure_capture.writ`: created
- `writ-golden/tests/golden/closure_capture.writil`: created (blessed snapshot)
- Commit `4d5d13d`: exists
- Commit `d0be7d1`: exists
