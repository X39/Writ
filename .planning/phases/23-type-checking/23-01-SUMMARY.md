---
phase: 23-type-checking
plan: 01
status: complete
started: "2026-03-03"
completed: "2026-03-03"
---

# Plan 23-01 Summary: Type Checker Foundation

## What Was Built
Created the complete `writ-compiler/src/check/` module with type checking infrastructure:
- `Ty(u32)` interned type with `TyInterner` structural deduplication
- `TypedExpr`/`TypedStmt`/`TypedDecl` IR with inline `ty: Ty` on every variant (no `Option<Ty>`)
- `TypeEnv` materializing function signatures, struct fields, enum variants, impl associations from AST
- `ena`-based `UnifyCtx` for generic type variable unification
- Expression checking: literals, identifiers, binary operators, function calls, generic inference, if/block
- Statement checking: let bindings (with annotation), return, for/while/break/continue
- Declaration checking: function bodies, impl methods, const/global
- Error codes E0100-E0119 defined in `writ-diagnostics/src/code.rs`
- `TypeError` enum with `From<TypeError> for Diagnostic` conversion producing multi-span errors
- 28 integration tests covering TYPE-01, TYPE-02, TYPE-03, TYPE-13

## Key Decisions
- Used `ena` 0.14.4 with `InferValue` wrapper to satisfy orphan rules (cannot impl `UnifyValue` for `Option<Ty>` directly)
- TypeEnv built by walking original ASTs matched to DefIds from DefMap (not extending DefMap)
- Stubs for later plans (MemberAccess, BracketAccess, Match, Lambda, etc.) return `TypedExpr::Error` for poison propagation
- Generic instantiation creates fresh InferVars, substitutes into param types, unifies with args

## Commits
- `f7196c5` feat(23-01): add type checker foundation with Ty interner, TypedIR, TypeEnv, UnifyCtx, and basic inference

## Self-Check: PASSED
- [x] All 28 typecheck tests pass
- [x] Full workspace tests pass (no regressions)
- [x] `Ty(u32)` with TyInterner deduplicates structurally
- [x] TypedExpr has no `Option<Ty>` fields
- [x] TypeEnv extracts fn sigs, struct fields, enum variants
- [x] ena-based UnifyCtx performs inference variable unification
- [x] Error poisoning prevents cascading (verified by test)

## Key Files
### Created
- `writ-compiler/src/check/mod.rs` - Entry point `typecheck()`
- `writ-compiler/src/check/ty.rs` - `Ty`, `TyKind`, `TyInterner`, `InferVar`
- `writ-compiler/src/check/ir.rs` - `TypedAst`, `TypedExpr`, `TypedStmt`, `TypedDecl`
- `writ-compiler/src/check/env.rs` - `TypeEnv`, `LocalEnv`, `Mutability`
- `writ-compiler/src/check/unify.rs` - `UnifyCtx`
- `writ-compiler/src/check/infer.rs` - `resolve_type_to_ty`, `instantiate_generic_fn`, `substitute`
- `writ-compiler/src/check/check_expr.rs` - `CheckCtx`, `check_expr`
- `writ-compiler/src/check/check_stmt.rs` - `check_stmt`
- `writ-compiler/src/check/check_decl.rs` - `check_decl`
- `writ-compiler/src/check/error.rs` - `TypeError` enum
- `writ-compiler/tests/typecheck_tests.rs` - 28 integration tests

### Modified
- `writ-compiler/Cargo.toml` - added `ena = "0.14.4"`
- `writ-compiler/src/lib.rs` - added `pub mod check`
- `writ-diagnostics/src/code.rs` - added error codes E0100-E0119
