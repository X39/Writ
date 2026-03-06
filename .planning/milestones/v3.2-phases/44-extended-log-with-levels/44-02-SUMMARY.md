---
phase: 44-extended-log-with-levels
plan: 02
subsystem: cli-host, fixtures, spec
tags: [writ-cli, writ-golden, writ-parser, log-namespace, cli-host, spec-update, golden-bless]

# Dependency graph
requires:
  - phase: 44-extended-log-with-levels/44-01
    provides: 5 synthetic ExternFn DefIds (log::trace..error) in compiler pipeline
provides:
  - CliHost on_request dispatch for log::trace/debug/info/warn/error -> on_log(LogLevel, msg)
  - UPPERCASE on_log format: [TRACE]/[DEBUG]/[INFO]/[WARN]/[ERROR]
  - Re-blessed golden snapshots for fn_log_say_choice + all other fixtures
  - All parser test cases using log::info instead of bare log
  - Spec §26.4 documenting log:: namespace with 5 leveled functions
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CliHost log dispatch: match on resolved extern name 'log::trace'...'log::error', call on_log with LogLevel variant"
    - "UPPERCASE on_log: match arm per LogLevel variant -> static str prefix, eprintln! with bracket format"
    - "Golden bless-all: BLESS=1 cargo test -p writ-golden to re-bless all fixtures at once after pipeline change"

key-files:
  created: []
  modified:
    - writ-cli/src/cli_host.rs
    - writ-golden/tests/golden/fn_log_say_choice.writ
    - writ-golden/tests/golden/fn_log_say_choice.writil
    - writ-golden/tests/golden/fn_basic_call.writil
    - writ-golden/tests/golden/fn_empty_main.writil
    - writ-golden/tests/golden/fn_optional.writil
    - writ-golden/tests/golden/fn_recursion.writil
    - writ-golden/tests/golden/fn_typed_params.writil
    - writ-cli/tests/fixtures/hello.writ
    - writ-parser/tests/cases/07_functions.writ
    - writ-parser/tests/cases/09_entities.writ
    - writ-parser/tests/cases/11_error_handling.writ
    - writ-parser/tests/cases/14_attributes.writ
    - writ-parser/tests/cases/15_ranges_indexing.writ
    - writ-parser/tests/cases/16_generics.writ
    - writ-parser/tests/cases/17_globals_atomic.writ
    - writ-parser/tests/cases/18_extern.writ
    - writ-parser/tests/cases/20_comprehensive.writ
    - language-spec/spec/27_26_standard_library_builtins.md

key-decisions:
  - "All golden snapshots re-blessed as a batch (not just fn_log_say_choice) because Plan 01 emitter injects synthetic ExternDef rows for all modules, not just those using log"
  - "hello.writ fixture updated to log::info(test_string()) with explicit call parens — test_string is a fn()->string, not a value"
  - "18_extern.writ: extern fn log declaration removed entirely — log:: is compiler-injected, not user-declared"

patterns-established:
  - "End-to-end log dispatch: Writ source log::info('msg') -> CALL_EXTERN -> CliHost matches 'log::info' -> on_log(Info, msg) -> eprintln!('[INFO] msg')"

requirements-completed: [TOOL-03]

# Metrics
duration: 7min
completed: 2026-03-06
---

# Phase 44 Plan 02: CliHost Dispatch, Fixture Migration, and Spec Update Summary

**Full end-to-end log dispatch: CliHost routes log::trace/debug/info/warn/error to on_log with UPPERCASE prefixes; all fixtures migrated from bare log() to log::info(); all golden snapshots re-blessed; spec §26.4 documents the leveled log:: namespace**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-03-06T20:01:00Z
- **Completed:** 2026-03-06T20:08:00Z
- **Tasks:** 2
- **Files modified:** 18

## Accomplishments

- `CliHost.on_request` gains 5 new match arms for `"log::trace"` through `"log::error"`, each extracting `msg` from `display_args` and calling `on_log(LogLevel::X, msg)` — closes the dispatch gap from Plan 01
- `CliHost.on_log` updated from `{level:?}` format to UPPERCASE static strings: `[TRACE]`, `[DEBUG]`, `[INFO]`, `[WARN]`, `[ERROR]`
- New test `test_on_log_uppercase_format_arms_compile` verifies all 5 log-level extern names return `Value(Void)` from `on_request`
- `fn_log_say_choice.writ` simplified: removed `extern fn log` declaration (log:: is compiler-injected), changed `::log("msg")` to `::log::info("msg")`
- All 6 golden `.writil` snapshots re-blessed to include the 5 synthetic `// .extern_fn "log::trace/debug/info/warn/error"` comment rows emitted by Plan 01
- `hello.writ` CLI fixture updated: `::log(test_string)` → `::log::info(test_string())`
- All 9 parser test files migrated: bare `log(...)` calls replaced with `log::info(...)`, `extern fn log` declaration removed from 18_extern.writ
- Spec §26.4 rewritten: removed single `log` entry; added `log::` namespace section with 5-row table documenting each level, `LogLevel` routing, root-qualified form, and shadowing behavior

## Task Commits

Each task was committed atomically:

1. **Task 1: CliHost dispatch for log::level names + UPPERCASE format** - `399b6dd` (feat)
2. **Task 2: Migrate all fixtures, re-bless golden, update spec** - `a637ecb` (feat)

## Files Created/Modified

- `writ-cli/src/cli_host.rs` - Added 5 log::level match arms in on_request; UPPERCASE on_log; new unit test
- `writ-golden/tests/golden/fn_log_say_choice.writ` - Removed extern fn log; uses ::log::info
- `writ-golden/tests/golden/fn_log_say_choice.writil` - Re-blessed: CALL_EXTERN for log::info (token 0x10000005)
- `writ-golden/tests/golden/fn_basic_call.writil` - Re-blessed: synthetic extern_fn comment rows added
- `writ-golden/tests/golden/fn_empty_main.writil` - Re-blessed: synthetic extern_fn comment rows added
- `writ-golden/tests/golden/fn_optional.writil` - Re-blessed: synthetic extern_fn comment rows added
- `writ-golden/tests/golden/fn_recursion.writil` - Re-blessed: synthetic extern_fn comment rows added
- `writ-golden/tests/golden/fn_typed_params.writil` - Re-blessed: synthetic extern_fn comment rows added
- `writ-cli/tests/fixtures/hello.writ` - ::log(test_string) → ::log::info(test_string())
- `writ-parser/tests/cases/07_functions.writ` - 5 log() → log::info() replacements
- `writ-parser/tests/cases/09_entities.writ` - log() → log::info() in on create hook
- `writ-parser/tests/cases/11_error_handling.writ` - 2 log() → log::info() replacements
- `writ-parser/tests/cases/14_attributes.writ` - 2 log() → log::info() replacements
- `writ-parser/tests/cases/15_ranges_indexing.writ` - 3 log() → log::info() replacements
- `writ-parser/tests/cases/16_generics.writ` - 2 log() → log::info() replacements
- `writ-parser/tests/cases/17_globals_atomic.writ` - log() → log::info() replacement
- `writ-parser/tests/cases/18_extern.writ` - Removed extern fn log; log() → log::info()
- `writ-parser/tests/cases/20_comprehensive.writ` - 4 log() → log::info() replacements
- `language-spec/spec/27_26_standard_library_builtins.md` - §26.4 rewritten with log:: namespace table

## Decisions Made

- All golden snapshots needed re-blessing (not just fn_log_say_choice) because the Plan 01 emitter now injects 5 synthetic ExternDef rows for all modules — any module that goes through `inject_log_extern_defs()` gains those rows
- `hello.writ` fixture changed from `::log(test_string)` to `::log::info(test_string())` with explicit call parens — `test_string` is a `fn() -> string`, so it must be called to produce the string argument
- `18_extern.writ` removes the `extern fn log` declaration entirely — log:: is a compiler-known namespace and should not appear as a user-declared extern

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Other golden snapshots failed after fn_log_say_choice re-bless**
- **Found during:** Task 2 (full workspace test after fn_log_say_choice bless)
- **Issue:** Plan 01's emitter injects 5 synthetic ExternDef rows in ALL modules (not just fn_log_say_choice). This means every golden snapshot's `.writil` now includes the 5 `// .extern_fn "log::..."` comment rows — causing 5 other golden tests to fail with diffs
- **Fix:** Ran `BLESS=1 cargo test -p writ-golden` (bless-all) to re-generate all 6 snapshots at once
- **Files modified:** fn_basic_call.writil, fn_empty_main.writil, fn_optional.writil, fn_recursion.writil, fn_typed_params.writil
- **Verification:** All 10 golden tests pass after bless-all

This deviation was expected per the Plan 01 SUMMARY's "Issues Encountered" section ("Plan 02 will re-bless snapshots"), but the scope was all snapshots, not just fn_log_say_choice.

**Total deviations:** 1 auto-fixed (broader bless scope than stated in plan)
**Impact on plan:** None — the fix was exactly the `BLESS=1 cargo test -p writ-golden` command the plan already provided.

## Self-Check: PASSED

- writ-cli/src/cli_host.rs: FOUND
- writ-golden/tests/golden/fn_log_say_choice.writ: FOUND
- writ-golden/tests/golden/fn_log_say_choice.writil: FOUND
- writ-cli/tests/fixtures/hello.writ: FOUND
- language-spec/spec/27_26_standard_library_builtins.md: FOUND
- Commit 399b6dd: FOUND
- Commit a637ecb: FOUND

---
*Phase: 44-extended-log-with-levels*
*Completed: 2026-03-06*
