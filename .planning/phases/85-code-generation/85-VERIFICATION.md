---
phase: 85-code-generation
verified: 2026-03-24T00:00:00Z
status: passed
score: 3/3 must-haves verified
re_verification: false
---

# Phase 85: Code Generation Verification Report

**Phase Goal:** Method calls on contract-typed receivers emit CALL_VIRT with correct contract_idx and slot, and the end-to-end repro script compiles and runs correctly
**Verified:** 2026-03-24T00:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                                      | Status     | Evidence                                                                                          |
| --- | ---------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------- |
| 1   | Method calls on contract-typed receivers emit CALL_VIRT (not CALL or CALL_INDIRECT)                       | VERIFIED   | `TyKind::Contract` block in `expr/mod.rs:275-310` emits `Instruction::CallVirt`; test `test_contract_receiver_emits_call_virt` passes and asserts no CALL or CALL_INDIRECT |
| 2   | CALL_VIRT carries the correct contract_idx (matching contract MetadataToken) and slot (0-based position)  | VERIFIED   | `contract_method_slot_by_name` (module_builder.rs:1078) uses enumerate over range for 0-based slot; test `test_contract_receiver_call_virt_correct_idx_and_slot` asserts non-zero contract_idx and slot=1 for second method |
| 3   | The repro script with a complete impl compiles without errors; the incomplete impl path produces E0123     | VERIFIED   | `test_contract_receiver_repro_complete_impl` and `test_contract_receiver_repro_incomplete_impl` both pass in typecheck_tests.rs |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact                                             | Expected                                               | Status     | Details                                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------ | ---------- | -------------------------------------------------------------------------------------------- |
| `writ-compiler/src/emit/module_builder.rs`           | `contract_method_slot_by_name` helper                  | VERIFIED   | Function exists at line 1078; uses `enumerate` over `contract_method_range` for slot position; returns `Option<u16>` |
| `writ-compiler/src/emit/body/expr/mod.rs`            | `TyKind::Contract` branch in TypedExpr::Call dispatch  | VERIFIED   | Block at lines 270-310; placed before Branch A's `!is_static_call` + Func-typed check; emits `Instruction::CallVirt` with `contract_token` and `slot` |
| `writ-compiler/tests/emit_body_tests.rs`             | Unit tests for CALL_VIRT on contract-typed receivers   | VERIFIED   | `test_contract_method_slot_by_name`, `test_contract_receiver_emits_call_virt`, `test_contract_receiver_call_virt_correct_idx_and_slot` all present and passing |
| `writ-compiler/tests/typecheck_tests.rs`             | End-to-end repro script tests for EMIT-04              | VERIFIED   | `test_contract_receiver_repro_complete_impl` and `test_contract_receiver_repro_incomplete_impl` both present and passing |

### Key Link Verification

| From                                              | To                              | Via                                          | Status  | Details                                                                    |
| ------------------------------------------------- | ------------------------------- | -------------------------------------------- | ------- | -------------------------------------------------------------------------- |
| `writ-compiler/src/emit/body/expr/mod.rs`         | `writ-compiler/src/emit/module_builder.rs` | `contract_method_slot_by_name` and `token_for_def` calls | WIRED   | Lines 289-293 call `emitter.builder.token_for_def(contract_def_id)` and `emitter.builder.contract_method_slot_by_name(contract_def_id, field)` |
| `writ-compiler/src/emit/body/expr/mod.rs`         | `writ-module (Instruction::CallVirt)` | CALL_VIRT emission for contract-typed receivers | WIRED   | Line 299 emits `Instruction::CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc }` |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces a compiler code-generation path, not a UI component rendering dynamic data. The data flow is verified by the unit and integration tests (slot lookup → CallVirt emission → test assertions).

### Behavioral Spot-Checks

| Behavior                                                  | Command                                                                                      | Result                                    | Status  |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------- | ------- |
| 3 new emit_body_tests contract tests pass                 | `cargo test -p writ-compiler --test emit_body_tests -- contract`                             | 7 passed; 0 failed                        | PASS    |
| 2 repro script typecheck tests pass                       | `cargo test -p writ-compiler --test typecheck_tests -- test_contract_receiver_repro`         | 2 passed; 0 failed                        | PASS    |
| Full compiler test suite passes with no regressions       | `cargo test -p writ-compiler`                                                                | 90 passed; 0 failed                       | PASS    |

### Requirements Coverage

| Requirement | Source Plan | Description                                                                               | Status    | Evidence                                                                                  |
| ----------- | ----------- | ----------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------- |
| EMIT-01     | 85-01       | Method calls on contract-typed receivers emit CALL_VIRT (not CALL)                       | SATISFIED | `TyKind::Contract` block in `expr/mod.rs:277` confirmed; `test_contract_receiver_emits_call_virt` passes |
| EMIT-02     | 85-01       | CALL_VIRT carries correct contract_idx and slot for contract-typed dispatch               | SATISFIED | `contract_method_slot_by_name` uses enumerate for 0-based slot; `test_contract_receiver_call_virt_correct_idx_and_slot` asserts correct values |
| EMIT-04     | 85-01       | The original 5-bug repro script compiles and runs correctly (implementedFunc succeeds, notImplementedFunc caught by E0123) | SATISFIED | Both `test_contract_receiver_repro_complete_impl` and `test_contract_receiver_repro_incomplete_impl` pass |

**Notes on EMIT-03:** EMIT-03 (E0122 removal) is tracked in REQUIREMENTS.md as belonging to Phase 84, not Phase 85. It is not declared in the 85-01-PLAN frontmatter `requirements` field and is not a gap for this phase.

### Anti-Patterns Found

| File                                              | Line | Pattern                         | Severity | Impact                                                                                                 |
| ------------------------------------------------- | ---- | ------------------------------- | -------- | ------------------------------------------------------------------------------------------------------ |
| `writ-compiler/src/emit/body/expr/mod.rs`         | 291  | `unwrap_or(0)` for contract_token | Info   | Defensive fallback for unregistered contract; normal execution path yields non-zero token (verified by test) |
| `writ-compiler/src/emit/body/expr/mod.rs`         | 293  | `unwrap_or(0)` for slot          | Info    | Defensive fallback for unknown method name; normal execution path yields correct slot (verified by test) |

Neither `unwrap_or(0)` is a blocker. Both are consistent with the existing pattern used elsewhere in the same file (lines 181, 384, 397, 433, 522). The test `test_contract_receiver_call_virt_correct_idx_and_slot` asserts the non-zero path is taken on the happy path.

### Human Verification Required

None. All observable behaviors are verifiable programmatically:

- CALL_VIRT emission is confirmed by unit tests that inspect `emitter.instructions` directly.
- Correct contract_idx and slot are asserted by value in tests.
- The E0123 diagnostic path is confirmed by the typecheck repro tests.

### Gaps Summary

No gaps found. All three must-have truths are verified, all four artifacts are substantive and wired, both key links are active, and all three requirement IDs (EMIT-01, EMIT-02, EMIT-04) are fully satisfied. The full `cargo test -p writ-compiler` suite passes with 90 tests and zero failures.

---

_Verified: 2026-03-24T00:00:00Z_
_Verifier: Claude (gsd-verifier)_
