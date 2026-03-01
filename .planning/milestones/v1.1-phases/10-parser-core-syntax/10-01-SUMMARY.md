---
phase: 10-parser-core-syntax
plan: 01
subsystem: parser, compiler
tags: [cst, ast, lexer, parser, lowering, new-expr, hex-bin-literals, struct-hooks, self-param, shift-ops, bitand-bitor]

requires:
  - phase: 09-cst-type-system-additions
    provides: "TypeExpr::Qualified, Expr::Path rooted flag, DlgDecl attrs/vis"
provides:
  - "Expr::New construction syntax with `new` keyword"
  - "Hex/binary literal atoms parsed as IntLit"
  - "StructMember enum with Field and OnHook variants for struct lifecycle hooks"
  - "FnParam enum with Regular and SelfParam variants"
  - "BinaryOp::Shl/Shr shift operators parsed as two-token sequences"
  - "OpSymbol::BitAnd/BitOr for operator overloading"
  - "BraceConstruct postfix removed — old Type { } syntax rejected"
affects: [11-parser-advanced-syntax]

tech-stack:
  added: []
  patterns: ["Shift operators parsed as two-token sequences to avoid >> generic ambiguity"]

key-files:
  created: []
  modified:
    - writ-parser/src/lexer.rs
    - writ-parser/src/cst.rs
    - writ-parser/src/parser.rs
    - writ-compiler/src/ast/expr.rs
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/src/lower/mod.rs
    - writ-compiler/src/lower/expr.rs
    - writ-compiler/src/lower/operator.rs
    - writ-compiler/src/lower/dialogue.rs
    - writ-compiler/src/lower/entity.rs
    - writ-parser/tests/parser_tests.rs
    - writ-parser/tests/cases/09_entities.writ
    - writ-parser/tests/cases/10_operators.writ

key-decisions:
  - "Shift operators << >> parsed as two-token sequences (Lt+Lt, Gt+Gt) in Pratt parser, NOT as dedicated lexer tokens — avoids >> ambiguity with nested generics like Map<string, List<int>>"
  - "KwNew added as contextual keyword: accepted in ident_token select! block so List::new() still works as a path"
  - "BraceConstruct postfix completely removed — spawn syntax updated to spawn new Guard {}"
  - "StructDecl.fields renamed to StructDecl.members using StructMember enum"
  - "FnParam enum wraps Param for fn/method declarations only — DlgDecl, OpDecl, entity On handlers keep raw Param"
  - "Custom 'requires new keyword' error for bare Type { } syntax deferred — generic parse error is sufficient"

patterns-established:
  - "Two-token sequence parsing for operators that conflict with delimiters (<<, >>)"
  - "Contextual keywords via ident_token select! block (new joins entity, component, use, on)"
  - "StructMember/FnParam wrapper enums for mixed-content declaration bodies"

requirements-completed: [PARSE-01, PARSE-02, DECL-01, DECL-02, EXPR-01, EXPR-02]

duration: ~45min
completed: 2026-03-01
---

# Plan 10-01: CST + Lexer + Parser + Lowering Summary

**Implemented all 6 Phase 10 syntax features across the full parse pipeline: lexer, CST, parser, AST, and lowering**

## Performance

- **Duration:** ~45 min (across two context windows)
- **Tasks:** 3 (type definitions, parser changes, lowering updates)
- **Files modified:** 13 (+ 39 snapshot files auto-accepted)

## Accomplishments

### PARSE-01: `new` keyword construction
- Added `KwNew` token to lexer
- Added `Expr::New` CST variant with `NewField` struct
- Added `new_expr` parser combinator (new + type_expr + { field_list })
- Removed `BraceConstruct` postfix entirely — old `Type { field: value }` syntax rejected
- Added `AstExpr::New` and `AstNewField` to AST
- Added lowering arm for `Expr::New` -> `AstExpr::New`
- Made `new` a contextual keyword (accepted in `ident_token` for paths like `List::new()`)

### PARSE-02: Hex/binary literal atoms
- Added `Token::HexLit` and `Token::BinLit` to the literal `select!` block
- Both map to `Expr::IntLit` preserving raw text for later semantic interpretation

### DECL-01: Struct lifecycle hooks
- Created `StructMember` enum with `Field(StructField)` and `OnHook { event, body }` variants
- Renamed `StructDecl.fields` to `StructDecl.members: Vec<Spanned<StructMember>>`
- Added `struct_on_hook` parser for `on event { body }` inside struct bodies
- Created `AstStructMember` enum and `lower_struct_member` function
- Fields and hooks can be freely interleaved

### DECL-02: Self/mut self parameters
- Created `FnParam` enum with `Regular(Param)` and `SelfParam { mutable }` variants
- Added `self_param` parser and `fn_param_list_with_self` combinator
- Updated `FnDecl.params` and `FnSig.params` to use `Vec<Spanned<FnParam>>`
- Created `AstFnParam` enum and `lower_fn_param` function
- Self param only applied to fn/method declarations, not DlgDecl or entity On handlers

### EXPR-01: Shift operators
- Added `BinaryOp::Shl` and `BinaryOp::Shr` to CST and AST
- Initially tried dedicated `Shl`/`Shr` lexer tokens but discovered `>>` ambiguity with nested generics
- Solved by parsing `<<` and `>>` as two-token sequences (`Lt+Lt`, `Gt+Gt`) in the Pratt parser at precedence level 10
- Added `lower_binop` arms for `Shl`/`Shr`

### EXPR-02: BitAnd/BitOr in OpSymbol
- Added `BitAnd` and `BitOr` to CST `OpSymbol` and AST `AstOpSymbol`
- Added parser mappings: `Token::Amp -> OpSymbol::BitAnd`, `Token::Pipe -> OpSymbol::BitOr`
- Added `lower_op_symbol` arms and `op_symbol_to_contract` mappings

### Cross-cutting changes
- Updated lowering in `operator.rs`, `dialogue.rs`, `entity.rs` to handle `AstFnParam`/`AstStructMember` wrappers
- Updated all existing parser test assertions for `fields->members` and `params->FnParam` changes
- Added `unwrap_struct_field` and `unwrap_regular_param` test helpers
- Updated test .writ files to use `spawn new Guard {}` syntax
- Accepted 39 lowering snapshot updates via `cargo insta accept --workspace`

## Test Results
- All 347 tests pass: 69 lowering + 13 unit + 74 lexer + 189 parser + 2 doctest
- Zero regressions

## Deviations from Plan
- **Shl/Shr tokens removed from lexer:** Plan specified dedicated `Shl`/`Shr` tokens, but `>>` caused ambiguity with nested generics (`Map<string, List<int>>`). Solved by parsing as two-token sequences instead.
- **KwNew contextual keyword:** `new` as a keyword broke `List::new()` paths. Added `KwNew` to `ident_token` select! block so it works as an identifier in path context.
- **BraceConstruct removal cascaded:** Required updating test .writ files for spawn syntax (`spawn Guard {}` -> `spawn new Guard {}`)
- **Custom "requires new keyword" error deferred:** The old syntax produces a generic parse error instead of a targeted message — sufficient for now.
- **Additional files modified:** `dialogue.rs`, `entity.rs` also needed lowering updates (not in plan's files_modified list)

## Issues Encountered
- **>> generic ambiguity:** Critical discovery that `>>` as a single token breaks nested generics. Two-token sequence approach is the standard solution (used by C++, Rust, Java).
- **39 snapshot failures:** Expected — structural changes to AST types change serialized output. Bulk-accepted.

## User Setup Required
None.

## Next Phase Readiness
- Plan 10-02 (comprehensive tests) ready to execute
- All CST/AST/lowering changes are in place for test verification

---
*Phase: 10-parser-core-syntax*
*Completed: 2026-03-01*
