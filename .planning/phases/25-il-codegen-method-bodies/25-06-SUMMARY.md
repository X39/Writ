---
phase: 25-il-codegen-method-bodies
plan: 06
subsystem: codegen
tags: [rust, il, codegen, tail-call, string-build, emit]

# Dependency graph
requires:
  - phase: 25-il-codegen-method-bodies
    provides: "emit_expr() infrastructure, BodyEmitter, string ops (StrConcat, StrLen), register packing patterns"
provides:
  - "TailCall emission for Return(Call(...)) tail-position patterns (EMIT-24)"
  - "StrBuild emission for 3+ part string concatenation chains (EMIT-20 completeness)"
  - "emit_tail_call() pub(crate) helper usable from stmt.rs"
  - "try_collect_str_build_parts() + collect_string_chain() for left-associative chain detection"
affects:
  - Phase 26 CLI integration
  - Any future codegen pass that emits dialogue transition code

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tail-call detection at Return node: check if value is Call before emitting sub-expression"
    - "Pre-emission chain collection: StrBuild optimization detected at top of emit_expr() before match dispatch"
    - "Left-associative binary tree flattening via recursive collect_string_chain()"

key-files:
  created: []
  modified:
    - "writ-compiler/src/emit/body/expr.rs"
    - "writ-compiler/src/emit/body/stmt.rs"
    - "writ-compiler/tests/emit_body_tests.rs"

key-decisions:
  - "TailCall emitted for ANY Return(Call(...)) pattern, not just dialogue-flagged calls — dialogue transitions lower to regular Return+Call at AST level with no special flag"
  - "StrBuild threshold is 3+ parts; 2-part chains continue to use StrConcat (avoids unnecessary register packing overhead)"
  - "emit_tail_call() made pub(crate) so stmt.rs can call it directly for TypedStmt::Return case"
  - "StrBuild chain detection runs at top of emit_expr() before the main match — necessary to access original TypedExpr nodes before sub-expressions are emitted"

patterns-established:
  - "Pre-emission optimization: detect patterns before entering match dispatch when sub-expression access is needed"
  - "Consecutive register packing: same r_block + Mov pattern used consistently for TailCall, StrBuild, Call, SpawnTask"

requirements-completed: [EMIT-20, EMIT-24]

# Metrics
duration: 8min
completed: 2026-03-03
---

# Phase 25 Plan 06: TailCall and StrBuild Emission Summary

**TailCall emission for dialogue transition Return(Call) patterns and StrBuild for 3+ part string concatenation chains, closing EMIT-24 (blocker) and EMIT-20 (completeness gap)**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-03T13:22:00Z
- **Completed:** 2026-03-03T13:30:33Z
- **Tasks:** 1 (TDD: RED + GREEN commits)
- **Files modified:** 3

## Accomplishments

- Implemented TailCall emission: `TypedExpr::Return { value: Some(Call(...)) }` now emits `TailCall` instead of `Call + Ret`
- Implemented `TypedStmt::Return` tail-call detection in stmt.rs using the shared `emit_tail_call()` helper
- Implemented StrBuild emission: 3+ part left-associative string Add chains emit `StrBuild { count, r_base }` instead of nested `StrConcat`
- 2-part string chains continue to use `StrConcat` (StrBuild only for 3+ as per EMIT-20 spec)
- 79 tests pass (73 existing + 6 new TDD tests added in RED phase)

## Task Commits

TDD task with RED then GREEN commits:

1. **RED: failing tests** - `2ff842d` (test: add failing tests for TailCall and StrBuild emission)
2. **GREEN: implementation** - `2906859` (feat: implement TailCall and StrBuild emission)

**Plan metadata:** (docs commit follows)

_TDD task: RED commit with 6 failing tests, GREEN commit with implementation passing all 79 tests_

## Files Created/Modified

- `writ-compiler/src/emit/body/expr.rs` - Added TailCall detection in Return handler; added `emit_tail_call()`, `try_collect_str_build_parts()`, `collect_string_chain()`, `emit_str_build()` helpers; added StrBuild pre-emission check at top of `emit_expr()`
- `writ-compiler/src/emit/body/stmt.rs` - Updated `TypedStmt::Return` handler to call `emit_tail_call()` for Call-valued returns
- `writ-compiler/tests/emit_body_tests.rs` - Added 6 new tests: tail-call expr, tail-call stmt, normal ret boundary, 3-part str-build, 4-part str-build, 2-part str-concat boundary

## Decisions Made

- TailCall emitted for any `Return(Call(...))` pattern — no dialogue-specific flag needed because dialogue transitions lower to regular `Return+Call` at the AST level
- StrBuild only for 3+ parts; 2-part keeps StrConcat to avoid unnecessary register packing for the common case
- `emit_tail_call()` is `pub(crate)` so stmt.rs can call it; avoids duplicating the consecutive register packing logic
- StrBuild detection goes at the very top of `emit_expr()` before the match arm, because `collect_string_chain()` needs the original `TypedExpr` references before any sub-expressions are emitted

## Deviations from Plan

None - plan executed exactly as written. The implementation matched the plan's pseudocode with only minor structural adjustments (e.g., `emit_tail_call` pub(crate) visibility).

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 25 (IL Codegen: Method Bodies) fully complete across all 6 plans
- All 79 emit_body_tests pass; 10 serialize tests pass
- TailCall and StrBuild now emit correctly, completing the instruction coverage for method body emission
- Phase 26 (CLI integration) can connect `emit_bodies()` to the writ-cli pipeline

## Self-Check: PASSED

- expr.rs: FOUND (contains TailCall and StrBuild emission)
- stmt.rs: FOUND (contains tail-call delegation to emit_tail_call)
- emit_body_tests.rs: FOUND (6 new tests added)
- 25-06-SUMMARY.md: FOUND
- Commit 2ff842d (RED tests): FOUND
- Commit 2906859 (GREEN impl): FOUND
- 79 tests passing: VERIFIED

---
*Phase: 25-il-codegen-method-bodies*
*Completed: 2026-03-03*
