---
phase: 41-fix-fn-log-say-choice
plan: "03"
subsystem: writ-compiler/check, writ-golden
tags: [golden-tests, CALL_EXTERN, check_call, path-fast-path, BUG-01, blessing]

# Dependency graph
requires:
  - phase: 41-01
    provides: Golden test harness with BOM strip and bless infrastructure
  - phase: 41-02
    provides: check_path :: normalization (::log resolves to log in DefMap)
provides:
  - Blessed fn_log_say_choice.writil with correct CALL_EXTERN IL (BUG-01 accepted)
  - check_call path fast-path for root-qualified single-segment paths
  - 41-NOTES.md root cause documentation
affects: [41-summary, 42-choice-option-rename, fn_log_say_choice regression anchor]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Path fast-path in check_call: strip :: prefix, find_fn_def_id, check_call_with_sig"
    - "BLESS=1 cargo test workflow for updating golden IL snapshots"

key-files:
  created:
    - .planning/phases/41-fix-fn-log-say-choice/41-NOTES.md
  modified:
    - writ-compiler/src/check/check_expr.rs
    - writ-golden/tests/golden/fn_log_say_choice.writ
    - writ-golden/tests/golden/fn_log_say_choice.writil

key-decisions:
  - "fn_log_say_choice.writ simplified to remove ::Option/Option::None — Phase 42 scope; extern fn declarations added for log/say/choice"
  - "check_call path fast-path strips leading :: and resolves def_id so CALL_EXTERN is emitted for root-qualified ExternFn calls"
  - "::choice() simplified to no-arg call to avoid ChoiceOption ambiguity (deferred to Phase 42)"

patterns-established:
  - "Path fast-path pattern: AstExpr::Path single-segment with :: prefix -> strip -> find_fn_def_id -> check_call_with_sig"

requirements-completed: [BUG-01]

# Metrics
duration: ~15min
completed: 2026-03-06
---

# Phase 41 Plan 03: Bless fn_log_say_choice Golden Snapshot Summary

**Blessed fn_log_say_choice.writil with 4x CALL_EXTERN instructions for ::log/::say/::choice calls, plus added check_call path fast-path so root-qualified ExternFn calls emit CALL_EXTERN instead of CALL_INDIRECT**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-06T14:15:00Z
- **Completed:** 2026-03-06T14:26:09Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Simplified `fn_log_say_choice.writ` to remove `::Option`/`Option::None` (Phase 42 scope boundary); added `extern fn log/say/choice` declarations
- Added path fast-path in `check_call` for root-qualified single-segment paths (e.g. `::log`) — sets `callee_def_id: Some(def_id)` so codegen emits `CALL_EXTERN` instead of `CALL_INDIRECT`
- Ran `BLESS=1 cargo test` to write correct IL to `fn_log_say_choice.writil` — 4x `CALL_EXTERN`, `RET_VOID`, no empty bodies
- All 9 `writ-golden` tests pass; all 65 `writ-compiler` tests pass
- Wrote `41-NOTES.md` documenting the 5-step root cause chain, all fixes, and the `::Option` scope boundary

## Task Commits

Each task was committed atomically:

1. **Task 1: Bless fn_log_say_choice.writil and verify full golden suite** - `e11209b` (feat)
2. **Task 2: Write 41-NOTES.md documenting root cause and fix** - `23e0005` (docs)

**Plan metadata:** _(docs commit follows)_

## Files Created/Modified

- `writ-compiler/src/check/check_expr.rs` — Added path fast-path in `check_call` for `AstExpr::Path` callees with single `::log`-style segments
- `writ-golden/tests/golden/fn_log_say_choice.writ` — Simplified: removed `::Option`/`Option::None`, added `pub extern fn` declarations, `::choice()` no-arg call
- `writ-golden/tests/golden/fn_log_say_choice.writil` — Blessed with correct IL: `CALL_EXTERN` for log/say/choice calls, UTF-8 no BOM
- `.planning/phases/41-fix-fn-log-say-choice/41-NOTES.md` — Root cause chain, all fixes documented, scope boundary

## Decisions Made

- **Path fast-path needed for CALL_EXTERN:** The plan required `CALL_EXTERN` in the output. After Phase 42's `check_path` normalization, `::log` resolves correctly but `callee_def_id` remains `None` for path-form calls (general `check_call` path). Added a path fast-path analogous to the existing Ident fast-path that sets `callee_def_id: Some(def_id)` — enables `CALL_EXTERN` dispatch.
- **Simplified writ source with extern declarations:** Without `extern fn` declarations, `log`/`say`/`choice` are not in the DefMap, so they would fail to resolve. The test file now declares them as `pub extern fn`. `::choice()` uses no-arg form to avoid the `ChoiceOption` type ambiguity.
- **`::choice()` no-arg form:** The original `::choice([::Option(...)])` uses `::Option` which is ambiguous with prelude `Option<T>`. Phase 42 fixes this. For Phase 41, `::choice()` with no args exercises the CALL_EXTERN path without requiring the type parameter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added path fast-path in check_call to emit CALL_EXTERN**
- **Found during:** Task 1 (bless fn_log_say_choice.writil)
- **Issue:** After simplifying the `.writ` source, `::log(...)` produced `CALL_INDIRECT` instead of `CALL_EXTERN`. The plan requires `contains: "CALL_EXTERN"`. Root cause: `callee_def_id` is `None` for `AstExpr::Path` callees in `check_call`'s general path — only the Ident fast-path sets it.
- **Fix:** Added path fast-path in `check_call` for single-segment `AstExpr::Path` callees; strips `::` prefix, calls `find_fn_def_id`, delegates to `check_call_with_sig` which sets `callee_def_id: Some(def_id)`.
- **Files modified:** `writ-compiler/src/check/check_expr.rs`
- **Verification:** All 65 compiler tests pass; `CALL_EXTERN` present in blessed `.writil`
- **Committed in:** `e11209b` (Task 1 commit)

**2. [Known Deviation] Simplified fn_log_say_choice.writ per scope boundary**
- **Found during:** Task 1 (pre-bless step)
- **Issue:** `::Option("label", fn() {...})` ambiguous with prelude `Option<T>`; `Option::None` hits enum variant stub → emit panic. Per the known deviation in the execution prompt.
- **Fix:** Replaced `::choice([::Option(...)])` with `::choice()` (empty call); added `pub extern fn` declarations; removed all `::Option`/`Option::None` uses.
- **Files modified:** `writ-golden/tests/golden/fn_log_say_choice.writ`
- **Verification:** No emit panic; compilation succeeds; `CALL_EXTERN` emitted for all three inbuilt calls
- **Committed in:** `e11209b` (Task 1 commit)

---

**Total deviations:** 2 (1 auto-fixed missing critical, 1 known deviation per prompt)
**Impact on plan:** Both necessary for correctness and spec compliance. No scope creep.

## Issues Encountered

- After Phase 42 fix, `::log(...)` still emitted `CALL_INDIRECT` because `callee_def_id` is `None` for path-form calls — required adding the path fast-path. This was an undiscovered gap: Phase 42 fixed resolution but not `callee_def_id` propagation.

## Next Phase Readiness

- Phase 41 BUG-01 fix is complete and locked: `test_fn_log_say_choice` is a regression anchor
- Phase 42 (ChoiceOption rename) can restore the full `::choice([ChoiceOption(...)])` form
- Phase 43 (None/Some) can restore `Option::None`/`Option::Some` usage in the test

---
*Phase: 41-fix-fn-log-say-choice*
*Completed: 2026-03-06*

## Self-Check: PASSED

- `writ-golden/tests/golden/fn_log_say_choice.writil` — FOUND
- `.planning/phases/41-fix-fn-log-say-choice/41-NOTES.md` — FOUND
- `writ-compiler/src/check/check_expr.rs` — FOUND (modified)
- Commit `e11209b` — FOUND (feat: bless golden snapshot)
- Commit `23e0005` — FOUND (docs: write 41-NOTES.md)
