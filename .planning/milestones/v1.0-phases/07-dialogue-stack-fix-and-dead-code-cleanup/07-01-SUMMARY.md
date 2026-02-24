---
phase: 07-dialogue-stack-fix-and-dead-code-cleanup
plan: 01
subsystem: compiler
tags: [rust, dialogue-lowering, dead-code, speaker-stack, doc-comments]

# Dependency graph
requires:
  - phase: 04-dialogue-lowering-and-localization
    provides: lower_dialogue(), lower_choice() with speaker stack infrastructure
  - phase: 06-pipeline-integration-and-snapshot-testing
    provides: 69 snapshot tests providing regression coverage for these surgical changes
provides:
  - Speaker stack save/restore in lower_dialogue() preventing SpeakerTag scope leaks across sequential dlg items
  - LoweringContext without dead loc_key_counter field and next_loc_key() method
  - Accurate doc comment in stmt.rs describing DlgDecl and Transition lowering
affects: [future-dialogue-phases, compiler-correctness]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Save/restore speaker stack depth around lower_dlg_lines in lower_dialogue() — identical pattern to lower_choice()"

key-files:
  created: []
  modified:
    - writ-compiler/src/lower/dialogue.rs
    - writ-compiler/src/lower/context.rs
    - writ-compiler/src/lower/stmt.rs

key-decisions:
  - "Speaker depth save goes after param_names/singleton_speakers pre-scan (neither touches stack) but before lower_dlg_lines — consistent with lower_choice pattern"
  - "Remove entire next_loc_key() method and loc_key_counter field — zero call sites confirmed before removal"
  - "Update LoweringContext struct doc comment to remove stale next_loc_key() bullet — doc comment accuracy equals code accuracy"

patterns-established:
  - "Stack save/restore: ctx.speaker_stack_depth() save before sub-scope, while ctx.speaker_stack_depth() > depth drain after — used in both lower_choice and now lower_dialogue"

requirements-completed: [R8, R11]

# Metrics
duration: 5min
completed: 2026-02-27
---

# Phase 7 Plan 01: Dialogue Stack Fix and Dead Code Cleanup Summary

**Speaker stack leak fixed in lower_dialogue() via save/restore drain pattern, loc_key_counter/next_loc_key() dead code removed from LoweringContext, stmt.rs doc comment updated to accurately describe DlgDecl/Transition lowering**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-27T11:02:01Z
- **Completed:** 2026-02-27T11:07:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Fixed speaker stack leak in `lower_dialogue()`: saves `ctx.speaker_stack_depth()` before `lower_dlg_lines` call and drains back to saved depth after, preventing `SpeakerTag` scopes from leaking across sequential `dlg` items in the same `lower()` call
- Removed dead `loc_key_counter: u32` field, `loc_key_counter: 0` initializer, and `next_loc_key()` method from `LoweringContext` — all confirmed zero call sites; FNV-1a key generation in `dialogue.rs` replaced this in Phase 4
- Updated `LoweringContext` struct doc comment to remove stale `next_loc_key()` bullet, and replaced stale `todo!()` reference in `stmt.rs` with accurate descriptions of `DlgDecl` and `Transition` lowering
- All 69 existing tests pass with zero snapshot changes and zero compiler warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix speaker stack leak in lower_dialogue()** - `27b2884` (fix)
2. **Task 2: Remove dead loc_key_counter code and fix stale doc comments** - `32f43d7` (chore)

## Files Created/Modified
- `writ-compiler/src/lower/dialogue.rs` - Added speaker depth save/restore around lower_dlg_lines in lower_dialogue()
- `writ-compiler/src/lower/context.rs` - Removed loc_key_counter field, initializer, and next_loc_key() method; updated struct doc comment
- `writ-compiler/src/lower/stmt.rs` - Replaced stale todo!() doc comment with accurate DlgDecl/Transition descriptions

## Decisions Made
- Speaker depth save goes AFTER param_names collection and singleton_speakers pre-scan (neither touches the speaker stack) but BEFORE `lower_dlg_lines` — matches the lower_choice pattern exactly
- Zero call sites for `next_loc_key()` confirmed via grep before removal — safe to delete without risk of breakage
- Struct-level doc comment on `LoweringContext` updated alongside field/method removal to prevent immediate doc drift

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. All three changes were surgical and low-risk as planned. The speaker stack fix correctly emits no AST nodes (drain is a pure state operation), so no snapshot changes occurred. The dead code removal confirmed zero call sites for `next_loc_key()` before deletion.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 7 Plan 01 is the only plan in Phase 7 — all three gap-closure items resolved
- Dialogue lowering correctness improved: sequential dlg items in a single lower() call now have isolated speaker state
- LoweringContext is clean — no dead fields or methods
- All documentation in lower/ now accurately reflects implementation
- Ready for any future phases that extend dialogue lowering

## Self-Check: PASSED

- FOUND: `writ-compiler/src/lower/dialogue.rs`
- FOUND: `writ-compiler/src/lower/context.rs`
- FOUND: `writ-compiler/src/lower/stmt.rs`
- FOUND: `.planning/phases/07-dialogue-stack-fix-and-dead-code-cleanup/07-01-SUMMARY.md`
- FOUND: commit `27b2884` (fix: speaker stack leak)
- FOUND: commit `32f43d7` (chore: dead code removal)
- CONFIRMED: 69 tests pass, zero warnings, zero snapshot changes

---
*Phase: 07-dialogue-stack-fix-and-dead-code-cleanup*
*Completed: 2026-02-27*
