---
phase: 104-writ-compiler-typeof-lowering-and-tykind
plan: 02
subsystem: compiler
tags: [typeof, reflection, TypeOf, IL-emission, type-idx, golden-tests]

# Dependency graph
requires:
  - phase: 104-01
    provides: TypedExpr::TypeOf with static_ty field; stub emit arm in body/expr/mod.rs
provides:
  - Instruction::TypeOf emission with correct type_idx for user-defined types (via token_for_def)
  - Instruction::TypeOf emission with correct type_idx for primitives (via type_ref_token_by_name)
  - TypeRef rows for Type, Int, Float, Bool, String in the module's TypeRef table
  - type_ref_token_by_name() lookup method on ModuleBuilder
affects:
  - writ-runtime Phase 103 (runtime receives TypeOf instruction with baked-in type_idx)
  - 105-reflectable-auto-impl (typeof() now fully emits correct IL)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "resolve_typeof_type_idx() dispatches on TyKind: token_for_def for user types, type_ref_token_by_name for primitives"
    - "type_ref_token_by_name() generalizes range_type_token() — both primitives and runtime classes use the same name-based lookup"
    - "TypeRef rows for runtime types are registered unconditionally in collect_defs (not on-demand) — same as Range"
    - "Golden test string heap indices shift when new TypeRef strings are added — re-bless is expected and correct"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/collect/mod.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-golden/tests/golden/adv_defer.writil
    - writ-golden/tests/golden/conditional_active.writil
    - writ-golden/tests/golden/conditional_inactive.writil
    - writ-golden/tests/golden/dlg_fn_mix.writil
    - writ-golden/tests/golden/dlg_quest_pattern.writil
    - writ-golden/tests/golden/expr_string_concat.writil
    - writ-golden/tests/golden/fn_log_say_choice.writil
    - writ-golden/tests/golden/fn_string_param.writil
    - writ-golden/tests/golden/force_unwrap.writil
    - writ-golden/tests/golden/quest_system.writil
    - writ-golden/tests/golden/result_methods.writil
    - writ-golden/tests/golden/type_class_new.writil

key-decisions:
  - "TypeRef rows for Type/Int/Float/Bool/String registered unconditionally in collect_defs — same pattern as Range — so typeof() never emits type_idx=0 for any registered type"
  - "resolve_typeof_type_idx() lives in body/expr/mod.rs (same file as emit arm) — private helper, no need for separate module"
  - "range_type_token() simplified to delegate to type_ref_token_by_name() — removes duplicated loop logic"
  - "Golden test re-blessing: 12 .writil files had LOAD_STRING indices shift by 5 positions due to 5 new string heap entries; this is correct and expected"

requirements-completed: [COMP-02, COMP-04, REFL-01]

# Metrics
duration: ~18min
completed: 2026-03-28
---

# Phase 104 Plan 02: typeof() IL Code Generation Summary

**TypeOf instruction emission wired end-to-end: TypeRef rows for writ-runtime Type class and all 4 primitive pseudo-TypeDefs registered, type_ref_token_by_name() lookup method added, real TypeOf IL emission arm replaces stub, 2 TDD tests confirm correctness, full workspace green after golden re-bless**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-03-28T12:27:50Z
- **Completed:** 2026-03-28T12:45:00Z
- **Tasks:** 2
- **Files modified:** 16

## Accomplishments

- Registered TypeRef rows for "Type", "Int", "Float", "Bool", "String" in `collect_defs` (mirrors Range registration pattern)
- Added `type_ref_token_by_name(name: &str)` general-purpose lookup on ModuleBuilder; simplified `range_type_token()` to delegate
- Replaced the stub TypeOf emit arm with real `Instruction::TypeOf { r_dst, type_idx }` emission
- Added `resolve_typeof_type_idx()` private helper: `token_for_def` for user-defined types, `type_ref_token_by_name` for primitives
- TDD RED: 2 tests added — both fail with stub (emit 0 instructions). GREEN: both pass with real implementation
- Blessed 12 golden test `.writil` files whose LOAD_STRING indices shifted by 5 due to new string heap entries

## Task Commits

1. **Task 1: Register TypeRef rows and add lookup method** - `ab107ea` (feat)
2. **Task 2 RED: failing tests for TypeOf emission** - `1c31d35` (test)
3. **Task 2 GREEN: TypeOf IL emission arm + blessed golden files** - `2b02cc9` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/collect/mod.rs` — 5 new `add_type_ref` calls for Type, Int, Float, Bool, String
- `writ-compiler/src/emit/module_builder.rs` — `type_ref_token_by_name()` added; `range_type_token()` simplified
- `writ-compiler/src/emit/body/expr/mod.rs` — stub TypeOf arm replaced; `resolve_typeof_type_idx()` helper added
- `writ-compiler/tests/emit_body_tests.rs` — `emit_typeof_struct` and `emit_typeof_primitive_int` TDD tests
- 12 golden `.writil` files — LOAD_STRING indices updated after string heap shift

## Decisions Made

- TypeRef rows registered unconditionally (not on-demand) — mirrors Range; avoids conditional logic at emit time
- `resolve_typeof_type_idx()` returns 0 for unsupported types (generic params, infer, void) — runtime handles gracefully
- Golden re-bless is correct: 5 new strings ("writ-runtime", "Type", "Int", "Float", "Bool", "String" — actually 5 type names, "writ" namespace already existed) shifted LOAD_STRING indices in all files that have string literals

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Golden test string heap indices shifted after TypeRef registration**
- **Found during:** Task 2 full workspace regression
- **Issue:** Adding 5 new type name strings to string heap shifted LOAD_STRING indices in 12 golden test files
- **Fix:** Re-blessed 12 .writil files with `BLESS=1 cargo test -p writ-golden`
- **Files modified:** 12 writ-golden/tests/golden/*.writil files
- **Commit:** 2b02cc9 (included in Task 2 GREEN commit)

This was an expected, correct update — the golden test infrastructure exists specifically to catch and record these changes.

## Known Stubs

None — typeof() emission is fully wired for struct, class, entity, enum, contract, int, float, bool, and string types.

## Self-Check: PASSED
