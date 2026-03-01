---
phase: 11-parser-declarations-and-expressions
plan: 01
subsystem: parser, compiler
tags: [chumsky, parser-combinators, cst, ast, lowering, impl-generics, operator-sigs, extern-visibility, spawn-detached, defer-block]

# Dependency graph
requires:
  - phase: 10-parser-core-syntax
    provides: Core parser infrastructure, CST types, expression/statement parsing
provides:
  - ImplDecl with generic parameter support in CST and AST
  - Bodyless operator signatures (OpSig) in contract declarations
  - Non-extern component hard parse error with recovery
  - Extern fn dotted qualified names (FnSig.qualifier)
  - Extern declaration visibility (pub extern fn/struct/component)
  - Contextual caret (^) restricted to bracket-access context
  - Fused spawn detached expression (Expr::SpawnDetached replacing separate Spawn+Detached)
  - Block-only defer expression
  - Attribute named arg separator changed from : to =
affects: [12-parser-remaining, tests, lowering]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fused multi-keyword expressions: spawn detached parsed as single SpawnDetached CST node"
    - "Contextual-only expressions: caret (^) parsed only within bracket-inner context, not in Pratt prefix table"
    - "Parse-time validation via chumsky validate(): component_decl.validate() emits error and recovers"

key-files:
  created: []
  modified:
    - writ-parser/src/cst.rs
    - writ-parser/src/parser.rs
    - writ-compiler/src/ast/expr.rs
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/src/lower/expr.rs
    - writ-compiler/src/lower/mod.rs
    - writ-compiler/src/lower/operator.rs
    - writ-compiler/src/lower/entity.rs
    - writ-parser/tests/cases/09_entities.writ
    - writ-parser/tests/cases/13_concurrency.writ
    - writ-parser/tests/cases/14_attributes.writ
    - writ-parser/tests/cases/18_extern.writ
    - writ-parser/tests/parser_tests.rs
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "Used separate bracket-inner parser for caret instead of Pratt prefix operator, enabling shallow contextual validation"
  - "Replaced PrefixOp::FromEnd in Pratt parser with Expr::FromEnd in bracket context only; PrefixOp::FromEnd retained for lowering compatibility"
  - "Non-extern component uses chumsky validate() with error recovery, returning Stmt::Expr(Error) for continuation"
  - "Extern dotted names parsed with try-then-fallback: attempt Type.name, fall back to simple name"
  - "spawn detached parsed by trying two-keyword prefix before single-keyword spawn, using chumsky choice ordering"

patterns-established:
  - "Contextual expressions: parse-context-dependent expressions via scoped parsers rather than Pratt table entries"
  - "Fused keyword expressions: multi-keyword constructs as single CST nodes rather than nested compositions"
  - "Parse-time restriction: use validate() to reject grammar that was previously allowed, with helpful error messages"

requirements-completed: [TYPE-03, DECL-03, DECL-05, DECL-06, DECL-07, EXPR-03, EXPR-04, EXPR-05, MISC-02]

# Metrics
duration: ~45min
completed: 2026-03-01
---

# Phase 11 Plan 01: Parser Declarations and Expressions Implementation Summary

**All 9 parser declaration/expression rules implemented: impl generics, bodyless op sigs, component error, extern dotted names, extern visibility, contextual caret, spawn-detached, defer-block-only, attribute separator fix**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 2
- **Files modified:** 14

## Accomplishments
- Implemented all 9 Phase 11 requirements across CST, parser, AST, and lowering layers
- Updated all existing tests and snapshot files to match new CST/AST structure
- Maintained backward compatibility: 212 tests pass with zero regressions

## Files Created/Modified
- `writ-parser/src/cst.rs` - Added ImplDecl.generics, OpSig, FnSig.qualifier, ExternDecl visibility, Expr::SpawnDetached (removed Expr::Detached)
- `writ-parser/src/parser.rs` - Parser combinators for all 9 requirements: impl generics, bodyless op sigs, component error, extern dotted names, extern vis, caret validation, spawn-detached, defer block-only, attr separator
- `writ-compiler/src/ast/expr.rs` - AstExpr::SpawnDetached replaces AstExpr::Detached
- `writ-compiler/src/ast/decl.rs` - AstImplDecl.generics, AstFnSig.qualifier/qualifier_span, AstExternDecl visibility
- `writ-compiler/src/lower/expr.rs` - SpawnDetached lowering
- `writ-compiler/src/lower/mod.rs` - Extern visibility, qualifier, generics passthrough in lowering
- `writ-compiler/src/lower/operator.rs` - ImplDecl generics passthrough in operator lowering
- `writ-compiler/src/lower/entity.rs` - Added generics field to all AstImplDecl constructions
- `writ-parser/tests/cases/09_entities.writ` - Changed script-defined component to extern component
- `writ-parser/tests/cases/13_concurrency.writ` - Updated defer to block syntax, detached to spawn detached
- `writ-parser/tests/cases/14_attributes.writ` - Changed component to pub extern component
- `writ-parser/tests/cases/18_extern.writ` - Changed attr separator from : to =
- `writ-parser/tests/parser_tests.rs` - Updated existing tests for new CST shapes, added component error test
- `writ-compiler/tests/lowering_tests.rs` - Updated test sources and all snapshots for new AST shapes

## Decisions Made
- Used bracket-inner parser for caret instead of Pratt prefix to enforce contextual restriction at parse time
- Retained PrefixOp::FromEnd in CST enum for lowering compatibility even though Pratt parser no longer produces it
- Non-extern component error uses validate() with recovery (returns error item) for continued parsing
- Extern dotted names use try-then-fallback pattern: attempt `ident.ident` first, fall back to simple `ident`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated existing test files to match new parser requirements**
- **Found during:** Task 2 (parser changes)
- **Issue:** Test files 09_entities.writ, 13_concurrency.writ, 14_attributes.writ, 18_extern.writ used old syntax (non-extern component, bare detached, non-block defer, colon attr separator)
- **Fix:** Updated all test files to use new syntax
- **Files modified:** writ-parser/tests/cases/{09,13,14,18}_*.writ
- **Verification:** All file-based parse tests pass

**2. [Rule 3 - Blocking] Fixed existing parser tests for new CST/AST structure**
- **Found during:** Task 2 (after parser changes)
- **Issue:** parser_tests.rs and lowering_tests.rs had pattern matches against old CST shapes (ExternDecl without visibility, Expr::Detached)
- **Fix:** Updated all pattern matches to destructure new CST shapes, replaced test_component_basic/test_component_with_method with test_component_basic_extern/test_component_non_extern_error
- **Files modified:** writ-parser/tests/parser_tests.rs, writ-compiler/tests/lowering_tests.rs
- **Verification:** 212 tests pass, all snapshots updated

**3. [Rule 3 - Blocking] Fixed Rust 2024 edition pattern binding issue**
- **Found during:** Task 1 (compilation)
- **Issue:** `ref mut` not allowed in implicitly-borrowing pattern under Rust 2024 edition
- **Fix:** Removed explicit `ref mut` qualifier from ExternDecl match arms
- **Files modified:** writ-compiler/src/lower/mod.rs
- **Verification:** Clean compilation

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All auto-fixes necessary for compilation and test passing. No scope creep.

## Issues Encountered
- Initial build had Expr::Detached reference in parser.rs detached_expr combinator after removing the CST variant; resolved by replacing with SpawnDetached
- Rust 2024 edition implicit binding mode conflict with `ref mut` in match patterns; resolved by using simple variable binding

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All CST/AST/parser/lowering changes complete and tested
- Ready for Plan 11-02 (comprehensive test suite)
- PrefixOp::FromEnd variant is technically dead code in the Pratt parser but retained for lowering compatibility

---
*Phase: 11-parser-declarations-and-expressions*
*Plan: 01*
*Completed: 2026-03-01*
