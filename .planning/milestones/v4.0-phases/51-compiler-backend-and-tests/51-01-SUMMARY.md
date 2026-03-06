---
phase: 51-compiler-backend-and-tests
plan: 01
subsystem: compiler
tags: [rust, codegen, struct-equality, closure, typedast, emit]

# Dependency graph
requires:
  - phase: 50-compiler-frontend
    provides: "class keyword, recursive struct detection, TypeEnv.struct_fields"
  - phase: 48-il-format
    provides: "TypeDefKind enum with Class=4, GetField/CmpEq/BitAnd/BitOr instructions"
provides:
  - "Field-by-field structural equality emission for value-type structs (emit_struct_eq)"
  - "Field-by-field structural inequality emission (emit_struct_neq)"
  - "Nested value-struct field comparison via recursion (emit_field_eq)"
  - "struct_field_types plumbed from TypeEnv through TypedAst into BodyEmitter"
  - "Closure capture TypeDef emitted as TypeDefKind::Class (kind=4)"
affects: [51-compiler-backend-and-tests, golden-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "struct_field_types: FxHashMap<DefId, Vec<(String, Ty)>> extracted from TypeEnv and carried on TypedAst"
    - "emit_binary receives operand_ty separately from result ty (ty=Bool for Eq/NotEq)"
    - "Structural equality via GET_FIELD + typed CmpEq + BitAnd chain per field"
    - "Structural inequality via GET_FIELD + CmpEq + Not + BitOr chain per field"
    - "Test helpers use static LazyLock/OnceLock empty FxHashMap for BodyEmitter/emit_all_bodies"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/mod.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/body/closure.rs
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs

key-decisions:
  - "operand_ty passed to emit_binary as separate parameter — ty is result type (Bool), not operand type"
  - "Closure capture TypeDef changed from TypeDefKind::Struct to TypeDefKind::Class — heap-allocated reference objects semantically correct as class"
  - "Empty struct == returns LoadTrue, empty struct != returns LoadFalse — empty structs always equal"
  - "Reference-type fields (Class, Entity, Delegate, Array) compared by CmpEqI (pointer identity)"

patterns-established:
  - "Field equality dispatch: Struct->recursive, Float->CmpEqF, Bool->CmpEqB, String->CmpEqS, _->CmpEqI"

requirements-completed: [COMP-05, COMP-06, COMP-07]

# Metrics
duration: 25min
completed: 2026-03-13
---

# Phase 51 Plan 01: Struct Equality Emission and Closure Class Kind Summary

**Field-by-field structural equality/inequality emission for value-type structs, with closure capture TypeDefs corrected to TypeDefKind::Class**

## Performance

- **Duration:** 25 min
- **Started:** 2026-03-13T00:00:00Z
- **Completed:** 2026-03-13T00:25:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `struct_field_types: FxHashMap<DefId, Vec<(String, Ty)>>` to `TypedAst`, populated from `TypeEnv.struct_fields` in `typecheck()`, threaded through `emit_all_bodies` into `BodyEmitter`
- `BinaryOp::Eq` and `BinaryOp::NotEq` now dispatch on operand type (not result type), correctly using CmpEqF/CmpEqB/CmpEqS for non-integer scalars and field-by-field expansion for structs
- `emit_struct_eq` emits GetField + typed CmpEq + BitAnd chain; nested value-struct fields recurse via `emit_field_eq`
- `emit_struct_neq` emits GetField + CmpEq + Not + BitOr chain for any-field-differs semantics
- Closure capture TypeDef corrected from `TypeDefKind::Struct` to `TypeDefKind::Class` (heap-allocated reference objects)

## Task Commits

1. **Task 1: Plumb struct_field_types and change closure kind** - `a595ea4` (feat)
2. **Task 2: Implement field-by-field struct equality emission** - `af9554f` (feat)

## Files Created/Modified

- `writ-compiler/src/check/ir.rs` - Added `struct_field_types` field to `TypedAst`
- `writ-compiler/src/check/mod.rs` - Populate `struct_field_types` from `type_env.struct_fields` before TypeEnv drops
- `writ-compiler/src/emit/body/mod.rs` - Added `struct_field_types` to `BodyEmitter`, updated `new()` and `emit_all_bodies` signatures
- `writ-compiler/src/emit/body/closure.rs` - Changed closure capture TypeDef from `TypeDefKind::Struct` to `TypeDefKind::Class`
- `writ-compiler/src/emit/body/expr.rs` - Added `operand_ty` parameter to `emit_binary`; rewrote Eq/NotEq arms; added `emit_struct_eq`, `emit_struct_neq`, `emit_field_eq`
- `writ-compiler/src/emit/mod.rs` - Passed `&typed_ast.struct_field_types` to `emit_all_bodies`
- `writ-compiler/tests/emit_body_tests.rs` - Updated test helpers with static empty map for new API
- `writ-compiler/tests/emit_serialize_tests.rs` - Updated test helpers with static empty map for new API

## Decisions Made

- `operand_ty` is passed separately to `emit_binary` because `ty` in `BinaryOp::Eq` is always `Bool` (the result type). The operand type determines which CmpEq variant to use.
- Closure capture TypeDefs are semantically reference objects (heap-allocated environment structs), so `TypeDefKind::Class` (kind=4) is correct — not `Struct` (kind=0).
- Empty structs return `LoadTrue`/`LoadFalse` directly from `emit_struct_eq`/`emit_struct_neq` (no fields to compare).

## Deviations from Plan

None - plan executed exactly as written. The linter (rust-analyzer) assisted by auto-applying `struct_field_types: FxHashMap::default()` to TypedAst construction sites in test files and fixing `BodyEmitter::new` call signatures during editing, which was equivalent to the planned Step 5 fix.

## Issues Encountered

- `LoadBool { r_dst, val }` instruction does not exist in the instruction set — only `LoadTrue`/`LoadFalse`. The plan referenced it as a conceptual notation. The linter auto-corrected this during editing. No functional impact.

## Next Phase Readiness

- Plan 01 complete: struct equality backend is implemented
- Plan 02 will update the `fn_log_say_choice` golden test to reflect the closure kind change from struct to class
- Entity declarations remain kind=2 (unchanged per COMP-06 decision)

---
*Phase: 51-compiler-backend-and-tests*
*Completed: 2026-03-13*

## Self-Check: PASSED

- writ-compiler/src/check/ir.rs: FOUND
- writ-compiler/src/emit/body/expr.rs: FOUND
- .planning/phases/51-compiler-backend-and-tests/51-01-SUMMARY.md: FOUND
- Commit a595ea4 (Task 1): FOUND
- Commit af9554f (Task 2): FOUND
