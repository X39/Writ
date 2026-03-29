---
phase: 25-il-codegen-method-bodies
plan: 02
subsystem: codegen
tags: [il-codegen, call-dispatch, object-model, boxing, entity-construction, field-access]

# Dependency graph
requires:
  - phase: 25-01
    provides: BodyEmitter struct, RegisterAllocator, LabelAllocator, emit_expr/emit_stmt infrastructure

provides:
  - call.rs: emit_call() with CallKind dispatch tree (Direct/Virtual/Extern/Indirect)
  - call.rs: emit_call_indirect() for Func-typed delegate callees
  - call.rs: emit_box_if_needed() / emit_unbox_if_needed() for generic boxing (EMIT-21)
  - call.rs: analyze_callee() — CALL_VIRT->CALL specialization for concrete receivers (EMIT-27)
  - expr.rs: TypedExpr::Call dispatch with consecutive argument packing (EMIT-09)
  - expr.rs: TypedExpr::Field -> GET_FIELD
  - expr.rs: TypedExpr::ComponentAccess -> GET_COMPONENT
  - expr.rs: TypedExpr::New -> NEW + SET_FIELD (struct) or SPAWN_ENTITY + SET_FIELD + INIT_ENTITY (entity)
  - expr.rs: TypedExpr::Assign where target is Field -> SET_FIELD
  - reg_alloc.rs: next() and type_of() helpers for argument packing
  - heaps.rs: get_str() for reverse string heap offset -> string lookup
  - module_builder.rs: field_token_by_name() for GET_FIELD/SET_FIELD field_idx resolution
  - module_builder.rs: contract_method_slot_by_def_id() (stub, full pipeline resolves)

affects:
  - 25-03 (closures/arrays/concurrency adds match arms to emit_expr/emit_stmt)
  - 25-04 (enums, string ops, debug info complete the emitter)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - CallKind enum dispatch: Direct/Virtual{slot}/Extern/Indirect drives call instruction selection
    - Argument packing: emit args into Vec<u16>, allocate consecutive block, MOV each non-consecutive arg
    - EMIT-27 specialization: analyze_callee() checks receiver TyKind::Struct/Entity -> CallKind::Direct
    - EMIT-21 boxing: emit_box_if_needed() checks is_value_type(arg) && is_generic_param(param)
    - emit_new() helper: dispatch on TyKind::Entity vs other for SPAWN_ENTITY vs NEW sequence
    - field_token_by_name(): iterate field_defs by parent TypeDefHandle, compare string heap

key-files:
  created:
    - writ-compiler/src/emit/body/call.rs
  modified:
    - writ-compiler/src/emit/body/mod.rs (add pub mod call)
    - writ-compiler/src/emit/body/expr.rs (Call/Field/ComponentAccess/New/Assign-field dispatch)
    - writ-compiler/src/emit/body/reg_alloc.rs (add next() and type_of())
    - writ-compiler/src/emit/heaps.rs (add get_str() for string heap reverse lookup)
    - writ-compiler/src/emit/module_builder.rs (add field_token_by_name, contract_method_slot helpers)
    - writ-compiler/tests/emit_body_tests.rs (11 new tests: 6 call + 5 object model)

key-decisions:
  - "emit_call() takes an explicit DefId + CallKind — callee analysis separated from instruction emission"
  - "emit_expr() Call handler uses inline arg packing (not emit_call) to avoid DefId requirement for general dispatch; full pipeline calls emit_call() directly with known DefId"
  - "contract_idx in CALL_VIRT is 0 as placeholder — full resolution requires impl->contract mapping from Phase 24 context, deferred to Plan 04"
  - "field_token_by_name() iterates field_defs post-finalize using string heap get_str() — correct because field_defs are sorted by parent during finalize()"
  - "extract_callee_def_id_opt() returns None — BodyEmitter holds no DefMap reference; method_idx=0 for general emit_expr() Call path; explicit DefId always passed by full pipeline"

patterns-established:
  - "call.rs public API: emit_call(emitter, expr, def_id, kind) + emit_call_indirect(emitter, expr, r_delegate)"
  - "Boxing helper: emit_box_if_needed(emitter, r_val, arg_ty, param_ty) -> u16; returns r_val unchanged if no boxing needed"
  - "Entity construction: SPAWN_ENTITY -> SET_FIELD(explicit only) -> INIT_ENTITY; default fields never get SET_FIELD"
  - "Field access: extract_type_def_id(emitter, ty) + field_token_by_name(def_id, name) -> GET_FIELD/SET_FIELD"

requirements-completed: [EMIT-09, EMIT-10, EMIT-11, EMIT-21, EMIT-27]

# Metrics
duration: 10min
completed: 2026-03-03
---

# Phase 25 Plan 02: Call Dispatch and Object Model Summary

**Four call instructions (CALL/CALL_VIRT/CALL_EXTERN/CALL_INDIRECT) with consecutive argument packing, CALL_VIRT-to-CALL specialization for concrete receivers, struct/entity construction sequences, GET_FIELD/SET_FIELD, GET_COMPONENT, and BOX/UNBOX at generic call boundaries — 30 tests passing**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-03T02:37:08Z
- **Completed:** 2026-03-03T02:47:31Z
- **Tasks:** 2 (TDD: RED + GREEN per task)
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- `call.rs` created: `emit_call()` dispatches via `CallKind` enum (Direct/Virtual/Extern/Indirect), packing arguments into consecutive register blocks per EMIT-09
- CALL_VIRT specializes to CALL when receiver is `TyKind::Struct` or `TyKind::Entity` (EMIT-27 optimization)
- `emit_call_indirect()` for delegate/Func-typed callees
- `emit_box_if_needed()` / `emit_unbox_if_needed()` for generic boxing (EMIT-21)
- `analyze_callee()` decision tree extracts call kind from callee expression structure
- `emit_expr()` updated: `TypedExpr::Call` → inline arg packing + instruction dispatch; `TypedExpr::Field` → GET_FIELD; `TypedExpr::ComponentAccess` → GET_COMPONENT; `TypedExpr::New` → `emit_new()` helper
- `emit_new()`: dispatches on `TyKind::Entity` vs other for entity vs struct construction
- Entity construction follows spec §2.16.7 exactly: SPAWN_ENTITY → SET_FIELD(explicit fields only) → INIT_ENTITY
- `TypedExpr::Assign` with Field target → SET_FIELD
- `field_token_by_name()` added to ModuleBuilder for field index resolution
- `get_str()` added to StringHeap for reverse offset → string lookup
- `next()` and `type_of()` added to RegisterAllocator for argument packing
- All 30 tests pass (19 from Plan 01 + 11 new)

## Task Commits

Each task was committed atomically:

1. **RED tests: call dispatch and object model** - `6fa76e2` (test)
2. **GREEN: call dispatch + object model implementation** - `23d393f` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/body/call.rs` — Created: call dispatch module with CallKind, emit_call, emit_call_indirect, emit_box_if_needed, analyze_callee
- `writ-compiler/src/emit/body/mod.rs` — Added `pub mod call;`
- `writ-compiler/src/emit/body/expr.rs` — Replaced Call/Field/ComponentAccess/New/Assign-field Nop placeholders with real implementations
- `writ-compiler/src/emit/body/reg_alloc.rs` — Added `next()` (peek at next register index) and `type_of(reg)` (type of allocated register)
- `writ-compiler/src/emit/heaps.rs` — Added `get_str(offset)` reverse lookup for string heap
- `writ-compiler/src/emit/module_builder.rs` — Added `field_token_by_name()` and `contract_method_slot_by_def_id()` helpers
- `writ-compiler/tests/emit_body_tests.rs` — 11 new integration tests

## Decisions Made

- `emit_call()` takes explicit `DefId` + `CallKind` — callee analysis is separated from instruction emission, enabling tests to bypass the full pipeline
- `emit_expr()` Call handler uses inline arg packing without `emit_call()` to avoid requiring a DefId in the general dispatch path; the full pipeline always calls `emit_call()` directly with the known DefId
- `contract_idx` in CALL_VIRT is 0 as placeholder; full resolution requires impl→contract mapping from Phase 24 context, deferred to Plan 04 (full wiring pass)
- `field_token_by_name()` iterates `field_defs` post-finalize using `get_str()` — correct because `finalize()` sorts field_defs by parent
- `extract_callee_def_id_opt()` returns None since BodyEmitter holds no DefMap reference; `method_idx=0` for the general `emit_expr()` Call path

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `CallVirt` struct has `contract_idx` field not in plan interfaces**
- **Found during:** Task 1 implementation
- **Issue:** Plan's interface snippet omitted the `contract_idx: u32` field from `Instruction::CallVirt`; actual definition is `CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc }`
- **Fix:** Added `contract_idx: 0` as placeholder; documented as decision above
- **Files modified:** `call.rs`, `expr.rs`
- **Commit:** 23d393f

**2. [Rule 1 - Bug] `GetComponent` uses `comp_type_idx` not `comp_idx`**
- **Found during:** Task 2 implementation
- **Issue:** Plan interface snippet used `comp_idx` but actual field is `comp_type_idx`
- **Fix:** Corrected field name in expr.rs
- **Files modified:** `expr.rs`
- **Commit:** 23d393f

**3. [Rule 2 - Missing functionality] StringHeap missing reverse lookup**
- **Found during:** Task 2 — field_token_by_name() needs to compare field names
- **Issue:** `StringHeap` had no way to read a string back from its heap offset; `field_defs` store names as heap offsets
- **Fix:** Added `get_str(offset) -> &str` method to StringHeap
- **Files modified:** `heaps.rs`
- **Commit:** 23d393f

## Next Phase Readiness

- emit_expr() now handles Call, Field, ComponentAccess, New (all Plan 02 variants)
- Remaining Plan 02 Nop placeholders: Index (Plan 03), Match (Plan 04), Lambda (Plan 03), ArrayLit/Range/Spawn/Join/Cancel/Defer (Plan 03)
- No blockers for Plan 03 (closures, arrays, concurrency)

---
*Phase: 25-il-codegen-method-bodies*
*Completed: 2026-03-03*

## Self-Check: PASSED

- `writ-compiler/src/emit/body/call.rs`: FOUND
- `writ-compiler/src/emit/body/expr.rs`: FOUND (modified)
- `.planning/phases/25-il-codegen-method-bodies/25-02-SUMMARY.md`: FOUND
- Commit 6fa76e2 (RED tests): FOUND
- Commit 23d393f (GREEN implementation): FOUND
- 30 tests pass: confirmed (`cargo test -p writ-compiler --test emit_body_tests`)
