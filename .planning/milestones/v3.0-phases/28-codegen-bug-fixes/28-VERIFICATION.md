---
phase: 28-codegen-bug-fixes
verified: 2026-03-03T18:00:00Z
status: passed
score: 4/4 success criteria verified
re_verification: false
---

# Phase 28: Codegen Bug Fixes Verification Report

**Phase Goal:** All integration and flow bugs identified in the v3.0 audit are fixed; method call resolution produces correct DefIds; CALL_VIRT uses correct contract indices; range expressions and defer blocks emit correct instructions
**Verified:** 2026-03-03
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `extract_callee_def_id_opt()` returns correct DefIds for method calls (MC-01 fixed) | VERIFIED | `callee_def_id: Option<DefId>` added to `TypedExpr::Call`; `check_call_with_sig` and `check_generic_call` populate it with `Some(def_id)`; emit path reads `callee_def_id` directly — `extract_callee_def_id_opt` is retained as dead code but is no longer called on any main emission path |
| 2 | CALL_VIRT instructions emit correct `contract_idx` derived from contract declaration order (BF-01 fixed) | VERIFIED | `emit_expr` Call arm uses `callee_def_id.and_then(|id| emitter.builder.contract_token_for_method_def_id(id))` for `contract_idx`; `test_call_virt_via_emit_expr_uses_callee_def_id_for_contract_idx` confirms non-zero `contract_idx` when mapping is registered |
| 3 | `TypedExpr::Range` emits proper range instruction sequence instead of Nop (BF-02 fixed) | VERIFIED | `TypedExpr::Range` match arm dispatches to `emit_range()`; `emit_range()` emits `New + SetField(0) + SetField(1) + SetField(2) + SetField(3)`; `test_range_emits_new_and_set_field` asserts no Nop and exactly 4 SetField instructions |
| 4 | `DeferPush` emits correct `handler_offset` pointing to the defer block handler (BF-03 fixed) | VERIFIED | `emit_defer()` uses post-emission patching: records `defer_push_idx`, emits `DeferPop`, emits `Br` (skip handler), records `handler_start_idx`, emits handler body, emits `DeferEnd`, then patches `DeferPush.method_idx = handler_start_idx`; `test_defer_emits_correct_handler_offset` confirms `method_idx != 0` and that `instrs[method_idx]` is the handler body start |

**Score:** 4/4 success criteria verified

### Required Artifacts

#### Plan 28-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/check/ir.rs` | `TypedExpr::Call` with `callee_def_id: Option<DefId>` field | VERIFIED | Line 42: `callee_def_id: Option<DefId>` present in `TypedExpr::Call` variant |
| `writ-compiler/src/check/check_expr.rs` | `callee_def_id: Some(def_id)` in success paths | VERIFIED | Line 870: `callee_def_id: Some(def_id)` in `check_call_with_sig`; line 1017: `callee_def_id: Some(def_id)` in `check_generic_call`; all error/fallback paths use `callee_def_id: None` |
| `writ-compiler/src/emit/body/expr.rs` | Fixed `extract_callee_def_id_opt` and Call match arm using `callee_def_id` | VERIFIED | Line 174: `TypedExpr::Call { callee, ty, callee_def_id, .. }`; lines 239-242: `method_idx` derived from `callee_def_id`; line 193: `let maybe_def_id = *callee_def_id` |

#### Plan 28-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/body/expr.rs` | Fixed `emit_defer` with handler_idx patching and Range construction sequence | VERIFIED | `emit_range()` function at line 799 emits `New + 4x SetField`; `emit_defer()` at line 747 uses `defer_push_idx` patching; `SetField` pattern confirmed |
| `writ-compiler/src/emit/module_builder.rs` | `range_type_token()` helper for Range TypeRef resolution | VERIFIED | `range_type_token()` at line 1015 searches TypeRef entries by name "Range"; falls back to `0` if not registered (matching `ArrayInit elem_type=0` pattern) |
| `writ-compiler/tests/emit_body_tests.rs` | Tests for Range emission and DeferPush handler offset | VERIFIED | `test_range_emits_new_and_set_field` at line 2739; `test_range_inclusive_emits_load_true_for_end` at line 2805; `test_defer_emits_correct_handler_offset` at line 2847; `test_defer_handler_offset_matches_handler_start` at line 2903 |

### Key Link Verification

#### Plan 28-01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `check_expr.rs` | `ir.rs` | `TypedExpr::Call` construction with `callee_def_id` populated | VERIFIED | `callee_def_id: Some(def_id)` confirmed at lines 870 and 1017 in `check_expr.rs`, matching `TypedExpr::Call` variant definition in `ir.rs` |
| `emit/body/expr.rs` | `emit/module_builder.rs` | `token_for_def` and `contract_token_for_method_def_id` using real DefId | VERIFIED | Lines 239-242: `callee_def_id.and_then(|id| emitter.builder.token_for_def(id))`; lines 257-260: `callee_def_id.and_then(|id| emitter.builder.contract_token_for_method_def_id(id))` |

#### Plan 28-02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `emit/body/expr.rs` | `emit/module_builder.rs` | `range_type_token()` call for Range `type_idx` in `New` instruction | VERIFIED | Line 806: `let range_type_idx = emitter.builder.range_type_token();` confirmed |
| `emit/body/expr.rs` | instruction Vec | DeferPush instruction index patching via direct Vec mutation | VERIFIED | Lines 752-773: `defer_push_idx = emitter.instructions.len()`, then `emitter.instructions[defer_push_idx]` patched with `handler_start_idx`; pattern `instructions[defer_push` confirmed |

### Requirements Coverage

The plans declare EMIT-09, EMIT-12, EMIT-15, EMIT-27, and FIX-02 as the requirements affected by Phase 28. These IDs are marked complete in `REQUIREMENTS.md` (attributed to Phases 25 and 26) because Phase 28 _corrects bug fixes_ in previously-implemented requirements rather than introducing new ones. Phase 28 is not listed in the traceability table — this is expected and consistent with the ROADMAP describing Phase 28 as "fixes integration/flow issues **affecting**" those IDs, not as the primary implementation phase.

| Requirement | Plan | Description | Status | Evidence |
|-------------|------|-------------|--------|----------|
| EMIT-09 | 28-01 | CALL, CALL_VIRT, CALL_EXTERN, CALL_INDIRECT with correct dispatch selection | SATISFIED (bug fixed) | MC-01 and BF-01 resolved: `method_idx` and `contract_idx` now derive from `callee_def_id` |
| EMIT-27 | 28-01 | CALL_VIRT specializes to CALL for concrete static receiver types | SATISFIED (bug fixed) | `method_idx` now non-zero for registered callee DefIds; dispatch-kind selection was already correct |
| FIX-02 | 28-01 | Runtime resolves generic contract specialization without collision | SATISFIED (bug fixed) | `contract_idx` now uses `contract_token_for_method_def_id`; correct specialization token emitted |
| EMIT-12 | 28-02 | Compiler emits all 9 array instructions | SATISFIED (bug fixed) | BF-02: Range emission replaced Nop with proper `New + 4x SetField` struct construction sequence |
| EMIT-15 | 28-02 | Compiler emits SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH/POP/END | SATISFIED (bug fixed) | BF-03: `DeferPush.method_idx` now holds correct handler instruction index via post-emission patching |

No orphaned requirements: Phase 28 is not mapped in the REQUIREMENTS.md traceability table, but its affected IDs (EMIT-09, EMIT-12, EMIT-15, EMIT-27, FIX-02) are all accounted for.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `emit/body/expr.rs` | 1314-1324 | `extract_callee_def_id_opt` always returns `None` | Info | Dead code retained intentionally — no longer called on any main emit path; noted in SUMMARY.md decisions |
| `emit/body/expr.rs` | 722, 724 | `SpawnDetached`/`SpawnTask` with `method_idx: 0` in non-Call spawn fallback | Warning | Fallback path for `spawn` expressions whose inner expression is not a `Call`; pre-existing known limitation; unrelated to BF-03 scope |
| `emit/module_builder.rs` | 1013 | `range_type_token()` returns `0` fallback when no TypeRef found | Info | Matches existing pattern for `ArrayInit elem_type=0`; cross-module TypeRef wiring deferred to Phase 29 per explicit TODO comment |

No blocker anti-patterns found. The Range match arm no longer emits `Instruction::Nop`. The `emit_defer` DeferPush uses `method_idx: 0` only as a placeholder that is immediately patched.

### Human Verification Required

None. All success criteria are verifiable programmatically. The test suite at 89 emit_body_tests (all passing) covers the specific instruction-level behaviors.

### Test Suite Results

Full workspace test suite: **1,100+ tests, 0 failures, 0 errors**.

Key test groups passing:
- `emit_body_tests`: **89 passed** (includes 7 new tests added in Phase 28)
- New tests added this phase:
  - `test_range_emits_new_and_set_field` — asserts `New + 4x SetField`, no `Nop`
  - `test_range_inclusive_emits_load_true_for_end` — asserts `LoadTrue` for `field_idx=3` when `inclusive=true`
  - `test_defer_emits_correct_handler_offset` — asserts `method_idx != 0`, `instrs[method_idx]` is handler body
  - `test_defer_handler_offset_matches_handler_start` — verifies exact `DeferPush -> DeferPop -> Br -> handler` layout
  - `test_call_with_callee_def_id_emits_correct_method_idx` — asserts non-zero `method_idx` in `CALL` when `callee_def_id` is `Some`
  - `test_call_virt_via_emit_expr_uses_callee_def_id_for_contract_idx` — asserts non-zero `contract_idx` in `CALL_VIRT`
  - `test_call_with_none_callee_def_id_emits_zero_method_idx` — backward-compat: `callee_def_id=None` falls back to `method_idx=0`

### Summary

All four phase goals are achieved:

1. **MC-01 fixed**: `TypedExpr::Call` now carries `callee_def_id: Option<DefId>`; type checker populates it with `Some(def_id)` on success paths; emitter reads it directly — no `DefMap` access required at emission time.

2. **BF-01 fixed**: `CALL_VIRT` `contract_idx` is derived from `callee_def_id` via `contract_token_for_method_def_id`; the existing Phase 26-04 side table is now actually used.

3. **BF-02 fixed**: `TypedExpr::Range` dispatches to `emit_range()` which constructs a `Range<T>` struct via `New + SetField x4`; the `Nop` placeholder is gone. The `range_type_token()` returns 0 as a temporary fallback (cross-module TypeRef wiring deferred to Phase 29) — this is a known, documented limitation, not a regression.

4. **BF-03 fixed**: `emit_defer()` uses post-emission Vec patching to set `DeferPush.method_idx` to the correct handler instruction index; a `Br` instruction skips the handler on normal execution paths.

---

_Verified: 2026-03-03_
_Verifier: Claude (gsd-verifier)_
