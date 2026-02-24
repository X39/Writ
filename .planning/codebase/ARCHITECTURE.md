# Architecture

**Analysis Date:** 2026-02-26

## Pattern Overview

**Overall:** Multi-stage compiler pipeline with modular crate separation

**Key Characteristics:**
- Workspace-based Rust project with 4 independent crates: parser, compiler, runtime, CLI
- Three-stage pipeline: Lexing → Parsing → AST/CST generation
- Full-fidelity Concrete Syntax Tree (CST) preservation enabling lossless roundtrip
- Roslyn-style trivia attachment for whitespace and comment preservation
- Pratt parser for operator precedence with 13+ levels
- Error recovery using chumsky's skip-then-retry strategy with balanced delimiter tracking

## Layers

**Lexer (Token Stream):**
- Purpose: Tokenize Writ source code into a stream of tokens with span tracking
- Location: `writ-parser/src/lexer.rs`
- Contains: Token definitions, logo-based lexer rules, nested comment handling, raw string parsing, formattable string tokenization
- Depends on: logos (lexer generator), chumsky (span types)
- Used by: Parser layer, test harness

**Parser (AST/CST Generation):**
- Purpose: Convert token stream into Concrete Syntax Tree preserving all source information
- Location: `writ-parser/src/parser.rs`
- Contains: Chumsky parser combinators for types, expressions, statements, declarations; mutual recursion handling; error recovery
- Depends on: Lexer, CST types, chumsky (pratt, recovery, combinators)
- Used by: CLI, compiler, test harness

**CST (Type Definitions):**
- Purpose: Define all Concrete Syntax Tree node types with full source span and trivia information
- Location: `writ-parser/src/cst.rs`
- Contains: 40+ enum/struct types representing program structure, declarations, expressions, statements, dialogue constructs
- Depends on: chumsky spans, standard library types
- Used by: Parser, downstream compilation, testing

**CLI (Entry Point):**
- Purpose: Command-line interface for parsing and processing Writ files
- Location: `writ-cli/src/main.rs`
- Triggers: User invocation of command-line tool
- Responsibilities: Placeholder implementation (current stub)

**Compiler (Backend):**
- Purpose: Code generation and semantic analysis from CST
- Location: `writ-compiler/src/main.rs`
- Responsibilities: Placeholder implementation (current stub)

**Runtime (Execution):**
- Purpose: Runtime environment for executing compiled Writ code
- Location: `writ-runtime/src/main.rs`
- Responsibilities: Placeholder implementation (current stub)

**Library Exports (Public API):**
- Location: `writ-parser/src/lib.rs`
- Exports: `cst`, `lexer` (lex function, Token), `parser` (parse function)
- Provides single entry point for downstream consumers

## Data Flow

**Source Code to CST:**

1. User source code string (e.g., "let x = 42;") enters `parse()` function
2. `parse()` calls `lexer::lex(src)` producing `Vec<(Token, SimpleSpan)>`
3. Trivia tokens (Whitespace, LineComment, BlockComment) filtered out
4. Non-trivia tokens wrapped in chumsky Stream with end-of-input marker
5. `program_parser()` combinator chain consumes token stream
6. Each parser combinator processes tokens, building nested CST nodes
7. `Into<Output, Errors>` unwraps producing `(Option<Vec<Spanned<Item>>>, Vec<RichError>)`

**Parse Error Handling:**

- Errors collected throughout parsing via chumsky's error accumulation
- Recovery triggered on parse failure using `recover_with(skip_then_retry_until())`
- Balanced delimiter tracking (braces, parens, brackets) ensures recovery consumes coherent blocks
- Invalid nodes marked as `Expr::Error` or `Stmt::Error` placeholders
- Original error messages preserved in error vector for diagnostic reporting

**State Management:**

- Immutable during parsing: source string lifetime ('src) borrowed throughout
- CST nodes reference source string directly (no allocation for identifiers/strings)
- Spans carry byte offsets into source for exact error location mapping
- Trivia preserved via parallel Token structure (leading/trailing fields)
- No mutable state within parser combinators—pure functional composition

## Key Abstractions

**Token:**
- Purpose: Represents lexical elements (keywords, identifiers, operators, literals)
- Examples: `Token::Ident(&'src str)`, `Token::IntLit(&'src str)`, `Token::KwFn`, `Token::LBrace`
- Pattern: Logos enum with custom callbacks for complex patterns (nested comments, raw strings, formattable strings)

**Spanned<T>:**
- Purpose: Wraps any value with its byte-offset span in source
- Examples: `(Expr, SimpleSpan)`, `(TypeExpr, SimpleSpan)`, `(Stmt, SimpleSpan)`
- Pattern: Type alias `pub type Spanned<T> = (T, SimpleSpan);` enabling tuple destructuring

**Trivia:**
- Purpose: Captures whitespace and comments for lossless roundtrip
- Variants: `Whitespace(String)`, `LineComment(String)`, `BlockComment(String)`
- Pattern: Roslyn-style leading/trailing attachment—trailing trivia on same line attaches to preceding token, leading trivia attaches to following token

**Item<'src>:**
- Purpose: Top-level program constructs (declarations, imports, statements)
- Variants: Namespace, Using, Fn, Dlg, Struct, Enum, Contract, Impl, Entity, Component, Extern, Const, Global, Stmt
- Pattern: Enum dispatching to specialized declaration types (FnDecl, StructDecl, DlgDecl, etc.)

**Expr<'src>:**
- Purpose: Expressions in Writ language—all value-producing constructs
- Variants: 40+ including IntLit, BoolLit, Ident, Binary, UnaryPrefix, UnaryPostfix, MemberAccess, BracketAccess, Call, GenericCall, If, IfLet, Match, Block, Lambda, Spawn, Detached, Join, Cancel, Defer, Try, FormattableString, ArrayLit, StructLit, Error
- Pattern: Recursive enum with Box<Spanned<Expr>> for sub-expressions, direct nesting for literals/identifiers

**Stmt<'src>:**
- Purpose: Statements—declarations and effects within blocks
- Variants: Let, Expr, For, While, Break, Continue, Return, Atomic, DlgDecl, Transition, Error
- Pattern: Enum with nested Spanned<Expr> and Spanned<Stmt> for recursion

**DlgLine<'src>:**
- Purpose: Lines within a dialogue block
- Variants: SpeakerLine, SpeakerTag, TextLine, CodeEscape, Choice, Condition, OnState, OnEvent, Label, Transition
- Pattern: Specialized enum for dialogue-specific grammar, embeds DlgTextSegment for speaker/text combinations

## Entry Points

**parse():**
- Location: `writ-parser/src/parser.rs:2908`
- Triggers: Called by CLI, compiler, or any consumer needing CST from source string
- Responsibilities: Orchestrate lexing, filter trivia, parse with recovery, return output and error list

**lex():**
- Location: `writ-parser/src/lexer.rs:445`
- Triggers: Called directly for tokenization, or indirectly via parse()
- Responsibilities: Scan source string, dispatch Logos lexer rules, produce token + span pairs

**program_parser():**
- Location: `writ-parser/src/parser.rs:2749` (recursive function inside module)
- Triggers: Internal—called by parse() to create parser combinator
- Responsibilities: Build mutually recursive parser for declarations and statements

## Error Handling

**Strategy:** Resilient parsing with error recovery—parse as much valid code as possible, collecting errors for reporting

**Patterns:**

- **Chumsky Rich Errors:** Each parse failure produces Rich<'static, Token, Span> containing span, context, expected tokens
- **Recovery Delimiters:** Balanced brace/paren/bracket tracking prevents recovery from skipping important closers
- **Placeholder Nodes:** Invalid expressions/statements replaced with Expr::Error / Stmt::Error maintaining CST structure
- **Error Accumulation:** All errors collected in Vec, not thrown early, enabling multi-error reporting
- **Span Preservation:** Even error nodes carry their source span for accurate diagnostics

## Cross-Cutting Concerns

**Logging:** Not implemented in Phase 1; parser operates silently, errors returned to caller

**Validation:** Type checking, name resolution deferred to compiler phase; parser only validates syntax structure

**Authentication:** Not applicable

**Lifetimes:** Parser extensively uses 'src lifetime to reference source string directly, avoiding allocations for identifiers and string content

---

*Architecture analysis: 2026-02-26*
