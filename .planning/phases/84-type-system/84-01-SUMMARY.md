---
phase: 84-type-system
plan: 01
subsystem: writ-compiler/check
tags: [type-system, contracts, assignability, TyKind]
dependency_graph:
  requires: [83-01]
  provides: [TyKind::Contract, contract-assignability, E0122-removed]
  affects: [writ-compiler/check, writ-compiler/emit]
tech_stack:
  added: []
  patterns: [contract-as-type, directional-assignability, poison-propagation]
key_files:
  created: []
  modified:
    - writ-compiler/src/check/ty.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/check/unify.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/check_expr/call.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/check/error.rs
    - writ-compiler/src/emit/type_sig.rs
    - writ-compiler/tests/typecheck_tests.rs
decisions:
  - "ContractAsType error variant deleted (superseded by contract-as-type becoming valid)"
  - "Contract assignability is directional: unify handles same-contract identity only; let-binding assignability is handled in check_stmt.rs"
  - "TyKind::Contract encodes as TypeRef (0x10) in IL — same as Struct/Class/Entity/Enum"
  - "Param/return-position tests ignored with TODO for 84-02 (method resolution on contract receivers)"
metrics:
  duration: "15 minutes"
  completed: "2026-03-23T23:07:33Z"
  tasks_completed: 2
  files_modified: 9
  tests_added: 5
  tests_passing: 83
requirements_satisfied: [TYPE-01, TYPE-02, TYPE-03, TYPE-04, EMIT-03]
---

# Phase 84 Plan 01: TyKind::Contract and Contract Assignability Summary

TyKind::Contract(DefId) added as a first-class type variant, def_id_to_ty wired, contract assignability enforced at let-binding sites, E0122 guard removed, and IL emitter updated to encode contract types.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add TyKind::Contract variant and wire all match sites | 80299d3 | ty.rs, env_build.rs, unify.rs, check_stmt.rs, call.rs, check_decl.rs, error.rs, type_sig.rs |
| 2 | Tests for contract-as-type assignability | d054359 | typecheck_tests.rs |

## What Was Built

**TyKind::Contract(DefId)** is now a first-class type variant:

- `ty.rs`: Added `Contract(DefId)` variant, `contract()` constructor, `display()` shows "contract", `display_named()` shows the contract name (e.g. "Movable")
- `env_build.rs`: `def_id_to_ty` now maps `DefKind::Contract => TyKind::Contract(def_id)` (was returning `TyKind::Error` with a comment "Contracts are not types")
- `unify.rs`: Added `(TyKind::Contract(a), TyKind::Contract(b)) if a == b => Ok(())` for same-contract identity
- `check_stmt.rs`: Removed E0122 guard entirely; replaced with contract assignability check — when the annotation is `TyKind::Contract(contract_id)`, checks that the inferred concrete type (Struct/Class/Entity) has an `ImplEntry` with `contract_def_id == Some(contract_id)` in `impl_index`; emits `MissingContractImpl` (E0112) on failure
- `check_expr/call.rs`: Added `TyKind::Contract(did)` to concrete_def_id extraction in generic bound checking
- `check_decl.rs`: Added `DefKind::Contract => Some(TyKind::Contract(...))` to `self_type` resolution in impl block checking
- `error.rs`: Deleted `TypeError::ContractAsType` variant and its `From<TypeError> for Diagnostic` arm
- `emit/type_sig.rs`: Added `TyKind::Contract(def_id)` to the 0x10 TypeRef encoding arm

## Tests

Five tests added to `typecheck_tests.rs`:
1. `test_contract_as_type_valid` — renamed from `test_contract_as_type_error`; now asserts no errors (MyClass implements MyContract, so `let c: MyContract = new MyClass{}` is valid)
2. `test_contract_as_type_valid_assignment` — Cat implements Movable, valid assignment
3. `test_contract_as_type_invalid_assignment` — Dog does NOT implement Movable, expects E0112
4. `test_contract_as_param_type` — `#[ignore]` (method resolution on contract receivers deferred to 84-02)
5. `test_contract_as_return_type` — `#[ignore]` (return-position contract type deferred to 84-02)

**Result: 83 tests pass, 2 ignored.**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] TyKind::Contract not covered in emit/type_sig.rs**
- **Found during:** Task 1 build verification
- **Issue:** Adding `TyKind::Contract` to the enum caused a non-exhaustive patterns error in `writ-compiler/src/emit/type_sig.rs` line 34
- **Fix:** Added `TyKind::Contract(def_id)` to the existing `Struct | Class | Entity | Enum` arm that encodes as TypeRef (0x10) — contract types have an associated DefId just like those nominal types
- **Files modified:** `writ-compiler/src/emit/type_sig.rs`
- **Commit:** 80299d3

## Decisions Made

1. **ContractAsType variant deleted** — the plan gave the option to keep with `#[allow(dead_code)]` or delete; deleted cleanly because the test was updated simultaneously in Task 2.
2. **Contract assignability is directional** — unification is symmetric so `Contract == Contract` identity is handled in `unify.rs`, but concrete-to-contract assignability is directional and lives in `check_stmt.rs`.
3. **Contract encodes as TypeRef (0x10)** — contracts have a DefId like other nominal types, and the IL TypeRef encoding is the natural fit for cross-module contract references.
4. **Param/return tests marked `#[ignore]`** — both tests require method resolution on contract-typed receivers (`s.speak()`) which is the domain of Plan 84-02. Keeping them with `#[ignore]` and TODO comments tracks the work without blocking this plan.

## Known Stubs

None — all core features of this plan are fully implemented. The two ignored tests represent intentional deferral to Plan 84-02 (method resolution on contract receivers), not missing functionality in this plan's scope.

## Self-Check

### Files Exist Check
- writ-compiler/src/check/ty.rs — exists
- writ-compiler/src/check/env_build.rs — exists
- writ-compiler/src/check/unify.rs — exists
- writ-compiler/src/check/check_stmt.rs — exists
- writ-compiler/tests/typecheck_tests.rs — exists

### Commits Exist Check
- 80299d3 — feat(84-01): add TyKind::Contract and wire contract type resolution
- d054359 — test(84-01): add contract-as-type assignability tests
