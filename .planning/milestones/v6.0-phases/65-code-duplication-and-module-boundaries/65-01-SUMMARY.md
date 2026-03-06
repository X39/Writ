---
phase: 65-code-duplication-and-module-boundaries
plan: 01
subsystem: compiler
tags: [rust, lowering, dialogue, fmt-string, refactor]

# Dependency graph
requires:
  - phase: 63-writ-compiler-file-splits
    provides: dialogue.rs and fmt_string.rs stable after Phase 63 review
  - phase: 64-cross-crate-file-splits
    provides: stable module boundaries before consolidation
provides:
  - lower_dlg_text delegates to lower_fmt_string via DlgTextSegment->StringSegment conversion
  - No duplicated left-associative Add chain fold logic across dialogue and fmt_string lowering
affects: [66-future-lowering-changes]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Segment type conversion pattern: convert DlgTextSegment->StringSegment before calling shared impl"

key-files:
  created: []
  modified:
    - writ-compiler/src/lower/dialogue.rs

key-decisions:
  - "lower_dlg_text converts DlgTextSegment->StringSegment then delegates to lower_fmt_string instead of duplicating the 48-line fold logic"
  - "BinaryOp removed from dialogue.rs imports — no longer used after delegation; AstType retained (still used in speaker getOrCreate call at line 131)"

patterns-established:
  - "Segment type conversion: when two CST segment enums are structurally identical, convert at call site rather than duplicating the shared impl"

requirements-completed: [DUP-01, DUP-02]

# Metrics
duration: 5min
completed: 2026-03-18
---

# Phase 65 Plan 01: Code Duplication Consolidation Summary

**lower_dlg_text refactored from 48-line duplicated fold to 12-line DlgTextSegment->StringSegment conversion + delegation to lower_fmt_string**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-18T00:00:00Z
- **Completed:** 2026-03-18T00:05:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Eliminated the only significant cross-function code duplication in the workspace (DUP-01, DUP-02)
- `lower_dlg_text` reduced from 48 lines to 12 lines — conversion of DlgTextSegment variants to StringSegment variants, then single delegation call
- Removed now-unused `BinaryOp` import from `dialogue.rs` (the fold was the only consumer)
- All 112 lowering snapshot tests pass with identical output — no behavior change

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace lower_dlg_text body with DlgTextSegment->StringSegment conversion and delegation** - `9b3260a` (refactor)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `writ-compiler/src/lower/dialogue.rs` - lower_dlg_text body replaced with conversion+delegation; StringSegment added to CST import; lower_fmt_string import added; BinaryOp removed from expr import

## Decisions Made
- `BinaryOp` removed from `dialogue.rs` imports — after the old fold body is gone, the only use was `BinaryOp::Add` in the fold. `AstType` is kept because it is still used at line 131 (speaker `getOrCreate` generic call).
- DlgTextSegment and StringSegment share identical variant structure (Text(&str) / Expr(Box<Expr>)) — the match conversion is a safe mechanical transformation with zero semantic risk.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- DUP-01 and DUP-02 satisfied; Phase 65 Plan 01 complete
- Phase 65 may continue with remaining module boundary work if any further plans exist

---
*Phase: 65-code-duplication-and-module-boundaries*
*Completed: 2026-03-18*
