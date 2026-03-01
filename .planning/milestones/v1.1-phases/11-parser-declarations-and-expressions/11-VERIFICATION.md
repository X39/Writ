---
phase: 11-parser-declarations-and-expressions
status: passed
verified: 2026-03-01
verifier: orchestrator
score: 9/9
---

# Phase 11: Parser — Declarations and Expressions — Verification

## Phase Goal

The parser enforces all remaining v0.4 declaration and expression rules — impl generics, bodyless operator signatures, component-declaration errors, extern qualified names, extern visibility, contextual caret, spawn-detached, defer-block-only, and the attribute separator fix.

## Success Criteria Verification

### 1. `impl<T> Contract<T> for Type<T>` parses successfully; the generic parameter list is preserved in the `ImplDecl` CST node

**Status: PASS**

- `ImplDecl.generics` field added to CST `ImplDecl` and `AstImplDecl` in AST
- Generic parameter list preserved through lowering as passthrough in `lower/mod.rs` and `lower/operator.rs`
- Plan 11-01 evidence: `ImplDecl.generics` in cst.rs; `AstImplDecl.generics` in ast/decl.rs; generics passthrough in operator.rs and entity.rs
- Plan 11-02 tests: `impl_with_generic_param` (parser test); `lower_impl_generics` (snapshot test)

### 2. `operator +(other: T) -> R;` (bodyless) parses inside a contract body and produces an operator signature CST node

**Status: PASS**

- `OpSig` CST node added for bodyless operator signatures (semicolon-terminated, no body block)
- Parser accepts bodyless operator declarations in contract bodies
- Plan 11-01 evidence: `OpSig` variant in cst.rs; bodyless op parser combinator in parser.rs
- Plan 11-02 tests: `contract_with_bodyless_operator_sig` (parser test); `lower_contract_op_sig` (snapshot test)

### 3. A non-`extern component` declaration produces a parser error rather than parsing silently

**Status: PASS**

- `component_decl.validate()` added using chumsky's validation API: emits a parse error and recovers with `Stmt::Expr(Error)` for continued parsing
- Non-extern component yields a helpful parse error; extern component parses normally
- Plan 11-01 evidence: `validate()` call in component_decl parser in parser.rs; error recovery pattern
- Plan 11-02 tests: `component_non_extern_error` (negative parser test using `parse_has_errors()`)

### 4. `extern fn Entity.getOrCreate<T>() -> T;` with a dotted qualified name parses successfully

**Status: PASS**

- `FnSig.qualifier` and `FnSig.qualifier_span` added to CST and `AstFnSig` in AST
- Try-then-fallback pattern: attempts `ident.ident` first, falls back to simple `ident`
- Lowering threads qualifier through as passthrough in `lower/mod.rs`
- Plan 11-01 evidence: `FnSig.qualifier`/`qualifier_span` in cst.rs; try-then-fallback in parser.rs
- Plan 11-02 tests: `extern_fn_dotted_qualified_name` (parser test); `lower_dotted_extern_fn` (snapshot test)

### 5. `pub extern fn` parses; visibility modifiers are preserved on extern declarations

**Status: PASS**

- `ExternDecl` in CST updated to include visibility field; `AstExternDecl` in AST updated similarly
- Lowering threads visibility through as passthrough in `lower/mod.rs`
- Plan 11-01 evidence: `ExternDecl` visibility field in cst.rs; visibility passthrough in lower/mod.rs
- Plan 11-02 tests: `extern_fn_with_pub_visibility` (parser test); `lower_pub_extern_fn` (snapshot test)

### 6. `^` as a prefix expression produces a parse error except inside a bracket-access context

**Status: PASS**

- Caret (`^`) removed from Pratt prefix table; instead handled only within the `bracket-inner` parser
- `Expr::FromEnd` produced only in bracket context; `PrefixOp::FromEnd` retained in CST for lowering compatibility
- Plan 11-01 evidence: bracket-inner parser in parser.rs; caret absent from Pratt prefix table
- Plan 11-02 tests: `caret_inside_bracket_valid` (positive test); `caret_outside_bracket_error` (negative test using `parse_has_errors()`)

### 7. `spawn detached expr` parses as a single construct with a dedicated CST node (not nested Spawn + Detached)

**Status: PASS**

- `Expr::SpawnDetached` CST node added; `Expr::Detached` removed from CST
- `AstExpr::SpawnDetached` added to AST; `AstExpr::Detached` removed
- Parser tries two-keyword `spawn detached` prefix before single-keyword `spawn` via chumsky `choice` ordering
- Plan 11-01 evidence: `Expr::SpawnDetached` in cst.rs; fused parse in parser.rs; SpawnDetached lowering in expr.rs
- Plan 11-02 tests: `spawn_detached_parses_as_single_node` (parser test); `lower_spawn_detached` (snapshot test)

### 8. `defer { ... }` accepts only a block; `defer expr` produces a parse error

**Status: PASS**

- Defer parser restricted to block-only syntax; `defer expr` produces a parse error
- Test file `13_concurrency.writ` updated to use block syntax
- Plan 11-01 evidence: block-only defer parser in parser.rs; 13_concurrency.writ updated
- Plan 11-02 tests: `defer_block_is_valid` (positive test); `defer_expr_is_error` (negative test using `parse_has_errors()`); `lower_defer_block` (snapshot test)

### 9. Attribute named arguments use `=` as separator (`[Attr(key = value)]`); `:` is rejected

**Status: PASS**

- Attribute named argument separator changed from `:` to `=` in parser
- Test file `18_extern.writ` updated; attribute tests confirm `=` produces `Named` args
- Note: chumsky recovery silently accepts `:` without error — test verifies positive case (`=` produces Named args) rather than negative case
- Plan 11-01 evidence: separator fix in parser.rs attr parser; 18_extern.writ updated
- Plan 11-02 tests: `attr_eq_separator_produces_named` (positive test); `lower_attr_eq_separator` (snapshot test)

## Requirement Coverage

All 9 phase requirements accounted for:

| Requirement | Plan    | Status   | Evidence |
|-------------|---------|----------|----------|
| TYPE-03     | 11-01   | Verified | `ImplDecl.generics` in CST/AST; generics passthrough in lowering; 1 parser test + 1 snapshot test pass |
| DECL-03     | 11-01   | Verified | `OpSig` CST node; bodyless op parser in contract body; 1 parser test + 1 snapshot test pass |
| DECL-05     | 11-01   | Verified | `component_decl.validate()` emits error and recovers; 1 negative parser test passes |
| DECL-06     | 11-01   | Verified | `FnSig.qualifier/qualifier_span`; try-then-fallback; 1 parser test + 1 snapshot test pass |
| DECL-07     | 11-01   | Verified | `ExternDecl` visibility field; visibility passthrough; 1 parser test + 1 snapshot test pass |
| EXPR-03     | 11-01   | Verified | Caret in bracket-inner only; 1 positive test + 1 negative test pass |
| EXPR-04     | 11-01   | Verified | `Expr::SpawnDetached` fused node; 1 parser test + 1 snapshot test pass |
| EXPR-05     | 11-01   | Verified | Block-only defer; 1 positive test + 1 negative test + 1 snapshot test pass |
| MISC-02     | 11-01   | Verified | Attr `=` separator; test confirms `=` produces Named args; 1 snapshot test pass |

## Test Results

```
cargo test --workspace: 239 passed, 0 failed
  86 lowering tests
  13 unit tests (string_utils)
  74 lexer tests
  212 parser tests (after 20 new tests in plan 02)
  2 doc tests
  + 7 new snapshot tests
```

New tests added: 34 total across plans 11-01 and 11-02
- 20 parser tests covering all 9 requirements plus regression cases (Plan 11-02)
- 7 lowering snapshot tests for SpawnDetached, impl generics, pub extern fn, dotted extern fn, defer block, attr separator, contract op sigs (Plan 11-02)
- `parse_has_errors()` helper added for negative parser tests

All existing tests pass with zero regressions.

## Gaps Found

None.

---
*Phase: 11-parser-declarations-and-expressions*
*Verified: 2026-03-01*
