---
phase: 37-fix-spurious-void-register-in-empty-function-bodies
plan: "01"
subsystem: compiler
tags: [writ-compiler, emit, IL, golden-tests, registers]

# Dependency graph
requires:
  - phase: 31.2-fix-register-convention-debug-info-and-parser-bugs
    provides: Golden test infrastructure and register allocation baseline

provides:
  - BUG-16 fixed: empty void blocks return dummy register 0 without allocating a .reg declaration
  - Re-blessed golden files for fn_empty_main, fn_basic_call, fn_recursion, fn_typed_params

affects:
  - any future phase that reads or extends emit/body/expr.rs Block arm

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Void-block short-circuit: return 0 (dummy register) for Ty(4) void blocks instead of alloc_void_reg()"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/body/expr.rs
    - writ-golden/tests/golden/fn_empty_main.expected
    - writ-golden/tests/golden/fn_basic_call.expected
    - writ-golden/tests/golden/fn_recursion.expected
    - writ-golden/tests/golden/fn_typed_params.expected

key-decisions:
  - "BUG-16: return 0 (not alloc_void_reg) for void blocks — caller emits RetVoid without using the register, so no .reg declaration is needed"
  - "Fix applies to both empty-block arm and the Let/While/etc tail arm — both produce void blocks that should not allocate registers"
  - "fn_recursion and fn_typed_params also had the spurious void register bug in their main() bodies (not mentioned in plan) — auto-fixed and re-blessed as same-rule deviation"

patterns-established:
  - "Void-block dummy register: when a block's ty == Ty(4) and the return value is unused, return 0 to skip register table entry"

requirements-completed:
  - BUG-16

# Metrics
duration: 8min
completed: 2026-03-04
---

# Phase 37 Plan 01: Fix Spurious Void Register in Empty Function Bodies Summary

**Empty void blocks now emit zero registers by returning dummy register 0 instead of calling alloc_void_reg(), eliminating spurious `.reg r0 void` declarations from IL output**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-04T19:50:25Z
- **Completed:** 2026-03-04T19:58:00Z
- **Tasks:** 1
- **Files modified:** 5

## Accomplishments

- Fixed BUG-16: Block emitter now captures `ty` and returns `0` for void blocks instead of calling `alloc_void_reg()`
- Re-blessed `fn_empty_main.expected` — method body starts directly with `RET_VOID`, no `.reg r0 void`
- Re-blessed `fn_basic_call.expected` — `greet` method body starts directly with `RET_VOID`; `main` still retains `.reg r0 void` (legitimately used as CALL destination)
- Also fixed `fn_recursion` and `fn_typed_params` which had the same spurious void reg in their `main()` void bodies
- All 7 golden tests pass; full workspace test suite (500+ tests) passes with zero failures

## Task Commits

1. **Task 1: Fix Block emitter — skip void reg allocation for empty void blocks** - `9c82c45` (fix)

## Files Created/Modified

- `writ-compiler/src/emit/body/expr.rs` - Captured `ty` in Block destructuring; return `0` for `Ty(4)` void blocks in both tail arms
- `writ-golden/tests/golden/fn_empty_main.expected` - Re-blessed: `.reg r0 void` removed
- `writ-golden/tests/golden/fn_basic_call.expected` - Re-blessed: `.reg r0 void` removed from `greet`; `main` unchanged
- `writ-golden/tests/golden/fn_recursion.expected` - Re-blessed: `.reg r2 void` removed from `main`
- `writ-golden/tests/golden/fn_typed_params.expected` - Re-blessed: `.reg r4 void` removed from `main`

## Decisions Made

- Return `0` (dummy) for void blocks rather than any allocated register — the caller in `mod.rs` checks `body.ty() == Ty(4)` and emits `RetVoid` without referencing the result register at all, so no `.reg` declaration is needed
- Apply the fix to both void-block arms: the empty-block arm (`else { alloc_void_reg() }`) and the non-expression-tail arm (block ending with `Let`/`While`/etc)
- fn_recursion and fn_typed_params: both had the same bug in their void `main()` bodies — auto-fixed and re-blessed in same commit (Rule 1: bug in same fix site)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Re-blessed fn_recursion and fn_typed_params (not in plan scope)**
- **Found during:** Task 1 (running cargo test -p writ-golden after fix)
- **Issue:** Plan listed only fn_empty_main and fn_basic_call as needing re-blessing; however fn_recursion and fn_typed_params also failed because their void `main()` bodies had the same spurious void register (`.reg r2 void` and `.reg r4 void` respectively)
- **Fix:** Ran `BLESS=1 cargo test -p writ-golden test_fn_recursion` and `BLESS=1 cargo test -p writ-golden test_fn_typed_params` to re-bless them
- **Files modified:** writ-golden/tests/golden/fn_recursion.expected, writ-golden/tests/golden/fn_typed_params.expected
- **Verification:** All 7 golden tests pass after blessing
- **Committed in:** 9c82c45 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - same bug, additional affected files)
**Impact on plan:** Necessary — the fix correctly removes the spurious void register from all void block contexts, which also affected the `main()` method in fn_recursion and fn_typed_params. No scope creep.

## Issues Encountered

None — fix was straightforward, compiled on first attempt.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- BUG-16 is fully resolved
- All 7 golden tests pass with correct, spec-compliant IL
- IL for empty void functions now matches the spec: no register declarations for registers that are never referenced
- No known remaining blockers

## Self-Check: PASSED

- FOUND: writ-compiler/src/emit/body/expr.rs
- FOUND: writ-golden/tests/golden/fn_empty_main.expected
- FOUND: writ-golden/tests/golden/fn_basic_call.expected
- FOUND: writ-golden/tests/golden/fn_recursion.expected
- FOUND: writ-golden/tests/golden/fn_typed_params.expected
- FOUND: .planning/phases/37-fix-spurious-void-register-in-empty-function-bodies/37-01-SUMMARY.md
- FOUND commit: 9c82c45 (fix(37-01): skip void register allocation for empty void blocks)

---
*Phase: 37-fix-spurious-void-register-in-empty-function-bodies*
*Completed: 2026-03-04*
