---
phase: 85-code-generation
plan: 01
subsystem: writ-compiler/emit
tags: [codegen, contracts, CALL_VIRT, virtual-dispatch, IL-emit]
dependency_graph:
  requires:
    - phase: 84-02
      provides: contract-method-resolution, TyKind::Contract arm in check_member_access
  provides:
    - contract-receiver-CALL_VIRT-emission
    - contract_method_slot_by_name helper
    - end-to-end repro script validation
  affects: [writ-runtime, 86-lsp]
tech_stack:
  added: []
  patterns: [contract-typed-receiver-dispatch, name-based-slot-lookup]
key_files:
  created: []
  modified:
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/typecheck_tests.rs
key_decisions:
  - "Contract-receiver CALL_VIRT uses token_for_def(contract_def_id) directly — no register_impl_method_contract needed on this path"
  - "contract_method_slot_by_name uses slot position (enumerate) not stored row.slot field to match assign_vtable_slots behavior"
  - "Contract branch placed BEFORE Branch A (!is_static_call + Func-typed check) to prevent CALL_INDIRECT fallthrough"
  - "callee_def_id is None for contract method calls — lookup must go through contract_def_id + method name from receiver type"
patterns-established:
  - "Pattern: contract receiver detection — check TyKind::Contract on receiver.ty() before Branch A intercept in emit_expr"
  - "Pattern: name-based slot lookup via contract_method_slot_by_name for any CALL_VIRT on contract-typed receivers"
requirements-completed: [EMIT-01, EMIT-02, EMIT-04]

duration: 4min
completed: "2026-03-23T23:45:47Z"
---

# Phase 85 Plan 01: Contract-Typed Receiver CALL_VIRT Emission Summary

**CALL_VIRT emission for contract-typed receivers: contract_method_slot_by_name added to ModuleBuilder; TyKind::Contract branch intercepts before CALL_INDIRECT fallthrough; repro script complete-impl compiles, incomplete-impl catches E0123**

## Performance

- **Duration:** ~3 minutes
- **Started:** 2026-03-23T23:42:45Z
- **Completed:** 2026-03-23T23:45:47Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Added `contract_method_slot_by_name` to `ModuleBuilder` — looks up slot by contract DefId + method name using `contract_method_range` + slot position enumeration
- Added `TyKind::Contract` branch in `emit_expr` TypedExpr::Call arm before Branch A's `!is_static_call` intercept — emits `Instruction::CallVirt` with `contract_idx` from `token_for_def(contract_def_id)` and `slot` from `contract_method_slot_by_name`
- 3 new unit tests in `emit_body_tests.rs`: slot-by-name (first=0, second=1, nonexistent=None), CALL_VIRT vs CALL/CALL_INDIRECT, correct contract_idx/slot
- 2 repro script tests in `typecheck_tests.rs`: complete impl path compiles cleanly (EMIT-04), incomplete impl path produces E0123

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED):** `e6ecd1b` — test(85-01): add failing tests for contract-typed receiver CALL_VIRT
2. **Task 1 (GREEN):** `bf24287` — feat(85-01): emit CALL_VIRT for contract-typed receivers (EMIT-01, EMIT-02)
3. **Task 2:** `a0b7ddd` — test(85-01): add end-to-end repro script tests for EMIT-04

## Files Created/Modified

- `writ-compiler/src/emit/module_builder.rs` — Added `contract_method_slot_by_name` method (uses enumerate over contract_method_range for slot position)
- `writ-compiler/src/emit/body/expr/mod.rs` — Added `TyKind::Contract` block before Branch A in TypedExpr::Call arm; emits `Instruction::CallVirt` with correct contract_idx and slot
- `writ-compiler/tests/emit_body_tests.rs` — Added 3 unit tests + `make_builder_with_contract_receiver` helper
- `writ-compiler/tests/typecheck_tests.rs` — Added 2 repro script end-to-end tests (EMIT-04)

## Decisions Made

1. **Name-based slot lookup** — The research noted `callee_def_id` is `None` for contract method calls (type checker doesn't thread it through). Resolution: use `receiver.ty()` to extract `contract_def_id`, then `token_for_def` for the contract token and `contract_method_slot_by_name` for the slot. No dependency on `register_impl_method_contract`.

2. **Enumerate over range for slot** — The implementation uses `range.enumerate()` to get the slot position rather than reading `row.slot` directly. This matches `assign_vtable_slots` behavior (slot = 0-based position in range) and is correct whether `finalize` has run or not.

3. **Placement before Branch A** — The contract receiver block is placed before Branch A's `!is_static_call && Func-typed` check. If placed inside Branch A, `extract_type_def_id` returns `None` for `TyKind::Contract` and falls through to `emit_call_indirect`. The pre-Branch-A placement avoids this entirely.

## Deviations from Plan

None — plan executed exactly as written. The `ret: Box::new(ty_void)` type in the plan's test snippets used `Box` (incorrect for `TyKind::Func` which takes `ret: Ty`), caught during initial compile and corrected inline.

## Known Stubs

None — all features fully implemented. CALL_VIRT emission for contract-typed receivers is complete.

## Self-Check

### Files Exist Check
- writ-compiler/src/emit/module_builder.rs — exists
- writ-compiler/src/emit/body/expr/mod.rs — exists
- writ-compiler/tests/emit_body_tests.rs — exists
- writ-compiler/tests/typecheck_tests.rs — exists

### Commits Exist Check
- e6ecd1b — test(85-01): add failing tests
- bf24287 — feat(85-01): emit CALL_VIRT
- a0b7ddd — test(85-01): repro script tests

## Self-Check: PASSED
