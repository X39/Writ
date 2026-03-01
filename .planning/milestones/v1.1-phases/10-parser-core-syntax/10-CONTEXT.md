# Phase 10: Parser — Core Syntax - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Parse 6 new syntax features into the CST: `new` keyword construction, hex/binary integer literal atoms, struct lifecycle hooks (`on create/finalize/serialize/deserialize`), `self`/`mut self` parameters, bit-shift operators (`<<`/`>>`), and `BitAnd`/`BitOr` in the operator overloading `OpSymbol` enum. The old brace-construction syntax (`Type { field: value }` without `new`) is rejected with a helpful error.

</domain>

<decisions>
## Implementation Decisions

### Struct body layout
- Fields and lifecycle hooks freely interleave in any order — no fields-first requirement
- Duplicate lifecycle hooks (e.g., two `on create` blocks) are NOT parse errors; defer duplicate detection to semantic analysis
- Universal lifecycle hooks (`on create`, `on finalize`, `on serialize`, `on deserialize`) are parsed in BOTH struct bodies and entity bodies (entities already have `on destroy`/`on interact` — extend to accept the four universal hooks too)
- Block required always: `on create { ... }` must have braces; `on create;` (empty shorthand) is a parse error

### Construction syntax (`new`)
- `new Type { field: value }` is the only valid construction syntax
- Trailing commas allowed: `new Type { name: "Tim", gold: 100, }` is valid
- Empty braces allowed: `new Type {}` is valid (all fields must have defaults — enforced semantically, not by parser)
- Supports generics: `new Type<T> { ... }` per the EBNF grammar
- Supports rooted paths: `new ::module::Type { ... }` per the EBNF grammar
- Spread/rest syntax (`..base`) is NOT supported now — noted as a potential future enhancement; keep CST extensible

### Migration error messaging
- When parser detects `Ident { Ident:` pattern (old construction without `new`), emit a targeted error: "Construction requires `new` keyword" with a suggestion to add `new`
- This is a parser-level diagnostic, not just a generic "unexpected `{`" error
- Better DX for users migrating from languages without explicit `new`

### Claude's Discretion
- Self parameter (`self`/`mut self`) CST representation and parser enforcement (position rules, free-function handling)
- Shift operator (`<<`/`>>`) tokenization strategy and generic disambiguation
- Hex/binary literal integration into expression atoms (straightforward — tokens already exist)
- `BitAnd`/`BitOr` addition to `OpSymbol` enum (straightforward)
- Near-miss error hints beyond the main `Ident { field: }` case (e.g., `new { ... }` missing type)

</decisions>

<specifics>
## Specific Ideas

- Migration error should feel helpful, not scolding — guide the user to the correct syntax
- Struct body interleaving lets authors group related fields near their hooks (e.g., `handle: int` near `on create` that initializes it)
- Spread syntax (`..base`) for construction is a good future idea — keep CST `Expr::New` field list extensible for it

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Token::HexLit` and `Token::BinLit`: Already tokenized by lexer — just need atom parsing in `parser.rs`
- `Token::KwSelf` and `Token::KwOn`: Keyword tokens exist in lexer
- `BinaryOp::BitAnd` and `BinaryOp::BitOr`: Already in the `BinaryOp` enum for binary expression parsing
- `Expr::Path` with `rooted: bool`: Supports `::module::Type` paths needed for `new` construction
- `EntityMember` enum: Already has `OnHandler` variant with `on` keyword parsing — pattern to follow for struct hooks

### Established Patterns
- Parser combinators (chumsky): All parsing uses chumsky combinators with Pratt parsing for precedence
- CST spanned nodes: Every node uses `Spanned<T>` wrapper for source locations
- Error recovery: Uses `nested_delimiters` and `skip_then_retry_until` for graceful failure
- `Param` struct: Currently `{ name, ty }` — needs expansion or new variant for `self`/`mut self`

### Integration Points
- `StructDecl.fields`: Currently `Vec<Spanned<StructField>>` — needs to become mixed-member (fields + hooks)
- `OpSymbol` enum: Needs `BitAnd` and `BitOr` variants for operator overloading parser
- Precedence table in `parser.rs`: Pratt parser needs new precedence levels for `<<`, `>>`, `&`, `|` (spec: shifts=5, &=6, |=7)
- `Expr` enum: Needs new `New` variant for construction expressions

</code_context>

<deferred>
## Deferred Ideas

- Spread/rest syntax for construction (`new Type { ..base, field: value }`) — potential future phase
- Compound assignment for bitwise ops (`&=`, `|=`, `<<=`, `>>=`) — check if spec includes these

</deferred>

---

*Phase: 10-parser-core-syntax*
*Context gathered: 2026-03-01*
