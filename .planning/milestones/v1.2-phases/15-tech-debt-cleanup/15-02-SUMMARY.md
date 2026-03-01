---
phase: 15-tech-debt-cleanup
plan: 02
subsystem: compiler-ast
tags: [rust, ast, tech-debt, dead-code, comments]

# Dependency graph
requires:
  - phase: 13-lowering-entity
    provides: AstEntityDecl, AstComponentSlot — the replacement model that made StructLit dead
provides:
  - Clean AstExpr enum with StructLit dead variant removed
  - Corrected AstComponentDecl doc comment referencing AstComponentSlot model
  - Updated lowering_tests.rs comments reflecting Phase 13 entity model
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - writ-compiler/src/ast/expr.rs
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "StructLit variant removal is clean: zero construction sites found, no pattern matches to update"
  - "Line 699 and 781 IndexMut references left as-is: they document MISC-01 (the fix from IndexMut to IndexSet), not stale usage"

patterns-established: []

requirements-completed: []

# Metrics
duration: 5min
completed: 2026-03-01
---

# Phase 15 Plan 02: Tech Debt Cleanup (Dead Code + Stale Comments) Summary

**Removed dead AstExpr::StructLit variant and updated 9 stale comments to reflect Phase 13's AstComponentSlot entity model**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-01T21:21:32Z
- **Completed:** 2026-03-01T21:22:55Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Removed `AstExpr::StructLit` dead variant (added Phase 5, dead since Phase 13 entity rewrite)
- Fixed `AstComponentDecl` doc comment: replaced "Struct + Impl + lifecycle registrations" with correct AstComponentSlot model description
- Updated 8 test comments in `lowering_tests.rs` to replace pre-Phase-13 references ($Health fields, ComponentAccess impls, StructLit initializers, IndexMut) with current AstComponentSlot terminology

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove AstExpr::StructLit and fix AstComponentDecl doc comment** - `386ad18` (refactor)
2. **Task 2: Update stale test comments in lowering_tests.rs** - `45f80a5` (refactor)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `writ-compiler/src/ast/expr.rs` - Removed StructLit variant (lines 135-144)
- `writ-compiler/src/ast/decl.rs` - Fixed AstComponentDecl doc comment (lines 360-361)
- `writ-compiler/tests/lowering_tests.rs` - Updated 8 stale comments across R13/entity sections

## Decisions Made

- Left line 699 (`MISC-01: IndexSet contract name (not IndexMut)`) and line 781 (`IndexSet contract name is "IndexSet" not "IndexMut"`) as-is — these correctly document the MISC-01 fix, they are not stale references
- No architectural changes needed; all changes are pure comment/dead-code cleanup

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 15 plan 02 complete; codebase has no references to pre-Phase-13 entity model in AST or test comments
- All 440 tests pass

---
*Phase: 15-tech-debt-cleanup*
*Completed: 2026-03-01*
