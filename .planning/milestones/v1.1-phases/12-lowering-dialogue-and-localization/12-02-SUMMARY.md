---
phase: 12-lowering-dialogue-and-localization
plan: 02
subsystem: testing
tags: [lowering-tests, insta-snapshots, dialogue-tests, speaker-scope, localization, regression-tests]

# Dependency graph
requires:
  - phase: 12-lowering-dialogue-and-localization
    provides: Plan 01 implementation (all 5 DLG fixes)
provides:
  - 11 new lowering snapshot tests covering all 5 DLG requirements
  - Namespace in loc key tests (single and multi-segment)
  - Interpolation slot preservation tests (simple ident and member access)
  - Choice label key emission test
  - say/say_localized dispatch tests (unkeyed, keyed, mixed)
  - Speaker scope isolation tests (if, if-else, match)
affects: [future-phases-testing]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SpeakerTag test pattern: @speaker immediately before $ sigil produces SpeakerTag (pushes to stack)"
    - "SpeakerLine test pattern: @speaker followed by text tokens is inline (does not push to stack)"

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__dlg_namespace_in_loc_key.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_namespace_multi_segment_in_loc_key.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_interpolation_slot_preserved.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_interpolation_member_access_preserved.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_label_key_emitted.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_say_without_key.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_say_localized_with_key.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_say_mixed_key_dispatch.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_scope_isolation_if.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_scope_isolation_if_else.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_scope_isolation_match.snap
  modified:
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "Speaker scope isolation tests use @speaker $ sigil pattern to produce SpeakerTag (stack push), not @speaker text (SpeakerLine, no push)"
  - "Choice test uses label { body } syntax (no => separator) matching parser grammar"

patterns-established:
  - "SpeakerTag vs SpeakerLine: @speaker before sigil ($, @, ->, }) = SpeakerTag; @speaker text = SpeakerLine"

requirements-completed: [DLG-01, DLG-02, DLG-03, DLG-04, DLG-05]

# Metrics
duration: ~15min
completed: 2026-03-01
---

# Phase 12 Plan 02: Comprehensive Tests Summary

**11 new snapshot tests added covering all 5 dialogue lowering requirements**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 1
- **Files modified:** 1 + 11 snapshots created

## Accomplishments
- Added 11 new lowering snapshot tests covering all 5 DLG requirements
- DLG-01: 2 tests (single-segment namespace, multi-segment namespace)
- DLG-02: 2 tests (simple ident interpolation slot, member access slot)
- DLG-03: 1 test (choice label key as second arg to Option())
- DLG-04: 3 tests (say without key, say_localized with key, mixed dispatch)
- DLG-05: 3 tests (if branch isolation, if-else isolation, match arm isolation)
- All 250 tests pass across the workspace (97 lowering tests, up from 86 pre-Phase 12)

## Files Created/Modified
- `writ-compiler/tests/lowering_tests.rs` - Added 11 new test functions with detailed comments explaining SpeakerTag vs SpeakerLine parsing behavior
- 11 new `.snap` files in `writ-compiler/tests/snapshots/` (see key-files above)

## Decisions Made
- Speaker scope isolation tests use `@player $ if` pattern (SpeakerTag before sigil) instead of `@player Hello. $ if` (SpeakerLine) because TextLine after branch needs active speaker on stack
- Choice label test uses `"label" { body }` syntax (no `=>`) matching the actual parser grammar for DlgChoiceArm

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed speaker scope test input syntax**
- **Found during:** Initial test run
- **Issue:** Speaker scope isolation tests used `@player Hello.` (SpeakerLine) before `$ if/match` branches, but TextLine after branch needs active speaker on stack (SpeakerTag push). SpeakerLine does not push to stack.
- **Fix:** Changed to `@player $ if` pattern where `@player` immediately before `$` sigil produces SpeakerTag (pushes to stack)
- **Files modified:** writ-compiler/tests/lowering_tests.rs
- **Verification:** All 3 speaker scope tests pass

**2. [Rule 3 - Blocking] Fixed choice label test syntax**
- **Found during:** Initial test run
- **Issue:** Choice arm test used `"label" => { body }` but parser expects `"label" { body }` (no `=>` separator)
- **Fix:** Removed `=>` from choice arm syntax in test input
- **Files modified:** writ-compiler/tests/lowering_tests.rs
- **Verification:** dlg_choice_label_key_emitted test passes

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes were test input syntax corrections. No implementation changes needed.

## Issues Encountered
- SpeakerLine vs SpeakerTag distinction requires understanding parser token flow: non-sigil tokens after `@speaker` merge into SpeakerLine (no stack push); only `@speaker` immediately before a sigil (`$`, `@`, `->`, `}`) produces SpeakerTag (stack push)
- DlgChoiceArm grammar does not use `=>` separator (unlike match arms)

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 12 fully complete: all 5 DLG requirements implemented and tested
- Ready for Phase 13 (Lowering -- Entity Model and Misc)

---
*Phase: 12-lowering-dialogue-and-localization*
*Plan: 02*
*Completed: 2026-03-01*
