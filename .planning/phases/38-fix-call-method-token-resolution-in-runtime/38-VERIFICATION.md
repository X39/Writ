---
phase: 38-fix-call-method-token-resolution-in-runtime
verified: 2026-03-04T22:15:00Z
status: passed
score: 3/3 must-haves verified
re_verification: false
---

# Phase 38: Fix CALL Method Token Resolution in Runtime — Verification Report

**Phase Goal:** Running any compiled `.writil` file that contains function calls succeeds — the VM resolves the MethodDef metadata token in `CALL` operands to the correct in-module method index instead of treating the raw token value as an array index

**Requirement:** BUG-17

**Verified:** 2026-03-04T22:15:00Z

**Status:** PASSED — All must-haves verified, goal achieved

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | All existing writ-runtime vm_tests pass after the fix | ✓ VERIFIED | `cargo test -p writ-runtime --test vm_tests`: 78/78 passed including `call_and_ret_delivers_return_value`, `nested_calls_unwind_correctly`, `tail_call_does_not_grow_stack`, `new_delegate_and_call_indirect`, and new `call_with_methoddef_token` |
| 2 | VM correctly resolves MethodDef metadata tokens to 0-based method body indices | ✓ VERIFIED | `decode_method_token()` helper extracts row_index via `token & 0x00FF_FFFF`, converts to 0-based by subtracting 1; applied in CALL (line 517), TailCall (line 722), NewDelegate (line 682), SpawnTask (line 1283), SpawnDetached (line 1300); regression test `call_with_methoddef_token` uses token 0x07000002 and verifies correct method execution |
| 3 | The "call to invalid method index" error no longer occurs | ✓ VERIFIED | vm_tests pass with proper MethodDef tokens (0x07000002, 0x07000003); no crash occurs when executing CALL instructions with tokens; all 7 writ-golden tests pass (fn_recursion, fn_basic_call, fn_typed_params, fn_empty_main) |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `writ-runtime/src/dispatch.rs` | CALL/TailCall/NewDelegate/SpawnTask/SpawnDetached handlers with metadata token decoding | ✓ VERIFIED | Function `decode_method_token(token: u32) -> Option<usize>` defined at line 150; decodes via `token & 0x00FF_FFFF - 1` as specified in PLAN; all five instruction handlers updated to call `decode_method_token()` before array access |
| `writ-runtime/tests/vm_tests.rs` | Updated CALL tests using proper MethodDef metadata tokens | ✓ VERIFIED | Lines 778-834: 5 existing tests updated with tokens (0x07000002, 0x07000003); new regression test `call_with_methoddef_token()` at line 807 explicitly demonstrates token decoding |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `dispatch.rs::decode_method_token()` | token value input | `token & 0x00FF_FFFF - 1` | ✓ WIRED | Function correctly extracts 1-based row_index from bits 23-0 and converts to 0-based array index |
| `dispatch.rs::Instruction::Call` handler | `module.decoded_bodies[0-based-idx]` | `decode_method_token(method_idx)` | ✓ WIRED | Line 517: `let method_idx = match decode_method_token(method_idx) { Some(idx) => idx, None => crash };` then line 524 accesses `module.module.method_bodies[method_idx]` |
| `dispatch.rs::Instruction::TailCall` handler | `module.decoded_bodies[0-based-idx]` | `decode_method_token(method_idx)` | ✓ WIRED | Line 722-726: token decoded before array access at line 729 `module.module.method_bodies[method_idx]` |
| `dispatch.rs::Instruction::NewDelegate` handler | `heap.alloc_delegate(decoded_idx, target)` | `decode_method_token(method_idx)` | ✓ WIRED | Line 682: token decoded to `decoded_idx`, passed to `heap.alloc_delegate()` at line 686 |
| `dispatch.rs::Instruction::SpawnTask` handler | `ExecutionResult::SpawnChild { method_idx: decoded_idx }` | `decode_method_token(method_idx)` | ✓ WIRED | Line 1283: token decoded to `decoded_idx`, returned in ExecutionResult at line 1294 |
| `dispatch.rs::Instruction::SpawnDetached` handler | `ExecutionResult::SpawnDetachedTask { method_idx: decoded_idx }` | `decode_method_token(method_idx)` | ✓ WIRED | Line 1300: token decoded to `decoded_idx`, returned in ExecutionResult at line 1311 |
| `writ-compiler/src/emit/body/call.rs::emit_call()` | `dispatch.rs::Instruction::Call { method_idx }` | `builder.token_for_def(callee_def_id)` | ✓ VERIFIED | Compiler already emits MethodDef tokens via `token_for_def()` (verified in earlier phases); runtime now correctly decodes them |

### Requirements Coverage

| Requirement | Phase | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| BUG-17 | 38 | `CALL` instruction method token (`0x07xxxxxx`) is resolved to the correct in-module method index at runtime — the VM does not crash with "call to invalid method index" when executing any compiled .writil file that contains function calls | ✓ SATISFIED | `decode_method_token()` decodes token bits to array index; all vm_tests pass; regression test `call_with_methoddef_token` explicitly verifies end-to-end correctness |

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
| --- | --- | --- | --- |
| (none in modified code) | — | — | — |

Pre-existing placeholders in dispatch.rs (lines for string slicing) are unrelated to BUG-17 and were not modified.

### Wiring Verification Summary

All five method-dispatch instructions now follow the same pattern:
1. Receive raw MethodDef token in `method_idx` field from IL instruction
2. Call `decode_method_token(method_idx)` to extract 1-based row_index and convert to 0-based
3. Return `None` (crash) if token is null (row_index = 0)
4. Use decoded index to access `module.decoded_bodies[]` or delegate to heap allocation

**Note on CallIndirect:** Not updated — delegate heap objects already store 0-based indices (decoded by NewDelegate at creation time). CallIndirect reads the pre-decoded index from the heap object.

**Note on CallExtern:** Not updated — ExternDef tokens use table_id=16, handled through separate extern resolution path.

## Summary

**Phase 38 Goal: ACHIEVED**

The runtime VM's method dispatch layer now correctly handles MethodDef metadata tokens emitted by the compiler. All five method-dispatch instructions (CALL, TailCall, NewDelegate, SpawnTask, SpawnDetached) decode tokens to 0-based indices before array access.

**Evidence:**
- `decode_method_token()` helper function implemented and used at all 5 dispatch points
- 78 vm_tests pass (including new regression test with explicit token 0x07000002)
- 7 golden tests pass (fn_recursion, fn_basic_call, fn_typed_params, fn_empty_main, and 3 harness tests)
- No crashes with "call to invalid method index" when executing CALL instructions
- BUG-17 requirement satisfied

**Test Results:**
```
cargo test -p writ-runtime --test vm_tests: 78/78 passed
cargo test -p writ-golden: 7/7 passed
```

---

_Verified: 2026-03-04T22:15:00Z_

_Verifier: Claude (gsd-verifier)_
