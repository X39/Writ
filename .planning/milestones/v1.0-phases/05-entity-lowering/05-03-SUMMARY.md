---
phase: 05-entity-lowering
plan: 03
subsystem: parser
tags: [rust, chumsky, entity-parser, snapshot-tests, insta]

# Dependency graph
requires:
  - phase: 05-entity-lowering/05-02
    provides: entity lowering snapshot tests including three broken snapshots using wrong (fallback) output
provides:
  - Fixed entity_property trailing-comma parser (optional comma via .or_not())
  - Three corrected snapshots: entity_property_fields, entity_singleton_attribute, entity_duplicate_property_error
  - All three verification gaps from 05-VERIFICATION.md closed (Gap 1, Gap 2, Gap 3)
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Optional trailing comma via just(Token::X).or_not() — matches entity_use pattern for entity_property"

key-files:
  created: []
  modified:
    - writ-parser/src/parser.rs
    - writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap
    - writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap

key-decisions:
  - "Optional trailing comma on entity_property uses just(Token::Comma).or_not() — one-line fix, strictly permissive superset"
  - "Re-acceptance via INSTA_UPDATE=always in a single pass — no separate cargo insta accept step needed"

patterns-established:
  - "Entity member parsers: all four (entity_use, entity_on, entity_fn, entity_property) should use .or_not() for trailing commas"

requirements-completed: [R12, R13]

# Metrics
duration: 5min
completed: 2026-02-27
---

# Phase 5 Plan 03: Entity Lowering Gap-Closure Summary

**One-line parser fix (just(Token::Comma).or_not() at parser.rs:2583) closes three verification gaps — property-only entities now parse as EntityDecl and reach lower_entity() instead of falling through to expression-statement recovery**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-02-27T08:40:10Z
- **Completed:** 2026-02-27T08:45:00Z
- **Tasks:** 1 (all done criteria met)
- **Files modified:** 4

## Accomplishments

- Fixed entity_property trailing-comma defect: changed `just(Token::Comma)` to `just(Token::Comma).or_not()` at parser.rs line 2583
- Re-accepted three broken snapshots that previously showed expression-statement fallback output
- entity_property_fields now shows `AstDecl::Struct` with two `AstStructField` entries (name: string, level: int = 1)
- entity_singleton_attribute now shows `AstDecl::Struct` with `attrs: [AstAttribute { name: "Singleton" }]`
- entity_duplicate_property_error now shows `(Ast with Struct, [DuplicateProperty { property: "name", entity: "Guard" }])`
- All 62 lowering tests pass without INSTA_UPDATE (snapshots stable)
- All 177 parser tests pass (zero regressions)
- All three verification gaps from 05-VERIFICATION.md closed: Gap 1 (Truth #1 PARTIAL), Gap 2 (Truth #4 FAILED), Gap 3 (Truth #5 PARTIAL)

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix entity_property trailing-comma parser defect and re-accept broken snapshots** - `e6c90a8` (fix)

## Files Created/Modified

- `writ-parser/src/parser.rs` - Changed just(Token::Comma) to just(Token::Comma).or_not() at entity_property parser (line 2583)
- `writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap` - Now shows AstDecl::Struct with two fields instead of expression-statement fallback
- `writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap` - Now shows AstDecl::Struct with [Singleton] attr instead of array-literal fallback
- `writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap` - Now shows (Struct + DuplicateProperty error) instead of (expression-fallback, [])

## Decisions Made

- Optional trailing comma uses `just(Token::Comma).or_not()` — exactly mirrors what entity_use already does at line 2545; the fix is one character change making the trailing comma a superset (accepts with or without comma)
- Re-acceptance in single INSTA_UPDATE=always pass without separate cargo insta accept — consistent with prior phase practice

## Deviations from Plan

None - plan executed exactly as written. The fix was precisely one token change as described in the plan interfaces section.

## Issues Encountered

None. The defect was confirmed at parser.rs line 2583 as documented. After the fix, cargo test -p writ-parser passed (177 tests), INSTA_UPDATE=always updated exactly the three expected snapshots, and cargo test -p writ-compiler confirmed all 62 tests stable.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 5 (Entity Lowering) is now completely verified: all 5 verification gaps from 05-VERIFICATION.md are closed
- Milestone v1.0 (CST-to-AST Lowering Pipeline) is complete: all 5 phases, all 11 plans (including this gap-closure plan) executed
- No blockers; lowering pipeline is production-ready for runtime integration

## Self-Check: PASSED

- FOUND: .planning/phases/05-entity-lowering/05-03-SUMMARY.md
- FOUND: writ-parser/src/parser.rs
- FOUND: writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap
- FOUND: writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap
- FOUND: writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap
- FOUND commit: e6c90a8 (fix(05-03): fix entity_property trailing-comma parser defect)

---
*Phase: 05-entity-lowering*
*Completed: 2026-02-27*
