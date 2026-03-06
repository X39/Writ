---
phase: 40-spec-cleanup
plan: 03
subsystem: compiler-config
tags: [serde, toml, config, scaffold, writ-compiler, writ-cli]

# Dependency graph
requires:
  - phase: 40-spec-cleanup
    provides: Phase context and SPEC-03 requirement definition
provides:
  - LocaleConfig with correct serde rename attributes (default/supported TOML keys)
  - Scaffold writ.toml with active [compiler] sources = ["sources/"] entry
  - Tests: parse_basic_config (updated), locale_without_supported (new), scaffold_toml_round_trips (new)
affects:
  - Any phase using load_config or LocaleConfig
  - writ new project scaffolding correctness

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "serde rename attributes to decouple Rust field names from TOML key names (avoids reserved keyword collision)"
    - "#[serde(default)] on Vec<String> to handle optional TOML sections without error"

key-files:
  created: []
  modified:
    - writ-compiler/src/config.rs
    - writ-cli/src/main.rs

key-decisions:
  - "Use #[serde(rename)] attributes instead of renaming Rust fields — 'default' is a reserved keyword in Rust"
  - "Pre-existing writ-golden snapshot failures are out-of-scope and logged as deferred; not introduced by this plan"

patterns-established:
  - "TDD flow: write failing tests first (RED commit), then fix implementation (GREEN commit)"
  - "serde rename pattern for TOML-facing structs where spec key names conflict with Rust keywords"

requirements-completed: [SPEC-03]

# Metrics
duration: 15min
completed: 2026-03-06
---

# Phase 40 Plan 03: Config Serde Rename and Scaffold Fix Summary

**LocaleConfig now deserializes spec-compliant TOML keys (default/supported) via serde rename; scaffold writ.toml emits active [compiler] sources pointing to the generated sources/ directory**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-06T00:00:00Z
- **Completed:** 2026-03-06T00:15:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Fixed `LocaleConfig` serde deserialization: TOML `default` key maps to `default_locale` field, `supported` maps to `locales` field (with `#[serde(default)]` so missing key deserializes to empty Vec)
- Updated `parse_basic_config` test to use correct TOML keys (`default`, `supported`)
- Added two new tests: `locale_without_supported` (optional field) and `scaffold_toml_round_trips` (end-to-end scaffold simulation)
- Uncommented `[compiler]` section and `sources = ["sources/"]` in the `writ new` scaffold template so `discover_source_files` finds `sources/main.writ` on a fresh project

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add failing tests** - `d6d5b06` (test)
2. **Task 1 GREEN: Fix LocaleConfig serde renames** - `a21d0b9` (feat)
3. **Task 2: Uncomment sources in writ new scaffold** - `b9f06ce` (feat)

_Note: TDD task 1 has two commits (test RED → feat GREEN)_

## Files Created/Modified
- `writ-compiler/src/config.rs` - Added `#[serde(rename = "default")]` and `#[serde(rename = "supported")]` + `#[serde(default)]` to `LocaleConfig`; updated `parse_basic_config` test TOML keys; added `locale_without_supported` and `scaffold_toml_round_trips` tests
- `writ-cli/src/main.rs` - Changed scaffold [compiler] section from commented-out block to active `[compiler]\nsources = ["sources/"]`

## Decisions Made
- Used `#[serde(rename)]` attributes rather than renaming Rust struct fields — the spec key `default` is a reserved keyword in Rust and cannot be used as a field name
- Pre-existing `writ-golden` snapshot test failures (4 failing: test_fn_basic_call, test_fn_empty_main, test_fn_recursion, test_fn_typed_params) confirmed to exist before this plan's changes; logged as deferred, not fixed

## Deviations from Plan

None - plan executed exactly as written. TDD flow applied as specified.

## Issues Encountered
- Workspace `cargo test --workspace` revealed pre-existing `writ-golden` snapshot failures. Confirmed by running tests on clean git state (before any changes). Failures are snapshot drift unrelated to this plan's scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- SPEC-03 complete: `load_config` correctly deserializes scaffold-generated `writ.toml` using spec-compliant TOML keys
- All 6 config tests green: `parse_basic_config`, `locale_without_supported`, `scaffold_toml_round_trips`, `default_sources_when_omitted`, `discover_writ_files`, `missing_toml_error`
- Pre-existing `writ-golden` snapshot failures should be addressed in a future phase

---
*Phase: 40-spec-cleanup*
*Completed: 2026-03-06*
