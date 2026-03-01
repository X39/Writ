# Phase 9: CST Type System Additions - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

CST type definitions carry all fields required by spec v0.4 — multi-segment qualified paths in TypeExpr, a rooted-path flag on Expr::Path (and TypeExpr::Qualified), attrs/vis on DlgDecl, and the dead Stmt::DlgDecl variant is removed. Requirements: TYPE-01, TYPE-02, DECL-04, MISC-03.

</domain>

<decisions>
## Implementation Decisions

### TypeExpr path model
- Add a new `TypeExpr::Qualified` variant alongside `Named` — single-segment types stay as `Named(&'src str)`, multi-segment paths use `Qualified`
- `Qualified` is a struct variant: `Qualified { segments: Vec<Spanned<&'src str>>, rooted: bool }`
- `rooted` is true when the type path begins with `::` (e.g., `::std::collections::Map`)
- `TypeExpr::Generic` composes naturally — a qualified generic like `a::b::List<T>` is `Generic(Qualified(...), [T])` with no extra work
- Parser strategy: collect `ident (:: ident)*` segments, then emit `Named` if 1 segment, `Qualified` if 2+

### Rooted path scope — Expr::Path
- Convert `Expr::Path` from tuple variant to struct variant: `Path { segments: Vec<Spanned<&'src str>>, rooted: bool }`
- `rooted` is true when the path begins with `::` (e.g., `::module::func`)
- Rooted single-segment (`::Foo`) promotes to `Expr::Path { segments: ["Foo"], rooted: true }` — does NOT stay as `Expr::Ident`
- `Expr::Ident` remains for bare unqualified names; `Expr::Path` for multi-segment or rooted
- Current Ident/Path split preserved — no unnecessary unification

### Rooted path scope — patterns
- `Pattern::EnumDestructure` does NOT gain a rooted flag in this phase
- Rooted pattern support is deferred to a future phase when pattern-matching improvements are scoped

### Claude's Discretion
- DlgDecl attrs/vis field design — should mirror FnDecl/StructDecl pattern (`attrs: Vec<Spanned<Vec<Attribute>>>`, `vis: Option<Visibility>`)
- Stmt::DlgDecl removal approach — delete variant, fix all compilation errors, update tests
- Parser combinator structure for `::` prefix detection
- Test case organization and coverage approach

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Follow existing codebase patterns for CST types and parser combinators.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `FnDecl`, `StructDecl`, `EnumDecl` patterns: all use `attrs: Vec<Spanned<Vec<Attribute<'src>>>>` + `vis: Option<Visibility>` — DlgDecl should match
- `NamespaceDecl::Declarative` already parses `Vec<Spanned<&'src str>>` for `::` -separated paths — same pattern reusable
- `Token::ColonColon` exists in lexer for `::` tokenization

### Established Patterns
- CST types in `writ-parser/src/cst.rs` (~893 lines) — all enums derive `Debug, Clone, PartialEq`
- Parser combinators in `writ-parser/src/parser.rs` (~2929 lines) — chumsky-based with Pratt parsing
- Struct variants used in Expr for multi-field forms (If, IfLet, Match, Lambda) — Path should follow same pattern
- Tuple variants used for simple single-payload forms (IntLit, StringLit, Ident)

### Integration Points
- `type_expr()` parser function in parser.rs — needs qualified path support
- `atom()` parser in parser.rs — handles Ident/Path expression parsing
- `Item::Dlg` in item parser — needs attrs/vis prefix parsing
- All `match` arms on `Expr::Path` throughout parser_tests.rs — need struct destructuring update
- All `match` arms on `Stmt::DlgDecl` — need removal

</code_context>

<deferred>
## Deferred Ideas

- Rooted paths in Pattern::EnumDestructure — future phase when pattern-matching improvements are scoped

</deferred>

---

*Phase: 09-cst-type-system-additions*
*Context gathered: 2026-03-01*
