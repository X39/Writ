---
phase: quick-260319-fxm
plan: 01
subsystem: compiler, lsp, golden-tests
tags: [intrinsic-methods, type-checker, result, option, lsp-completions, golden-tests]
dependency_graph:
  requires: []
  provides: [Option/Result intrinsic method end-to-end pipeline]
  affects: [writ-compiler, writ-lsp, writ-golden]
tech_stack:
  added: []
  patterns: [sub-prelude builtin injection, intrinsic method resolution in check_member_access]
key_files:
  created:
    - writ-golden/tests/golden/option_methods.writ
    - writ-golden/tests/golden/option_methods.writil
    - writ-golden/tests/golden/result_methods.writ
    - writ-golden/tests/golden/result_methods.writil
  modified:
    - writ-compiler/src/check/check_expr/access.rs
    - writ-compiler/src/check/check_expr/ident.rs
    - writ-compiler/src/emit/body/expr/builtins.rs
    - writ-lsp/src/queries/completion.rs
    - writ-golden/tests/golden_tests.rs
decisions:
  - "Ok/Err added as sub-prelude builtin constructors in check_ident alongside None/Some — returns Func type (fn(T)->Result<T,E>) enabling type-guided emitter dispatch"
  - "unwrap alias added to emitter Result arm so user-facing name matches spec (unwrap not unwrap_ok)"
metrics:
  duration: ~20 minutes
  completed: 2026-03-19T10:38:00Z
  tasks_completed: 2
  files_modified: 5
  files_created: 4
---

# Phase quick-260319-fxm Plan 01: Implement Intrinsic Methods on Result<T,E> Summary

**One-liner:** End-to-end Option/Result intrinsic method pipeline: type checker resolution + Ok/Err constructor injection + emitter spec alias + LSP completions + golden tests with IS_SOME/IS_NONE/UNWRAP/IS_OK/IS_ERR/UNWRAP_OK/EXTRACT_ERR IL verification.

## Tasks Completed

| # | Name | Commit | Files |
|---|------|--------|-------|
| 1 | Add intrinsic method resolution in type checker + fix emitter spec compliance | e0a63f2 | access.rs, builtins.rs |
| 2 | Add LSP completions for Result + golden tests for Option and Result methods | 2ac2081 | completion.rs, ident.rs, golden_tests.rs, 4 golden files |

## What Was Built

### Task 1: Type Checker + Emitter

**`writ-compiler/src/check/check_expr/access.rs`** — Added two new match arms to `check_member_access` before the `_ =>` catch-all:

- `TyKind::Option(inner_ty)`: resolves `is_some`/`is_none` to `fn()->bool`, `unwrap` to `fn()->T`
- `TyKind::Result(ok_ty, err_ty)`: resolves `is_ok`/`is_err` to `fn()->bool`, `unwrap` to `fn()->T`, `unwrap_err` to `fn()->E`

Unknown fields on these types still fall through to `UnknownField` error.

**`writ-compiler/src/emit/body/expr/builtins.rs`** — Added `"unwrap"` alias alongside `"unwrap_ok"` in the `TyKind::Result` emitter arm so the user-facing spec name (`unwrap`) emits `Instruction::UnwrapOk`.

### Task 2: LSP + Golden Tests + Ok/Err Constructor Injection

**`writ-lsp/src/queries/completion.rs`** — Added `TyKind::Result(_, _)` arm with completions for `is_ok`, `is_err`, `unwrap`, `unwrap_err`.

**`writ-compiler/src/check/check_expr/ident.rs`** — Added `Ok` and `Err` as sub-prelude builtin constructors (Rule 2 auto-fix: missing critical functionality). `Ok` resolves to `fn(T)->Result<T,E>`, `Err` resolves to `fn(E)->Result<T,E>`. This was required because `Ok(42)` in the test source compiled to "undefined variable `Ok`" — the emitter already had WrapOk/WrapErr support but the type checker had no way to resolve the bare constructors.

**Golden test files:**
- `option_methods.writ` / `option_methods.writil`: IS_SOME, IS_NONE, UNWRAP instructions verified
- `result_methods.writ` / `result_methods.writil`: IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR, WRAP_OK, WRAP_ERR instructions verified

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] Added Ok/Err as sub-prelude builtin constructors**
- **Found during:** Task 2 (bless phase) — `test_result_methods` failed with "undefined variable `Ok`; undefined variable `Err`"
- **Issue:** The result_methods.writ golden test uses bare `Ok(42)` and `Err("bad")` constructors. The emitter had WrapOk/WrapErr support but the type checker had no resolution path for bare `Ok`/`Err` identifiers. Only `None` and `Some` were in the sub-prelude builtin list.
- **Fix:** Added `Ok` and `Err` cases to `check_ident` alongside `None`/`Some`, returning `fn(T)->Result<T,E>` and `fn(E)->Result<T,E>` function types with fresh inference variables for T and E.
- **Files modified:** `writ-compiler/src/check/check_expr/ident.rs`
- **Commit:** 2ac2081

## Pre-existing Failures (Out of Scope)

17 golden tests were already failing before this task due to "; line:X col:Y" comments being added to CALL instructions in the disassembler output, which the existing blessed .writil files don't include. These are pre-existing regressions confirmed by stashing changes and verifying they fail identically on the committed e0a63f2 state.

The LSP integration test suite (`test_hover_protocol`) has pre-existing compilation failures due to missing tokio feature flags (tokio::time, AsyncReadExt/AsyncWriteExt). All 98 LSP library unit tests pass.

## Verification

```
cargo test -p writ-golden -- test_option_methods test_result_methods
# 2 passed, 0 failed

cargo test -p writ-lsp --lib
# 98 passed, 0 failed

cargo build -p writ-compiler
# Finished dev profile
```

IL output confirmed:
- `option_methods.writil`: WRAP_SOME, IS_SOME, IS_NONE, UNWRAP
- `result_methods.writil`: WRAP_OK, IS_OK, IS_ERR, UNWRAP_OK, WRAP_ERR, EXTRACT_ERR

## Self-Check: PASSED

Files exist:
- writ-compiler/src/check/check_expr/access.rs: FOUND
- writ-compiler/src/check/check_expr/ident.rs: FOUND
- writ-compiler/src/emit/body/expr/builtins.rs: FOUND
- writ-lsp/src/queries/completion.rs: FOUND
- writ-golden/tests/golden/option_methods.writ: FOUND
- writ-golden/tests/golden/option_methods.writil: FOUND
- writ-golden/tests/golden/result_methods.writ: FOUND
- writ-golden/tests/golden/result_methods.writil: FOUND

Commits exist:
- e0a63f2: FOUND (Task 1)
- 2ac2081: FOUND (Task 2)
