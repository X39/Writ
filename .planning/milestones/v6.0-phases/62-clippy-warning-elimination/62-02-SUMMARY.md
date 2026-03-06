---
phase: 62-clippy-warning-elimination
plan: 02
subsystem: build-quality
tags: [rust, clippy, lints, code-quality]

# Dependency graph
requires:
  - phase: 62-01
    provides: auto-fixed 155 clippy warnings, never_loop error resolved
provides:
  - Zero-warning clippy workspace (cargo clippy --workspace exits clean)
  - All #[allow(...)] suppressions have justifying comments
affects: [future-phases]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "#[allow(clippy::too_many_arguments)] with justifying comment for internal hot-path dispatch functions"
    - "Collapse match-with-one-arm to if let for single-pattern matching"
    - "Collapse if let inside match arm into the outer match pattern"
    - "Use match tuple for eliminating is_some()/unwrap() patterns"
    - "&mut [u8] slice instead of &mut Vec<u8> for write-only parameters"

key-files:
  created: []
  modified:
    - writ-module/src/writer.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/src/scheduler.rs
    - writ-parser/src/parser.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/mod.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/check/pattern.rs
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/src/emit/body/const_fold.rs
    - writ-compiler/src/emit/body/labels.rs
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/body/call.rs
    - writ-compiler/src/resolve/validate.rs
    - writ-dap/src/server.rs
    - writ-dap/src/variables.rs
    - writ-lsp/src/analysis_host.rs
    - writ-lsp/src/queries.rs

key-decisions:
  - "Use #[allow(clippy::too_many_arguments)] with comments for dispatch/scheduler/compiler functions where independent mutable borrows make struct consolidation impractical"
  - "Use #[allow(clippy::only_used_in_recursion)] with comments for interner/module params reserved for future type-aware operations"
  - "Use #[allow(clippy::type_complexity)] with comments for parser combinator closures and return types inherently complex by nature of parser combinators"
  - "Add Default impl for Domain struct to satisfy new_without_default lint"
  - "Prefer match-based pattern for is_some()+unwrap() elimination over duplicated else branches"

patterns-established:
  - "Every #[allow(clippy::...)] must have a justifying // comment on the same line"

requirements-completed: [WARN-01, WARN-02]

# Metrics
duration: 25min
completed: 2026-03-18
---

# Phase 62 Plan 02: Clippy Warning Elimination (Manual Fixes) Summary

**Manually resolved 20+ remaining clippy warnings across 19 files — cargo clippy --workspace now exits clean with zero warnings and zero errors across all 9 Rust crates**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-18T00:50:00Z
- **Completed:** 2026-03-18T01:15:00Z
- **Tasks:** 1
- **Files modified:** 19

## Accomplishments

- Eliminated all remaining clippy warnings that Plan 01's auto-fix pass could not handle
- Added 8x `#[allow(clippy::too_many_arguments)]` suppressions with justifying comments for internal dispatch/scheduler/compiler functions
- Fixed 3x `type_complexity` warnings with `#[allow]` and comments (parser combinators + dialogue sig table)
- Fixed `if_same_then_else` in writer.rs by merging identical branches with combined condition
- Fixed 2x `unnecessary_unwrap` via `if let Some` / `match` refactors (DAP server + LSP analysis host)
- Fixed 2x `only_used_in_recursion` with `#[allow]` and comments (const_fold interner + DAP module param)
- Fixed 3x `single_match` by converting to `if let` patterns
- Fixed 2x `collapsible_match` in LSP queries.rs by collapsing `if let Some` into outer match arms
- Fixed `collapsible_match` in validate.rs and `unnecessary_filter_map` in check_decl.rs
- Fixed `ptr_arg` in labels.rs (`&mut Vec<u8>` -> `&mut [u8]`)
- Fixed `doc_lazy_continuation` in call.rs doc comment formatting
- Added `Default` impl for `Domain` struct to satisfy `new_without_default`
- All 1000+ existing tests still pass with zero regressions

## Task Commits

1. **Task 1: Resolve all manual-fix clippy warnings across all crates** - `c16c179` (fix)

## Files Created/Modified

- `writ-module/src/writer.rs` - merged identical if/else branches (if_same_then_else fix)
- `writ-runtime/src/dispatch/mod.rs` - 4x `#[allow(clippy::too_many_arguments)]` with comments
- `writ-runtime/src/domain.rs` - added `impl Default for Domain`
- `writ-runtime/src/scheduler.rs` - 2x `#[allow(clippy::too_many_arguments)]` with comments
- `writ-parser/src/parser.rs` - 2x `#[allow(clippy::type_complexity)]` on program_parser and parse
- `writ-compiler/src/check/env.rs` - `#[allow(clippy::type_complexity)]` on dialogue_sigs binding
- `writ-compiler/src/check/mod.rs` - `#[allow(clippy::too_many_arguments)]` + match->if let for dfs_struct
- `writ-compiler/src/check/check_decl.rs` - filter_map->map (unnecessary_filter_map fix)
- `writ-compiler/src/check/pattern.rs` - collapsed if let into outer match arm (single_match fix)
- `writ-compiler/src/emit/collect.rs` - `#[allow(clippy::too_many_arguments)]` with comment
- `writ-compiler/src/emit/body/const_fold.rs` - `#[allow(clippy::only_used_in_recursion)]` with comment
- `writ-compiler/src/emit/body/labels.rs` - `&mut Vec<u8>` -> `&mut [u8]` (ptr_arg fix)
- `writ-compiler/src/emit/body/expr.rs` - match with one arm -> if (single_match fix)
- `writ-compiler/src/emit/body/call.rs` - added blank line in doc comment (doc_lazy_continuation fix)
- `writ-compiler/src/resolve/validate.rs` - collapsed if let into outer match (collapsible_match fix)
- `writ-dap/src/server.rs` - is_some()+unwrap() -> if let Some (unnecessary_unwrap fix)
- `writ-dap/src/variables.rs` - `#[allow(clippy::only_used_in_recursion)]` with comment
- `writ-lsp/src/analysis_host.rs` - is_some()+unwrap() -> match tuple (unnecessary_unwrap fix)
- `writ-lsp/src/queries.rs` - collapsed 2x if let Some into outer match arms (collapsible_match fix)

## Decisions Made

- Used `#[allow(clippy::too_many_arguments)]` with justifying comments for dispatch/scheduler/compiler functions: independent mutable borrows into runtime state make struct bundling impractical due to borrow checker aliasing constraints.
- Used `#[allow(clippy::only_used_in_recursion)]` for `interner` in const_fold and `module` in format_value: both params are reserved for future type-aware operations and keeping them in the signature is intentional.
- Used match-based pattern `match (trigger_source.as_ref(), trigger_canonical.as_ref()) { (Some(ts), Some(tc)) if ... => ... }` for LSP analysis_host to avoid both the unwrap and branch duplication.
- Added `impl Default for Domain` rather than suppressing `new_without_default` — Domain::new() takes no args so Default is genuinely implementable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] single_match warning in writ-compiler/src/check/mod.rs not in original plan list**
- **Found during:** Task 1 (live clippy run)
- **Issue:** Plan listed the `dfs_struct` function as needing only `#[allow(too_many_arguments)]`, but a separate `single_match` warning also existed in the same function body for the inner `match interner.kind(*field_ty)` expression.
- **Fix:** Converted the match with a single non-trivial arm + wildcard to `if let TyKind::Struct(...)`.
- **Files modified:** writ-compiler/src/check/mod.rs
- **Committed in:** c16c179

---

**Total deviations:** 1 auto-fixed (Rule 1 - additional warning in same function)
**Impact on plan:** Required fix to achieve zero-warning goal. No scope creep.

## Issues Encountered

None - all fixes applied cleanly in a single pass. The pattern.rs collapsible_match fix required using a struct field pattern in the match arm which required verifying the TypedLiteral variant was accessible in scope, but the fix compiled correctly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `cargo clippy --workspace` exits clean with zero warnings and zero errors — WARN-01 and WARN-02 complete
- Phase 62 is complete: both Plan 01 (auto-fixes) and Plan 02 (manual fixes) delivered a zero-warning workspace
- Phase 63 (writ-compiler splits) can begin: clean baseline established before structural refactoring

---
*Phase: 62-clippy-warning-elimination*
*Completed: 2026-03-18*
