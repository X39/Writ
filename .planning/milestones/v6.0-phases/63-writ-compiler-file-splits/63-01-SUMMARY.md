---
phase: 63-writ-compiler-file-splits
plan: 01
subsystem: compiler
tags: [rust, refactor, check-expr, type-checking, module-split]

requires: []
provides:
  - check_expr/ folder module with 10 focused subfiles (mod, ident, path, binary, call, control, access, match_, lambda, construction)
  - pub API maintained: CheckCtx, check_expr, check_block_stmts, check_assignment_mutability
affects:
  - 63-writ-compiler-file-splits

tech-stack:
  added: []
  patterns:
    - "Folder module split: large monolithic .rs file replaced by dir/mod.rs + submodule files, each under 500 lines"
    - "pub(super) visibility for submodule functions called only within check_expr tree"
    - "Explicit imports per submodule (no use super::* globs)"
    - "Cross-submodule calls via super::check_expr, super::check_block_stmts for recursive checking"

key-files:
  created:
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/src/check/check_expr/ident.rs
    - writ-compiler/src/check/check_expr/path.rs
    - writ-compiler/src/check/check_expr/binary.rs
    - writ-compiler/src/check/check_expr/call.rs
    - writ-compiler/src/check/check_expr/control.rs
    - writ-compiler/src/check/check_expr/access.rs
    - writ-compiler/src/check/check_expr/match_.rs
    - writ-compiler/src/check/check_expr/lambda.rs
    - writ-compiler/src/check/check_expr/construction.rs
  modified:
    - writ-compiler/src/check/check_expr.rs (deleted)

key-decisions:
  - "check_block made pub(super) in mod.rs so control.rs and match_ submodules can call it via super::check_block"
  - "find_fn_def_id made pub(super) so call.rs can use super::find_fn_def_id without making it fully public"
  - "AstType import in call.rs uses crate::ast::types::AstType directly (re-export through expr module is private)"

patterns-established:
  - "Folder module split: delete the .rs file, create dir/mod.rs with pub API + submod declarations, subfiles use explicit super:: imports"

requirements-completed: [SPLIT-03]

duration: 8min
completed: 2026-03-18
---

# Phase 63 Plan 01: check_expr File Split Summary

**check_expr.rs (2134 lines) replaced by check_expr/ folder module with 10 single-responsibility subfiles, each under 500 lines, with all 75 typecheck tests passing and zero clippy warnings**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-18T10:09:55Z
- **Completed:** 2026-03-18T10:18:10Z
- **Tasks:** 1
- **Files modified:** 11 (1 deleted, 10 created)

## Accomplishments
- Deleted 2134-line monolithic check_expr.rs
- Created check_expr/ with mod.rs + 9 focused submodules; largest file is mod.rs at 499 lines
- All public API preserved at check_expr:: path (CheckCtx, check_expr, check_block_stmts, check_assignment_mutability)
- No glob re-exports; all submodule functions use explicit imports and pub(super) visibility
- All 75 writ-compiler tests pass; zero clippy warnings

## Task Commits

1. **Task 1: Split check_expr.rs into check_expr/ folder module** - `f562d40` (refactor)

## Files Created/Modified
- `writ-compiler/src/check/check_expr.rs` - Deleted (replaced by folder)
- `writ-compiler/src/check/check_expr/mod.rs` - CheckCtx struct, check_expr dispatch, check_block_stmts, check_assignment_mutability, find_fn_def_id, find_root_binding
- `writ-compiler/src/check/check_expr/ident.rs` - check_ident (identifier/local/DefMap lookup)
- `writ-compiler/src/check/check_expr/path.rs` - check_path (qualified name / enum variant resolution)
- `writ-compiler/src/check/check_expr/binary.rs` - check_binary, check_unary_prefix
- `writ-compiler/src/check/check_expr/call.rs` - check_call, check_call_with_sig, check_contract_bounds, check_generic_call
- `writ-compiler/src/check/check_expr/control.rs` - check_if
- `writ-compiler/src/check/check_expr/access.rs` - check_member_access, check_bracket_access
- `writ-compiler/src/check/check_expr/match_.rs` - check_match, check_pattern
- `writ-compiler/src/check/check_expr/lambda.rs` - check_lambda
- `writ-compiler/src/check/check_expr/construction.rs` - check_new_construction, check_array_lit

## Decisions Made
- `check_block` made `pub(super)` so `control.rs` can call `super::check_block` for block bodies
- `find_fn_def_id` made `pub(super)` so `call.rs` can use `super::find_fn_def_id` without leaking to full public API
- AstType in call.rs imported via `crate::ast::types::AstType` directly — the re-export through `crate::ast::expr` is private

## Deviations from Plan

None - plan executed exactly as written. Minor import path correction (AstType import) was a straightforward Rule 3 fix during compilation.

## Issues Encountered
- `AstType` was re-exported inside `ast::expr` privately — had to import from `crate::ast::types::AstType` directly. Fixed immediately.
- Some unused imports in `mod.rs` (FnSig, AstType, instantiate_generic_fn) — all moved to the submodules that actually use them; removed from mod.rs.

## Next Phase Readiness
- check_expr/ module is the split target for SPLIT-03; ready for Phase 63 plan 02
- No blockers

---
*Phase: 63-writ-compiler-file-splits*
*Completed: 2026-03-18*
