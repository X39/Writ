# Phase 8: Lexer Fixes - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the lexer correctly validate raw string delimiters (opening must be followed by newline, closing must be on its own line), strip leading whitespace dedentation from raw strings, handle `\u{XXXX}` unicode escapes inside formattable strings without triggering interpolation, and reject invalid escape sequences. Covers LEX-01 through LEX-05.

</domain>

<decisions>
## Implementation Decisions

### Dedentation rules
- Common whitespace prefix computed from **content lines only** (minimum indentation among non-blank lines)
- Blank/whitespace-only lines are **excluded** from prefix calculation
- Mixed tabs and spaces handled via **character-by-character match** — tab and space are different characters; common prefix is the longest character-identical prefix
- Closing delimiter line's indentation does NOT set the dedent level — content lines determine it
- First line (after opening `"""`) and closing delimiter line are **stripped** — they are structural, not content

### Error diagnostics
- **Specific error variants** for each failure type — "opening `"""` must be followed by newline", "closing `"""` must be on its own line", "invalid escape `\q`"
- Raw string **structural failures** (no newline after opening, closing not on own line) produce an **Error token**
- Invalid escape sequences: the string token is **produced successfully**, invalid escapes left as-is in content — a **post-lex validation pass** scans string content and reports errors
- This split: structural failures = error tokens (string is fundamentally broken), escape errors = flag separately (string is parseable, escape is wrong)

### Unicode escape handling
- Lexer validates `\u{...}` contains **1-6 hex digits** (syntax check only)
- `\u{}` (empty, zero digits) is **rejected** at lex time as syntactically malformed
- Codepoint **range validation** (0-10FFFF, no surrogates) **deferred** to a semantic pass
- Only `\u{` triggers the special pattern — bare `\{` is NOT treated as suppressing interpolation; `\{` is an invalid escape

### Token content and processing
- Raw string tokens carry **original verbatim source** — spans cover exact source bytes including delimiters and all whitespace
- CST **lossless roundtrip** guarantee preserved — `src[span]` always matches source text
- Dedentation applied via a **standalone utility function** (`dedent_raw_string`) in the lexer module — callable by parser, lowering, tests, or any consumer
- Escape processing also gets a **standalone utility function** (`process_escapes`) — validates AND transforms in a single pass (resolves `\n` to actual newline, etc.)
- Both utilities are independently testable and reusable

### Claude's Discretion
- Internal implementation of the logos callbacks vs regex patterns
- Error message exact wording
- Test organization and coverage strategy
- Whether dedent/escape utilities go in `lexer.rs` or a separate `string_utils.rs` module

</decisions>

<specifics>
## Specific Ideas

- Dedentation algorithm follows Rust-style: minimum indentation of content lines, not closing delimiter position
- Error token approach mirrors logos convention — callbacks return `false` for structural failures
- The escape processing utility is a "two birds, one stone" function: both validates and produces the resolved runtime string value
- Post-lex validation for escapes keeps `lex()` signature unchanged (`Vec<(Token, Span)>`)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `raw_string` callback (`lexer.rs:32-72`): Scans for N-quote matching, needs validation logic added for newline/own-line rules
- `formattable_string` callback (`lexer.rs:77-110`): Tracks brace depth, needs `\u{` detection to avoid treating `{` as interpolation
- `formattable_raw_string` callback (`lexer.rs:115-146`): Same N-quote matching as raw_string, same `\u{` fix needed
- Existing test infrastructure (`lexer_tests.rs`): Roundtrip tests, error token tests, token-type assertions — all patterns to follow
- Test case files (`tests/cases/*.writ`): Numbered convention, `02_string_literals.writ` already covers basic strings

### Established Patterns
- Logos callbacks return `bool` (true = matched, false = error) — structural failures naturally map to `false`
- Token enum uses `&'src str` payloads for content tokens (`StringLit`, `IntLit`, etc.)
- `RawStringLit` and `FormattableRawStringLit` are unit variants (no payload) — span covers the source
- Basic string uses regex pattern `r#""([^"\\]|\\.)*""#` — currently accepts any `\.` escape
- `lex()` returns `Vec<(Token, SimpleSpan)>` — no secondary error channel

### Integration Points
- `lex()` function is the public API — signature should remain `fn lex(src: &str) -> Vec<(Token, SimpleSpan)>`
- Parser consumes token stream via chumsky — raw string tokens flow through unchanged
- CST `Expr::Literal` nodes reference string tokens — dedent/escape utilities will be called during lowering
- Test case files used by both lexer and parser tests

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-lexer-fixes*
*Context gathered: 2026-03-01*
