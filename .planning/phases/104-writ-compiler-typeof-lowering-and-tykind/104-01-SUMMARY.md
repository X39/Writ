---
phase: 104-writ-compiler-typeof-lowering-and-tykind
plan: 01
subsystem: compiler
tags: [typeof, reflection, TyKind, TypedExpr, parser, type-checker, lowering]

# Dependency graph
requires:
  - phase: 103-writ-runtime-reflectionindex-and-intrinsic-dispatch
    provides: TypeOf opcode at 0x0A30 already in writ-module and runtime dispatch
provides:
  - KwTypeof lexer token
  - Expr::TypeOf CST variant
  - AstExpr::TypeOf AST variant
  - TyKind::ReflectionType(Ty) type variant with reflection_type() constructor
  - TypedExpr::TypeOf { ty, span, static_ty } typed IR variant
  - AstExpr::TypeOf type checker arm returning TyKind::ReflectionType
  - TypeOf stub arm in IL emitter (Plan 02 wires up actual IL)
  - 2 lowering snapshot tests + 5 typecheck unit tests
affects:
  - 104-02 (IL code generation for typeof — needs TypedExpr::TypeOf.static_ty)
  - 105-reflectable-auto-impl (needs TyKind::ReflectionType for Reflectable contract)
  - writ-lsp (hover, completions, semantic tokens all use TypedExpr exhaustive matches)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "typeof(expr) lowers to AstExpr::TypeOf (not a Call) — same pass-through pattern as Spawn/Try"
    - "TyKind::ReflectionType(Ty) carries static_ty so emitter can bake type_idx into TypeOf instruction"
    - "TypedExpr::TypeOf has static_ty field (inner expr's type) separate from ty (result ReflectionType)"
    - "Stub emit arm in body/expr/mod.rs; full emission deferred to Plan 02"

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__lower_typeof_snapshot.snap
  modified:
    - writ-parser/src/lexer.rs
    - writ-parser/src/cst.rs
    - writ-parser/src/parser/program.rs
    - writ-compiler/src/ast/expr.rs
    - writ-compiler/src/lower/expr.rs
    - writ-compiler/src/check/ty.rs
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/src/emit/type_sig.rs
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/src/emit/collect/walker.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/body/closure.rs
    - writ-compiler/tests/lowering_tests.rs
    - writ-compiler/tests/typecheck_tests.rs

key-decisions:
  - "typeof(expr) is a static compile-time query — NOT a function call. Pattern follows Spawn/Try pass-through approach."
  - "TypedExpr::TypeOf carries both ty (TyKind::ReflectionType) and static_ty (inner expr type) — emitter needs static_ty to bake type_idx"
  - "TyKind::ReflectionType encodes as TypeSpec placeholder 0x11 in type_sig.rs for Plan 02 to replace with real TypeRef"
  - "typeof(ReflectionType) + int produces type error — ReflectionType does not unify with numeric types"

patterns-established:
  - "One new arm per layer: lexer -> CST -> parser -> AstExpr -> lower -> TyKind -> TypedExpr -> check -> emit_stub"
  - "All exhaustive TypedExpr matches in emit/body/mod.rs and emit/body/closure.rs require TypeOf arms"

requirements-completed: [COMP-01, COMP-05]

# Metrics
duration: 35min
completed: 2026-03-28
---

# Phase 104 Plan 01: typeof() Frontend Pipeline Summary

**typeof(expr) threaded through full compiler frontend: KwTypeof keyword, CST Expr::TypeOf, AstExpr::TypeOf lowering, TyKind::ReflectionType type system, TypedExpr::TypeOf checker output, with 7 tests all passing**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-03-28T12:00:00Z
- **Completed:** 2026-03-28T12:35:00Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments

- Added `KwTypeof` keyword to logos lexer; `typeof` no longer lexes as Ident
- Wired `Expr::TypeOf` through parser with `delimited_by(LParen, RParen)` (typeof requires parentheses)
- Added `AstExpr::TypeOf { expr, span }` and lowering arm following the Spawn/Try pass-through pattern
- Added `TyKind::ReflectionType(Ty)` to type system with `reflection_type()` convenience constructor and `display()` showing "Type"
- Added `TypedExpr::TypeOf { ty, span, static_ty }` with exhaustive `.ty()` and `.span()` arms
- Type checker arm: checks inner expr, propagates errors, creates `TyKind::ReflectionType(static_ty)` result
- Added placeholder arms in all exhaustive TypedExpr matches (6 files in compiler + LSP uses wildcards)
- 2 lowering snapshot tests + 5 typecheck unit tests, all passing; full workspace suite green (zero failures)

## Task Commits

1. **Task 1: Lexer, CST, Parser, AstExpr, Lowering** - `a5d8ad6` (feat)
2. **Task 2: TyKind::ReflectionType, TypedExpr::TypeOf, type checker arm** - `9f93295` (feat)

## Files Created/Modified

- `writ-parser/src/lexer.rs` - Added KwTypeof in Keywords — Reflection section
- `writ-parser/src/cst.rs` - Added Expr::TypeOf(Box<Spanned<Expr>>) variant after Try
- `writ-parser/src/parser/program.rs` - Added typeof_expr parser atom with paren-delimited inner expr
- `writ-compiler/src/ast/expr.rs` - Added AstExpr::TypeOf { expr, span } variant in Reflection section
- `writ-compiler/src/lower/expr.rs` - Added Expr::TypeOf lowering arm near Spawn/Try arms
- `writ-compiler/src/check/ty.rs` - Added TyKind::ReflectionType(Ty), reflection_type() constructor, display arm
- `writ-compiler/src/check/ir.rs` - Added TypedExpr::TypeOf { ty, span, static_ty }, updated ty()/span() matches
- `writ-compiler/src/check/check_expr/mod.rs` - Added AstExpr::TypeOf arm with error propagation
- `writ-compiler/src/emit/type_sig.rs` - Added TyKind::ReflectionType arm (placeholder 0x11 TypeSpec)
- `writ-compiler/src/emit/body/expr/mod.rs` - Added TypeOf stub arm (alloc_reg only)
- `writ-compiler/src/emit/collect/walker.rs` - Added TypeOf leaf arm
- `writ-compiler/src/emit/body/mod.rs` - Added TypeOf arms in expr_has_error and collect_lambda_bodies
- `writ-compiler/src/emit/body/closure.rs` - Added TypeOf leaf arm in scan_expr_for_lambdas
- `writ-compiler/tests/lowering_tests.rs` - Added lower_typeof_basic and lower_typeof_snapshot tests
- `writ-compiler/tests/typecheck_tests.rs` - Added 5 typeof typecheck tests

## Decisions Made

- TypeOf carries `static_ty` field separate from `ty` (the ReflectionType result) — the emitter needs static_ty to determine the type_idx baked into the TypeOf IL instruction
- typeof(x) + 1 produces a type error (E0100 TypeMismatch) — ReflectionType does not unify with int
- ReflectionType encoded as 0x11 TypeSpec placeholder in type_sig.rs until Plan 02 registers a real TypeRef for the "Type" class

## Deviations from Plan

None — plan executed exactly as written. The exhaustive match arms in `emit/body/mod.rs` and `emit/body/closure.rs` were expected (noted in plan step 7) and added as planned.

## Issues Encountered

- The insta snapshot test `lower_typeof_snapshot` required accepting the new snapshot with `cp .snap.new .snap` (expected first-time behavior for new snapshot tests)
- Background bash commands produced empty output files on Windows; used synchronous commands with direct output instead

## Next Phase Readiness

- Plan 02 (104-02) can now focus purely on IL code generation: replace the stub TypeOf emit arm with real `Instruction::TypeOf { r_dst, type_idx }` emission using `static_ty` to resolve the type_idx
- The `static_ty` field in TypedExpr::TypeOf is ready for Plan 02's `resolve_type_idx_for_static_ty` function

---
*Phase: 104-writ-compiler-typeof-lowering-and-tykind*
*Completed: 2026-03-28*
