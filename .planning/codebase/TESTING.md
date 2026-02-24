# Testing Patterns

**Analysis Date:** 2026-02-26

## Test Framework

**Runner:**
- Rust `cargo test` - built-in test framework
- Dependencies:
  - `logos` 0.16 - lexer generator with test support
  - `chumsky` 0.12.0 - parser combinator library
  - `insta` 1.x - snapshot testing with RON format (dev dependency)
  - `ariadne` 0.6 - error reporting (dev dependency)
- Config: `Cargo.toml` in `/d/dev/git/Writ/writ-parser/`

**Assertion Library:**
- Standard Rust assertions: `assert!()`, `assert_eq!()`, `assert_ne!()`
- Custom match-based assertions in test helpers for detailed error messages

**Run Commands:**
```bash
cargo test                    # Run all tests
cargo test --lib            # Run library unit tests only
cargo test --test '*'        # Run integration tests
cargo test -- --nocapture    # Show println! output
cargo test -- --test-threads=1  # Run tests sequentially
```

## Test File Organization

**Location:**
- Lexer tests: `/d/dev/git/Writ/writ-parser/tests/lexer_tests.rs`
- Parser tests: `/d/dev/git/Writ/writ-parser/tests/parser_tests.rs`
- Both co-located in standard `tests/` directory (integration-style tests)
- Test case data: `/d/dev/git/Writ/writ-parser/tests/cases/` - numbered Writ source files

**Naming:**
- Test functions: descriptive snake_case like `lossless_roundtrip_comments()`, `type_simple()`, `precedence_mul_before_add()`
- Test case files: numbered with underscore-separated descriptions: `01_comments.writ`, `02_string_literals.writ`, etc.
- Helper functions: descriptive for clarity: `parse_ok()`, `parse_ok_items()`, `let_value()`, `let_type()`

**Structure:**
```
tests/
├── lexer_tests.rs      # Token and lexical analysis tests
├── parser_tests.rs     # AST and parser tests
└── cases/              # Test case data files
    ├── 01_comments.writ
    ├── 02_string_literals.writ
    ├── 08_dialogue.writ
    └── ... (16 Writ files total)
```

## Test Structure

**Suite Organization:**
Test functions are grouped by feature area with header comments:

```rust
// =============================================================
// Lossless Roundtrip Tests (LEX-02, INTG-02)
// =============================================================

#[test]
fn lossless_roundtrip_comments() {
    let src = include_str!("cases/01_comments.writ");
    let tokens = lex(src);
    let reconstructed: String = tokens
        .iter()
        .map(|(_, span)| &src[span.start..span.end])
        .collect();
    assert_eq!(
        src, reconstructed,
        "Lossless roundtrip failed for 01_comments.writ"
    );
}
```

**Patterns:**

1. **Lossless Roundtrip Pattern** (lexer tests):
   - Tokenize source, reconstruct from token spans, compare to original
   - Verifies no data loss during lexing
   - Example: `lossless_roundtrip_comments()`, `lossless_roundtrip_string_literals()`

2. **Error Token Detection** (lexer validation):
   - Tokenize and filter for `Token::Error` variants
   - Assert empty error list with detailed diagnostics
   - Example:
   ```rust
   #[test]
   fn no_error_tokens_in_comments_writ() {
       let src = include_str!("cases/01_comments.writ");
       let tokens = lex(src);
       let errors: Vec<_> = tokens
           .iter()
           .filter(|(t, _)| matches!(t, Token::Error))
           .collect();
       assert!(
           errors.is_empty(),
           "Found {} error tokens in 01_comments.writ. First error at byte offset {:?}: '{}'",
           errors.len(),
           errors.first().map(|(_, s)| (s.start, s.end)),
           errors.first().map(|(_, s)| &src[s.start..s.end]).unwrap_or("")
       );
   }
   ```

3. **Parse-and-Inspect Pattern** (parser tests):
   - Parse source code with `parse(src)` returning `(output, errors)`
   - Assert no errors: `assert!(errors.is_empty(), ...)`
   - Extract and pattern-match specific CST nodes
   - Example:
   ```rust
   fn parse_ok(src: &'static str) -> Vec<Spanned<Stmt<'static>>> {
       let items = parse_ok_items(src);
       items
           .into_iter()
           .map(|(item, span)| match item {
               Item::Stmt(s) => s,
               Item::Dlg(decl) => (Stmt::DlgDecl(decl), span),
               other => panic!("Expected Item::Stmt or Item::Dlg, got {:?}", other),
           })
           .collect()
   }
   ```

4. **Assertion-by-Pattern Matching** (parser validation):
   - Match on CST enum variants and assert structure
   - Use panic with descriptive message for invalid structures
   - Example:
   ```rust
   #[test]
   fn type_generic() {
       let stmts = parse_ok("let x: List<int> = null;");
       assert_eq!(stmts.len(), 1);
       match let_type(&stmts[0]) {
           TypeExpr::Generic(base, args) => {
               assert!(matches!(base.0, TypeExpr::Named("List")));
               assert_eq!(args.len(), 1);
               assert!(matches!(args[0].0, TypeExpr::Named("int")));
           }
           other => panic!("Expected TypeExpr::Generic, got {:?}", other),
       }
   }
   ```

## Mocking

**Framework:** Not used in parser codebase

**Patterns:**
- No mocking needed - parser is pure function with no side effects
- Test data via `include_str!()` macro for embedded source files
- Fixtures use static Writ source strings

**What to Mock:**
- N/A - codebase has no external dependencies to mock

**What NOT to Mock:**
- Parser combinators (test real parser behavior)
- Lexer tokens (test real tokenization)
- Source text (use actual Writ code examples)

## Fixtures and Factories

**Test Data:**
- Embedded source strings in test functions:
  ```rust
  let stmts = parse_ok("let x: int = 42;");
  let stmts = parse_ok("let x = a + b * c;");
  let stmts = parse_ok("if damaged { playSound(\"hit\"); }");
  ```

- Included external Writ files via `include_str!()`:
  ```rust
  let src = include_str!("cases/01_comments.writ");
  let src = include_str!("cases/08_dialogue.writ");
  ```

**Location:**
- Inline source strings in test functions for simple cases
- External files in `/d/dev/git/Writ/writ-parser/tests/cases/` for complex grammar coverage
- 16 test case files covering: comments, strings, variables, structs, enums, contracts, functions, dialogue, entities, operators, error handling, namespaces, concurrency, attributes, ranges/indexing, generics

## Coverage

**Requirements:** Not explicitly enforced - no coverage tool configured in `Cargo.toml`

**Test Count:** 177 test functions across both files
- Lexer tests: ~60 tests covering tokens, comments, strings, special cases
- Parser tests: ~177 tests covering types, expressions, control flow, declarations

**View Coverage:**
```bash
# Using tarpaulin (install: cargo install cargo-tarpaulin)
cargo tarpaulin --out Html

# Using llvm-cov (requires nightly)
cargo +nightly llvm-cov
```

## Test Types

**Unit Tests:**
- Scope: Individual parser combinator functions and token types
- Approach: Test-driven validation of parsing rules and precedence
- Example: `precedence_mul_before_add()`, `type_array()`, `postfix_null_propagate()`
- Location: `tests/lexer_tests.rs`, `tests/parser_tests.rs`

**Integration Tests:**
- Scope: Full source code → AST workflow (end-to-end parsing)
- Approach: Parse complete Writ programs and validate resulting CST structure
- Example: `lossless_roundtrip_*()` tests, complex dialogue parsing
- Location: Both test files (all tests are integration-style in `tests/` directory)

**E2E Tests:**
- Not used - parser is library component, not standalone application
- End-to-end covered by integration tests (full source to CST)

## Common Patterns

**Async Testing:**
- Not applicable - Rust parser is synchronous

**Error Testing:**
```rust
#[test]
fn no_error_tokens_in_string_literals_writ() {
    let src = include_str!("cases/02_string_literals.writ");
    let tokens = lex(src);
    let errors: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "Found {} error tokens in 02_string_literals.writ. First error at byte offset {:?}: '{}'",
        errors.len(),
        errors.first().map(|(_, s)| (s.start, s.end)),
        errors.first().map(|(_, s)| &src[s.start..s.end]).unwrap_or("")
    );
}
```

**Operator Precedence Testing:**
```rust
#[test]
fn precedence_mul_before_add() {
    // a + b * c => Binary(Ident(a), Add, Binary(Ident(b), Mul, Ident(c)))
    let stmts = parse_ok("let x = a + b * c;");
    match let_value(&stmts[0]) {
        Expr::Binary(lhs, BinaryOp::Add, rhs) => {
            assert!(matches!(lhs.0, Expr::Ident("a")));
            match &rhs.0 {
                Expr::Binary(rl, BinaryOp::Mul, rr) => {
                    assert!(matches!(rl.0, Expr::Ident("b")));
                    assert!(matches!(rr.0, Expr::Ident("c")));
                }
                other => panic!("Expected Binary(Mul), got {:?}", other),
            }
        }
        other => panic!("Expected Binary(Add), got {:?}", other),
    }
}
```

**Type Annotation Testing:**
```rust
fn let_type<'a>(stmt: &'a Spanned<Stmt<'a>>) -> &'a TypeExpr<'a> {
    match &stmt.0 {
        Stmt::Let { ty: Some(t), .. } => &t.0,
        Stmt::Let { ty: None, .. } => panic!("Expected type annotation, got None"),
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}
```

## Test Organization by Phase

Tests are organized by Writ language feature implementation phases:

**Phase 1: Lexer (LEX-01 through LEX-03)**
- Comment tokenization, string literals, raw strings, formattable strings
- Lossless roundtrip validation
- Error token detection

**Phase 2: Types, Expressions, Control Flow (TYPE-01 through CTRL-07)**
- Type expressions: simple, generic, array, nullable, function types
- Expressions: binary operators, unary operators, postfix operators
- Control flow: if/else, match, loops, lambda expressions
- ~100 test functions

**Phase 3: Dialogue Blocks (DLG-01 through DLG-09)**
- Dialogue declaration parsing
- Speaker/text/flag segment parsing
- Nested blocks and complex syntax

**Phase 4: Declarations (DECL-01 through DECL-13)**
- Function, struct, enum, contract, impl declarations
- Entity and component declarations
- Namespace and extern declarations

---

*Testing analysis: 2026-02-26*
