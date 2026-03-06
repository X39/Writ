---
phase: 45-writ-toml-project-file-compilation
plan: 01
subsystem: compiler
tags: [config, toml, serde, emit, debug-info, profiles]

# Dependency graph
requires: []
provides:
  - "ProfileConfig/ProfilesConfig structs in config.rs with serde deserialization from [profile.debug]/[profile.release] TOML"
  - "WritConfig.profile field with default debug_info=true for debug, debug_info=false for release"
  - "emit_bodies() emit_debug_info: bool parameter gating DebugLocal emission and header flags"
  - "serialize::translate() and serialize::serialize() emit_debug_info parameter"
affects:
  - "45-02 (writ build command) — depends on ProfileConfig types and emit_bodies(emit_debug_info) for profile-aware builds"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "serde default functions for ProfileConfig deserialization"
    - "Boolean flag threaded through emit pipeline to gate debug-only IL rows"

key-files:
  created: []
  modified:
    - "writ-compiler/src/config.rs"
    - "writ-compiler/src/emit/mod.rs"
    - "writ-compiler/src/emit/serialize.rs"
    - "writ-cli/src/main.rs"
    - "writ-golden/tests/golden_tests.rs"
    - "writ-cli/tests/e2e_compile_tests.rs"
    - "writ-compiler/tests/emit_serialize_tests.rs"
    - "writ-compiler/tests/emit_body_tests.rs"

key-decisions:
  - "emit_debug_info threaded as explicit boolean parameter (not stored on builder) — avoids mutating data structures post-emission and keeps the API explicit"
  - "header.flags=1 when emit_debug_info=true, flags=0 when false — consistent with spec debug flag semantics"
  - "All existing callers (tests, cmd_compile) pass true — no behavior change for current pipeline"

patterns-established:
  - "ProfileConfig pattern: use serde default functions (not impl Default) for field-level defaults to allow partial overrides in TOML"

requirements-completed: [TOOL-02]

# Metrics
duration: 3min
completed: 2026-03-06
---

# Phase 45 Plan 01: Profile Config and Debug-Info Emit Gating Summary

**ProfileConfig/ProfilesConfig with serde-deserialized TOML profile sections, plus emit_debug_info bool threaded through emit_bodies/serialize to gate DebugLocal row emission for release builds**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T21:56:09Z
- **Completed:** 2026-03-06T21:59:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added ProfileConfig (debug_info: bool) and ProfilesConfig (debug/release sub-sections) to config.rs with serde deserialization and correct defaults (debug=true, release=false)
- Extended WritConfig with `profile: ProfilesConfig` field, enabling `[profile.debug]` / `[profile.release]` TOML sections to control per-profile settings
- Added `emit_debug_info: bool` to `emit_bodies()`, `serialize::translate()`, and `serialize::serialize()` — when false, DebugLocal rows are empty Vec and header flags=0
- Updated all 7 call sites (cmd_compile, golden tests, e2e tests, unit tests) to pass `true`, preserving existing behavior
- Added 3 new unit tests covering profile defaults, explicit override, and partial override

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ProfileConfig to config.rs and extend WritConfig** - `89000b4` (feat)
2. **Task 2: Thread emit_debug_info through emit pipeline and gate DebugLocal emission** - `0a2301c` (feat)

## Files Created/Modified

- `writ-compiler/src/config.rs` - Added ProfileConfig, ProfilesConfig structs; extended WritConfig with profile field; added 3 new unit tests
- `writ-compiler/src/emit/mod.rs` - emit_bodies() gains emit_debug_info: bool parameter; passes to serialize::serialize
- `writ-compiler/src/emit/serialize.rs` - translate() and serialize() gain emit_debug_info: bool; DebugLocal gated; header.flags gated
- `writ-cli/src/main.rs` - cmd_compile passes true for emit_debug_info
- `writ-golden/tests/golden_tests.rs` - Updated emit_bodies call to pass true
- `writ-cli/tests/e2e_compile_tests.rs` - Updated emit_bodies call to pass true
- `writ-compiler/tests/emit_serialize_tests.rs` - Updated all serialize::serialize and emit_bodies calls to pass true
- `writ-compiler/tests/emit_body_tests.rs` - Updated emit_bodies call to pass true

## Decisions Made

- emit_debug_info threaded as an explicit boolean parameter rather than stored on the builder — keeps the API explicit and avoids mutating data structures post-emission. Plan 02 can pass the flag directly from the resolved profile config.
- header.flags=1 when emit_debug_info=true, flags=0 when false — consistent with the spec's debug flag semantics.
- All existing callers pass true — no behavior change for current single-file compile pipeline. Plan 02 will wire the actual profile selection.

## Deviations from Plan

None — plan executed exactly as written. The plan correctly predicted 5 call sites (writ-cli main, golden tests, e2e tests, emit_serialize_tests, emit_body_tests). Discovered `serialize::serialize` was also called directly in emit_serialize_tests (3 times), which were updated as part of the same task under the "any other callers" instruction.

## Issues Encountered

None.

## Next Phase Readiness

- Plan 02 can now pass `config.profile.debug.debug_info` or `config.profile.release.debug_info` directly to `emit_bodies()` based on the active build profile
- ProfileConfig and ProfilesConfig types are ready for use in the `writ build` subcommand

---
*Phase: 45-writ-toml-project-file-compilation*
*Completed: 2026-03-06*
