---
phase: 01-ast-foundation
plan: 02
subsystem: compiler
tags: [rust, thiserror, LoweringError, LoweringContext, lower, pipeline, SimpleSpan]

# Dependency graph
requires:
  - "AstExpr, AstStmt, AstDecl, AstType enums (Plan 01)"
  - "Ast::empty() constructor (Plan 01)"
  - "writ_parser::cst::Item and Spanned types"
provides:
  - "LoweringError enum with 5 thiserror-derived variants, each span-bearing"
  - "LoweringContext struct with errors accumulator, speaker_stack, loc_key_counter, and full accessor API"
  - "SpeakerScope struct for active-speaker tracking"
  - "lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>) public entry point stub"
  - "Public re-exports: writ_compiler::lower, writ_compiler::Ast, writ_compiler::LoweringError, writ_compiler::LoweringContext"
affects: [02-expressions-types-and-control-flow, all subsequent phases]

# Tech tracking
tech-stack:
  added:
    - "thiserror 2.0 (already in Cargo.toml from Plan 01) — used for LoweringError derive"
  patterns:
    - "Accumulator pattern: all passes emit to LoweringContext::errors; pipeline never halts on error"
    - "Private fields / public API: LoweringContext fields are private; all access via emit_error(), take_errors(), push_speaker(), pop_speaker(), current_speaker(), next_loc_key()"
    - "Consuming drain: take_errors(self) consumes context at pipeline exit; errors(&self) provides non-consuming inspection during passes"
    - "Pass ordering documented in lower/mod.rs: expression helpers invoked from structural passes (not top-level)"

key-files:
  created:
    - "writ-compiler/src/lower/error.rs — LoweringError enum (5 variants: UnknownSpeaker, NonTerminalTransition, DuplicateLocKey, ConflictingComponentMethod, Generic)"
    - "writ-compiler/src/lower/context.rs — LoweringContext struct + SpeakerScope struct"
    - "writ-compiler/src/lower/mod.rs — lower() entry point stub with pass ordering documentation"
  modified:
    - "writ-compiler/src/lib.rs — added pub mod lower + public re-exports for lower, Ast, LoweringError, LoweringContext"

key-decisions:
  - "Private fields on LoweringContext — access via methods preserves ability to change internal representation; fields are not pub"
  - "take_errors(self) consumes context — pipeline exit point naturally owns the context; no clone needed"
  - "Generic variant as escape hatch — early development uses Generic for unclassified errors; typed variants preferred when category is known"
  - "Clone + PartialEq on LoweringError — enables snapshot tests to compare error vectors by value"

patterns-established:
  - "Accumulator pattern: no pass halts the pipeline; all errors accumulate in LoweringContext"
  - "Every LoweringError variant carries at least one SimpleSpan for source location (R14)"

requirements-completed: [R2, R14]

# Metrics
duration: 2min
completed: 2026-02-26
---

# Phase 01 Plan 02: Pipeline Infrastructure Summary

**LoweringError (thiserror, 5 variants), LoweringContext (accumulator + speaker stack + loc key counter), and lower() stub entry point returning Ast::empty() — pipeline infrastructure for all subsequent lowering passes**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-26T15:18:20Z
- **Completed:** 2026-02-26T15:20:23Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Created `LoweringError` enum with `thiserror` derive: 5 typed variants (UnknownSpeaker, NonTerminalTransition, DuplicateLocKey, ConflictingComponentMethod, Generic), every variant carries `SimpleSpan`
- Created `SpeakerScope` struct for named + spanned active-speaker tracking in dialogue lowering
- Created `LoweringContext` struct with private fields and full accessor API: `emit_error()`, `take_errors()`, `errors()`, `push_speaker()`, `pop_speaker()`, `current_speaker()`, `next_loc_key()`
- Created `lower()` public entry point stub: correct signature `(Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>)`, returns `(Ast::empty(), vec![])`, pass ordering fully documented with rationale
- Updated `lib.rs` with public re-exports: `writ_compiler::lower`, `writ_compiler::Ast`, `writ_compiler::LoweringError`, `writ_compiler::LoweringContext`
- `cargo build -p writ-compiler` and `cargo test -p writ-compiler` both pass cleanly with no errors or warnings

## Task Commits

1. **Task 1: Create LoweringError and LoweringContext** - `decd242` (feat)
2. **Task 2: Create lower() stub and wire public API** - `91c0cb4` (feat)

## Files Created/Modified

- `writ-compiler/src/lower/error.rs` — `LoweringError` enum with 5 thiserror-derived variants
- `writ-compiler/src/lower/context.rs` — `LoweringContext` struct + `SpeakerScope` struct with full accessor API
- `writ-compiler/src/lower/mod.rs` — `lower()` entry point stub with documented pass ordering
- `writ-compiler/src/lib.rs` — Added `pub mod lower` and re-exports for `lower`, `Ast`, `LoweringError`, `LoweringContext`

## Decisions Made

- **Private fields on LoweringContext:** All three fields (`errors`, `speaker_stack`, `loc_key_counter`) are private. Access is exclusively via methods. This preserves the ability to change internal representation in later phases without breaking callers.
- **`take_errors(self)` consumes context:** The `lower()` function creates the context, runs all passes, then calls `take_errors()` to drain and return errors. No clone is needed; consumption is the natural endpoint.
- **`Generic` variant as escape hatch:** During early development, unclassified errors use `Generic { message, span }`. Typed variants are preferred when the error category is known. This matches the plan's design intent and avoids blocking early pass development.
- **`Clone + PartialEq` on `LoweringError`:** Required for snapshot tests that compare `Vec<LoweringError>` by value. Added proactively per plan guidance.

## Deviations from Plan

None — plan executed exactly as written. Both files compiled on first attempt. No dependency issues, no type mismatches, no build failures.

## Issues Encountered

None.

## User Setup Required

None.

## Next Phase Readiness

- `lower()` stub is in place — Phase 2 (expressions, types, control flow) can begin immediately
- `LoweringContext` API is complete — all pass authors receive `&mut LoweringContext` and call `emit_error()`, `push_speaker()`, `next_loc_key()`
- Public re-exports are wired — downstream consumers import `writ_compiler::lower`, `writ_compiler::LoweringError`, etc.
- No blocking concerns for Phase 2+

---
*Phase: 01-ast-foundation*
*Completed: 2026-02-26*
