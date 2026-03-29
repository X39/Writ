---
phase: 104-writ-compiler-typeof-lowering-and-tykind
verified: 2026-03-28T14:00:00Z
status: gaps_found
score: 8/9 must-haves verified
re_verification: false
gaps:
  - truth: "Passing typeof(x) where a Type parameter is expected compiles without error (BOX/UNBOX infrastructure ready)"
    status: partial
    reason: "The typeof_passes_to_type_param test was explicitly planned in Plan 02 Task 2 to validate COMP-04, but was never written. The BOX/UNBOX infrastructure in call.rs exists for generic params, but there is no test confirming that typeof(x) passed to a function accepting Type is accepted by the type checker. The COMP-04 behavioral claim is unverified at the test level."
    artifacts:
      - path: "writ-compiler/tests/typecheck_tests.rs"
        issue: "typeof_passes_to_type_param test is absent — listed in plan acceptance criteria but not implemented"
    missing:
      - "Add typecheck test: fn consume(t: Type) {} fn test(x: int) { consume(typeof(x)); } — assert has_no_errors"
human_verification:
  - test: "End-to-end compile of a .writ file using typeof"
    expected: "writ-compiler compiles fn test(x: int) { typeof(x); } to a module containing a TypeOf instruction with non-zero type_idx"
    why_human: "No golden test for typeof was added — all emit tests are unit tests in emit_body_tests.rs, not full-pipeline golden tests"
---

# Phase 104: Writ Compiler typeof Lowering and TyKind Verification Report

**Phase Goal:** A Writ source file using typeof(expr) compiles to a module with the TypeOf instruction baked in, and the type checker assigns TyKind::ReflectionType(Type) to typeof expressions
**Verified:** 2026-03-28T14:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | typeof(x) in source code parses, lowers to AstExpr::TypeOf, and type-checks to TyKind::ReflectionType | VERIFIED | KwTypeof in lexer.rs:327; Expr::TypeOf in cst.rs:642; AstExpr::TypeOf in ast/expr.rs:140; TyKind::ReflectionType in ty.rs:47; TypedExpr::TypeOf in ir.rs:186; AstExpr::TypeOf arm in check_expr/mod.rs:375. Tests: lower_typeof_basic, lower_typeof_snapshot, typeof_primitive_type, typeof_struct_type all pass. |
| 2 | An incorrect use of typeof (e.g. typeof(x) + 1) produces a type error, not a crash | VERIFIED | typeof_type_error_on_arithmetic test passes; test asserts at least one Severity::Error diagnostic is emitted. |
| 3 | Existing tests pass with zero regressions after the new variants are added | VERIFIED | Full workspace `cargo test --workspace` shows 0 failures across all test suites. |
| 4 | typeof(x) on a user-defined struct emits a TypeOf instruction with the correct type_idx baked in | VERIFIED | emit_typeof_struct test in emit_body_tests.rs:3835 passes; asserts Instruction::TypeOf with type_idx == token_for_def(struct_def_id). Real emit arm at body/expr/mod.rs:497-502. |
| 5 | typeof(42) on a primitive emits a TypeOf instruction with the correct writ-runtime TypeRef token | VERIFIED | emit_typeof_primitive_int test at emit_body_tests.rs:3880 passes; asserts TypeOf with type_idx == type_ref_token_by_name("Int"). |
| 6 | TyKind::ReflectionType encodes correctly in type signatures using the Type TypeRef | VERIFIED | type_sig.rs:82 has TyKind::ReflectionType arm encoding 0x11 TypeSpec placeholder. No crash on compilation. |
| 7 | TypeRef rows for Type, Int, Float, Bool, String are registered in the module | VERIFIED | collect/mod.rs:64-68 shows add_type_ref calls for "Type", "Int", "Float", "Bool", "String". |
| 8 | type_ref_token_by_name lookup generalizes range_type_token | VERIFIED | module_builder.rs:1104 has type_ref_token_by_name; range_type_token delegates to it at line 1119. |
| 9 | Passing typeof(x) where a Type parameter is expected compiles without error (BOX/UNBOX infrastructure ready) | FAILED | typeof_passes_to_type_param test was listed in Plan 02 Task 2 acceptance criteria but was not written. BOX/UNBOX infrastructure (emit_box_if_needed in call.rs:180) exists for generic params, but there is no test confirming this path works for ReflectionType at reflection API call boundaries. |

**Score:** 8/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-parser/src/lexer.rs` | KwTypeof token variant | VERIFIED | `KwTypeof,` at line 327 |
| `writ-parser/src/cst.rs` | Expr::TypeOf CST variant | VERIFIED | `TypeOf(Box<Spanned<Expr<'src>>>),` at line 642 |
| `writ-compiler/src/ast/expr.rs` | AstExpr::TypeOf variant | VERIFIED | `TypeOf { expr: Box<AstExpr>, span: SimpleSpan },` at line 140 |
| `writ-compiler/src/check/ty.rs` | TyKind::ReflectionType variant | VERIFIED | `ReflectionType(Ty),` at line 47; `reflection_type()` constructor at line 167; `display()` arm at line 197 |
| `writ-compiler/src/check/ir.rs` | TypedExpr::TypeOf variant | VERIFIED | `TypeOf { ty, span, static_ty }` at line 186; `.ty()` arm at 222; `.span()` arm at 254 |
| `writ-compiler/src/check/check_expr/mod.rs` | Type checker arm for AstExpr::TypeOf | VERIFIED | `AstExpr::TypeOf { expr: inner_expr, span }` arm at line 375 with error propagation and reflection_ty creation |
| `writ-compiler/src/emit/collect/mod.rs` | TypeRef registrations for Type and primitive pseudo-TypeDefs | VERIFIED | Lines 64-68: add_type_ref for "Type", "Int", "Float", "Bool", "String" |
| `writ-compiler/src/emit/module_builder.rs` | type_ref_token_by_name lookup method | VERIFIED | `type_ref_token_by_name` at line 1104; `range_type_token` delegates at 1119 |
| `writ-compiler/src/emit/body/expr/mod.rs` | TypeOf IL emission arm | VERIFIED | Real `Instruction::TypeOf { r_dst, type_idx }` emission at line 497-502; `resolve_typeof_type_idx` helper at line 518-535 |
| `writ-compiler/src/emit/type_sig.rs` | ReflectionType encoding using Type TypeRef | VERIFIED | `TyKind::ReflectionType(_inner)` arm at line 82 encodes 0x11 TypeSpec placeholder |
| `writ-compiler/tests/snapshots/lowering_tests__lower_typeof_snapshot.snap` | Lowering snapshot for typeof | VERIFIED | Snapshot exists and contains `TypeOf { expr: Ident { name: "x" } }` at correct span |
| `writ-compiler/tests/typecheck_tests.rs` | `typeof_passes_to_type_param` test (COMP-04 validator) | MISSING | Test listed in plan acceptance criteria and explicitly called out as validating COMP-04, but absent from test file. 5 other typeof tests are present and passing. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-parser/src/parser/program.rs` | `writ-parser/src/cst.rs` | Parser atom produces Expr::TypeOf | WIRED | `just(Token::KwTypeof)` at line 650 with delimited_by(LParen, RParen) |
| `writ-compiler/src/lower/expr.rs` | `writ-compiler/src/ast/expr.rs` | Lowering maps Expr::TypeOf to AstExpr::TypeOf | WIRED | `Expr::TypeOf(e) => AstExpr::TypeOf { expr: Box::new(lower_expr(*e, ctx)), span }` at line 266 |
| `writ-compiler/src/check/check_expr/mod.rs` | `writ-compiler/src/check/ty.rs` | Type checker creates TyKind::ReflectionType for typeof expressions | WIRED | `ctx.interner.reflection_type(static_ty)` called at line 381; returns TyKind::ReflectionType |
| `writ-compiler/src/emit/body/expr/mod.rs` | `writ-compiler/src/emit/module_builder.rs` | Emitter calls type_ref_token_by_name or token_for_def to resolve type_idx | WIRED | `resolve_typeof_type_idx` calls `emitter.builder.token_for_def` and `emitter.builder.type_ref_token_by_name` at lines 525 and 529-532 |
| `writ-compiler/src/emit/collect/mod.rs` | `writ-compiler/src/emit/module_builder.rs` | collect_defs registers TypeRef rows for Type and primitives | WIRED | `builder.add_type_ref(runtime_mod_idx, "Type", "writ")` and primitives at lines 64-68 |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces a compiler pipeline, not a data-rendering component. The behavioral spot-checks cover data flow.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| lower_typeof_basic passes | `cargo test -p writ-compiler -- lower_typeof_basic` | ok | PASS |
| lower_typeof_snapshot passes | `cargo test -p writ-compiler -- lower_typeof_snapshot` | ok | PASS |
| typeof_primitive_type passes | `cargo test -p writ-compiler -- typeof_primitive_type` | ok | PASS |
| typeof_struct_type passes | `cargo test -p writ-compiler -- typeof_struct_type` | ok | PASS |
| typeof_type_error_on_arithmetic passes | `cargo test -p writ-compiler -- typeof_type_error_on_arithmetic` | ok | PASS |
| emit_typeof_struct passes | `cargo test -p writ-compiler -- emit_typeof_struct` | ok | PASS |
| emit_typeof_primitive_int passes | `cargo test -p writ-compiler -- emit_typeof_primitive_int` | ok | PASS |
| Full workspace regression | `cargo test --workspace` | 0 failures | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| COMP-01 | 104-01 | typeof(expr) lowered to AstExpr::TypeOf AST node (not a function call) | SATISFIED | `Expr::TypeOf(e) => AstExpr::TypeOf { ... }` in lower/expr.rs:266; lower_typeof tests confirm |
| COMP-02 | 104-02 | TypeOf instruction emitted with compile-time type index baked into instruction | SATISFIED | `emitter.emit(Instruction::TypeOf { r_dst, type_idx })` at body/expr/mod.rs:500; emit_typeof tests confirm type_idx is non-zero and matches token |
| COMP-04 | 104-02 | BOX/UNBOX coercion auto-inserted at reflection API boundaries | PARTIAL | BOX/UNBOX infrastructure exists in call.rs (emit_box_if_needed at line 180); REQUIREMENTS.md marks it Complete. However, `typeof_passes_to_type_param` test explicitly planned as COMP-04 validator was not implemented. The "auto-inserted at reflection API boundaries" semantic is unverified by test. |
| COMP-05 | 104-01 | TyKind::ReflectionType added to type checker for reflection builtin types | SATISFIED | `TyKind::ReflectionType(Ty)` in ty.rs:47; reflection_type() constructor; display() arm; TypedExpr::TypeOf; checker arm; all exhaustive match arms updated in 6 files |
| REFL-01 | 104-02 | typeof(expr) returns Type for any type expression (structs, classes, enums, entities, contracts, primitives) | SATISFIED | resolve_typeof_type_idx handles Struct/Class/Entity/Enum/Contract (via token_for_def) and Int/Float/Bool/String (via type_ref_token_by_name); emit_typeof_struct and emit_typeof_primitive_int confirm correctness |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-compiler/src/emit/body/expr/mod.rs` | (formerly) | Stub emit arm (alloc_reg only, no instruction) | None | Resolved — Plan 02 replaced it with real Instruction::TypeOf emission |

No active anti-patterns found in the phase output. All stubs identified in Plan 01 were resolved in Plan 02 as designed.

### Human Verification Required

#### 1. End-to-End Full-Pipeline Compile with typeof

**Test:** Compile a .writ source file containing `fn test(x: int) { let t = typeof(x); }` through the full compiler pipeline (parser -> lower -> resolve -> typecheck -> emit) and inspect the resulting .writil or binary module.
**Expected:** The emitted module contains a TypeOf instruction with a non-zero type_idx corresponding to the writ-runtime "Int" TypeRef token.
**Why human:** No golden test was added for a typeof-containing .writ source file. All emit tests (emit_typeof_struct, emit_typeof_primitive_int) are unit tests constructing TypedExpr::TypeOf directly — they bypass the parser/lower/typecheck pipeline.

### Gaps Summary

**Gap 1 (COMP-04 validator test missing):** The plan's Plan 02 Task 2 acceptance criteria explicitly required a `typeof_passes_to_type_param` test: `fn consume(t: Type) {} fn test(x: int) { consume(typeof(x)); }` asserting `has_no_errors`. This test was to validate that ReflectionType unifies correctly when passing typeof result to a function expecting the writ-runtime `Type` class. The test was not implemented. The REQUIREMENTS.md marks COMP-04 as Complete, but the behavioral claim "passing typeof(x) where a Type parameter is expected compiles without error" has no test coverage.

The BOX/UNBOX infrastructure (call.rs `emit_box_if_needed`) is present but applies to generic params, not specifically to reflection boundaries. Whether the type checker accepts `typeof(x)` at a `Type`-typed call site depends on how `Type` is resolved as a named type in the AST — this path is untested.

This is a **warning-level** gap: the infrastructure is plausibly correct (the SUMMARY claims no stubs and no deviations), but the explicit test was skipped. The fix is a single test function addition.

---

_Verified: 2026-03-28T14:00:00Z_
_Verifier: Claude (gsd-verifier)_
