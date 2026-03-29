---
phase: 106-read-only-introspection-integration-tests-and-lsp
plan: 01
subsystem: testing
tags: [writ-runtime, reflection, integration-tests, GC, intrinsics]

# Dependency graph
requires:
  - phase: 105-writ-compiler-reflectable-auto-impl-emission
    provides: TypeMethods/TypeContracts/TypeImplements intrinsics and ReflectionIndex singleton caching
provides:
  - 6 new integration tests covering REFL-04, REFL-06, REFL-07, REFL-09 in writ-runtime/tests/reflection_tests.rs
  - Full coverage of all read-only reflection intrinsics (all 6 REFL-03 through REFL-09 requirements now tested)
affects:
  - 106-02 (LSP hover tests now independent of runtime test gaps)
  - future compiler phases that call reflection APIs

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Multi-task interning test: spawn two tasks returning TypeOf same typedef, compare HeapRef values at Rust level — avoids CmpEqI limitation (CmpEqI calls extract_int which returns 0 for Value::Ref)"
    - "TypeImplements argument protocol: pass TypeOf on a second TypeDef with the same name as the contract — TypeImplements reads field 0 (name string) from the contract_href Type object"
    - "ImplDef registration pattern: add_type_def + add_contract_def + add_impl_def(typedef_token, contract_token) + add_method(impl_body)"

key-files:
  created: []
  modified:
    - writ-runtime/tests/reflection_tests.rs

key-decisions:
  - "CmpEqI does NOT work for HeapRef identity: extract_int returns 0 for Value::Ref, making CmpEqI always return true for non-int values. Research document was incorrect. Used multi-task pattern instead: spawn two tasks, compare returned HeapRef values in Rust."
  - "TypeImplements test strategy: Use TypeOf on a second TypeDef named after the contract (e.g., 'Drawable'). TypeImplements reads field 0 (name) from the contract_href Type object and compares to contract names in ImplDef table — no direct ContractDef TypeOf needed."
  - "GC survival test: Type singletons and FieldInfo singletons survive GC as permanent roots. Temporary Array objects from fields() calls are NOT cached and ARE freed by GC. Test validates post-GC TypeOf still returns the same HeapRef."
  - "Method order matters for spawn_task index: methods are indexed in add_method call order. When adding impl methods before main, spawn_task must use the correct 0-based index for main."

patterns-established:
  - "Pattern: Spawn two tasks with identical TypeOf instructions to test singleton caching without requiring HeapRef equality instruction"
  - "Pattern: Test contract implementation with local ContractDef + ImplDef + second TypeDef for TypeImplements argument"

requirements-completed: [REFL-03, REFL-04, REFL-06, REFL-07, REFL-08, REFL-09]

# Metrics
duration: 25min
completed: 2026-03-28
---

# Phase 106 Plan 01: Read-Only Introspection Integration Tests Summary

**6 new runtime integration tests for TypeMethods, TypeContracts, TypeImplements, and Type equality interning, bringing total reflection test coverage to 12 tests spanning all REFL-03 through REFL-09 requirements**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-28T00:00:00Z
- **Completed:** 2026-03-28T00:25:00Z
- **Tasks:** 2 (combined into 1 commit since all tests are in the same file)
- **Files modified:** 1

## Accomplishments
- Added `test_type_methods_returns_array` — REFL-04: TypeMethods intrinsic returns Array of MethodInfo; verifies method name "greet" via field 0 of MethodInfo heap object
- Added `test_type_contracts_returns_array` — REFL-06: TypeContracts intrinsic returns Array of ContractInfo for types with ImplDef; verifies contract name "Drawable" via field 0
- Added `test_type_implements_returns_bool` — REFL-07: TypeImplements(type, contract_type) returns true for implemented contracts; contract_type is a TypeOf on a same-named TypeDef
- Added `test_type_equality_same_type` — REFL-09: same typedef produces the same singleton HeapRef from two separate TypeOf calls (multi-task pattern)
- Added `test_type_inequality_different_types` — REFL-09: different typedef indices produce different HeapRefs
- Added `test_gc_survival_after_reflection_ops` — REFL-09 GC: TypeOf singleton HeapRef is identical before and after GC collection, confirming permanent root registration

## Task Commits

Each task was committed atomically:

1. **Task 1+2: Add TypeMethods, TypeContracts, TypeImplements, Type equality, and GC tests** - `d7091bf` (test)

## Files Created/Modified
- `writ-runtime/tests/reflection_tests.rs` - Extended with 6 new #[test] functions; updated module doc comment to include Phase 106 additions

## Decisions Made

- **CmpEqI cannot compare HeapRefs**: `extract_int` returns 0 for `Value::Ref`, so CmpEqI always returns true for two Refs. The research document's claim that "CmpEqI performs pointer/value identity on i64 representation of HeapRef" was incorrect. Fixed by spawning two tasks and comparing returned `Value::Ref(HeapRef)` values at the Rust level.
- **TypeImplements argument protocol**: Takes a Type heap object as the second argument; reads field 0 (name string) and compares against contract names in ImplDef table. Test strategy: create a second TypeDef with the same name as the contract, use TypeOf on it to produce a Type object with the right name field.
- **GC survival assertion**: The temporary Array from `Type.fields()` is NOT a permanent root and IS freed by GC. Changed assertion from `objects_freed == 0` to verifying that the Type singleton HeapRef is the same before and after GC.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CmpEqI cannot test HeapRef identity**
- **Found during:** Task 2 (Type equality tests)
- **Issue:** `CmpEqI` uses `extract_int` which returns 0 for `Value::Ref`, making `CmpEqI` always return `true` for two `Value::Ref` inputs. The test using `CmpEqI(TypeOf(T), TypeOf(T))` would pass trivially, and `CmpEqI(TypeOf(T), TypeOf(U))` would also return `true` (wrong).
- **Fix:** Replaced IL-level equality assertion with multi-task Rust-level HeapRef comparison. Each task returns a `Value::Ref`; the test asserts `href0 == href1` (same type) or `href_alpha != href_beta` (different types).
- **Files modified:** writ-runtime/tests/reflection_tests.rs
- **Verification:** `test_type_equality_same_type` and `test_type_inequality_different_types` both pass with correct semantics.
- **Committed in:** d7091bf

**2. [Rule 1 - Bug] GC test assertion was wrong**
- **Found during:** Task 2 (GC survival test)
- **Issue:** Original assertion `objects_freed == 0` failed with `freed=2` because the temporary Array object from `Type.fields()` and the Sample instance are NOT permanent roots — only Type and FieldInfo singletons are.
- **Fix:** Changed test to verify the post-GC TypeOf returns the same HeapRef as pre-GC. The test now confirms the Type singleton persists through GC without asserting zero frees.
- **Files modified:** writ-runtime/tests/reflection_tests.rs
- **Verification:** `test_gc_survival_after_reflection_ops` passes.
- **Committed in:** d7091bf

---

**Total deviations:** 2 auto-fixed (both Rule 1 - Bug)
**Impact on plan:** Both fixes needed for correctness. No scope creep. All 6 planned tests delivered.

## Issues Encountered
- Multi-task method index confusion: when adding impl methods before `main`, `spawn_task` index must account for all previously added methods. Fixed by carefully tracking method insertion order.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All REFL-03 through REFL-09 requirements now have passing integration tests
- Phase 106 Plan 02 (LSP hover tests) can proceed independently
- No blockers

---
*Phase: 106-read-only-introspection-integration-tests-and-lsp*
*Completed: 2026-03-28*
