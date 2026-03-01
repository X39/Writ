---
phase: 10-parser-core-syntax
status: passed
verified: 2026-03-01
verifier: orchestrator
score: 6/6
---

# Phase 10: Parser — Core Syntax — Verification

## Phase Goal

The parser accepts `new` keyword construction, hex and binary integer literals, struct lifecycle hooks, self/mut-self parameters, and the bit-shift and bitwise operators.

## Success Criteria Verification

### 1. `new Type { field: value }` parses successfully and produces an `Expr::New` CST node; the old brace-construction syntax is rejected

**Status: PASS**

- `KwNew` token added to lexer; `Expr::New` CST variant with `NewField` struct added to `cst.rs`
- `new_expr` parser combinator: `new + type_expr + { field_list }`
- `BraceConstruct` postfix entirely removed — bare `Type { field: value }` produces a generic parse error
- `new` made contextual keyword (accepted in `ident_token` so `List::new()` paths still work)
- Lowering arm `Expr::New` -> `AstExpr::New` added
- Plan 10-01 evidence: KwNew token, Expr::New CST, BraceConstruct removed, spawn syntax updated
- Plan 10-02 tests: `new_construction_basic`, `new_construction_empty`, `new_construction_trailing_comma`, `new_construction_generic_type`, `new_construction_rooted_path`, `old_construction_syntax_rejected` (6 parser tests); `lower_new_construction_basic`, `lower_new_construction_empty`, `lower_new_construction_generic` (3 snapshot tests)

### 2. `0xFF` and `0b1010` parse as integer literal atoms in expressions

**Status: PASS**

- `Token::HexLit` and `Token::BinLit` added to the literal `select!` block in the parser
- Both map to `Expr::IntLit`, preserving raw text for later semantic interpretation
- Plan 10-01 evidence: HexLit/BinLit in select! block in `parser.rs`
- Plan 10-02 tests: `hex_literal_parses_as_int`, `binary_literal_parses_as_int`, `hex_literal_uppercase`, `binary_literal_uppercase` (4 parser tests); `lower_hex_binary_literals` (1 snapshot test)

### 3. `on create`, `on finalize`, `on serialize`, and `on deserialize` parse inside struct bodies and produce lifecycle hook CST nodes

**Status: PASS**

- `StructMember` enum with `Field(StructField)` and `OnHook { event, body }` variants added to CST
- `StructDecl.fields` renamed to `StructDecl.members: Vec<Spanned<StructMember>>`
- `struct_on_hook` parser added for `on event { body }` inside struct bodies
- Fields and hooks can be freely interleaved
- `AstStructMember` enum and `lower_struct_member` function added to lowering
- Plan 10-01 evidence: StructMember enum, struct_on_hook parser combinator, AstStructMember in AST
- Plan 10-02 tests: `struct_with_on_create_hook`, `struct_interleaved_fields_and_hooks`, `struct_all_four_lifecycle_hooks`, `struct_hook_with_body` (4 parser tests); `lower_struct_lifecycle_hook`, `lower_struct_multiple_hooks` (2 snapshot tests)

### 4. `self` and `mut self` are accepted as the first parameter in a function or method declaration and produce a distinct CST parameter variant

**Status: PASS**

- `FnParam` enum with `Regular(Param)` and `SelfParam { mutable }` variants added
- `self_param` parser and `fn_param_list_with_self` combinator added
- `FnDecl.params` and `FnSig.params` updated to use `Vec<Spanned<FnParam>>`
- `AstFnParam` enum and `lower_fn_param` function added to lowering
- Self param applied to fn/method declarations; DlgDecl and entity On handlers keep raw `Param`
- Plan 10-01 evidence: FnParam enum, self_param parser, AstFnParam in ast/decl.rs
- Plan 10-02 tests: `fn_with_self_param`, `fn_with_mut_self_param`, `fn_self_with_regular_params` (3 parser tests); `lower_self_param`, `lower_mut_self_with_regular_param` (2 snapshot tests)

### 5. `<<` and `>>` lex and parse as binary operators and appear in the `BinaryOp` enum

**Status: PASS**

- `BinaryOp::Shl` and `BinaryOp::Shr` added to CST and AST
- Implemented as two-token sequences (`Lt+Lt`, `Gt+Gt`) in the Pratt parser at precedence level 10 — avoids `>>` ambiguity with nested generics (e.g., `Map<string, List<int>>`)
- `lower_binop` arms for `Shl`/`Shr` added to lowering
- Plan 10-01 evidence: BinaryOp::Shl/Shr in cst.rs, two-token sequence parsing in parser.rs
- Plan 10-02 tests: `shift_left_operator`, `shift_right_operator`, `shift_precedence_below_additive`, `shift_precedence_above_comparison` (4 parser tests); `lower_shift_operators` (1 snapshot test)

### 6. `BitAnd` and `BitOr` (`&` and `|`) are added to the `OpSymbol` enum and are accepted by the operator overloading parser

**Status: PASS**

- `BitAnd` and `BitOr` added to CST `OpSymbol` and AST `AstOpSymbol`
- Parser mappings added: `Token::Amp -> OpSymbol::BitAnd`, `Token::Pipe -> OpSymbol::BitOr`
- `lower_op_symbol` arms and `op_symbol_to_contract` mappings added
- Plan 10-01 evidence: OpSymbol::BitAnd/BitOr additions in cst.rs, parser mappings in parser.rs
- Plan 10-02 tests: `operator_bitand_in_impl`, `operator_bitor_in_impl` (2 parser tests); `lower_bitand_bitor_operators` (1 snapshot test)

## Requirement Coverage

All 6 phase requirements accounted for:

| Requirement | Plan    | Status   | Evidence |
|-------------|---------|----------|----------|
| PARSE-01    | 10-01   | Verified | `Expr::New` CST node; old brace-construction syntax rejected; 6 parser tests + 3 snapshot tests pass |
| PARSE-02    | 10-01   | Verified | `Token::HexLit`/`BinLit` in select! block → `Expr::IntLit`; 4 parser tests + 1 snapshot test pass |
| DECL-01     | 10-01   | Verified | `StructMember::OnHook` in CST; `struct_on_hook` parser; 4 parser tests + 2 snapshot tests pass |
| DECL-02     | 10-01   | Verified | `FnParam::SelfParam` in CST; `self_param` parser; 3 parser tests + 2 snapshot tests pass |
| EXPR-01     | 10-01   | Verified | `BinaryOp::Shl/Shr`; two-token sequence parsing; 4 parser tests + 1 snapshot test pass |
| EXPR-02     | 10-01   | Verified | `OpSymbol::BitAnd/BitOr`; Token::Amp/Pipe mappings; 2 parser tests + 1 snapshot test pass |

## Test Results

```
cargo test --workspace: 380 passed, 0 failed
  79 lowering tests
  13 unit tests (string_utils)
  74 lexer tests
  212 parser tests
  2 doc tests
```

New tests added: 33 total
- 23 parser tests covering all 6 requirements (Plan 10-02)
- 10 lowering snapshot tests covering all 6 requirements (Plan 10-02)

All existing tests pass with zero regressions.

## Gaps Found

None.

---
*Phase: 10-parser-core-syntax*
*Verified: 2026-03-01*
