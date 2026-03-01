---
phase: 30-critical-bug-fixes
plan: "02"
subsystem: emit/body
tags: [bug-fix, codegen, call-dispatch, register-allocation, dead-code]
dependency_graph:
  requires: [30-01]
  provides: [correct-call-dispatch, correct-return-register, clean-arg-packing]
  affects: [emit/body/expr.rs, emit/body/call.rs, emit/body/mod.rs, emit/collect.rs, emit/serialize.rs]
tech_stack:
  added: []
  patterns: [shared-result-register, pack-args-consecutive, extern-token-check]
key_files:
  created: []
  modified:
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/body/call.rs
    - writ-compiler/src/emit/body/closure.rs
    - writ-compiler/src/emit/body/patterns.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs
    - writ-compiler/src/emit/mod.rs
decisions:
  - "emit_if allocates a shared r_result register and MOVs both then/else branches into it (BUG-04); simpler than phi-node approach and sufficient for correctness"
  - "pack_args_consecutive() centralized in call.rs as the canonical packing helper; all 7 sites replaced (BUG-06)"
  - "ExternDef check added directly in emit_expr TypedExpr::Call _ arm, not via analyze_callee() which is separate from main emit path (BUG-05)"
metrics:
  duration: "25 minutes"
  completed_date: "2026-03-04T13:48:49Z"
  tasks_completed: 3
  files_modified: 10
---

# Phase 30 Plan 02: Call Dispatch and Return Register Bug Fixes Summary

Fix call dispatch (BUG-03/BUG-05), return register wiring (BUG-04), and phantom MOV elimination (BUG-06) so all call instructions carry correct tokens, extern functions use CALL_EXTERN, returns reference actual computed values, and argument setup never emits MOV from uninitialized registers.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Fix extern call dispatch and CALL_EXTERN emission | d5d44fe | expr.rs, serialize.rs, mod.rs, emit_body_tests.rs, emit_serialize_tests.rs |
| 2 | Fix phantom MOV elimination in argument packing | 26b0ef3 | call.rs, expr.rs |
| 3 | Fix return register wiring and remove dead code | bb289e6 | expr.rs, closure.rs, patterns.rs, stmt.rs, collect.rs, serialize.rs, emit_body_tests.rs |

## What Was Built

### BUG-05: Extern Call Dispatch (Task 1)

The `TypedExpr::Call` arm in `emit_expr` had an inline dispatch that defaulted to `CallKind::Direct` for all non-Field callees. It never checked whether `callee_def_id` resolved to an `ExternDef` token.

**Fix:** Added an `is_extern` check in the `_ =>` arm of the callee kind dispatch:
```rust
let is_extern = maybe_def_id
    .and_then(|id| emitter.builder.token_for_def(id))
    .map(|t| t.table() == TableId::ExternDef)
    .unwrap_or(false);
```
When true, the call emits `CallKind::Extern` which routes to `Instruction::CallExtern`.

Also removed `extract_callee_def_id_opt` (dead code that always returned `None`).

### BUG-06: Phantom MOV Elimination (Task 2)

All 7 argument packing sites independently allocated a new consecutive register block starting at `regs.next()`, which is always past all emitted arg registers. This caused `arg_reg != slot_reg` to always be true, emitting a MOV for every argument even when args were already consecutive.

**Fix:** Added `pack_args_consecutive()` in `call.rs`:
- If `arg_regs == [N, N+1, N+2, ...]`, returns `N` with no MOV emitted
- Otherwise allocates a new block and emits MOVs only as needed
- All 7 sites (`emit_call`, `emit_call_indirect`, inline Call arm, `emit_tail_call`, `emit_spawn`, `emit_array_lit`, `emit_str_build`) now use this helper

### BUG-04: Return Register Wiring (Task 3)

`emit_if` returned `r_then` (the then-branch result register), discarding `r_else`. When the runtime takes the else branch, `r_then` is uninitialized, making the `RET` instruction reference an unwritten register.

**Fix:** `emit_if` now allocates a shared `r_result` register before emitting branches. Both branches MOV their results into `r_result`. The caller receives `r_result` which is valid on all paths.

### Dead Code Cleanup (Task 3)

Removed all dead functions and unused imports that generated 13 compiler warnings:
- `extract_callee_def_id_opt` in `expr.rs` (always returned `None`)
- `convert_token` in `serialize.rs` (never called)
- `encode_ty` and `resolve_type` in `collect.rs` (never called)
- Unused imports: `Ty`, `super::type_sig` from `collect.rs`; `emit_expr` from `closure.rs`; inline `TypedStmt` use in `closure.rs`
- Unused variables: `ty` in `SelfRef`/`Block`/`emit_if`/`Let`; `bool_ty` in `patterns.rs`

Also fixed pre-existing blocking bug from 30-01 working tree changes: `serialize::translate` signature changed to `&mut ModuleBuilder` but `serialize::serialize` and test callers still passed `&ModuleBuilder`. Updated all call sites.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed serialize/translate signature mismatch**
- **Found during:** Task 1 (first build attempt)
- **Issue:** `serialize.rs` had `translate(builder: &mut ModuleBuilder, ...)` but `serialize(builder: &ModuleBuilder, ...)` passed immutable ref; blocked compilation
- **Fix:** Updated `serialize()` signature to `&mut ModuleBuilder` and updated callers in `emit/mod.rs` and `emit_serialize_tests.rs`
- **Files modified:** `writ-compiler/src/emit/serialize.rs`, `writ-compiler/src/emit/mod.rs`, `writ-compiler/tests/emit_serialize_tests.rs`
- **Commit:** d5d44fe

**2. [Rule 1 - Bug] Updated test_emit_if_else for BUG-04 fix**
- **Found during:** Task 3 (test run after emit_if fix)
- **Issue:** Test expected old sequence `[LoadTrue, BrFalse, LoadInt(1), Br, LoadInt(2)]` but BUG-04 fix adds shared Mov instructions
- **Fix:** Updated test to verify new sequence `[LoadTrue, BrFalse, LoadInt(1), Mov(r_result), Br, LoadInt(2), Mov(r_result)]`
- **Files modified:** `writ-compiler/tests/emit_body_tests.rs`
- **Commit:** bb289e6

## Tests Added

| Test | What It Verifies |
|------|-----------------|
| `test_emit_expr_extern_call_emits_call_extern` | BUG-05: emit_expr emits CALL_EXTERN when callee_def_id is ExternDef token |
| `test_emit_expr_non_extern_call_emits_call` | BUG-05 negative: regular fn callee_def_id still produces CALL |
| `test_emit_if_else` (updated) | BUG-04: both branches MOV into shared r_result register |

## Self-Check: PASSED

- SUMMARY.md: FOUND
- Commit d5d44fe (Task 1 - extern dispatch): FOUND
- Commit 26b0ef3 (Task 2 - pack_args_consecutive): FOUND
- Commit bb289e6 (Task 3 - return register + dead code): FOUND
- Key files: expr.rs, call.rs, mod.rs, collect.rs all present
