---
phase: 09-cst-type-system-additions
plan: 02
subsystem: parser
tags: [cst, dlg-decl, attrs, visibility, dead-code-removal]

requires:
  - phase: 09-cst-type-system-additions
    provides: "Plan 01 TypeExpr::Qualified and Expr::Path changes to cst.rs"
provides:
  - "DlgDecl with attrs and vis fields"
  - "Stmt::DlgDecl variant removed from CST"
  - "All dialogue tests refactored to use Item::Dlg directly"
affects: [12-lowering-dialogue]

tech-stack:
  added: []
  patterns: ["All declaration types now carry attrs/vis uniformly"]

key-files:
  created: []
  modified:
    - writ-parser/src/cst.rs
    - writ-parser/src/parser.rs
    - writ-parser/tests/parser_tests.rs
    - writ-compiler/src/lower/stmt.rs

key-decisions:
  - "DlgDecl attrs/vis follows exact FnDecl/StructDecl pattern"
  - "Stmt::DlgDecl removal is safe — dialogue declarations only appear as Item::Dlg at top level"
  - "parse_ok helper now only handles Item::Stmt; dialogue tests use parse_ok_items"

patterns-established:
  - "All top-level declaration types carry attrs and vis fields uniformly"

requirements-completed: [DECL-04, MISC-03]

duration: 7min
completed: 2026-03-01
---

# Plan 09-02: DlgDecl attrs/vis + Stmt::DlgDecl Removal Summary

**Added attrs/vis fields to DlgDecl and removed dead Stmt::DlgDecl variant, completing uniform declaration attributes across all CST types**

## Performance

- **Duration:** 7 min
- **Tasks:** 2 (combined into single commit)
- **Files modified:** 4

## Accomplishments
- Added attrs and vis fields to DlgDecl struct matching FnDecl/StructDecl pattern
- Wired parser attrs_vis_decl to attach attrs/vis to DlgDecl
- Removed Stmt::DlgDecl variant from Stmt enum
- Removed Stmt::DlgDecl lowering arm from stmt.rs
- Refactored 15 dialogue tests from Stmt::DlgDecl to Item::Dlg
- Added 4 new tests verifying attrs/vis on DlgDecl
- All 189 tests pass

## Task Commits

1. **Task 1+2: DlgDecl attrs/vis + Stmt::DlgDecl removal** - `60bf84c` (feat)

## Files Created/Modified
- `writ-parser/src/cst.rs` - Added attrs/vis to DlgDecl, removed Stmt::DlgDecl
- `writ-parser/src/parser.rs` - Wired attrs/vis attachment, updated constructor
- `writ-parser/tests/parser_tests.rs` - Refactored 15 tests, added 4 new tests
- `writ-compiler/src/lower/stmt.rs` - Removed DlgDecl arm, cleaned imports

## Decisions Made
- Combined both tasks into single commit since they're tightly coupled

## Deviations from Plan
None - plan executed as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All CST type system additions complete
- Phase 10 (Parser — Core Syntax) can build on these foundations

---
*Phase: 09-cst-type-system-additions*
*Completed: 2026-03-01*
