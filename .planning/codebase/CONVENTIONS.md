# Coding Conventions

**Analysis Date:** 2026-02-26

## Naming Patterns

**Files:**
- Module files use lowercase with underscores: `lexer.rs`, `parser.rs`, `cst.rs`
- Test files follow pattern: `{module}_tests.rs` (e.g., `lexer_tests.rs`, `parser_tests.rs`)
- Test case data files in `tests/cases/` use numbered prefix and lowercase: `01_comments.writ`, `02_string_literals.writ`, `08_dialogue.writ`

**Functions:**
- Public functions use snake_case: `type_expr()`, `generic_params()`, `parse()`, `lex()`
- Private helper functions also use snake_case: `nested_block_comment()`, `raw_string()`, `formattable_string()`
- Single-purpose combinators/helpers are concise: `pattern()`, `atom()`, `named()`
- Test functions are descriptive: `type_simple()`, `precedence_mul_before_add()`, `postfix_null_propagate()`

**Variables:**
- snake_case for local variables and bindings: `depth`, `remainder`, `bytes`, `quote_count`, `brace_depth`
- Single-letter for loop counters: `i`, `j`, `k`
- Mutable bindings marked with `mut`: `let mut depth: u32 = 1;`

**Types:**
- Enum and struct names use PascalCase: `Trivia`, `CstToken`, `Program`, `Item`, `FnDecl`, `Expr`, `BinaryOp`
- Type aliases use snake_case: `type Span = SimpleSpan;`
- Generic parameters use uppercase letters: `T`, `U`, `I`, `Tokens`, `Src`
- Lifetime parameters use lowercase: `'src`, `'tokens`

## Code Style

**Formatting:**
- Rust formatter (rustfmt) is the implicit standard
- 4-space indentation
- Line wrapping at logical boundaries for long function signatures
- Opening braces on same line (K&R style)

**Linting:**
- No explicit `.clippy.toml` or `.rustfmt.toml` found - follows Rust defaults
- Code is well-structured with no obvious linting violations
- Comprehensive use of `Result` and `Option` types for error handling

**Module Documentation:**
- Module-level documentation uses `//!` syntax at top of file
- Example from `src/parser.rs`:
  ```rust
  //! Parser module for the Writ language.
  //!
  //! Converts the logos token stream into chumsky parser input and provides
  //! parsers for type expressions, generic parameters, expressions, and statements.
  ```
- Module docs explain: purpose, design decisions, and key implementation details

## Import Organization

**Order:**
1. External crate imports: `use logos::Logos;`, `use chumsky::prelude::*;`
2. Internal crate imports: `use crate::cst;`, `use crate::lexer::Token;`
3. Type aliases: `type Span = SimpleSpan;`

**Path Aliases:**
- Relative module imports use `crate::` prefix for clarity
- Wildcard imports used for prelude-style modules: `use chumsky::prelude::*;`
- Specific trait/function imports for precision: `use chumsky::recovery::{nested_delimiters, skip_then_retry_until};`

**Example from `src/parser.rs`:**
```rust
use chumsky::input::{Stream, ValueInput};
use chumsky::pratt::*;
use chumsky::prelude::*;
use chumsky::recovery::{nested_delimiters, skip_then_retry_until};

use crate::cst;
use crate::lexer::Token;

type Span = SimpleSpan;
```

## Error Handling

**Patterns:**
- Parser combinators use chumsky's `Rich<Token, Span>` error type for detailed diagnostics
- Lexer errors use `logos` framework's error token: `Token::Error` variant
- Test assertions use `assert!()` and `assert_eq!()` with custom error messages
- Panic-based validation in tests for invalid parse states: `panic!("Expected X, got {:?}", other)`
- Error recovery in parser via `nested_delimiters` and `skip_then_retry_until` for graceful failure

**Example error handling from tests:**
```rust
assert!(
    errors.is_empty(),
    "Found {} error tokens in 01_comments.writ. First error at byte offset {:?}: '{}'",
    errors.len(),
    errors.first().map(|(_, s)| (s.start, s.end)),
    errors.first().map(|(_, s)| &src[s.start..s.end]).unwrap_or("")
);
```

## Logging

**Framework:** None explicitly used in parser code

**Patterns:**
- Primarily assertion-based verification in tests
- Test output via panic messages for diagnostics
- Debug formatting via `{:?}` for complex types like AST nodes

## Comments

**When to Comment:**
- Module-level documentation for public modules explaining purpose and design
- Doc comments (`///`) on public types and functions explaining semantics
- Inline comments for complex algorithms (e.g., nested brace counting in `formattable_string()`)
- Section headers with `// =========` for test organization

**JSDoc/TSDoc:**
- Not applicable to Rust (uses `///` for rustdoc instead)
- Example from `src/cst.rs`:
  ```rust
  /// Every CST node carries its byte-offset span.
  /// `Spanned<T>` wraps a value with its source location.
  pub type Spanned<T> = (T, SimpleSpan);
  ```

## Function Design

**Size:**
- Functions average 20-100 lines
- Complex parsers are modularized into smaller combinator functions
- Parser functions are composed from smaller building blocks (e.g., `type_expr()` includes `named`, `fn_type`, `atom`, postfix application)

**Parameters:**
- Generic over input stream type `I: ValueInput<'tokens, Token = Token<'src>, Span = Span>`
- Lifetime parameters explicit for borrowed data: `'tokens`, `'src`
- Functions that need state (like lexer callbacks) receive `lex: &mut logos::Lexer<'src, Token<'src>>`

**Return Values:**
- Parser functions return `impl Parser<'tokens, I, OutputType, Error>`
- Combinators chain and compose via trait implementations
- Test helpers return `Vec<Spanned<...>>` for inspection
- Error types wrapped in `Result<T, E>` or `Option<T>` where applicable

## Module Design

**Exports:**
- Public API surface limited and explicit in `src/lib.rs`
- Example:
  ```rust
  pub mod cst;
  pub mod lexer;
  pub mod parser;

  pub use cst::*;
  pub use lexer::{lex, Token};
  pub use parser::parse;
  ```
- Internal helpers marked `private` (default visibility)

**Barrel Files:**
- `lib.rs` serves as main barrel file exporting key types and functions
- No nested barrel files in subdirectories (simple 3-module structure)

## Architectural Patterns

**Parser Combinators:**
- Built on chumsky combinator library with Pratt parsing for precedence
- Functions compose via `.then()`, `.or()`, `.delimited_by()`, `recursive()` combinators
- Type-safe parser composition prevents runtime errors

**Concrete Syntax Tree (CST):**
- Full-fidelity representation preserving trivia (whitespace and comments)
- Every node wrapped in `Spanned<T>` for source locations
- Supports lossless roundtrip: source → CST → source

**Lifetimes:**
- `'src` tracks borrowed source string slice throughout AST
- No heap allocation for source text (zero-copy design)
- `'tokens` tracks token stream lifetime for parser input

---

*Convention analysis: 2026-02-26*
