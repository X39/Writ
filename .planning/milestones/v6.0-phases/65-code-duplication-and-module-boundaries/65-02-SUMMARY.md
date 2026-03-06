---
phase: 65-code-duplication-and-module-boundaries
plan: 02
subsystem: compiler
tags: [rust, imports, wildcard, module-boundaries, refactoring]

# Dependency graph
requires:
  - phase: 65-01
    provides: DUP-01/DUP-02 satisfied — lower_dlg_text consolidation complete
provides:
  - Explicit import lists replacing all Tier 1/2 internal wildcard imports
  - Documented intent comments on all Tier 3 domain-vocabulary wildcards
  - MOD-02 requirement satisfied
affects: [phase-65-03, future compiler maintenance]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Tier 1/2 internal wildcards replaced with exact explicit import lists derived via cargo check"
    - "Tier 3 domain-vocabulary wildcards retained but annotated with // Intentional wildcard: comment"
    - "Assembler AST wildcards replaced with explicit lists (sub-20 items)"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/desugar.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/resolve/resolver.rs
    - writ-assembler/src/assembler.rs
    - writ-assembler/src/parser.rs
    - writ-module/src/builder.rs
    - writ-module/src/module.rs
    - writ-module/src/reader.rs
    - writ-module/src/writer.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-parser/src/lib.rs

key-decisions:
  - "check_stmt.rs uses only TypedStmt from ir (1 of 9 items) — wildcard was hiding this sparseness"
  - "desugar.rs uses TypedExpr, TypedArm, TypedPattern from ir (3 of 9 items)"
  - "resolver.rs uses only ResolvedDecl, ResolvedType from resolve::ir (2 of 4 items)"
  - "resolver.rs uses 8 of 32 decl items — explicit list is clean; prior wildcard was overkill"
  - "env_build.rs uses 19 of 32 decl items — under the 25-item threshold so replaced with grouped explicit list"
  - "check_decl.rs uses 7 decl items and 2 ir items — both wildcards replaced"
  - "assembler.rs/parser.rs use 10/18 of 20 ast items — replaced with explicit lists despite Tier 3 classification in research (plan takes precedence)"
  - "writ-module tables::* and emit/metadata::* retained as wildcards with // Intentional wildcard: comments"

patterns-established:
  - "Wildcard removal procedure: remove wildcard -> cargo check -> copy undefined names -> write explicit import"
  - "// Intentional wildcard: comment format for documented domain-vocabulary wildcards"
  - "// Intentional re-export: comment format for public API re-exports"

requirements-completed: [MOD-02]

# Metrics
duration: 15min
completed: 2026-03-18
---

# Phase 65 Plan 02: Wildcard Import Cleanup Summary

**Replaced 11 internal wildcard imports with explicit lists and documented 7 Tier 3 domain-vocabulary wildcards with intent comments, satisfying MOD-02**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-18
- **Completed:** 2026-03-18
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments
- Replaced all `use super::ir::*` wildcards in writ-compiler/check/ with exact explicit import lists
- Replaced `use crate::resolve::ir::*` in resolver.rs with 2-item explicit list
- Replaced `use crate::ast::decl::*` in check_decl.rs, env_build.rs, resolver.rs with grouped explicit lists
- Replaced `use crate::ast::expr::*` in check_expr/mod.rs with 3-item explicit list
- Replaced `use crate::ast::*` in writ-assembler assembler.rs and parser.rs with explicit lists
- Added `// Intentional wildcard:` comments to all 4 writ-module tables::* imports
- Added `// Intentional wildcard:` comments to emit/module_builder.rs and emit/serialize.rs
- Added `// Intentional re-export:` comment to writ-parser/src/lib.rs pub use cst::*

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace Tier 1/2 internal wildcards with explicit imports** - `e2d53f3` (refactor)
2. **Task 2: Document Tier 3 domain-vocabulary wildcards with intent comments** - `97441eb` (docs)

## Files Created/Modified
- `writ-compiler/src/check/check_stmt.rs` - `use super::ir::TypedStmt` (1 item, was wildcard)
- `writ-compiler/src/check/desugar.rs` - `use super::ir::{TypedExpr, TypedArm, TypedPattern}` (3 items)
- `writ-compiler/src/check/check_decl.rs` - explicit ir (2) and decl (7) import lists
- `writ-compiler/src/check/check_expr/mod.rs` - explicit ir (3) and expr (3) import lists
- `writ-compiler/src/check/env_build.rs` - 19-item grouped decl import list
- `writ-compiler/src/resolve/resolver.rs` - explicit resolve::ir (2) and decl (8) import lists
- `writ-assembler/src/assembler.rs` - 10-item explicit ast import list
- `writ-assembler/src/parser.rs` - 18-item explicit ast import list
- `writ-module/src/builder.rs` - Intentional wildcard comment added
- `writ-module/src/module.rs` - Intentional wildcard comment added
- `writ-module/src/reader.rs` - Intentional wildcard comment added
- `writ-module/src/writer.rs` - Intentional wildcard comment added
- `writ-compiler/src/emit/module_builder.rs` - Intentional wildcard comment added
- `writ-compiler/src/emit/serialize.rs` - Intentional wildcard comment added
- `writ-parser/src/lib.rs` - Intentional re-export comment added

## Decisions Made
- Used `cargo check` after removing each wildcard to determine exact used items — compiler output is the ground truth
- Trimmed unused items from initial guesses (e.g., check_stmt.rs only uses TypedStmt, not TypedExpr; desugar.rs only uses 3 not 7 ir items)
- env_build.rs 19-item list written as grouped block for readability (under 25-item threshold so replaced rather than documented)
- assembler.rs/parser.rs treated as Tier 1/2 despite research Tier 3 classification — plan specification takes precedence

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Initial guesses at explicit import lists had unused items — corrected via iterative `cargo check` feedback
- check_stmt.rs appeared to use TypedExpr but didn't (the submodules handle all expr work); TypedStmt was the sole needed import

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- MOD-02 satisfied: all internal wildcards are either replaced with explicit lists or documented with intent comments
- External library preludes (chumsky, lsp_types, dap) remain as-is per research exemption
- Test-file wildcards (`use super::*`) remain as-is per research exemption
- Ready for Phase 65 Plan 03 (MOD-01: lib.rs module doc headers and MOD-03: pub narrowing)

---
*Phase: 65-code-duplication-and-module-boundaries*
*Completed: 2026-03-18*
