---
phase: quick-260323-vkg
plan: 01
subsystem: writ-compiler/check
tags: [bug-fix, contracts, type-checking, dispatch]
dependency_graph:
  requires: []
  provides: [E0122-contract-as-type, E0123-incomplete-contract-impl, class-dispatch-fix]
  affects: [writ-compiler/check, writ-compiler/emit, writ-diagnostics]
tech_stack:
  added: []
  patterns: [validate-on-build, emit-error-at-call-site]
key_files:
  created: []
  modified:
    - writ-diagnostics/src/code.rs
    - writ-compiler/src/check/error.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/emit/body/call.rs
    - writ-compiler/tests/typecheck_tests.rs
decisions:
  - "DefKind::Contract gets explicit arm in def_id_to_ty (returns error) to document intent; E0122 diagnostic emitted at check_stmt call site where span and name are available"
  - "validate_contract_impls added as method on TypeEnv and called at end of build() so all impl/contract methods are fully registered before validation"
  - "TyKind::Class added to the Struct|Entity arm in analyze_callee — class method calls always use Direct dispatch (concrete receiver, known method)"
metrics:
  duration: "~20 minutes"
  completed: "2026-03-23"
  tasks_completed: 2
  files_changed: 7
  commits: 2
---

# Quick Task 260323-vkg: Fix Contract Typing, Dispatch, and Incomplete Impl

**One-liner:** Three contract compiler bugs fixed: E0122 for contract-as-type, E0123 for incomplete impl validation, and TyKind::Class added to concrete dispatch path.

## Objective Achieved

All three bugs that caused silent failures and runtime crashes when using contracts are now caught at compile time:
1. Incomplete contract implementations produce E0123 with missing method names listed
2. Using a contract name as a type annotation produces E0122 with guidance to use generic bounds
3. Method calls on class-typed receivers now dispatch correctly (Direct, not fallthrough)

## Changes Made

### Task 1: Error types, error codes, and contract-as-type diagnostic

**`writ-diagnostics/src/code.rs`**
- Added `E0122` (contract used as type annotation)
- Added `E0123` (incomplete contract implementation)

**`writ-compiler/src/check/error.rs`**
- Added `ContractAsType { contract_name, span, file }` variant to `TypeError`
- Added `IncompleteContractImpl { ty_name, contract_name, missing_methods, span, file }` variant
- Added `From<TypeError> for Diagnostic` match arms for both new variants with helpful error messages and guidance text

**`writ-compiler/src/check/env_build.rs`**
- Added explicit `DefKind::Contract => interner.error()` arm in `def_id_to_ty` with a comment documenting that diagnostics are emitted at the call site

**`writ-compiler/src/check/check_stmt.rs`**
- Added import for `DefKind`
- After resolving a `Let` type annotation via `resolve_ast_type_with_file`, check if the annotation names a `DefKind::Contract` definition and emit `TypeError::ContractAsType` (E0122)

### Task 2: Contract impl validation, class dispatch fix, and tests

**`writ-compiler/src/check/env.rs`**
- Added `use crate::resolve::def_map::DefMap` import
- Added `validate_contract_impls(&self, def_map: &DefMap) -> Vec<TypeError>` method on `TypeEnv`
- Called `validate_contract_impls` at the end of `TypeEnv::build()`, collecting E0123 diagnostics into the returned diagnostics vec

**`writ-compiler/src/emit/body/call.rs`**
- In `analyze_callee`, added `TyKind::Class(_)` to the concrete receiver arm alongside `TyKind::Struct` and `TyKind::Entity`, returning `CallKind::Direct`

**`writ-compiler/tests/typecheck_tests.rs`**
- Added `test_incomplete_contract_impl_error`: E0123 emitted when impl is missing a required method
- Added `test_complete_contract_impl_no_error`: no false positive when impl is complete
- Added `test_contract_as_type_error`: E0122 emitted when contract name used as type annotation
- Added `test_class_method_call_no_error`: no error when calling methods on class receivers

## Test Results

```
test test_class_method_call_no_error ... ok
test test_incomplete_contract_impl_error ... ok
test test_contract_as_type_error ... ok
test test_complete_contract_impl_no_error ... ok

Full suite: 81 passed; 0 failed (typecheck_tests)
All crate tests: 394 passed; 0 failed
```

## Deviations from Plan

None - plan executed exactly as written. The "simplest approach" for ContractAsType was implemented: check DefKind at the check_stmt call site where span and diagnostic context are available, rather than threading diagnostics through the resolver.

## Known Stubs

None.

## Self-Check: PASSED

Files modified exist and contain expected content:
- `writ-diagnostics/src/code.rs` contains `E0122`
- `writ-compiler/src/check/env.rs` contains `validate_contract_impls`
- `writ-compiler/src/emit/body/call.rs` contains `TyKind::Class`
- `writ-compiler/tests/typecheck_tests.rs` contains `incomplete_contract_impl`

Commits:
- e3b12e9: fix(quick-260323-vkg-01): add E0122/E0123 error codes and contract-as-type diagnostic
- 7a10b39: fix(quick-260323-vkg-01): add contract impl validation, class dispatch fix, and tests
