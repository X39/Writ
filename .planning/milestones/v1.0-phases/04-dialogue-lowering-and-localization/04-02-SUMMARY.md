---
phase: 04-dialogue-lowering-and-localization
plan: 02
subsystem: compiler
tags: [rust, dialogue, lowering, snapshot-tests, insta, localization, fnv1a, speaker-resolution]

# Dependency graph
requires:
  - phase: 04-dialogue-lowering-and-localization
    plan: 01
    provides: lower_dialogue() implementation with FNV-1a keys, speaker resolution, and all 8 DlgLine variants
provides:
  - lower_src_with_errors() test helper for error-path snapshot testing
  - R8 snapshot tests: Tier 1/2 speaker resolution, SpeakerTag active-speaker, text interpolation, code escape, choice, conditional if, transition
  - R9 snapshot tests: FNV-1a 8-char hex key format, distinct keys for duplicate text, manual #key override
  - R10 snapshot tests: DuplicateLocKey collision detection via #key
  - R11 snapshot tests: NonTerminalTransition error, UnknownSpeaker error, choice arm scope isolation
affects: [05-entity-lowering]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "lower_src_with_errors(): returns (Ast, Vec<LoweringError>) without asserting clean — enables error-path snapshot testing"
    - "LoweringError re-exported from lib.rs — accessible in test crate as writ_compiler::LoweringError"
    - "Speaker scope isolation test: @Speaker immediately before $ sigil = SpeakerTag (pushed to stack); whitespace is trivia-filtered before parsing"
    - "insta::assert_debug_snapshot!((ast, errors)) — tuple snapshot for AST + error list together"

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_param_tier1.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_singleton_tier2.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_tag_sets_active.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_text_interpolation.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_code_escape_statement.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_basic.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_conditional_if.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_transition_at_end.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_transition_with_args.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_multiple_speakers_hoisting.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_loc_key_is_8char_hex.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_loc_key_distinct_for_duplicate_text.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_loc_key_manual_override.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_loc_key_duplicate_collision.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_non_terminal_transition_error.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_text_without_speaker_error.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_speaker_scope_isolation.snap
  modified:
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "Whitespace (including newlines) is trivia-filtered before parsing — @Narrator followed by newline then text = SpeakerLine (inline), NOT SpeakerTag then TextLine; SpeakerTag only when @Speaker is followed directly by a sigil"
  - "Speaker scope isolation test uses @Narrator immediately before $ choice as SpeakerTag — only way to push Narrator onto the speaker stack for scope save/restore testing"
  - "lower_src_with_errors uses assert! on parse errors but not on lower errors — matches the helper contract in the plan"

patterns-established:
  - "lower_src_with_errors(): complement to lower_src() for error-path tests; import LoweringError via use writ_compiler::LoweringError"
  - "Tuple snapshot pattern: insta::assert_debug_snapshot!((ast, errors)) captures both AST output and error list in one snapshot"

requirements-completed: [R8, R9, R10, R11]

# Metrics
duration: 5min
completed: 2026-02-26
---

# Phase 04 Plan 02: Dialogue Lowering Snapshot Tests Summary

**17 insta snapshot tests locking down dialogue lowering: Tier 1/2 speaker resolution, FNV-1a 8-char hex keys, #key override and collision, non-terminal transition and unknown speaker errors, choice arm scope isolation**

## Performance

- **Duration:** ~5 min (wall clock)
- **Started:** 2026-02-26T19:55:20Z
- **Completed:** 2026-02-26T20:00:35Z
- **Tasks:** 2
- **Files modified:** 18 (1 modified, 17 created)

## Accomplishments
- Added `lower_src_with_errors()` helper returning `(Ast, Vec<LoweringError>)` without asserting errors empty — enables error-path snapshot testing
- Added 10 R8 happy-path snapshot tests covering all dialogue lowering behaviors: Tier 1 param speaker direct ref, Tier 2 singleton hoisting (`let _narrator = Entity.getOrCreate<Narrator>()`), standalone `@Speaker` tag active-speaker tracking, `{expr}` text interpolation, `$ let` code escape, basic `$ choice` with two arms, `$ if` conditional, `->` transition at end, `->` with args, multiple speaker pre-scan hoisting
- Added 7 R9/R10/R11 snapshot tests: 8-char hex FNV-1a key format, distinct keys for identical text (occurrence_index), manual `#key` override, `DuplicateLocKey` error, `NonTerminalTransition` error, `UnknownSpeaker` error, choice arm speaker scope isolation
- All 46 tests pass (29 existing Phase 2+3 + 17 new Phase 4 tests); zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Add lower_src_with_errors helper and R8 dialogue lowering happy-path snapshot tests** - `5874e8d` (test)
2. **Task 2: Add R9/R10/R11 localization key and error-path snapshot tests** - `ac479b0` (test)

## Files Created/Modified
- `writ-compiler/tests/lowering_tests.rs` — Added `lower_src_with_errors()` helper + 17 new dialogue snapshot tests
- `writ-compiler/tests/snapshots/lowering_tests__dlg_*.snap` — 17 new accepted insta snapshot files

## Decisions Made
- **Whitespace is trivia-filtered, not line-based**: `@Narrator` followed by any non-sigil tokens (even after newlines) becomes a `SpeakerLine` with inline text, not a `SpeakerTag`. This is because the lexer preserves `Whitespace` tokens but the parser filters them out before parsing begins. Speaker scope isolation test required restructuring to use `@Narrator $` (SpeakerTag before a sigil) rather than `@Narrator\n` (which merges with subsequent text).
- **Speaker scope isolation semantics**: `SpeakerTag` (`@Speaker` with no text, immediately before a sigil) pushes to the speaker stack and is restored via `speaker_stack_depth()` save/restore in `lower_choice`. `SpeakerLine` (inline text after `@Speaker`) does NOT push to the stack.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed dlg_choice_speaker_scope_isolation test — incorrect test source**
- **Found during:** Task 2 (test execution)
- **Issue:** Test used `@Narrator Ask. $ choice {...}` — `@Narrator Ask.` is a `SpeakerLine` (does not push to speaker stack), so arm "B" had no active speaker and emitted `UnknownSpeaker` error instead of correctly using Narrator
- **Fix:** Changed outer source to `@Narrator $ choice {...}` — `@Narrator` immediately before `$` sigil becomes a `SpeakerTag` that pushes Narrator to the stack; arm B then correctly finds Narrator
- **Files modified:** writ-compiler/tests/lowering_tests.rs
- **Verification:** Test passes, snapshot shows arm B uses `_narrator` not `_player`
- **Committed in:** ac479b0 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - test source bug)
**Impact on plan:** Fix was necessary for correctness; the test now correctly proves scope isolation rather than incorrectly proving an error path. No scope creep.

## Issues Encountered
- The Writ dialogue parser is token-based (whitespace trivia-filtered), not line-based. `@Speaker\ntext` and `@Speaker text` are identical to the parser and both produce `SpeakerLine`. This required changing the scope isolation test source.

## Next Phase Readiness
- All 4 requirements (R8-R11) have accepted snapshot tests locking down dialogue lowering behavior
- Phase 4 complete; ready for Phase 5 (Entity Lowering)
- The 17 new snapshots serve as regression guards for all dialogue lowering code paths

## Self-Check: PASSED

- writ-compiler/tests/lowering_tests.rs — FOUND
- writ-compiler/tests/snapshots/lowering_tests__dlg_speaker_singleton_tier2.snap — FOUND
- writ-compiler/tests/snapshots/lowering_tests__dlg_choice_speaker_scope_isolation.snap — FOUND
- .planning/phases/04-dialogue-lowering-and-localization/04-02-SUMMARY.md — FOUND
- Commit 5874e8d — FOUND
- Commit ac479b0 — FOUND

---
*Phase: 04-dialogue-lowering-and-localization*
*Completed: 2026-02-26*
