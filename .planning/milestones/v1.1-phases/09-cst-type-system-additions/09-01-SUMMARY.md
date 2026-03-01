---
phase: 09-cst-type-system-additions
plan: 01
subsystem: parser
tags: [cst, type-system, qualified-paths, rooted-paths]

requires:
  - phase: 08-lexer-fixes
    provides: "Clean lexer foundation with Token::ColonColon"
provides:
  - "TypeExpr::Qualified variant for multi-segment type paths"
  - "Expr::Path struct variant with rooted flag"
  - "Parser support for qualified type paths and rooted expression paths"
  - "Lowering support for TypeExpr::Qualified and struct-variant Expr::Path"
affects: [10-parser-core-syntax, 11-parser-declarations-expressions, 12-lowering-dialogue]

tech-stack:
  added: []
  patterns: ["Qualified { segments, rooted } struct variant pattern for path nodes"]

key-files:
  created: []
  modified:
    - writ-parser/src/cst.rs
    - writ-parser/src/parser.rs
    - writ-parser/tests/parser_tests.rs
    - writ-compiler/src/lower/expr.rs
    - writ-compiler/src/lower/optional.rs

key-decisions:
  - "TypeExpr::Qualified uses struct variant with segments Vec and rooted bool"
  - "Expr::Path converted from tuple to struct variant matching TypeExpr::Qualified shape"
  - "Single-segment types remain TypeExpr::Named; multi-segment or rooted become Qualified"
  - "Rooted paths in lowering prepend :: to first segment string"

patterns-established:
  - "Path nodes use struct variant { segments, rooted } pattern for both TypeExpr and Expr"

requirements-completed: [TYPE-01, TYPE-02]

duration: 8min
completed: 2026-03-01
---

# Plan 09-01: TypeExpr::Qualified + Expr::Path Rooted Flag Summary

**Added multi-segment qualified type paths (TypeExpr::Qualified) and rooted flag on Expr::Path for spec v0.4 path syntax conformance**

## Performance

- **Duration:** 8 min
- **Tasks:** 2 (combined into single commit)
- **Files modified:** 5

## Accomplishments
- Added TypeExpr::Qualified { segments, rooted } variant alongside existing Named
- Converted Expr::Path from tuple variant to struct variant with rooted bool
- Updated type_expr() parser to collect ident :: ident segments and emit Named or Qualified
- Updated ident_or_path parser to set rooted flag from :: prefix
- Updated lowering for both TypeExpr::Qualified and struct-variant Expr::Path
- Added 8 new tests covering qualified types, rooted types, and rooted/unrooted expression paths
- All 185 tests pass

## Task Commits

1. **Task 1+2: TypeExpr::Qualified + Expr::Path rooted flag** - `859f500` (feat)

## Files Created/Modified
- `writ-parser/src/cst.rs` - Added TypeExpr::Qualified variant, converted Expr::Path to struct variant
- `writ-parser/src/parser.rs` - Updated type_expr() and ident_or_path parsers
- `writ-parser/tests/parser_tests.rs` - Updated existing path test, added 8 new tests
- `writ-compiler/src/lower/expr.rs` - Updated Expr::Path destructure for struct variant
- `writ-compiler/src/lower/optional.rs` - Added TypeExpr::Qualified lowering, updated Generic base handling

## Decisions Made
- Combined both tasks into a single commit since they both touch cst.rs and are tightly coupled
- Rooted paths in lowering prepend "::" to first segment rather than using empty segment marker

## Deviations from Plan
None - plan executed as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CST path types are now spec-compliant for Phase 10 and beyond
- DlgDecl attrs/vis and Stmt::DlgDecl removal ready as Plan 02

---
*Phase: 09-cst-type-system-additions*
*Completed: 2026-03-01*
