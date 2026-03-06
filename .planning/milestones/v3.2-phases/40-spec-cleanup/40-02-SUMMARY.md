---
phase: 40-spec-cleanup
plan: 02
subsystem: spec
tags: [language-spec, dialogue, namespace, inbuilt-calls, log, say, choice]

# Dependency graph
requires: []
provides:
  - "Section 26.4 Root-Namespace Inbuilt Calls: log/say/choice documented as root-namespace inbuilts"
  - "Section 13.9 updated: no longer references Runtime namespace or 'not callable directly'"
  - "Section 28.5 updated: no longer references Runtime namespace"
affects: [41-fn-log-say-choice, 42-lower, 44-tooling]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Root-namespace inbuilt calls: log/say/choice available with no qualifier in any scope"

key-files:
  created: []
  modified:
    - language-spec/spec/27_26_standard_library_builtins.md
    - language-spec/spec/14_13_dialogue_blocks_dlg.md
    - language-spec/spec/29_28_lowering_reference.md

key-decisions:
  - "log/say/choice are root-namespace inbuilt calls — no Runtime:: qualifier needed or accepted"
  - "§26.4 is the canonical definition; §13.9 and §28.5 cross-reference it"

patterns-established:
  - "Inbuilt call pattern: compiler resolves from root namespace, no qualifier, callable as bare names"

requirements-completed: [SPEC-02]

# Metrics
duration: 1min
completed: 2026-03-06
---

# Phase 40 Plan 02: Root-Namespace Inbuilt Calls Spec Fix Summary

**Added §26.4 defining log/say/choice as root-namespace inbuilt calls; removed contradictory "Runtime namespace" wording from §13.9 and §28.5 across three spec files**

## Performance

- **Duration:** ~1 min
- **Started:** 2026-03-06T13:05:49Z
- **Completed:** 2026-03-06T13:06:51Z
- **Tasks:** 1
- **Files modified:** 3

## Accomplishments

- Added §26.4 "Root-Namespace Inbuilt Calls" to `27_26_standard_library_builtins.md` with a table of `log`, `say`, and `choice` — their signatures, purpose, and no-qualifier contract
- Rewrote §13.9 in `14_13_dialogue_blocks_dlg.md`: removed "Runtime namespace" and "not callable directly" wording; functions now listed without `Runtime.` prefix; cross-reference to §26.4 added
- Rewrote §28.5 preamble in `29_28_lowering_reference.md`: replaced "Runtime namespace" with "root-namespace inbuilts (§26.4)"; table and heading unchanged
- Zero occurrences of "Runtime namespace" remain across all spec files under `language-spec/spec/`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add §26.4 and fix §13.9 and §28.5** - `cd67679` (feat)

**Plan metadata:** (docs commit to follow)

## Files Created/Modified

- `language-spec/spec/27_26_standard_library_builtins.md` - Added §26.4 Root-Namespace Inbuilt Calls (table + contract prose)
- `language-spec/spec/14_13_dialogue_blocks_dlg.md` - §13.9 rewritten: Runtime namespace removed, §26.4 cross-reference added
- `language-spec/spec/29_28_lowering_reference.md` - §28.5 preamble rewritten: Runtime namespace removed, §26.4 cross-reference added

## Decisions Made

- `log`, `say`, and `choice` are **inbuilt calls** resolved from the root namespace — no `writ::`, `Runtime::`, or other qualifier is needed or accepted. This is the canonical definition in §26.4; all other sections cross-reference it.
- `say` and `choice` are transition points (VM suspends); `log` is fire-and-forget (does not suspend). Both behaviors documented in §26.4.
- The `writ-runtime` module name used in IL spec binary-format sections is unrelated and was not changed.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- SPEC-02 requirement satisfied: §26.4 is the authoritative definition of root-namespace inbuilt calls
- Phase 41 (fn_log_say_choice fix) can now reference §26.4 as the spec anchor when diagnosing why empty method bodies occur for `::log` resolution
- All three spec locations are internally consistent — no contradictory namespace wording remains

---
*Phase: 40-spec-cleanup*
*Completed: 2026-03-06*
