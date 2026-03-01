---
phase: 28-codegen-bug-fixes
plan: "01"
subsystem: codegen
tags: [compiler, ir, codegen, call-dispatch, defid, method_idx, contract_idx]

# Dependency graph
requires:
  - phase: 26-codegen
    provides: "emit_call/emit_call_indirect/CALL_VIRT emission; ModuleBuilder token_for_def and contract_token_for_method_def_id APIs"
  - phase: 23-typecheck
    provides: "TypedExpr::Call variant and check_call_with_sig/check_generic_call with resolved DefIds"
provides:
  - "TypedExpr::Call carries callee_def_id: Option<DefId> across the full pipeline"
  - "CALL/CALL_VIRT/TailCall/SpawnTask emit correct method_idx and contract_idx from callee_def_id"
  - "MC-01 bug fixed: method_idx no longer always 0 on emit_expr path"
  - "BF-01 bug fixed: contract_idx no longer always 0 on emit_expr path"
affects: [codegen, runtime-dispatch, writ-runtime]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "callee_def_id propagation: check_call_with_sig/check_generic_call thread DefId into TypedExpr::Call.callee_def_id; emit_expr reads it directly without needing DefMap reference"
    - "emit_tail_call signature extended with callee_def_id: Option<DefId> parameter replacing extract_callee_def_id_opt stub"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_expr.rs
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-compiler/tests/emit_body_tests.rs

key-decisions:
  - "callee_def_id field added to TypedExpr::Call after args field; ty() and span() methods already used .. so no changes needed there"
  - "extract_callee_def_id_opt retained but no longer used in main paths — kept as dead code for potential future use"
  - "emit_tail_call extended with explicit callee_def_id parameter rather than passing the sub-callee expression; callee parameter retained for signature compatibility"
  - "All error/arity-mismatch/Func-typed callee paths use callee_def_id: None (correct: no resolved fn DefId in those paths)"

patterns-established:
  - "DefId propagation pattern: type checker threads resolved DefIds into TypedExpr nodes; code emitter reads them directly, never needs DefMap reference at emission time"

requirements-completed:
  - EMIT-09
  - EMIT-27
  - FIX-02

# Metrics
duration: 6min
completed: 2026-03-03
---

# Phase 28 Plan 01: Codegen Bug Fixes (MC-01 + BF-01) Summary

**`callee_def_id: Option<DefId>` field added to `TypedExpr::Call`; check_call_with_sig/check_generic_call populate it during type checking; emit_expr, emit_tail_call, and emit_spawn use it for correct CALL/CALL_VIRT method_idx and contract_idx**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-03-03T17:16:56Z
- **Completed:** 2026-03-03T17:22:34Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Fixed MC-01: CALL and TailCall instructions now emit non-zero method_idx when the callee has a registered token (previously always 0 on the emit_expr path)
- Fixed BF-01: CALL_VIRT instructions now emit correct contract_idx from callee_def_id when the method has a registered impl-method-to-contract mapping
- Added 3 new tests verifying MC-01 and BF-01 behavior plus backward-compat fallback to method_idx=0 when callee_def_id=None
- All 89 emit_body_tests pass; full workspace test suite passes with 0 failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Add callee_def_id to TypedExpr::Call and populate during type checking** - `dc1bde8` (feat)
2. **Task 2: Fix emit_expr Call dispatch to use callee_def_id for method_idx and contract_idx** - `d329811` (feat)

**Plan metadata:** (to be added by final commit)

## Files Created/Modified
- `writ-compiler/src/check/ir.rs` - Added `callee_def_id: Option<DefId>` field to `TypedExpr::Call` variant
- `writ-compiler/src/check/check_expr.rs` - Populated `callee_def_id: Some(def_id)` in check_call_with_sig and check_generic_call success paths; `None` in error/fallback paths
- `writ-compiler/src/emit/body/expr.rs` - Updated Call arm to destructure callee_def_id; updated emit_tail_call signature; updated emit_spawn to use callee_def_id
- `writ-compiler/src/emit/body/stmt.rs` - Updated Return statement tail-call detection to pass callee_def_id to emit_tail_call
- `writ-compiler/tests/emit_body_tests.rs` - Added callee_def_id: None to all existing Call construction sites; added 3 new tests for MC-01/BF-01

## Decisions Made
- `extract_callee_def_id_opt` retained as dead code rather than removed — it may be useful for debugging or future inspection of callee sub-expressions
- `emit_tail_call` callee parameter retained despite no longer being used for DefId resolution — kept for potential future use or debugging
- Error/arity-mismatch Call construction sites use `callee_def_id: None` — correct because no resolved DefId is available on those error paths

## Deviations from Plan

None - plan executed exactly as written. The TDD RED phase test commits (`39201d6`, `ecd5d07`) pre-existed and had already implemented BF-02 (Range construction) and BF-03 (DeferPush handler offset), so those tests went from failing to passing as a bonus outcome of the pre-implementation.

## Issues Encountered
- The prior research phase commit (`ecd5d07`) had already pre-implemented `expr.rs` changes (emit_range, emit_defer patching, and the exact callee_def_id refactoring in emit_expr and emit_tail_call). This meant my Task 2 changes to `expr.rs` were already committed, and my `git add` for that file was a no-op. The `stmt.rs` changes and new tests were the only files committed in my Task 2 commit.
- Python script used to patch test construction sites correctly added `callee_def_id: None` to 23 of 26 TypedExpr::Call sites; 3 remaining edge cases (nested inside Return/Spawn nodes) required manual edits.

## Next Phase Readiness
- MC-01 and BF-01 are resolved; CALL/CALL_VIRT/TailCall/SpawnTask emit correct method tokens
- BF-02 (Range construction) and BF-03 (DeferPush handler offset) were also resolved by the pre-implementation commits
- Next: Plan 28-02 (BF-02/BF-03 if not already done, or other remaining codegen bugs)

---
*Phase: 28-codegen-bug-fixes*
*Completed: 2026-03-03*
