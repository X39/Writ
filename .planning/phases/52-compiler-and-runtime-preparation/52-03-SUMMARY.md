---
phase: 52-compiler-and-runtime-preparation
plan: 03
subsystem: compiler
tags: [codegen, error-tolerance, diagnostics, lsp-prep, tdd]

requires:
  - phase: 52-compiler-and-runtime-preparation
    plan: 01
    provides: [emit_bodies-sources-parameter, SourceSpan-line-col]

provides:
  - per-function-error-skip-in-emit_all_bodies
  - E9001-diagnostic-per-skipped-function
  - partial-result-emit_bodies

affects: [53-lsp, writ-compiler/src/emit/body/mod.rs, writ-compiler/src/emit/mod.rs]

tech-stack:
  added: []
  patterns: [tdd-red-green, per-function-error-tolerance, partial-codegen-result]

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/tests/emit_body_tests.rs

key-decisions:
  - "emit_all_bodies skips broken function bodies individually via continue (not early return) so siblings still compile"
  - "E9001 is emitted per-skipped item (Fn, method, Const, Global) with name from def_map arena"
  - "emit_bodies returns Err only when bodies is empty AND diagnostics exist — partial results are allowed for LSP"
  - "has_error_nodes retained as #[allow(dead_code)] utility; no longer a codegen gate"

patterns-established:
  - "Per-declaration error check: call expr_has_error(body) before emitter construction, use continue on error"
  - "Partial codegen: Ok(bytes) is valid even when some functions were skipped with E9001 diagnostics"

requirements-completed: [PREP-02]

duration: ~15min
completed: "2026-03-14"
---

# Phase 52 Plan 03: Per-Function Error Tolerance in Codegen Summary

**File-level codegen abort replaced with per-function error skip: broken functions emit E9001 and are skipped while valid sibling functions still produce IL bodies.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-14
- **Completed:** 2026-03-14
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 3

## Accomplishments

- Removed the `has_error_nodes` file-level abort from both `emit_all_bodies` (body/mod.rs) and `emit_bodies` (emit/mod.rs)
- Added per-declaration error checks (`expr_has_error`) with `continue` in `emit_all_bodies` for `Fn`, impl methods, `Const`, and `Global`
- Each skipped declaration emits an E9001 diagnostic with the function/method/const/global name
- `emit_bodies` now only returns `Err` when zero bodies were emitted AND diagnostics exist; partial results flow through to serialization
- `has_error_nodes` retained as `#[allow(dead_code)]` utility (used by existing tests, not as a gate)
- Three TDD tests added and passing: one-broken/one-valid, both-valid, both-broken

## Task Commits

1. **RED phase (failing tests)** - `9d721df` (test)
2. **GREEN phase (implementation)** - `3bdf961` (feat)

## Files Created/Modified

- `writ-compiler/src/emit/body/mod.rs` — removed file-level error guard in `emit_all_bodies`; added per-Fn/method/Const/Global `expr_has_error` check with `continue` and E9001 diagnostic
- `writ-compiler/src/emit/mod.rs` — removed E9000 pre-pass; changed post-body-emit guard from "any diags -> Err" to "empty bodies AND diags -> Err"
- `writ-compiler/tests/emit_body_tests.rs` — added `test_emit_all_bodies_skips_error_fn_emits_valid_fn`, `test_emit_all_bodies_both_valid_emits_both`, `test_emit_all_bodies_all_error_fns_skipped`

## Decisions Made

- **Per-function skip via `continue`**: The critical insight is `continue` (not `return`) so the loop keeps processing subsequent declarations. This is the difference between file-granular and function-granular error tolerance.
- **E9001 per-item diagnostic**: Each skipped item gets its own diagnostic with code E9001 and a message identifying the function/method/const/global name. E9000 (file-level abort) is now unused.
- **Partial result in `emit_bodies`**: Previously `if !diags.is_empty() { return Err(diags) }` after body emission would abort even if some bodies were emitted. Changed to only abort when `bodies.is_empty() && !diags.is_empty()`.
- **Per-impl-method granularity**: Applied the error check per method inside impl blocks, not per impl block. One broken method does not skip other methods in the same impl.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## Verification

- `cargo test -p writ-compiler --lib`: 26 passed, 0 failed
- `cargo test -p writ-compiler test_emit_all_bodies`: 3 new tests passed
- `cargo test --workspace`: 0 failures across all test suites

## Next Phase Readiness

- Compiler now produces partial IL output when files contain syntax errors in some functions
- LSP (Phase 53) can call `emit_bodies` on a file with parse errors and receive valid bodies for all clean functions
- No regressions in any existing test

---
## Self-Check: PASSED

- writ-compiler/src/emit/body/mod.rs: FOUND (per-function error skip present)
- writ-compiler/src/emit/mod.rs: FOUND (E9000 pre-pass removed)
- 52-03-SUMMARY.md: FOUND
- Commit 9d721df: FOUND (TDD RED)
- Commit 3bdf961: FOUND (TDD GREEN)
- cargo test --workspace: 40 test suites, 0 failures

*Phase: 52-compiler-and-runtime-preparation*
*Completed: 2026-03-14*
