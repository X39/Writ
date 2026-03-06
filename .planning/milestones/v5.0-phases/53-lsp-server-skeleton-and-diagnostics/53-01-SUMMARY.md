---
phase: 53-lsp-server-skeleton-and-diagnostics
plan: 01
subsystem: lsp
tags: [tower-lsp, lsp-types, chumsky, writ-compiler, diagnostics, analysis-host]

# Dependency graph
requires:
  - phase: 52-compiler-and-runtime-preparation
    provides: emit_all_bodies per-function error tolerance, SourceSpan line numbers
provides:
  - writ-lsp crate with UTF-16-aware span conversion (convert.rs)
  - AnalysisHost with standalone and project-mode analysis (analysis_host.rs)
  - Structured LSP Diagnostic output from all 4 compiler stages
affects: [53-03-lsp-backend, 53-04-tower-lsp-server, 54-semantic-features]

# Tech tracking
tech-stack:
  added: [tower-lsp 0.20, lsp-types 0.94, tokio 1, dashmap 6, url 2, serde_json 1]
  patterns:
    - Box::leak for &'static str lifetime (consistent with run_pipeline in writ-cli)
    - catch_unwind around resolve/typecheck to prevent panics crashing the server
    - Cascade strategy: all 4 pipeline stages run even when earlier stages have errors
    - source_for_file: &dyn Fn(FileId) -> &'static str for span conversion closures

key-files:
  created:
    - writ-lsp/Cargo.toml
    - writ-lsp/src/lib.rs
    - writ-lsp/src/main.rs
    - writ-lsp/src/convert.rs
    - writ-lsp/src/analysis_host.rs
  modified:
    - Cargo.toml (added writ-lsp to workspace members)

key-decisions:
  - "source_for_file closure uses &'static str (not &str) matching the Box::leak pattern used throughout the pipeline"
  - "parse_error_to_diag uses Debug formatting for Token since Token does not implement Display"
  - "AnalysisHost is stateless; caching/incremental analysis deferred to Phase 54+"
  - "catch_unwind wraps resolve and typecheck: error AST nodes can cause panics, server must not crash"
  - "Cascade strategy: run all 4 stages regardless of earlier errors (maximizes diagnostic coverage)"

patterns-established:
  - "Box::leak pattern: leak source string to get &'static str for parser (mirrors writ-cli run_pipeline)"
  - "Internal panic diagnostic (E9999): emit structured error when a compiler stage panics rather than propagating"

requirements-completed: [LSP-01, LSP-08]

# Metrics
duration: 14min
completed: 2026-03-14
---

# Phase 53 Plan 01: writ-lsp Crate with Diagnostic Conversion and AnalysisHost Summary

**writ-lsp crate with UTF-16-aware span conversion and full 4-stage compiler pipeline AnalysisHost for LSP diagnostics**

## Performance

- **Duration:** 14 min
- **Started:** 2026-03-14T00:25:38Z
- **Completed:** 2026-03-14T00:39:59Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- UTF-16-aware LSP Position/Range conversion with correct multi-byte character handling (16 tests)
- AnalysisHost running full parse/lower/resolve/typecheck pipeline with cascade error collection (7 tests)
- parse_error_to_diag converts chumsky Rich errors to structured LSP diagnostics (no silent drops)
- analyze_project with writ.toml project discovery and standalone fallback on missing toml
- catch_unwind guards prevent single malformed file from crashing the language server

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-lsp crate with diagnostic conversion layer** - `0b5b90a` (feat)
2. **Task 2: Create AnalysisHost with standalone and project-mode analysis** - `05709da` (feat)

**Plan metadata:** (created in this step)

_Note: TDD tasks had RED (implicit in first compile errors) then GREEN phases within each commit_

## Files Created/Modified

- `Cargo.toml` - Added writ-lsp to workspace members array
- `writ-lsp/Cargo.toml` - Crate manifest with tower-lsp 0.20, lsp-types 0.94, tokio 1, dashmap 6
- `writ-lsp/src/lib.rs` - Public module declarations (convert, analysis_host)
- `writ-lsp/src/main.rs` - Minimal placeholder binary (tower-lsp wiring in Plan 53-03)
- `writ-lsp/src/convert.rs` - offset_to_position (UTF-16), span_to_range, severity_to_lsp, writ_diag_to_lsp, parse_error_to_diag
- `writ-lsp/src/analysis_host.rs` - AnalysisResult, AnalysisHost::analyze_standalone, AnalysisHost::analyze_project

## Decisions Made

- **source_for_file closure signature**: Used `&'static str` instead of `&str` to match the Box::leak lifetime used throughout the pipeline. The Rust borrow checker requires a named lifetime or 'static for the dyn Fn return type; 'static is already the established pattern.
- **Token debug formatting**: `writ_parser::Token` does not implement `Display`, only `Debug`. Used `{:?}` for expected token formatting in parse error messages.
- **Stateless AnalysisHost**: No caching for v5.0 — each call recompiles from scratch. Incremental analysis (salsa) is explicitly out of scope per REQUIREMENTS.md.
- **catch_unwind on resolve/typecheck**: The compiler stages may panic on error AST nodes produced by partial parse recovery. Wrapping in catch_unwind means a broken file causes a structured E9999 diagnostic rather than a server crash.
- **Cascade strategy**: All 4 stages run regardless of earlier errors. This maximizes the number of diagnostics shown to the user in one pass, which is more useful than stopping at the first stage with errors.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed source_for_file lifetime in writ_diag_to_lsp**
- **Found during:** Task 1 (convert.rs compilation)
- **Issue:** `&dyn Fn(FileId) -> &str` requires a named lifetime — the compiler could not infer the return lifetime
- **Fix:** Changed to `&dyn Fn(FileId) -> &'static str` matching the Box::leak pattern already established
- **Files modified:** writ-lsp/src/convert.rs
- **Verification:** Compiles without error
- **Committed in:** 0b5b90a (Task 1 commit)

**2. [Rule 1 - Bug] Fixed DiagnosticBuilder::new does not exist**
- **Found during:** Task 1 (convert.rs test compilation)
- **Issue:** Plan spec said `DiagnosticBuilder::new(...)` but the API uses `Diagnostic::error(...)` / `Diagnostic::warning(...)` as constructors
- **Fix:** Changed all test and implementation uses to `Diagnostic::error(code, msg).with_primary(...).build()`
- **Files modified:** writ-lsp/src/convert.rs
- **Verification:** All 16 convert tests pass
- **Committed in:** 0b5b90a (Task 1 commit)

**3. [Rule 1 - Bug] Fixed Token Display formatting in parse_error_to_diag**
- **Found during:** Task 1 (convert.rs compilation)
- **Issue:** `format!("{}", err)` fails because `Rich<Token>` requires `Token: Display` which is not implemented; Token only implements Debug
- **Fix:** Manually format message from `err.expected()` and `err.found()` using `{:?}` debug formatting
- **Files modified:** writ-lsp/src/convert.rs
- **Verification:** parse_error_to_diag test produces non-empty message
- **Committed in:** 0b5b90a (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 - bugs from API/type system mismatch)
**Impact on plan:** All fixes were for compile errors from incorrect API assumptions in the plan spec. No scope creep.

## Issues Encountered

- **Disk space**: The D: drive reached 100% capacity during workspace-wide test execution (`cargo test --workspace`). The workspace test run failed with linker errors due to no disk space. This is an infrastructure issue, not a code regression. The writ-lsp library tests (23/23) passed cleanly on the first full test run before disk exhaustion.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- convert.rs and analysis_host.rs are the data/logic layer that Plan 53-03 (tower-lsp backend) will call
- `AnalysisHost::analyze_standalone` and `AnalysisHost::analyze_project` are the primary integration points
- `writ_diag_to_lsp` in convert.rs converts structured diagnostics to LSP format for the backend to push to clients
- Disk space needs to be freed before next workspace build/test run

---
*Phase: 53-lsp-server-skeleton-and-diagnostics*
*Completed: 2026-03-14*
