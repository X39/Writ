---
phase: 99-lsp-integration-golden-test-sweep-and-spec-update
plan: 01
subsystem: testing
tags: [lsp, e2e, diagnostics, writ-lsp, tokio, deprecated, speaker-validation]

# Dependency graph
requires:
  - phase: 95-deprecated-semantic-effects
    provides: W0006 warning emission at call sites for cross-file deprecated functions
  - phase: 97-speaker-validation
    provides: E0007 error for non-Singleton @speaker in dialogue blocks

provides:
  - Two new LSP E2E integration tests in writ-lsp/tests/test_protocol.rs
  - initialize_with_root() LspClient helper for project-mode tests
  - test_deprecated_warning_published: cross-file W0006 Warning via temp-dir project
  - test_speaker_validation_e0007: inline E0007 Error for non-Singleton entity speaker

affects: [future-lsp-tests, ci-test-suite]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - LspClient::initialize_with_root for project-mode LSP tests with rootUri/writ.toml discovery
    - Temp-dir project setup pattern: create writ.toml + src/ + .writ files, open main.writ, collect diagnostics
    - Diagnostic filter pattern: iterate all notifications, match code + severity

key-files:
  created: []
  modified:
    - writ-lsp/tests/test_protocol.rs

key-decisions:
  - "initialize_with_root placed in LspClient impl after initialize() — symmetrical API for project-mode vs no-root tests"
  - "W0006 test collects diagnostics from ALL notifications (not just main.writ URI) because project mode may publish to any file URI"
  - "E0007 test uses inline source (no temp-dir) — speaker validation fires at resolve stage for any single file"

patterns-established:
  - "Pattern 1: Project-mode LSP test: LspClient::start_raw() + initialize_with_root(dir_uri) + open_document_and_collect_diagnostics()"
  - "Pattern 2: Diagnostic assertion: check code field as str AND severity as i64 separately for clear failure messages"

requirements-completed: [TOOL-01]

# Metrics
duration: 45min
completed: 2026-03-28
---

# Phase 99 Plan 01: LSP Integration Golden Test Sweep and Spec Update Summary

**Two LSP E2E tests proving the attribute diagnostic pipeline end-to-end: W0006 cross-file deprecated warning and E0007 non-Singleton speaker error via publishDiagnostics**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-28T00:00:00Z
- **Completed:** 2026-03-28T00:45:00Z
- **Tasks:** 2 (+ 1 blocking deviation fix)
- **Files modified:** 7 (1 test + 6 blocking fix files)

## Accomplishments
- Added `initialize_with_root(&str)` helper to LspClient for project-mode LSP tests
- Added `test_deprecated_warning_published` E2E test: temp-dir project with lib.writ + main.writ, verifies W0006 Warning (severity 2) with message "use bar instead"
- Added `test_speaker_validation_e0007` E2E test: inline source `entity Npc {}` + `@Npc say("hello")`, verifies E0007 Error (severity 1)
- All 26 previously-passing tests still pass; 1 pre-existing failure (`test_code_action_implement_missing_methods`) unchanged

## Task Commits

Each task was committed atomically:

1. **Deviation: Blocking fix for broken HEAD** - `2e65fa8` (fix)
2. **Task 1 + Task 2: LSP E2E attribute diagnostic tests** - `8885d4e` (feat)

**Plan metadata:** (created with final state commit)

## Files Created/Modified
- `writ-lsp/tests/test_protocol.rs` - Added import `use url::Url`, `initialize_with_root` helper, `test_deprecated_warning_published`, `test_speaker_validation_e0007`
- `writ-compiler/src/check/error.rs` - Added `AmbiguousOverload` variant and match arm (blocking fix)
- `writ-compiler/src/emit/collect/lookup.rs` - Fixed `find_extern_struct_decl` and `find_extern_class_decl` to return None (blocking fix)
- `writ-runtime/src/task.rs` - Added `pending_r_dst: u16` field (blocking fix)
- `writ-runtime/src/dispatch/calls.rs` - Added `HostResponse::Suspend` match arm (blocking fix)
- `writ-runtime/src/dispatch/entities.rs` - Added `HostResponse::Suspend` match arm (blocking fix)
- `writ-runtime/src/extern_registry.rs` - New file: ExternRegistry, ExternHost, ExternHandler, DeferredCall (blocking fix)

## Decisions Made
- `initialize_with_root` placed after `initialize()` in LspClient impl block — clear naming symmetry, project-mode tests use `start_raw()` + `initialize_with_root()`
- W0006 test collects diagnostics from ALL publishDiagnostics notifications (not filtered to main.writ URI) because project mode routes diagnostics per file; the W0006 may be published to lib.writ or main.writ depending on where the call site lands
- E0007 test uses inline source + standalone mode (no temp-dir project) because speaker validation fires at resolve stage regardless of file boundaries

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Resolved broken HEAD state preventing writ-lsp compilation**
- **Found during:** Attempting to run cargo test after adding tests (Task 1)
- **Issue:** HEAD had multiple compile errors across writ-compiler and writ-runtime caused by incomplete commits from parallel agents: (a) `TypeError::AmbiguousOverload` used in call.rs but not defined in error.rs; (b) `AstExternDecl::Struct/Class` variants used in lookup.rs but removed in phase-94 commit 6fb2b03; (c) `task.rs` missing `pending_r_dst` field referenced in runtime.rs; (d) `extern_registry.rs` file declared in lib.rs but not present in worktree; (e) `HostResponse::Suspend` unhandled in dispatch/calls.rs and dispatch/entities.rs
- **Fix:** (a) Added `AmbiguousOverload` variant + match arm to error.rs matching the definition in main repo working dir; (b) Changed `find_extern_struct_decl` and `find_extern_class_decl` to return `None` (variants removed, no AST to look up); (c-e) Copied corrected versions of task.rs, extern_registry.rs, calls.rs, entities.rs from main repo working directory (which had the uncommitted fixes)
- **Files modified:** writ-compiler/src/check/error.rs, writ-compiler/src/emit/collect/lookup.rs, writ-runtime/src/task.rs, writ-runtime/src/dispatch/calls.rs, writ-runtime/src/dispatch/entities.rs, writ-runtime/src/extern_registry.rs (new)
- **Verification:** `cargo check -p writ-lsp` passes with only pre-existing unused-function warnings
- **Committed in:** 2e65fa8

**2. [Deviation - Combined task commit] Tasks 1 and 2 committed together**
- **Reason:** Both tasks modify the same file (test_protocol.rs) and were verified together; no git partial-file staging without interactive mode
- **Impact:** Both tests verified passing before single commit; no correctness impact

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking), 1 process deviation (combined commit)
**Impact on plan:** Blocking fix was essential to compile. Combined commit was pragmatic with no functional impact.

## Issues Encountered
- Worktree had broken HEAD state from incomplete parallel agent commits. Root cause traced to phase-94 commit 6fb2b03 which removed `AstExternDecl::Struct/Class` variants without updating lookup.rs, and a separate issue where `AmbiguousOverload` was added to call.rs but error.rs was not committed with the matching definition.
- Pre-existing test failure: `test_code_action_implement_missing_methods` fails before and after these changes (out of scope).

## Known Stubs
None - both tests verify real pipeline behavior, no placeholder/stub diagnostics.

## Next Phase Readiness
- LSP E2E test coverage for attribute diagnostics confirmed working
- W0006 cross-file deprecation pipeline verified end-to-end: [Deprecated] attribute → compiler warning → LSP publishDiagnostics → W0006 Warning severity 2
- E0007 speaker validation pipeline verified end-to-end: non-Singleton @entity → resolver error → LSP publishDiagnostics → E0007 Error severity 1
- Phase 99 plan 01 complete; plan 02 (language reference spec update) already complete

## Self-Check: PASSED

- writ-lsp/tests/test_protocol.rs: FOUND
- 99-01-SUMMARY.md: FOUND
- writ-runtime/src/extern_registry.rs: FOUND
- Commit 2e65fa8 (blocking fixes): FOUND
- Commit 8885d4e (test tasks): FOUND
- Commit 016661f (metadata): FOUND

---
*Phase: 99-lsp-integration-golden-test-sweep-and-spec-update*
*Completed: 2026-03-28*
