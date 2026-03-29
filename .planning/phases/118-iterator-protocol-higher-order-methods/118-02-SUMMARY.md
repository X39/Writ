---
phase: 118-iterator-protocol-higher-order-methods
plan: "02"
subsystem: compiler-typechecker, compiler-emitter, runtime-tests
tags: [iterator-protocol, for-in-loop, CALL_VIRT, Iterable, Iterator, golden-tests, integration-tests]
dependency_graph:
  requires: [118-01]
  provides: [ITER-01, ITER-02, ITER-03, ITER-04, COLL-04]
  affects: [writ-compiler/check, writ-compiler/emit, writ-assembler, writ-golden, writ-runtime/tests]
tech_stack:
  added: []
  patterns:
    - CALL_VIRT-based iterator protocol desugaring (iterator() + next() loop)
    - InferVar resolution in binary arithmetic type checking
    - Spec-locked virtual module contract tokens for prelude contracts
key_files:
  created:
    - writ-golden/tests/golden/iter_for_in_list.writ
    - writ-golden/tests/golden/iter_for_in_list.writil
  modified:
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/check_expr/binary.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-compiler/src/emit/collect/contracts.rs
    - writ-compiler/src/emit/collect/mod.rs
    - writ-assembler/src/disassembler.rs
    - writ-runtime/tests/coll_integration_tests.rs
    - writ-golden/tests/golden_tests.rs
    - writ-compiler/tests/emit_body_tests.rs
    - 22 golden snapshot .writil files (string heap offset rebless)
decisions:
  - Use spec-locked virtual module ContractDef tokens (ITERABLE=(10<<24)|14, ITERATOR=(10<<24)|15) for both ImplDef.contract and CALL_VIRT contract_idx, matching existing REFLECTABLE pattern
  - Use InferVar for elem_ty in for-in class iteration (generic specialization deferred to Phase 119+); fix binary type checker to resolve InferVars before arithmetic kind matching
  - Method-name matching (detecting "iterator" method) to identify Iterable<T> implementations since prelude contracts have no user-module DefId
  - Clamp disassembler ImplDef method_list range to avoid panic on non-monotonic values (multi-class modules have Reflectable impls interleaved with user impls)
metrics:
  duration_minutes: 120
  tasks_completed: 2
  files_modified: 10
  files_created: 2
  golden_tests_reblessed: 22
  completed_date: "2026-03-29"
---

# Phase 118 Plan 02: For-in Loop Desugaring for Class Iterable Types Summary

Compiler and runtime support for `for x in collection` where collection is a class implementing Iterable<T>. Desugars to CALL_VIRT iterator() + loop(CALL_VIRT next() + IS_NONE + UNWRAP). All ITER-01 through ITER-04 and COLL-04 requirements validated by integration tests.

## What Was Built

**Task 1: Compiler Support for Class Iterable For-in Loops**

Extended the compiler pipeline to desugar `for x in collection` for class types:

1. `TypedStmt::For` in `ir.rs` — added two optional fields: `iterable_contract_def_id: Option<DefId>` and `iterator_contract_def_id: Option<DefId>`. Both are `None` for array/range iteration and carry the contract DefIds for class Iterable iteration (though in Phase 118, both remain `None` since prelude contracts have no user-module DefIds — the emit phase uses spec-locked tokens directly).

2. `check_stmt.rs` — extended the `For` match arm to detect class types with Iterable<T> implementations via method-name matching ("iterator" method in impl_index). Creates a fresh `InferVar` as elem_ty (body expressions constrain it via unification, e.g., `sum + x` unifies `x: ?3` with `int`).

3. `emit/body/stmt.rs` — added `TyKind::Class` arm to `emit_for_loop`. Emits the full iterator protocol:
   - `CALL_VIRT r_collection, ITERABLE_CONTRACT_TOKEN, slot=0, argc=1` → `r_iter`
   - Loop: `CALL_VIRT r_iter, ITERATOR_CONTRACT_TOKEN, slot=0, argc=1` → `r_next`
   - `IS_NONE r_next` → branch to loop_end on true
   - `UNWRAP r_next` → `r_elem` (bound to loop variable)
   - Body → `BR loop_start` → `loop_end`

4. `emit/collect/contracts.rs` — added `ITERABLE_CONTRACT_TOKEN = MetadataToken((10<<24)|14)` and `ITERATOR_CONTRACT_TOKEN = MetadataToken((10<<24)|15)` as pub(crate) constants (spec-locked virtual module row positions). Added contract_name capture for prelude contract fallback in ImplDef token resolution.

5. `emit/collect/mod.rs` — added TypeRef registrations for "Iterable" and "Iterator" from writ-runtime.

**Task 2: Tests for All ITER/COLL-04 Requirements**

- Golden test: `iter_for_in_list.writ` + `.writil` snapshot showing CALL_VIRT with contract indices 167772174/167772175 (= (10<<24)|14/15), IS_NONE, and UNWRAP in main's body.
- 5 integration tests in `coll_integration_tests.rs`:
  - `iter_for_in_list` (ITER-01): List<int> with 3 elements, for-in sum
  - `iter_for_in_set` (ITER-02): Set<int> with 3 elements (dedup), for-in sum
  - `iter_for_map_keys` (ITER-03): Map<string, int> with 3 entries, for-in key count
  - `iter_custom_iterable` (ITER-04): Custom Counter class, for-in 0..5 sum
  - `coll_list_map_filter_reduce` (COLL-04): [1..5].map(x*2).filter(x>4).reduce(0, +)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] check_binary failed on arithmetic with InferVar operands**

- **Found during:** Task 2 (integration test run — `expected 'int', found '?3'`)
- **Issue:** `check_binary` for Add/Sub/Mul/Div/Mod did exact TyKind pattern matching. When one operand is `TyKind::Infer(var)` (the fresh var from for-in elem_ty), the match `(TyKind::Int, TyKind::Int)` fails, falling through to an incorrect TypeMismatch error.
- **Fix:** Before TyKind matching, resolve InferVars with `ctx.unify.resolve_ty()` and attempt unification when one side is still Infer. Re-resolve both sides after potential unification before proceeding with the match.
- **Files modified:** `writ-compiler/src/check/check_expr/binary.rs`
- **Commit:** `ae4bedb`

**2. [Rule 1 - Bug] Disassembler panic on non-monotonic ImplDef method_list values**

- **Found during:** Task 2 (golden test blessing attempt panicked at `disassembler.rs:159`)
- **Issue:** The disassembler computes `method_start..method_end` for each ImplDef using the next ImplDef's `method_list` as the end. With multiple classes, the Reflectable ImplDef for the second class has a non-zero `method_list` (e.g., 4), while the user impl blocks before it have `method_list=0`. This causes `[3..0]` invalid range panic.
- **Fix:** Added `.max(method_start)` clamping to `method_end_raw` in both the main impl iteration loop and `compute_method_ownership`. This prevents invalid range creation when method_list=0 follows a non-zero value.
- **Files modified:** `writ-assembler/src/disassembler.rs`
- **Commit:** `ae4bedb`

**3. [Rule 1 - Bug] emit_body_tests.rs TypedStmt::For missing new fields**

- **Found during:** Task 2 (cargo test -p writ-compiler compilation failure)
- **Issue:** The test at `emit_body_tests.rs:1301` constructed `TypedStmt::For` without the two new optional fields added to the IR.
- **Fix:** Added `iterable_contract_def_id: None` and `iterator_contract_def_id: None` to the test's For construction.
- **Files modified:** `writ-compiler/tests/emit_body_tests.rs`
- **Commit:** `ae4bedb`

### Planned Design Adjustments

**Golden snapshot reblessing (22 files):** Adding "Iterable" and "Iterator" TypeRef entries to `collect/mod.rs` shifts string heap offsets by 24 bytes for all modules that use string literals. All affected golden tests were re-blessed with `BLESS=1`.

**iter_for_map_keys using string keys:** The plan suggested `Map<int, int>` for the map keys test, but `K: Ord + Eq` generic return type `K[]` from `get_keys()` resolves to `GenericParam(0)` in the type checker without specialization. Using string keys avoids arithmetic on unresolved generics. ITER-03 requirement is still satisfied (for-in over map.get_keys() array).

## Test Results

All test suites pass:
- `cargo test -p writ-compiler` — 101 tests pass
- `cargo test -p writ-runtime --test coll_integration_tests` — 9 pass, 1 ignored (pre-existing cross-module limitation)
- `cargo test -p writ-golden` — 71 tests pass (including new iter_for_in_list)
- `cargo build -p writ-cli` — builds cleanly

## Known Stubs

None. The for-in desugaring is fully functional for class types implementing Iterable<T>. Generic specialization (determining exact element types from class type parameters) is deferred to Phase 119+ as documented — the InferVar approach is a working interim solution because loop bodies constrain the InferVar through normal unification.

## Self-Check: PASSED

- SUMMARY.md created at correct path
- Commit 65b49d0 (Task 1) found in git log
- Commit ae4bedb (Task 2) found in git log
- All test suites pass (compiler 101, integration 9, golden 71)
