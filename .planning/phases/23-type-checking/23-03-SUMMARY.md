---
phase: 23-type-checking
plan: 03
status: complete
started: "2026-03-03"
completed: "2026-03-03"
---

# Plan 23-03 Summary: Advanced Type Features

## What Was Built
Extended `writ-compiler/src/check/` with ?/!/try desugaring, new construction, array literals, lambda/closure checking, spawn/join/cancel typing, enum exhaustiveness, and for-loop element binding:

- `desugar.rs`: `desugar_question()`, `desugar_unwrap()`, `desugar_try()` - desugar ?/!/try operators to typed Match nodes. No UnaryPostfix nodes remain in TypedAst output
- `pattern.rs`: `check_exhaustiveness()` - verifies enum match covers all variants (or has wildcard), emits E0116 with missing variant names
- `check_lambda()`: resolves param types (annotated or InferVar), checks body, builds Func type
- `check_new_construction()`: resolves target type, checks all fields present (E0117), verifies field types (E0100), rejects unknown fields (E0106)
- `check_array_lit()`: unifies all element types, produces `Array(elem_ty)`, catches mixed types
- Spawn/join/cancel: `spawn expr` produces `TaskHandle<T>`, `join` unwraps to T, `cancel` produces void, `spawn detached` produces void
- Range/FromEnd: range checks start/end type compatibility, FromEnd verifies int index
- IfLet: binds pattern variables in then-block scope, unifies branch types
- 12 new integration tests (61 total), all passing

## Key Decisions
- ?/!/try desugaring produces Match nodes with Variable/Wildcard patterns rather than synthetic EnumVariant patterns, avoiding the need for fake prelude DefIds. Codegen recognizes these by match structure on Option/Result types
- Empty array literals use InferVar for element type, resolved by context
- Lambda params without annotations get InferVars for bidirectional inference
- Closure capture inference is stubbed (empty captures list) - full capture tracking deferred to codegen phase where it's more naturally implemented
- Exhaustiveness checking covers enum types (variant-by-variant) and bool types (true/false)

## Commits
- (pending commit) feat(23-03): add ?/!/try desugaring, new construction, array literals, lambda, spawn/join, exhaustiveness

## Self-Check: PASSED
- [x] All 61 typecheck tests pass
- [x] Full workspace tests pass (no regressions)
- [x] `expr?` on Option produces typed Match (no UnaryPostfix in output)
- [x] `expr!` on Option/Result produces typed Match with crash arm
- [x] `try expr` on Result produces typed Match with return-Err arm
- [x] `new S { x: 1, y: 2 }` checks all field types and presence
- [x] `new S { x: 1 }` missing field produces E0117
- [x] `[1, 2, 3]` produces `Array<int>`, mixed types produce E0100
- [x] `spawn work()` produces TaskHandle<T>
- [x] `spawn detached work()` produces void
- [x] Lambda with typed params builds Func type
- [x] Full program test exercises struct creation, field access, generics, arrays, if/else
- [x] Exhaustiveness checking wired into match expression handler

## Key Files
### Created
- `writ-compiler/src/check/desugar.rs` - ?/!/try desugaring to typed Match nodes
- `writ-compiler/src/check/pattern.rs` - Exhaustiveness checking for enum and bool matches

### Modified
- `writ-compiler/src/check/check_expr.rs` - Replaced all stubs: UnaryPostfix (desugar), Lambda, Spawn/SpawnDetached/Join/Cancel/Defer, Try, New, ArrayLit, IfLet, Range, FromEnd. Added `check_lambda`, `check_new_construction`, `check_array_lit`
- `writ-compiler/src/check/mod.rs` - Added `pub mod desugar;` and `pub mod pattern;`
- `writ-compiler/tests/typecheck_tests.rs` - 12 new tests (61 total)
