---
phase: 96-conditional-semantic-effects
plan: 02
subsystem: compiler
tags: [rust, emit, conditional-compilation, cli, golden-tests, attributes]

# Dependency graph
requires:
  - phase: 96-conditional-semantic-effects
    plan: 01
    provides: conditional_fns and fallback_for_conditional fields on TypedAst; E0009/E0010 codes

provides:
  - active_conditions parameter threaded from CLI through run_pipeline to emit_bodies and collect_defs
  - skipped_def_ids pre-scan in collect_defs for conditional/fallback emit-time filtering
  - E0010 emitted when multiple active conditions target the same fallback function
  - --condition CLI flag on Compile and Build subcommands (Vec<String>, clap Append)
  - writ build merges writ.toml [conditions] true values with --condition flags
  - compile_and_disassemble_with_conditions helper in golden_tests.rs
  - conditional_active.writil and conditional_inactive.writil golden snapshots

affects:
  - writ-runtime (conditions may influence runtime module loading in future)
  - any future phase reading emit_bodies signature

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pre-scan pass in collect_defs: compute skipped_def_ids before main decl iteration loop"
    - "HashSet<String> active_conditions threaded as parameter through all pipeline stages"
    - "CLI --condition Vec<String> with clap ArgAction::Append for repeatable flags"
    - "writ build merges toml conditions (true) + CLI flags; neither suppresses the other"
    - "Compile-time empty HashSet for callers without condition support (DAP, LSP, compile_source)"

key-files:
  created:
    - writ-golden/tests/golden/conditional_active.writ
    - writ-golden/tests/golden/conditional_active.writil
    - writ-golden/tests/golden/conditional_inactive.writ
    - writ-golden/tests/golden/conditional_inactive.writil
  modified:
    - writ-compiler/src/emit/collect/mod.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/lib.rs
    - writ-cli/src/pipeline.rs
    - writ-cli/src/main.rs
    - writ-cli/src/commands/compile.rs
    - writ-cli/src/commands/build.rs
    - writ-dap/src/launch.rs
    - writ-lsp/src/analysis_host.rs
    - writ-golden/tests/golden_tests.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs
    - writ-cli/tests/e2e_compile_tests.rs

key-decisions:
  - "Callers without condition support (DAP, LSP, compile_source, tests) pass empty HashSet — no API breakage"
  - "Inactive conditional variant skip is unconditional: the fn is always suppressed when condition not active"
  - "Golden files use log::info (single-arg) not say (two-arg) to avoid entity-speaker requirement"
  - "Both conditional_active.writ and conditional_inactive.writ are identical source — difference is compile-time conditions only"

patterns-established:
  - "Pre-scan pattern in collect_defs: compute skip set before iteration, check at top of Fn arm"
  - "active_conditions as HashSet<String> reference passed all the way from CLI dispatch to collect_defs"

requirements-completed: [COND-01, COND-02, COND-03, COND-04]

# Metrics
duration: 22min
completed: 2026-03-27
---

# Phase 96 Plan 02: Conditional Compilation Pipeline and Golden Tests Summary

**Active-conditions HashSet threaded from --condition CLI flag through run_pipeline and emit_bodies to collect_defs emit-time filtering, with E0010 for ambiguous conditions and golden tests proving COND-01/COND-02 via round-trip compile+disassemble.**

## Performance

- **Duration:** 22 min
- **Started:** 2026-03-27T20:30:00Z
- **Completed:** 2026-03-27T20:52:57Z
- **Tasks:** 2/2
- **Files modified:** 13

## Accomplishments

- Added `active_conditions: &HashSet<String>` parameter to `collect_defs`, `emit_bodies`, and `run_pipeline`; pre-scan in `collect_defs` computes `skipped_def_ids` to suppress inactive conditional variants and active fallbacks
- E0010 fires when multiple active conditions target the same fallback function (COND-03 emit-time check)
- `--condition name` CLI flag added to `Compile` and `Build` subcommands; `writ build` merges writ.toml `[conditions]` true values with CLI flags
- All non-condition-aware callers (writ-dap, writ-lsp, compile_source, test suites) updated to pass empty HashSet — no breaking API changes to external crates
- Golden tests `test_conditional_active` (conditions=["debug"]) and `test_conditional_inactive` (conditions=[]) both pass, proving COND-01 and COND-02 end-to-end

## Task Commits

Each task was committed atomically:

1. **Task 1: Emit-time filtering, pipeline threading, CLI flag** - `f667da9` (feat)
2. **Task 2: Golden tests for conditional active and inactive scenarios** - `7944a68` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/collect/mod.rs` - added `active_conditions` param, `skipped_def_ids` pre-scan, E0010 emission
- `writ-compiler/src/emit/mod.rs` - added `active_conditions` param to `emit_bodies`; `emit()` uses empty set
- `writ-compiler/src/lib.rs` - updated `compile_source` to pass empty active_conditions
- `writ-cli/src/pipeline.rs` - added `active_conditions` param to `run_pipeline`
- `writ-cli/src/main.rs` - added `--condition` flag to Build and Compile subcommands
- `writ-cli/src/commands/compile.rs` - accepts `condition: Vec<String>`, constructs HashSet, passes to pipeline
- `writ-cli/src/commands/build.rs` - merges writ.toml conditions + CLI flags, passes to pipeline
- `writ-dap/src/launch.rs` - updated to pass empty active_conditions (DAP has no condition support)
- `writ-lsp/src/analysis_host.rs` - updated to pass empty active_conditions (LSP has no condition support)
- `writ-golden/tests/golden_tests.rs` - added `compile_and_disassemble_with_conditions` and `run_golden_test_with_conditions` helpers; two new test entries
- `writ-golden/tests/golden/conditional_active.writ` - source for active condition test (uses log::info)
- `writ-golden/tests/golden/conditional_active.writil` - blessed output: one greet with "Debug greeting"
- `writ-golden/tests/golden/conditional_inactive.writ` - same source as active (proves emit-time-only)
- `writ-golden/tests/golden/conditional_inactive.writil` - blessed output: one greet with "Default greeting"
- `writ-compiler/tests/emit_body_tests.rs`, `emit_serialize_tests.rs`, `writ-cli/tests/e2e_compile_tests.rs` - updated to pass empty active_conditions

## Decisions Made

- Golden test source files intentionally use `log::info` instead of `say` — `say` requires a speaker entity argument and would complicate the test unnecessarily
- `emit()` (metadata-only path) passes an empty HashSet — conditional filtering only needed for full binary emission
- Both conditional_active.writ and conditional_inactive.writ are identical source, proving elision is emit-time only

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed say() arity error in golden source files**
- **Found during:** Task 2 bless attempt
- **Issue:** Golden source used `say("message")` with 1 arg, but `say` signature is `(Entity, string) -> void` — arity error at type-check
- **Fix:** Replaced `say("...")` with `log::info("...")` which takes a single string argument
- **Files modified:** conditional_active.writ, conditional_inactive.writ
- **Verification:** BLESS=1 run succeeded; both tests pass
- **Committed in:** 7944a68 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — wrong builtin signature in test source)
**Impact on plan:** Fix was minimal, no scope change. Plan goal achieved.

## Issues Encountered

None beyond the say() arity fix above.

## Next Phase Readiness

- COND-01 through COND-04 all complete. The [Conditional] feature is fully wired end-to-end.
- writ-lsp and writ-dap pass empty conditions; conditional compilation is a CLI-only feature for now.
- Phase 96 is complete.

## Self-Check: PASSED

- SUMMARY.md exists at .planning/phases/96-conditional-semantic-effects/96-02-SUMMARY.md
- conditional_active.writ, conditional_active.writil, conditional_inactive.writ, conditional_inactive.writil all exist
- Commit f667da9 exists (Task 1: pipeline threading + CLI flag)
- Commit 7944a68 exists (Task 2: golden tests)
- cargo test -p writ-golden conditional passes 2/2
- cargo test --workspace passes 0 failures

---
*Phase: 96-conditional-semantic-effects*
*Completed: 2026-03-27*
