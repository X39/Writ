---
phase: 97-speaker-validation
plan: 01
subsystem: compiler
tags: [resolve, validation, dialogue, speaker, singleton, E0007, E0003]

# Dependency graph
requires:
  - phase: 96-conditional-attribute
    provides: validate.rs scaffold, dialogue lowering (Entity.getOrCreate<Name>() hoisted let pattern)
provides:
  - validate_speakers() full implementation detecting hoisted let speaker pattern
  - E0007 (InvalidSpeaker) for non-[Singleton] entity speakers with updated message
  - E0003 (UnresolvedName) for non-existent entity speakers
  - 4 new resolve_tests covering all speaker validation scenarios
affects: [writ-lsp, golden-tests, future-dialogue-phases]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hoisted let pattern: Entity.getOrCreate<Name>() uniquely identifies @Speaker references in lowered AST"
    - "Two-set entity collection: singletons and all-entities built in single pass over ASTs"
    - "FxHashSet for entity name lookup consistent with codebase rustc-hash conventions"

key-files:
  created: []
  modified:
    - writ-compiler/src/resolve/validate.rs
    - writ-compiler/src/resolve/error.rs
    - writ-compiler/tests/resolve_tests.rs

key-decisions:
  - "validate_speakers detects Entity.getOrCreate<Name>() hoisted let pattern rather than walking CST @Speaker tokens — lowering already does the disambiguation"
  - "Contract-typed param speakers (npc: Entity) are naturally invisible because they don't generate getOrCreate calls"
  - "E0007 primary label updated to 'entity is not [Singleton]' with actionable help text pointing to fix"
  - "Two-pass entity collection: build sets first, then walk fn bodies — avoids forward-reference ordering issues"

patterns-established:
  - "Speaker validation hooks into existing validate_speakers() stub, no mod.rs changes needed"
  - "FxHashSet<String> for O(1) entity-name lookup consistent with rustc-hash codebase convention"

requirements-completed: [SPKR-01, SPKR-02]

# Metrics
duration: 12min
completed: 2026-03-27
---

# Phase 97 Plan 01: Speaker Validation Summary

**validate_speakers() implemented — E0007 for non-[Singleton] entity speakers, E0003 for non-existent speakers, detected via Entity.getOrCreate<Name>() hoisted let pattern in lowered AST**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-27T21:30:00Z
- **Completed:** 2026-03-27T21:42:00Z
- **Tasks:** 2 (TDD RED + GREEN)
- **Files modified:** 3

## Accomplishments

- Implemented full `validate_speakers()` body in `writ-compiler/src/resolve/validate.rs` (replaces empty stub)
- Detects dialogue `@Speaker` references via lowered `let _name = Entity.getOrCreate<Name>()` hoisted let pattern
- E0007 emitted for non-[Singleton] entity speakers; E0003 emitted for non-existent entity speakers
- Contract-typed param speakers (`npc: Entity`) produce no false positives — they never generate a hoisted let
- Updated E0007 primary label from "speaker not found" to "entity is not [Singleton]" with actionable help text
- All 4 new speaker validation tests pass; 90 compiler tests + 48 golden tests pass with 0 regressions

## Task Commits

1. **Task 1: Write speaker validation tests** - `8ec2701` (test — TDD RED)
2. **Task 2: Implement validate_speakers() body** - `ca501e3` (feat — TDD GREEN)

## Files Created/Modified

- `writ-compiler/src/resolve/validate.rs` — Full `validate_speakers()` implementation with 4 helper functions: `collect_entity_sets`, `collect_entities_in_items`, `validate_speakers_in_items`, `check_stmts_for_speakers`, `extract_get_or_create_entity`
- `writ-compiler/src/resolve/error.rs` — Updated E0007 `InvalidSpeaker` primary label to "entity is not [Singleton]" with actionable help text
- `writ-compiler/tests/resolve_tests.rs` — 4 speaker validation test cases in new `// === Speaker validation (Phase 97) ===` section

## Decisions Made

- Entity name lookup uses two separate `FxHashSet<String>` sets (singleton_names and all_entity_names) built in a single pass over all ASTs before walking function bodies, avoiding forward-reference ordering issues
- Entity.getOrCreate pattern match checks: args.is_empty(), type_args.len() == 1, field == "getOrCreate", obj == "Entity" — all conditions required to avoid false positives on similar call shapes
- `AstImplDecl.members` (not `.methods`) contains `AstImplMember::Fn(f)` variants — matched correctly after initial compile error

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] AstImplDecl field name mismatch**
- **Found during:** Task 2 (Implement validate_speakers() body)
- **Issue:** Plan pseudocode referenced `impl_decl.methods` but the actual struct field is `impl_decl.members` containing `AstImplMember::Fn(f)` variants
- **Fix:** Changed to iterate `impl_decl.members`, matching `AstImplMember::Fn(f)` arm
- **Files modified:** writ-compiler/src/resolve/validate.rs
- **Verification:** Cargo build succeeded after fix; all tests pass
- **Committed in:** ca501e3 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1 bug — field name mismatch in plan pseudocode)
**Impact on plan:** Minor structural correction, no scope creep.

## Issues Encountered

None beyond the AstImplDecl field name mismatch documented above.

## Known Stubs

None — validate_speakers() is fully implemented, not a stub.

## Next Phase Readiness

- Speaker validation (SPKR-01, SPKR-02) is complete — RES-09 tech debt resolved
- Phase 98 (or next active phase) can proceed without speaker-validation blockers
- The `validate_speakers` call in `resolve/mod.rs` was already wired; no pipeline changes needed

---
*Phase: 97-speaker-validation*
*Completed: 2026-03-27*
