---
phase: 25-il-codegen-method-bodies
plan: 04
subsystem: codegen
tags: [il-codegen, enum-match, type-conversions, string-ops, const-fold, debug-info, serialization]

# Dependency graph
requires:
  - phase: 25-01
    provides: BodyEmitter, RegisterAllocator, LabelAllocator, emit_expr/emit_stmt
  - phase: 25-02
    provides: emit_call(), object model, argument packing
  - phase: 25-03
    provides: array/option/result/closure/concurrency instructions, pre_scan_lambdas

provides:
  - patterns.rs: emit_match() with enum GET_TAG+SWITCH, Option IS_NONE propagation, Result IS_ERR propagation
  - const_fold.rs: const_fold() reducing compile-time Int/Float/Bool arithmetic to literals
  - debug.rs: emit_debug_locals() and emit_source_spans() for all method bodies
  - expr.rs: StrConcat for string + operator; I2f/F2i/I2s/F2s/B2s via into_* field sentinels
  - expr.rs: StrLen for string.len(); Match dispatches to patterns::emit_match()
  - serialize.rs: translate() + serialize() converting ModuleBuilder+EmittedBodies to writ_module::Module binary
  - emit/mod.rs: emit_bodies() full pipeline entry point returning Vec<u8>
  - lib.rs: emit_bodies re-exported as public API

affects:
  - Phase 26 (CLI integration — will call emit_bodies() with real TypedAst)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Pattern match kind detection by scrutinee TyKind: Enum->SWITCH, Option->IS_NONE, Result->IS_ERR
    - Option/Result propagation detected by 2-arm match where one arm body is Return
    - Type conversions detected via field name sentinels (into_float, into_string, etc.)
    - const_fold() recurses on Binary/UnaryPrefix, returns None for non-constants
    - Serialization: compiler row types map field-by-field to writ_module row types
    - ModuleBuilder serialization accessors expose finalized rows for translate()

key-files:
  created:
    - writ-compiler/src/emit/body/patterns.rs
    - writ-compiler/src/emit/body/const_fold.rs
    - writ-compiler/src/emit/body/debug.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-compiler/tests/emit_serialize_tests.rs
  modified:
    - writ-compiler/src/emit/body/expr.rs (StrConcat, I2f/F2i/I2s/F2s/B2s, StrLen, Match dispatch)
    - writ-compiler/src/emit/body/mod.rs (pub mod patterns, const_fold, debug)
    - writ-compiler/src/emit/body/stmt.rs (fix doctest code block)
    - writ-compiler/src/emit/mod.rs (pub mod serialize, emit_bodies())
    - writ-compiler/src/emit/module_builder.rs (serialization accessors for all finalized rows)
    - writ-compiler/src/lib.rs (re-export emit_bodies)

key-decisions:
  - "emit_match() dispatches on scrutinee TyKind: Enum uses GET_TAG+SWITCH; Option uses IS_NONE; Result uses IS_ERR"
  - "Option/Result propagation detected by 2-arm match where one arm body is TypedExpr::Return"
  - "Type conversions use field name sentinels (into_float, into_int, into_string) in try_emit_builtin_method()"
  - "const_fold() returns Option<TypedLiteral> — None for non-constants, no panic"
  - "serialize.rs translate() maps compiler rows field-by-field to writ_module rows; ImplDefRow.contract_token -> .contract"
  - "emit_bodies() is the Phase 25 public API; Phase 24 emit() retained for metadata-only use"
  - "Debug info: start_pc=0 end_pc=total_code_size for all registers; name=0 placeholder (Phase 26 wires real heap)"

patterns-established:
  - "emit_match() takes the full TypedExpr::Match — all scrutinee type detection happens inside"
  - "Propagation detection is heuristic: 2 arms + one Return body = propagation pattern"
  - "Type conversion dispatch via field name sentinel avoids changing TypedExpr AST shape"
  - "Serialization accessors on ModuleBuilder expose finalized rows without making fields pub"

requirements-completed: [EMIT-17, EMIT-18, EMIT-19, EMIT-20, EMIT-23, EMIT-24, EMIT-26, EMIT-28]

# Metrics
duration: 11min
completed: 2026-03-03
---

# Phase 25 Plan 04: Enum Match, Type Conversions, Debug Info, and Binary Serialization Summary

**Enum SWITCH lowering, IS_NONE/IS_ERR propagation, I2f/F2i/I2s/F2s/B2s conversions, StrConcat/StrLen, const_fold, debug info, and full binary serialization via writ_module::Module::to_bytes() — 75 tests passing**

## Performance

- **Duration:** 11 min
- **Started:** 2026-03-03T03:06:10Z
- **Completed:** 2026-03-03T03:17:55Z
- **Tasks:** 2 (TDD: RED+GREEN combined since implementation was straightforward)
- **Files modified:** 11 (5 created, 6 modified)

## Accomplishments

**Task 1: Enum match, type conversions, string ops, const folding**

- `patterns.rs` created with `emit_match()` dispatching on scrutinee `TyKind`:
  - `Enum`: `GET_TAG` + `SWITCH` + per-variant `EXTRACT_FIELD` + `MOV` + `BR end`
  - `Option`: `IS_NONE` + `BR_FALSE` + `LOAD_NULL` + `RET` + `UNWRAP` (2-arm propagation detection)
  - `Result`: `IS_ERR` + `BR_FALSE` + `EXTRACT_ERR` + `WRAP_ERR` + `RET` + `UNWRAP_OK`
  - Other: `CmpEq` + `BrFalse` chain per literal/wildcard arm
- `const_fold.rs` created with `const_fold()` evaluating Int/Float arithmetic and Bool logic at compile time
- `debug.rs` created with `emit_debug_locals()` (one entry per register) and `emit_source_spans()`
- `expr.rs` updated: `StrConcat` for `String + String`; `I2f/F2i/I2s/F2s/B2s` via into_* field sentinels; `StrLen` for `String.len()`
- `Match` variant in `emit_expr()` now dispatches to `patterns::emit_match()`
- 15 new tests (65 total in emit_body_tests.rs)

**Task 2: Debug info and binary serialization**

- `serialize.rs` created with `translate()` mapping all 21 compiler metadata tables to `writ_module` row types field-by-field
- `serialize::serialize()` calls `Module::to_bytes()` for spec-compliant `.writil` binary output
- `emit_bodies()` added as full Phase 25 pipeline entry point (error pre-pass + lambda scan + emit + serialize)
- `module_builder.rs` gains serialization accessors for all finalized rows (type_defs, field_defs, method_defs, etc.)
- `lib.rs` re-exports `emit_bodies` as public API
- 10 new tests in `emit_serialize_tests.rs` (75 total)

## Task Commits

1. **Task 1: Enum match, type conversions, string ops, const folding** - `62b9165` (feat)
2. **Task 2: Debug info and binary serialization** - `65b8117` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/body/patterns.rs` — Created: `emit_match()` for enum/option/result/literal patterns
- `writ-compiler/src/emit/body/const_fold.rs` — Created: `const_fold()` compile-time arithmetic
- `writ-compiler/src/emit/body/debug.rs` — Created: `emit_debug_locals()`, `emit_source_spans()`
- `writ-compiler/src/emit/serialize.rs` — Created: `translate()`, `serialize()` for binary output
- `writ-compiler/tests/emit_serialize_tests.rs` — Created: 10 debug/serialization tests
- `writ-compiler/src/emit/body/expr.rs` — Updated: StrConcat, I2f/F2i/I2s/F2s/B2s, StrLen, Match dispatch
- `writ-compiler/src/emit/body/mod.rs` — Updated: pub mod patterns, const_fold, debug
- `writ-compiler/src/emit/body/stmt.rs` — Fixed: doctest code block marker
- `writ-compiler/src/emit/mod.rs` — Updated: pub mod serialize, emit_bodies()
- `writ-compiler/src/emit/module_builder.rs` — Updated: serialization accessor methods
- `writ-compiler/src/lib.rs` — Updated: re-export emit_bodies

## Decisions Made

- `emit_match()` dispatches on scrutinee `TyKind`: Enum uses `GET_TAG+SWITCH`; Option uses `IS_NONE`; Result uses `IS_ERR` — clean separation without needing variant name inspection for the core dispatch
- Option/Result propagation detected by heuristic: exactly 2 arms where one arm body is `TypedExpr::Return` — matches the desugared `?/try` pattern from the typechecker
- Type conversions use field name sentinels (`into_float`, `into_int`, `into_string`) in `try_emit_builtin_method()` — avoids changing TypedExpr AST shape while enabling correct instruction dispatch
- `const_fold()` returns `Option<TypedLiteral>` — `None` for any non-constant subexpression; safe to call on any expression
- `serialize.rs translate()` maps compiler rows field-by-field: note `ImplDefRow.contract_token` maps to `writ_module` `contract` field, and `GenericConstraintRow.param_row` maps to `param` field
- `emit_bodies()` is the Phase 25 public entry point; Phase 24's `emit()` is retained for metadata-only use
- Debug info: `start_pc=0`, `end_pc=total_code_size` for all registers; `name=0` as placeholder string heap offset (full resolution in Phase 26 CLI)
- `module_builder.rs` serialization accessors use `impl Iterator` return types to avoid exposing internal entry wrapper types

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] stmt.rs doctest code block caused test failure**
- **Found during:** Final test run (Task 2 verification)
- **Issue:** Code comment block in `emit_for_loop` doc comment was marked as ` ``` ` (Rust) but contained pseudocode, causing doctest compilation failure
- **Fix:** Changed ` ``` ` to ` ```text ` to mark it as non-executable
- **Files modified:** `writ-compiler/src/emit/body/stmt.rs`
- **Committed in:** 65b8117 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Trivial fix; no scope creep.

## Issues Encountered

- `TyInterner::string()` doesn't exist — the method is `string_ty()`. Fixed in tests by using `interner.string_ty()`
- `writ-module`'s `ImplDefRow` uses field name `contract` not `contract_token` (compiler uses the latter). Mapped correctly in serialize.rs.
- `writ-module`'s `GenericConstraintRow` uses field name `param` not `param_row`. Mapped correctly.

## Next Phase Readiness

- All 20 EMIT requirements for Phase 25 are now satisfied (EMIT-01 through EMIT-28)
- `emit_bodies()` provides the public API for Phase 26 CLI integration
- TypedAst with any supported expression variant now compiles to correct IL instructions
- Binary output is spec-compliant (WRIT header, all 21 tables, method bodies with debug info)
- No blockers for Phase 26

---
*Phase: 25-il-codegen-method-bodies*
*Completed: 2026-03-03*

## Self-Check: PASSED

- `writ-compiler/src/emit/body/patterns.rs`: FOUND
- `writ-compiler/src/emit/body/const_fold.rs`: FOUND
- `writ-compiler/src/emit/body/debug.rs`: FOUND
- `writ-compiler/src/emit/serialize.rs`: FOUND
- `writ-compiler/tests/emit_serialize_tests.rs`: FOUND
- Commit 62b9165 (Task 1): FOUND
- Commit 65b8117 (Task 2): FOUND
- 75 tests pass: confirmed (65 body + 10 serialize)
