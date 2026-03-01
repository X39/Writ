---
phase: 12-lowering-dialogue-and-localization
plan: 01
subsystem: compiler, lowering
tags: [dialogue, localization, fnv1a, speaker-scope, namespace, interpolation, say-dispatch]

# Dependency graph
requires:
  - phase: 9-cst-type-system-additions
    provides: DlgDecl attrs/vis, namespace declarations
  - phase: 10-parser-core-syntax
    provides: CST expression types (MemberAccess tuple-style, Call tuple-style)
provides:
  - Namespace-prefixed FNV-1a localization keys (DLG-01)
  - Preserved interpolation slot identities in loc content (DLG-02)
  - Choice label loc keys emitted as args (DLG-03)
  - Conditional say vs say_localized dispatch (DLG-04)
  - Speaker scope isolation across if/else/match branches (DLG-05)
affects: [13-lowering-entity-model-and-misc, tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Namespace threading: LoweringContext tracks namespace stack, dialogue lowering reads at init"
    - "Speaker scope save/restore: speaker_stack_depth() + pop loop at branch boundaries"
    - "Conditional say dispatch: say() for unkeyed lines, say_localized() for #key lines"

key-files:
  created: []
  modified:
    - writ-compiler/src/lower/context.rs
    - writ-compiler/src/lower/dialogue.rs
    - writ-compiler/src/lower/mod.rs

key-decisions:
  - "Namespace stored as joined :: string in DlgLowerState, not as segments, since FNV input needs flat string"
  - "expr_to_slot_text() recursively reconstructs slot text from CST Expr nodes (Ident, MemberAccess, Call)"
  - "Choice label keys emitted as second arg to Option(label, key, fn() { body }) instead of discarded with let _ = key"
  - "Auto FNV keys still computed for unkeyed lines (for CSV tooling) but only say() is emitted at runtime"
  - "Speaker scope save/restore uses same pattern as existing choice arm handling (speaker_stack_depth + pop loop)"

patterns-established:
  - "Namespace tracking API: push_namespace/pop_namespace/set_namespace/current_namespace on LoweringContext"
  - "make_say/make_say_localized: factored call-emission helpers for dialogue lowering"

requirements-completed: [DLG-01, DLG-02, DLG-03, DLG-04, DLG-05]

# Metrics
duration: ~30min
completed: 2026-03-01
---

# Phase 12 Plan 01: Dialogue Lowering Fixes Implementation Summary

**All 5 dialogue lowering requirements implemented: namespace in loc keys, slot identity preservation, choice key emission, say/say_localized dispatch, speaker scope isolation**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- DLG-01: Added namespace tracking API to LoweringContext and threaded through lower_namespace and lower_dialogue
- DLG-02: Replaced generic `{expr}` placeholder with expr_to_slot_text() that preserves `{name}`, `{player.name}`, `{fn(..)}`
- DLG-03: Changed choice label loc key from `let _ = key` discard to second arg in `Option(label, key, fn() { body })`
- DLG-04: Added make_say() helper; say() for unkeyed lines, say_localized() only for lines with manual #key
- DLG-05: Added speaker scope save/restore to lower_dlg_if, lower_dlg_else, and lower_dlg_match
- All 239 existing tests pass with zero regressions; 17 existing dialogue snapshots updated

## Files Created/Modified
- `writ-compiler/src/lower/context.rs` - Added `namespace_stack: Vec<String>` field and 4 API methods (push_namespace, pop_namespace, set_namespace, current_namespace)
- `writ-compiler/src/lower/dialogue.rs` - All 5 DLG fixes: namespace init from ctx, expr_to_slot_text() helper, choice key emission, make_say/make_say_localized helpers, scope save/restore in if/else/match
- `writ-compiler/src/lower/mod.rs` - Updated lower_namespace() to thread namespace into LoweringContext via push/pop/set

## Decisions Made
- Used CST tuple-style destructuring for Expr::MemberAccess(object, (field, _span)) and Expr::Call(callee, _args) in expr_to_slot_text()
- Auto FNV keys are still computed for unkeyed lines (occurrence tracking + CSV tooling) but only make_say() is emitted at runtime
- Speaker scope restore uses the same pop-loop pattern already established for choice arms

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] CST Expr tuple-style destructuring**
- **Found during:** Task 2 (DLG-02 implementation)
- **Issue:** Initial expr_to_slot_text() used struct-style field names (`Expr::MemberAccess { object, field, .. }`) but CST Expr uses tuple-style variants
- **Fix:** Changed to `Expr::MemberAccess(object, (field, _field_span))` and `Expr::Call(callee, _args)`
- **Files modified:** writ-compiler/src/lower/dialogue.rs
- **Verification:** Clean compilation, all tests pass

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial fix, no scope change.

## Issues Encountered
- CST Expr enum uses tuple-style variants not struct-style, caught at compilation

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 DLG requirements implemented and existing tests pass
- Ready for Plan 12-02 (comprehensive test suite)

---
*Phase: 12-lowering-dialogue-and-localization*
*Plan: 01*
*Completed: 2026-03-01*
