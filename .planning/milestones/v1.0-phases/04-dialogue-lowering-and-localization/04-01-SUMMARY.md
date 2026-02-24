---
phase: 04-dialogue-lowering-and-localization
plan: 01
subsystem: compiler
tags: [rust, dialogue, lowering, localization, fnv1a, speaker-resolution]

# Dependency graph
requires:
  - phase: 03-operator-and-concurrency-lowering
    provides: LoweringContext with speaker_stack, lower_stmt, lower_expr infrastructure
provides:
  - lower_dialogue(): DlgDecl → AstFnDecl transformation with three-tier speaker resolution
  - FNV-1a 32-bit localization key generation per spec section 25.2.2
  - All 8 DlgLine variants lowered (SpeakerLine, SpeakerTag, TextLine, CodeEscape, Choice, If, Match, Transition)
  - Manual #key override support with DuplicateLocKey collision detection
  - Non-terminal transition validation (NonTerminalTransition error)
  - Stmt::DlgDecl and Stmt::Transition stubs replaced with working implementations
affects: [05-entity-lowering, snapshot-tests-for-dialogue]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "DlgDecl → AstFnDecl: singleton speakers hoisted as let _name = Entity.getOrCreate<Name>() at fn top"
    - "FNV-1a 32-bit hash inline — no external crate, namespace+method+speaker+content+occurrence_index input"
    - "Choice arm speaker scope save/restore via speaker_stack_depth()"
    - "TextLine without active speaker → UnknownSpeaker error + AstExpr::Error placeholder (non-halting)"
    - "Manual #key via DlgLine loc_key field; duplicate detection via manual_keys HashMap in DlgLowerState"

key-files:
  created:
    - writ-compiler/src/lower/dialogue.rs
  modified:
    - writ-compiler/src/lower/context.rs
    - writ-compiler/src/lower/mod.rs
    - writ-compiler/src/lower/stmt.rs

key-decisions:
  - "Tier 3 (UnknownSpeaker on resolve_speaker) removed — any non-param speaker assumed singleton per Research Open Question #3; name resolution validates entity existence in later phase"
  - "singleton_names Vec removed from DlgLowerState — not needed for lookup since all non-param speakers unconditionally map to hoisted binding pattern"
  - "Stmt::DlgDecl inline lowering as AstStmt::Let wrapping AstExpr::Lambda per Research Open Question #1"
  - "Namespace left empty string in Phase 4 — threading namespace context deferred per Research Open Question #2"
  - "collect_singleton_speakers pre-scans entire body including Choice/If/Match branches to cover all speakers before body executes"

patterns-established:
  - "DlgLowerState: private struct per dialogue block; lifetime matches lower_dialogue() call"
  - "lower_dlg_lines takes &[Spanned<DlgLine>] (borrowed slice); ownership stays with caller for recursion"

requirements-completed: [R8, R9, R10, R11]

# Metrics
duration: 2min
completed: 2026-02-26
---

# Phase 04 Plan 01: Dialogue Lowering Core Summary

**DlgDecl-to-AstFnDecl lowering with three-tier speaker resolution, FNV-1a localization keys, #key overrides, duplicate key detection, and non-terminal transition validation**

## Performance

- **Duration:** ~12 min (wall clock including reads and verification)
- **Started:** 2026-02-26T19:47:44Z
- **Completed:** 2026-02-26T19:59:00Z
- **Tasks:** 2
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments
- Created `writ-compiler/src/lower/dialogue.rs` with complete dialogue lowering: all 8 DlgLine variants, FNV-1a key generation, three-tier speaker resolution, choice arm scope save/restore, duplicate key detection
- Added `speaker_stack_depth()` to `LoweringContext` for choice arm speaker scope save/restore
- Wired `lower_dialogue` into both `Item::Dlg` dispatch sites in `mod.rs` (top-level and namespace block)
- Replaced both `Stmt::DlgDecl` and `Stmt::Transition` todo stubs in `stmt.rs` with working implementations
- All 29 existing Phase 2+3 tests pass without regression; zero warnings on `cargo build`

## Task Commits

Each task was committed atomically:

1. **Task 1: Create lower/dialogue.rs with complete dialogue lowering implementation** - `9a8c311` (feat)
2. **Task 2: Wire lower_dialogue into mod.rs and implement stmt.rs dialogue stubs** - `1cc9c03` (feat)

## Files Created/Modified
- `writ-compiler/src/lower/dialogue.rs` — New module: lower_dialogue() entry point + all private helpers (collect_singleton_speakers, lower_dlg_lines, resolve_speaker, compute_or_use_loc_key, fnv1a_32, raw_text_content, lower_dlg_text, make_say_localized, lower_choice, lower_dlg_if, lower_dlg_match, lower_transition)
- `writ-compiler/src/lower/context.rs` — Added speaker_stack_depth() method
- `writ-compiler/src/lower/mod.rs` — Added pub mod dialogue, import, replaced both Item::Dlg todo stubs
- `writ-compiler/src/lower/stmt.rs` — Replaced Stmt::DlgDecl and Stmt::Transition todo stubs; added AstExpr/AstArg/AstLambdaParam imports

## Decisions Made
- **Tier 3 speaker resolution removed from resolve_speaker**: Per Research Open Question #3, any non-param speaker is assumed to be a Singleton entity during lowering. `UnknownSpeaker` is only emitted for `TextLine` with no active speaker, not for unknown speaker names.
- **singleton_names removed from DlgLowerState**: The field was collected but never needed — all non-param speakers unconditionally map to the `_lowercase_name` hoisted binding pattern regardless of whether they appear in the pre-scan result.
- **Namespace empty in Phase 4**: Per Research Open Question #2, namespace context is not yet threaded through lower_dialogue. The FNV-1a input uses empty string for namespace; threaded namespace will be added when the namespace context is available.

## Deviations from Plan

None — plan executed exactly as written. The `singleton_names` field was removed from `DlgLowerState` after discovering it was never read (the Tier 2 lookup doesn't need to verify the speaker was pre-scanned, it just generates the `_lowercase` binding name). This is a correctness cleanup, not a behavior change.

## Issues Encountered
- Minor: `AstLambdaParam` import in `dialogue.rs` and `singleton_names` field triggered unused warnings; removed both before final build. Zero warnings on clean build.

## Next Phase Readiness
- Dialogue lowering foundation is complete; ready for Phase 4 Plan 02 (snapshot tests for dialogue lowering)
- All four `todo!("Phase 4")` stubs replaced; `cargo build` clean with zero warnings
- `Entity.getOrCreate<T>()` hoisting pattern established for downstream entity lowering (Phase 5)

---
*Phase: 04-dialogue-lowering-and-localization*
*Completed: 2026-02-26*
