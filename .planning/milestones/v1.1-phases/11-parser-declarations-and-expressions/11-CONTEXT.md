# Phase 11: Parser — Declarations and Expressions - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Enforce all remaining v0.4 declaration and expression rules in the parser: impl generics, bodyless operator signatures, component-declaration errors, extern qualified names, extern visibility, contextual caret, spawn-detached, defer-block-only, and the attribute separator fix. Nine requirements (TYPE-03, DECL-03, DECL-05, DECL-06, DECL-07, EXPR-03, EXPR-04, EXPR-05, MISC-02).

</domain>

<decisions>
## Implementation Decisions

### Error message design
- All new restrictions produce **hard parse errors** (not warnings)
- Errors are **specific with fix suggestions** — name the violation and suggest the correct syntax
- Migration-aware errors for changed syntax: attribute separator `:` → `=` gets a specific message ("attribute arguments use `=` not `:` — change `key: value` to `key = value`")
- Non-extern `component` error should suggest `extern component`
- `defer` without block should say "defer requires a block body `defer { ... }` — single expressions are not allowed"
- Parser uses **error recovery** (chumsky's existing patterns) to continue parsing after each restriction violation — report all errors in one pass

### Caret `^` scope rules
- `^` (from-end indexing) is **only valid inside `[]` bracket-access context**
- Validation enforced at **parse time** — `^expr` outside `[]` produces a parse error immediately, no invalid CST nodes
- **Shallow validation**: only direct `[]` index position and range operands within `[]` are valid contexts (e.g., `arr[^1]`, `arr[^3..^1]`)
- Nested usage like `arr[func(^1)]` is **not valid** — `^` cannot appear inside a function call even if that call is within brackets
- Error message: "The `^` (from-end) operator is only valid inside `[]` indexing — e.g., `arr[^1]` or `arr[^2..^1]`"

### spawn detached semantics
- `spawn detached expr` parses as a **fused `Expr::SpawnDetached(expr)` CST node** — not nested Spawn + Detached
- **Remove standalone `Expr::Detached`** entirely — `detached expr` without `spawn` prefix is a parse error
- The `detached` keyword **remains reserved** — it is not a valid identifier even though it's no longer a standalone expression keyword
- This matches the IL's `SPAWN_DETACHED` instruction 1:1

### Extern dotted paths
- `extern fn Entity.getOrCreate<T>()` uses **single-level dotted names only** — exactly `Type.method`, one dot, two segments
- Multi-level `A.B.C()` is not supported; namespace paths use `::` not `.`
- CST representation: **separate qualifier + name** — add an optional `qualifier: Option<Spanned<&str>>` to the extern fn signature (e.g., qualifier=`Entity`, name=`getOrCreate`)
- Visibility goes **before extern**: `pub extern fn foo()` — consistent with `pub fn`, `pub struct`, `pub entity` patterns. `extern pub fn` is a parse error
- Visibility applies to **all extern declaration types**: `pub extern fn`, `pub extern struct`, `pub extern component`

### Claude's Discretion
- Exact chumsky combinator structure for each new restriction
- How to thread bracket-context flag through the expression parser for caret validation
- Error recovery strategy details (which chumsky recovery combinators to use per restriction)
- Whether to add `qualifier` field to existing `FnSig` or create a new `ExternFnSig` type
- Bodyless operator signature CST representation (new variant vs reuse of OpDecl with optional body)
- ImplDecl generic parameter list placement in the CST

</decisions>

<specifics>
## Specific Ideas

- Error messages should include examples of the correct syntax wherever possible (e.g., "e.g., `arr[^1]` or `arr[^2..^1]`")
- The pattern for all 5 new restrictions: detect the invalid form, produce a contextual error with suggestion, and recover to continue parsing
- `spawn detached` should feel like a single compound keyword to users, not two separate keywords composed together

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `chumsky::recovery::{nested_delimiters, skip_then_retry_until}`: Already imported and used for error recovery throughout the parser
- `ImplDecl` struct (cst.rs:346): Exists but lacks generic parameters — needs a `generics` field added
- `ExternDecl` enum (cst.rs:433-442): Already handles `Fn`, `Struct`, `Component` variants — needs visibility field
- `OpDecl` struct (cst.rs:330-342): Has `symbol`, `params`, `return_type`, `body` — body could become `Option` for bodyless sigs
- `ComponentDecl` struct (cst.rs:411-422): Exists for both standalone and extern components — parser needs to reject the non-extern path

### Established Patterns
- Visibility parsing: `visibility` parser used for `fn`, `struct`, `enum`, `entity`, `contract`, `component` — already composable, can be prepended to extern declarations
- Keyword-prefixed expressions: `spawn`, `join`, `cancel`, `defer` all follow the `just(Token::Kw*).ignore_then(expr)` pattern — `spawn detached` extends this with a two-keyword prefix
- `Expr::Spawn` and `Expr::Detached` are currently separate variants (cst.rs:579-582) — both will be replaced by fused `SpawnDetached`
- `Expr::Defer` currently accepts any expression (cst.rs:587-588) — needs restriction to block-only
- Attribute parsing uses `:` separator currently — needs migration to `=`

### Integration Points
- `writ-parser/src/cst.rs`: CST type additions (ImplDecl generics, SpawnDetached, extern qualifier, extern visibility)
- `writ-parser/src/parser.rs`: Parser combinator changes (~8 distinct areas)
- `writ-parser/tests/parser_tests.rs`: New test cases for each requirement
- `writ-parser/tests/cases/`: May need updates to existing test files (10_operators.writ, 18_extern.writ, 14_attributes.writ)
- `writ-compiler/src/lower/`: Lowering pass may need updates for new CST nodes (SpawnDetached, bodyless ops, etc.)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 11-parser-declarations-and-expressions*
*Context gathered: 2026-03-01*
