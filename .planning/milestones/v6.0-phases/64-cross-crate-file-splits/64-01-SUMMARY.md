---
phase: 64-cross-crate-file-splits
plan: 01
subsystem: parser
tags: [rust, chumsky, writ-parser, file-split, module-structure]

# Dependency graph
requires: []
provides:
  - "writ-parser/src/parser/ folder module with 5 subfiles replacing monolithic parser.rs"
  - "parser/mod.rs with TypePostfix, ExprPostfix enums and pub use re-exports"
  - "parser/type_expr.rs with pub fn type_expr()"
  - "parser/generic_params.rs with pub fn generic_params()"
  - "parser/pattern.rs with pub(super) fn pattern()"
  - "parser/program.rs with program_parser(), parse(), and string helper functions (documented 500-line exception)"
affects: [writ-lsp, writ-compiler, writ-dap]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Rust folder module pattern: parser.rs -> parser/mod.rs + submodules"
    - "pub(super) for module-internal functions callable by siblings"
    - "super::module::fn() call path for cross-sibling module calls"
    - "Chumsky recursive() documented exception: single closure scope prevents splitting"

key-files:
  created:
    - writ-parser/src/parser/mod.rs
    - writ-parser/src/parser/type_expr.rs
    - writ-parser/src/parser/generic_params.rs
    - writ-parser/src/parser/pattern.rs
    - writ-parser/src/parser/program.rs
  modified:
    - writ-parser/src/parser.rs (deleted)

key-decisions:
  - "pattern() changed to pub(super) so program.rs can call super::pattern::pattern() across sibling module boundary"
  - "program.rs is documented exception to 500-line target (~3,014 lines): Chumsky recursive() requires all grammar productions in one closure scope"
  - "ExprPostfix enum kept in mod.rs (not program.rs) as it is a shared type — program.rs imports it via use super::ExprPostfix"
  - "TypePostfix enum kept in mod.rs alongside ExprPostfix for symmetry, imported by type_expr.rs via use super::TypePostfix"
  - "lib.rs unchanged: pub mod parser and pub use parser::parse remain valid since Rust folder module semantics are transparent"

patterns-established:
  - "Folder module split: delete parser.rs, create parser/mod.rs + subfiles — Rust resolves pub mod parser to the folder"
  - "Sibling submodule call: program.rs calls pattern() via super::pattern::pattern()"

requirements-completed: [SPLIT-01]

# Metrics
duration: 9min
completed: 2026-03-18
---

# Phase 64 Plan 01: writ-parser Parser Split Summary

**Monolithic writ-parser/src/parser.rs (3,345 lines) split into parser/ folder module with 5 subfiles: mod.rs, type_expr.rs, generic_params.rs, pattern.rs, program.rs**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-18T03:01:15Z
- **Completed:** 2026-03-18T03:10:30Z
- **Tasks:** 1 of 1
- **Files modified:** 6 (1 deleted, 5 created)

## Accomplishments
- Deleted monolithic parser.rs (3,345 lines) and replaced with parser/ folder module
- Created mod.rs containing shared TypePostfix/ExprPostfix enums and pub use re-exports that preserve the full public API
- Extracted type_expr() into parser/type_expr.rs (112 lines)
- Extracted generic_params() into parser/generic_params.rs (51 lines)
- Extracted pattern() into parser/pattern.rs (124 lines) with pub(super) visibility
- Created parser/program.rs (~3,014 lines) with all helper functions and program_parser/parse entry points
- All 239 parser tests pass without modification; zero clippy warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Split parser.rs into parser/ folder module with 5 subfiles** - `8614dba` (feat)

**Plan metadata:** (see final commit below)

## Files Created/Modified
- `writ-parser/src/parser.rs` - DELETED (3,345 lines)
- `writ-parser/src/parser/mod.rs` - Module root: TypePostfix, ExprPostfix enums + pub use re-exports
- `writ-parser/src/parser/type_expr.rs` - pub fn type_expr() parser combinator
- `writ-parser/src/parser/generic_params.rs` - pub fn generic_params() parser combinator
- `writ-parser/src/parser/pattern.rs` - pub(super) fn pattern() parser combinator
- `writ-parser/src/parser/program.rs` - program_parser(), parse(), parse_formattable_string(), split_dlg_text_segments(), parse_expr_from_source() (documented 500-line exception)

## Decisions Made
- pattern() changed to pub(super) so program.rs can call super::pattern::pattern() across the sibling module boundary
- program.rs is a documented exception to the 500-line target (~3,014 lines): Chumsky's recursive() combinator requires all grammar productions to be visible simultaneously in a single closure scope — splitting the recursive closure is not structurally sound
- ExprPostfix and TypePostfix enums placed in mod.rs (shared module) so subfiles can import them via use super::{ExprPostfix, TypePostfix}
- lib.rs unchanged: Rust's folder module semantics make parser.rs and parser/mod.rs interchangeable from the crate consumer's perspective

## Deviations from Plan

None - plan executed exactly as written. The bash extraction strategy for program.rs (plan's critical instruction) was followed precisely.

## Issues Encountered
- None. GPG signing timeout on first commit attempt; resolved by retrying after brief pause.

## Next Phase Readiness
- writ-parser parser split complete; SPLIT-01 requirement satisfied
- Ready for 64-02 (next crate split in phase 64)
- All downstream crates (writ-compiler, writ-lsp, writ-dap) unaffected — lib.rs API unchanged

## Self-Check: PASSED

All files present, commit 8614dba exists. Note: `pub use program::parse` is expressed as `pub use program::{parse, program_parser};` (combined re-export) in mod.rs — this satisfies the requirement that `parse` is re-exported from the `program` submodule.

---
*Phase: 64-cross-crate-file-splits*
*Completed: 2026-03-18*
