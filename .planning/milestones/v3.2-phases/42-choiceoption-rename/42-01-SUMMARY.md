---
phase: 42-choiceoption-rename
plan: 01
subsystem: compiler
tags: [lowering, dialogue, snapshot, rename, emit, insta]

# Dependency graph
requires:
  - phase: 41-fix-fn-log-say-choice
    provides: fn_log_say_choice golden test passing with simplified ::choice() form

provides:
  - ChoiceOption renamed from Option in the choice option constructor emit site
  - Four insta snapshots re-blessed with ChoiceOption in choice arm callee positions
  - Spec 29_28_lowering_reference.md updated with ChoiceOption at lines 53 and 57
  - choice_option_emits_externdef integration test proving end-to-end ExternDef path
affects: [phase-43-none-some, any phase adding dialogue or choice features]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "cargo insta test --accept for snapshot reblessing after intentional name changes"
    - "Use Tier 1 speaker (Entity param) in emit tests to avoid Entity.getOrCreate<T> hoisting which breaks typecheck"

key-files:
  created:
    - writ-compiler/tests/emit_tests.rs (new test section + choice_option_emits_externdef)
  modified:
    - writ-compiler/src/lower/dialogue.rs
    - language-spec/spec/29_28_lowering_reference.md
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_basic.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_label_key_emitted.snap
    - writ-compiler/tests/snapshots/lowering_tests__dlg_choice_speaker_scope_isolation.snap
    - writ-compiler/tests/snapshots/lowering_tests__integration_all_constructs.snap

key-decisions:
  - "Tier 1 speaker (Entity param) used in emit test to avoid Entity.getOrCreate<T> resolver failure — generic extern fn declarations do not push type params into scope"
  - "extern fn Entity.getOrCreate<T>() -> T removed from test source — virtual module intrinsics already provide it without needing explicit declaration"

patterns-established:
  - "Pattern: emit tests requiring dlg blocks with speakers should use an Entity param (Tier 1) not singleton speakers (Tier 2) to avoid Entity.getOrCreate hoisting"

requirements-completed: [LANG-01]

# Metrics
duration: 20min
completed: 2026-03-06
---

# Phase 42 Plan 01: ChoiceOption Rename Summary

**Atomic rename of dialogue choice option constructor from "Option" to "ChoiceOption" across lower/dialogue.rs, four insta snapshots, and spec — plus choice_option_emits_externdef integration test**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-06T14:50:00Z
- **Completed:** 2026-03-06T15:09:32Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Single emit site in `lower/dialogue.rs` renamed: `name: "ChoiceOption".to_string()`
- Four insta snapshots re-blessed via `cargo insta test --accept`: dlg_choice_basic, dlg_choice_label_key_emitted, dlg_choice_speaker_scope_isolation, integration_all_constructs
- Spec file `29_28_lowering_reference.md` updated at both Option(...) occurrences (lines 53 and 57)
- New integration test `choice_option_emits_externdef` verifies the full emit pipeline produces an ExternDef row named "ChoiceOption"
- `option_is_enum_with_one_generic_param` runtime test passes confirming Option<T> TypeDef is untouched
- `cargo test --workspace` passes with zero failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Rename emit site, update spec, bless four snapshots** - `23b1f65` (feat)
2. **Task 2: Add choice_option_emits_externdef integration test** - `27d54e8` (test)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `writ-compiler/src/lower/dialogue.rs` - Single emit site renamed: "ChoiceOption".to_string() at the Build comment and callee name
- `language-spec/spec/29_28_lowering_reference.md` - Two spec occurrences updated: lines 53 and 57
- `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_basic.snap` - Re-blessed with "ChoiceOption" in both choice arm callees
- `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_label_key_emitted.snap` - Re-blessed with "ChoiceOption" in both choice arm callees
- `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_speaker_scope_isolation.snap` - Re-blessed with "ChoiceOption" in both choice arm callees
- `writ-compiler/tests/snapshots/lowering_tests__integration_all_constructs.snap` - Re-blessed with "ChoiceOption" in two choice arm callees (Option<string> type reference untouched)
- `writ-compiler/tests/emit_tests.rs` - Added ChoiceOption rename tests section with choice_option_emits_externdef test

## Decisions Made

- **Tier 1 speaker in emit test**: The test uses `dlg ask(narrator: Entity)` with `@narrator` (a dlg param, Tier 1) instead of `@Narrator` (singleton, Tier 2). Tier 2 triggers `Entity.getOrCreate<Narrator>()` hoisting in lowered AST; the typecheck step then fails with E0102 "undefined variable `Entity`" because the typecheck treats `Entity` as a variable identifier, not a type namespace. Using Tier 1 avoids the hoist entirely.
- **No extern fn Entity.getOrCreate<T>(): The explicit generic extern fn declaration fails resolver with E0003 "cannot find name T in scope" — the resolver's ExternDecl::Fn handler (lines 441-458 in resolver.rs) does not push generic type params into scope before resolving param/return types. Removed the declaration from the test; the virtual module's intrinsic already provides Entity static methods.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Generic extern fn declarations do not push type params into scope**
- **Found during:** Task 2 (choice_option_emits_externdef test)
- **Issue:** `extern fn Entity.getOrCreate<T>() -> T` caused resolver E0003 "cannot find name T in scope" — `resolver.rs` lines 441-458 handle `AstExternDecl::Fn` without pushing generics onto the scope chain first, unlike regular fn/struct/contract handlers which call `scope.push_generics(generic_names)` before resolving bodies.
- **Fix:** Removed the problematic `extern fn Entity.getOrCreate<T>() -> T` declaration from the test source. Used a dlg param (Tier 1 speaker) to avoid Entity.getOrCreate hoisting entirely. The virtual module already registers `entity_get_or_create` as an intrinsic.
- **Files modified:** `writ-compiler/tests/emit_tests.rs`
- **Verification:** `cargo test -p writ-compiler --test emit_tests -- choice_option_emits_externdef` passes
- **Committed in:** `27d54e8` (Task 2 commit)

**Note:** The root bug (generic extern fn resolver gap) is deferred — it does not affect correctness of the rename or the test's coverage of the ExternDef path. Logged as a known limitation.

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug workaround)
**Impact on plan:** Test adjusted to achieve the same ExternDef verification goal via a simpler source pattern. No scope creep; rename coverage is complete.

## Issues Encountered

- The test source in the plan used `@Narrator` (singleton Tier 2 speaker) which triggered Entity.getOrCreate hoisting that the typecheck couldn't resolve. Switched to `@narrator` (dlg param Tier 1) which avoids hoisting entirely while still exercising the full choice option lowering path.

## Next Phase Readiness

- ChoiceOption rename complete across all four layers — Phase 43 (None/Some injection) can proceed
- The generic extern fn resolver gap (E0003 on type params in return types) is a known limitation that should be addressed in a future plan; deferred to deferred-items

---
*Phase: 42-choiceoption-rename*
*Completed: 2026-03-06*
