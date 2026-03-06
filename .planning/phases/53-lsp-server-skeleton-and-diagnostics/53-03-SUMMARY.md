---
phase: 53-lsp-server-skeleton-and-diagnostics
plan: 03
subsystem: lsp
tags: [tower-lsp, backend, diagnostics, publish-diagnostics, lsp-server, tokio]

# Dependency graph
requires:
  - phase: 53-01
    provides: AnalysisHost, convert.rs (writ_diag_to_lsp, parse_error_to_diag)
provides:
  - writ-lsp binary that speaks LSP over stdio
  - Backend struct with tower-lsp LanguageServer impl
  - Cross-file diagnostic publishing via publish_grouped_diagnostics
  - Stale squiggle clearing via published_uris tracking
affects: [53-04-vscode-extension, 54-semantic-features]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Backend::new constructor pattern for cross-crate struct construction
    - Box::leak for &'static str source text in publish_grouped_diagnostics (consistent with AnalysisHost)
    - spawn_blocking wraps synchronous AnalysisHost calls to keep async executor free
    - published_uris DashMap tracks active diagnostic URIs for stale-clear on fix

key-files:
  created:
    - writ-lsp/src/backend.rs
  modified:
    - writ-lsp/src/lib.rs
    - writ-lsp/src/main.rs
    - Cargo.lock

key-decisions:
  - "Backend::new constructor added so main.rs (binary crate) can construct Backend without pub(crate) field access across crate boundary"
  - "display_path_to_url falls back to trigger_uri for non-absolute paths (handles in-memory/virtual documents)"
  - "did_change triggers analysis immediately (no debounce) since spawn_blocking queues naturally and first-class live feedback is preferred"

patterns-established:
  - "Backend::new for cross-crate struct construction (avoids pub field leakage)"

requirements-completed: [LSP-01, LSP-08]

# Metrics
duration: 13min
completed: 2026-03-14
---

# Phase 53 Plan 03: tower-lsp Backend and Binary Entry Point Summary

**writ-lsp binary with tower-lsp Backend routing cross-file diagnostics per-URI and clearing stale squiggles on fix**

## Performance

- **Duration:** 13 min
- **Started:** 2026-03-14T00:42:54Z
- **Completed:** 2026-03-14T00:55:59Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Backend struct with tower-lsp LanguageServer impl (initialize, shutdown, did_open, did_change, did_save, did_close)
- publish_diagnostics_for: project-mode (writ.toml present) vs standalone analysis selection via spawn_blocking
- publish_grouped_diagnostics: routes diagnostics to correct per-URI channels, cross-file support for LSP-08
- Stale squiggle clearing: published_uris DashMap tracks active URIs, clears any not in current analysis result
- did_close clears diagnostics for the closed file immediately
- writ-lsp.exe binary builds at target/debug/writ-lsp.exe, responds to LSP JSON-RPC over stdio
- All 23 existing writ-lsp tests pass (0 regressions), full workspace test suite clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Create tower-lsp Backend with document store and diagnostic publishing** - `5242884` (feat)
2. **Task 2: Wire main.rs entry point and verify full binary builds** - `588927d` (feat)

## Files Created/Modified

- `writ-lsp/src/backend.rs` - Backend struct, LanguageServer impl, publish_diagnostics_for, publish_grouped_diagnostics, Backend::new
- `writ-lsp/src/lib.rs` - Added `pub mod backend;`
- `writ-lsp/src/main.rs` - Full tokio::main entry point using LspService::new(Backend::new)
- `Cargo.lock` - Updated after clean rebuild

## Decisions Made

- **Backend::new constructor**: The plan spec used struct literal construction with `pub(crate)` fields. This does not work across Rust crate boundaries (main.rs is the `writ-lsp` binary crate, lib.rs is the `writ_lsp` library crate). Added `Backend::new(client: Client) -> Self` so main.rs uses `LspService::new(Backend::new)`, which is clean and idiomatic.

- **display_path_to_url helper**: When converting a display_path string to a `Url` for diagnostic routing, absolute filesystem paths are preferred via `Url::from_file_path`. Non-absolute paths (in-memory buffers, virtual documents) fall back to the trigger_uri — ensuring the diagnostic always appears somewhere rather than being silently dropped.

- **No debounce on did_change**: The plan noted this was at Claude's discretion. Chose to publish on every change for immediate live feedback. spawn_blocking queues naturally so there is no executor starvation risk.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Backend struct construction fails across crate boundary**
- **Found during:** Task 2 (main.rs compilation)
- **Issue:** Plan spec used struct literal with `pub(crate)` fields (`document_map: DashMap::new()`, etc.). This does not compile from main.rs which is a separate binary crate that links the `writ_lsp` library — `pub(crate)` fields are not visible across crate boundaries.
- **Fix:** Added `Backend::new(client: Client) -> Self` constructor that constructs the struct internally, and updated main.rs to `LspService::new(Backend::new)`.
- **Files modified:** writ-lsp/src/backend.rs, writ-lsp/src/main.rs
- **Verification:** `cargo build -p writ-lsp` succeeds
- **Committed in:** 5242884 (Task 1), 588927d (Task 2)

---

**Total deviations:** 1 auto-fixed (Rule 1 — cross-crate visibility mismatch in plan spec)
**Impact on plan:** Minor fix, no scope creep. The constructor pattern is cleaner than exposing all fields publicly.

## Infrastructure Note

The D: drive was at 100% capacity at the start of this plan (carried from Plan 53-01). `cargo clean` freed 16.4 GiB of build artifacts before the first successful compilation. The clean rebuild took ~4 minutes (full dependency rebuild from scratch).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- writ-lsp binary is complete and ready for VS Code extension integration (Plan 53-04)
- Backend handles all LSP document lifecycle events and publishes diagnostics per-URI
- Cross-file diagnostic routing (LSP-08) is implemented via publish_grouped_diagnostics
- Stale squiggle clearing is implemented via published_uris tracking

## Self-Check: PASSED

- writ-lsp/src/backend.rs: FOUND
- writ-lsp/src/lib.rs: FOUND
- writ-lsp/src/main.rs: FOUND
- 53-03-SUMMARY.md: FOUND
- Commit 5242884: FOUND
- Commit 588927d: FOUND
- All 23 writ-lsp tests: PASSED
- Workspace tests: PASSED (no regressions)
- Binary target/debug/writ-lsp.exe: FOUND (17 MB)

---
*Phase: 53-lsp-server-skeleton-and-diagnostics*
*Completed: 2026-03-14*
