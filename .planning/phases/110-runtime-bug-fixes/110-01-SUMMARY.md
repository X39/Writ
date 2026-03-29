---
phase: 110-runtime-bug-fixes
plan: 01
subsystem: writ-golden, writ-cli, writ-compiler
tags: [testing, golden-tests, e2e, rt-02, rt-03, regression-anchor]
dependency_graph:
  requires: []
  provides: [RT-02 golden anchor, RT-02 E2E assertion, RT-03 regression test, RT-03 golden anchor]
  affects: [writ-golden, writ-cli]
tech_stack:
  added: []
  patterns: [run_golden_test(), compile_source()+RuntimeBuilder round-trip, BLESS=1 snapshot workflow]
key_files:
  created:
    - writ-golden/tests/golden/expr_str_len.writ
    - writ-golden/tests/golden/expr_str_len.writil
    - writ-golden/tests/golden/fn_multi_choice.writ
    - writ-golden/tests/golden/fn_multi_choice.writil
  modified:
    - writ-golden/tests/golden_tests.rs
    - writ-cli/tests/e2e_compile_tests.rs
    - writ-golden/tests/golden/dlg_fn_mix.writ
    - writ-golden/tests/golden/dlg_fn_mix.writil
decisions:
  - "RT-03 (::choice serialization) was already fixed as a side effect of Phase 109 closure capture work — no serialize.rs changes required; only regression tests added"
  - "test_multi_fn_choice_round_trip uses entity + helper fn + 1 lambda; the stale dlg_fn_mix comment is updated to reflect RT-03 is now tested"
metrics:
  duration: 6 minutes
  completed: 2026-03-29
  tasks_completed: 2
  files_created: 4
  files_modified: 4
---

# Phase 110 Plan 01: Runtime Bug Fixes (RT-02 + RT-03) Summary

**One-liner:** E2E golden tests and round-trip assertions for `s.len()` byte count (RT-02) and multi-fn `::choice` lambda serialization (RT-03), including discovery that RT-03 was already fixed by Phase 109 closure work.

## Tasks Completed

### Task 1: Add E2E golden test for s.len() byte length (RT-02)

- Created `writ-golden/tests/golden/expr_str_len.writ` with `pub fn main() -> int { let s: string = "hello"; s.len() }`
- Blessed `expr_str_len.writil` golden snapshot (shows `LOAD_STRING r0, ... / STR_LEN r1, r0 / RET r1`)
- Registered `test_expr_str_len()` in `writ-golden/tests/golden_tests.rs` Section D
- Added `test_str_len_returns_byte_length()` in `writ-cli/tests/e2e_compile_tests.rs` asserting `Value::Int(5)` for `"hello".len()`

**Commit:** `82e083b` — feat(110-01): add s.len() E2E golden test and compile+run assertion (RT-02)

### Task 2: Fix ::choice serialization in multi-function modules and add regression test (RT-03)

During bug reproduction investigation, all test patterns passed immediately — including the complex multi-function + enum + entity + 3-lambda pattern from the original `quest_system.writ` crash. Code analysis confirms Phase 109 (closure capture) reworked `emit_all_bodies` and `pre_scan_lambdas` in a way that stabilized the orphaned MethodDef-to-body matching. The `serialize.rs` positional cursor still exists but no longer produces ordering mismatches.

Actions taken:
- Created `writ-golden/tests/golden/fn_multi_choice.writ` (entity + helper fn + choice lambda)
- Blessed `fn_multi_choice.writil` golden snapshot
- Registered `test_fn_multi_choice()` in `golden_tests.rs`
- Added `test_multi_fn_choice_round_trip()` E2E test asserting `Value::Int(42)` via full compile+run
- Updated `dlg_fn_mix.writ` line 3 comment from "avoids known ::choice serialization bug" to "choice serialization tested in fn_multi_choice.writ"
- Re-blessed `dlg_fn_mix.writil` after comment change

**Commit:** `51b5b82` — feat(110-01): add ::choice multi-fn golden test and update dlg_fn_mix comment (RT-03)

## Deviations from Plan

### Auto-discovered: RT-03 already fixed by Phase 109

**Found during:** Task 2 reproduction attempt
**Issue:** The plan expected to fix the orphaned body matching positional cursor in `serialize.rs`. When testing the reproduction pattern (entity + enums + multiple functions + 3 choice lambdas), all cases passed without any serializer changes.
**Root cause (post-analysis):** Phase 109 reworked `pre_scan_lambdas` and `emit_all_bodies` to add capture analysis. The lambda MethodDefs are now added to `__closure_N` TypeDefs whose parent indices appear after all user-defined TypeDefs in the MethodDef sort. Bodies are emitted in the same relative order. The positional cursor in `serialize.rs` still works because the two orderings match.
**Fix applied:** None needed — registered tests as regression anchors without serializer changes.
**Classified as:** [Rule 1 - Bug confirmed-fixed] — the accept criteria required the test to pass, which it does.

## Acceptance Criteria Verification

| Criterion | Status |
|-----------|--------|
| `writ-golden/tests/golden/expr_str_len.writ` exists and contains `s.len()` | PASS |
| `writ-golden/tests/golden/expr_str_len.writil` exists (blessed snapshot) | PASS |
| `golden_tests.rs` contains `fn test_expr_str_len()` | PASS |
| `golden_tests.rs` contains `run_golden_test("expr_str_len")` | PASS |
| `e2e_compile_tests.rs` contains `fn test_str_len_returns_byte_length()` | PASS |
| `e2e_compile_tests.rs` contains `Value::Int(5)` | PASS |
| `writ-golden/tests/golden/fn_multi_choice.writ` exists and contains `::choice` | PASS |
| `writ-golden/tests/golden/fn_multi_choice.writ` contains `entity Narrator` | PASS |
| `writ-golden/tests/golden/fn_multi_choice.writil` exists (blessed snapshot) | PASS |
| `golden_tests.rs` contains `fn test_fn_multi_choice()` | PASS |
| `dlg_fn_mix.writ` does NOT contain "avoids known ::choice serialization bug" | PASS |
| `e2e_compile_tests.rs` contains `fn test_multi_fn_choice_round_trip()` | PASS |
| `e2e_compile_tests.rs` contains `"module should deserialize without UnexpectedEof"` | PASS |
| `cargo test -p writ-golden test_expr_str_len` exits 0 | PASS |
| `cargo test -p writ-cli test_str_len_returns_byte_length` exits 0 | PASS |
| `cargo test -p writ-cli test_multi_fn_choice_round_trip` exits 0 | PASS |
| `cargo test -p writ-golden test_fn_multi_choice` exits 0 | PASS |
| `cargo test` exits 0 (no regressions) | PASS |

## Self-Check: PASSED

Files verified:
- `writ-golden/tests/golden/expr_str_len.writ` — exists
- `writ-golden/tests/golden/expr_str_len.writil` — exists
- `writ-golden/tests/golden/fn_multi_choice.writ` — exists
- `writ-golden/tests/golden/fn_multi_choice.writil` — exists

Commits verified:
- `82e083b` — exists (feat(110-01): add s.len() E2E golden test)
- `51b5b82` — exists (feat(110-01): add ::choice multi-fn golden test)
