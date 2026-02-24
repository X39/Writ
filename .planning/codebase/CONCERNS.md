# Codebase Concerns

**Analysis Date:** 2026-02-26

## Tech Debt

### Unsafe unwrap() in Pattern Parsing
- **Issue:** Single-element vector unwrap without safety check in pattern parsing
- **Files:** `writ-parser/src/parser.rs:294`
- **Code:** `.map_with(|pats: Vec<...>, e| { if pats.len() == 1 { pats.into_iter().next().unwrap() } ... })`
- **Impact:** Parser will panic if logic error occurs in pattern collection (len check == 1 but next() returns None). Should use `expect()` with context message or direct indexing for clarity.
- **Fix approach:** Replace with `pats[0].clone()` or `expect("pattern vec guaranteed non-empty")`

### Error Suppression in parse_expr_from_source
- **Issue:** Errors silently discarded when parsing string interpolation expressions
- **Files:** `writ-parser/src/parser.rs:596-606`
- **Code:** `let (output, _errors) = program_parser(expr_src).parse(token_stream).into_output_errors();` then fallback `Some((cst::Expr::NullLit, span))`
- **Impact:** Malformed expressions inside `{}` interpolation and `{}` dialogue segments are replaced with `null` rather than propagated as diagnostics. Users see incorrect AST instead of error messages.
- **Fix approach:** Collect and return errors alongside parsed expressions, or at minimum log to stderr. Requires changes to return type of `parse_expr_from_source` and callers (`parse_formattable_string`, `split_dlg_text_segments`).

### Redundant String Parsing in Interpolations
- **Issue:** String interpolation expressions are re-lexed and re-parsed as complete programs
- **Files:** `writ-parser/src/parser.rs:567-607` (`parse_expr_from_source`)
- **Impact:** Full parser overhead (recursion, item collection, error recovery) for single expressions. Inefficient for deeply nested formattable strings or dialogues with heavy interpolation.
- **Fix approach:** Extract expression-only parser branch to avoid item parsing wrapper. Requires factoring out expression parser entry point separate from `program_parser()`.

### Limited Byte Offset Validation
- **Issue:** Formattable string parsing uses byte offset arithmetic without explicit bounds checking in critical paths
- **Files:** `writ-parser/src/parser.rs:307-415` (parse_formattable_string), `431-560` (split_dlg_text_segments)
- **Impact:** Off-by-one errors or slice panics possible if UTF-8 boundaries crossed incorrectly (though unlikely with current ASCII-centric token handling). Expressions spans calculated via arithmetic without validation.
- **Fix approach:** Add comprehensive bounds assertions at span boundary calculations. Document UTF-8 assumptions.

## Known Bugs

### Formattable String Span Calculation for Nested Expressions
- **Symptoms:** Expression spans inside `{}` interpolations may have inaccurate positions
- **Files:** `writ-parser/src/parser.rs:374-390`
- **Trigger:** Complex nested expressions with multiple levels of braces: `$"outer {inner {x + y}}"`
- **Workaround:** Currently works due to the depth-tracking loop, but the `expr_src_offset` calculation (line 379) compounding offsets is fragile
- **Details:** The offset `expr_src_offset = content_start + expr_start` (line 371) is correctly applied, but the re-parsing via `parse_expr_from_source` operates on substring offsets. If that substring contains further expressions (nested), their internal spans are relative to that substring, not the source. Adjustment at line 378-380 attempts to fix this but assumes no further re-parsing.

### Formattable Raw String Quote Matching
- **Symptoms:** Edge case where raw string has more than 6 quotes may not parse correctly
- **Files:** `writ-parser/src/lexer.rs:115-145` (formattable_raw_string)
- **Trigger:** `$""""""" ... """""""` (7+ quotes) — the lexer counts extra quotes correctly, but closing delimiter matching is exact, leaving edge cases with unbalanced quote counts
- **Current mitigation:** Tests likely don't cover this case (575 lexer tests)

## Security Considerations

### No Input Size Limits
- **Risk:** Parser has no maximum recursion depth, token count, or total input size limits. Pathological inputs could cause stack overflow or DoS.
- **Files:** `writ-parser/src/parser.rs:2708-2929` (recursive parser structure)
- **Current mitigation:** Chumsky parser has internal limits (default 128 parser stack depth), but not enforced or documented
- **Recommendations:**
  - Add explicit checks in `parse()` entry point for input size > 10MB (configurable)
  - Document maximum nesting depth (typically ~100 levels for game scripts)
  - Consider iterative parser or depth guards for deeply nested structures

### String Interpolation Code Injection Path
- **Risk:** Expressions inside `{}` and dialogue `{}` are re-parsed and evaluated. No static validation that expressions are safe (though CST doesn't evaluate at parse time).
- **Files:** `writ-parser/src/parser.rs:567-607`, `417-560`
- **Current mitigation:** Lexer/parser only validates syntax, not semantics. Runtime enforces access control.
- **Recommendations:**
  - Document that interpolation expressions undergo full parsing and semantic checks later
  - Consider compile-time validation for known-dangerous patterns (e.g., system calls without capabilities)

## Performance Bottlenecks

### Repeated Lexing for Interpolation Expressions
- **Problem:** Each `{}` expression in formattable strings and dialogue text triggers a full `lex()` call
- **Files:** `writ-parser/src/parser.rs:574` (in `parse_expr_from_source`)
- **Cause:** Direct call to `crate::lexer::lex(expr_src)` for every interpolation segment
- **Scaling:** O(n) string interpolations = O(n) lexing passes. Game scripts with 50+ interpolations per file will re-lex same patterns repeatedly.
- **Improvement path:** Cache lexer token stream for repeated expressions, or provide streaming lexer interface that resumes from position rather than re-lexing substring

### Parser Combines Multiple Concerns
- **Problem:** `program_parser()` handles items, declarations, statements, and expressions in single recursive structure
- **Files:** `writ-parser/src/parser.rs:2708-2894`
- **Cause:** Chumsky's recursive combinator requires mutual recursion through single `recursive()` call; no separation of concerns
- **Impact:** Even simple expression parsing (e.g., in interpolations) triggers full statement/item parser machinery. Stack frame overhead.
- **Improvement path:** Extract expression-only parser into separate entry point for use in interpolation contexts

## Fragile Areas

### Pattern Collection with length check
- **Files:** `writ-parser/src/parser.rs:287-299`
- **Why fragile:** Code assumes `pats.len() == 1` check is correct but calls `.next().unwrap()` on iterator. If collection logic changes, unwrap will panic.
- **Safe modification:** Replace with direct indexing or explicit match/expect
- **Test coverage:** `parser_tests.rs` has tests for patterns but may not cover single-element or-patterns specifically

### Formattable String Offset Arithmetic
- **Files:** `writ-parser/src/parser.rs:307-415`, `431-560`
- **Why fragile:** Multiple layers of offset calculation (token span → content → text segments → expression offsets). Easy to introduce off-by-one errors.
- **Safe modification:** Add comprehensive unit tests for edge cases:
  - Empty strings `$""`
  - Only interpolation `${x}`
  - Escaped braces `$"{{x}}"`
  - Nested expressions `$"{f(g{x})}"`
- **Test coverage:** Parser tests exist but formattable string edge cases not exhaustively covered (parser_tests.rs lines may need inspection)

### Dialogue Text Parsing with Continuation
- **Files:** `writ-parser/src/parser.rs:431-560` (`split_dlg_text_segments`)
- **Why fragile:** Line continuation handling (`\ ` followed by newline) introduces special span tracking (line 539-542). The space span calculation relies on correct position tracking through multiple state transitions.
- **Safe modification:** Add explicit tests for:
  - Continuation at start of dialogue `"\ \n text"`
  - Continuation before interpolation `"text \ \n {expr}"`
  - Windows line endings `\ \r\n`
- **Test coverage:** DLG-03 tests cover continuation but manual verification needed for edge cases

### Generic Parameter Default Unwrapping
- **Files:** `writ-parser/src/parser.rs:178`, `265`, `2163`
- **Why fragile:** Multiple `unwrap_or_default()` calls assume empty defaults are acceptable
- **Pattern:** `bounds: bounds.unwrap_or_default()` — assumes None → empty bounds Vec is correct semantics
- **Safe modification:** Verify that None semantics are truly equivalent to empty Vec for bounds, params, and args

## Scaling Limits

### Parser Input Size
- **Current capacity:** Tested up to ~50KB source files (typical game script)
- **Limit:** No hard limit enforced; pathological O(n²) inputs could degrade performance beyond ~1MB
- **Cause:** Chumsky parser has no streaming mode; entire source must be tokenized then parsed in memory
- **Scaling path:** For large files (>10MB), consider breaking into modules/chunks. Chumsky handles streaming tokens, but current `lex()` returns `Vec<Token>`.

### Nesting Depth
- **Current capacity:** ~20+ levels of nesting (match arms with nested if/blocks/lambdas)
- **Limit:** Chumsky recursion stack ~128 levels (default), runtime stack ~2MB
- **Scaling path:** For highly nested structures, iterative parser or explicit depth limits. Game scripts rarely exceed 10 levels in practice.

### Interpolation Count
- **Current capacity:** ~100 interpolations per file (each triggers lexing + parsing)
- **Limit:** No practical limit but O(n) relex per interpolation
- **Scaling path:** Cache lexer results or use streaming lexer

## Dependencies at Risk

### Chumsky Parser Combinator Library
- **Risk:** Nightly-only feature (`0.12.0`) with sparse documentation. Version `0.13.x` in development may introduce breaking changes.
- **Impact:** Parser API deeply coupled to chumsky's `recursive()`, `pratt()`, `recover_with()` combinators. Upgrade requires full rewrite.
- **Migration plan:** Monitor chumsky releases; consider vendoring critical parser patterns (recursive descent, Pratt) if upstream abandons the crate.

### Logos Lexer
- **Risk:** Stable but minimal maintenance. No streaming mode; requires full source in memory.
- **Impact:** Large source files must be fully lexed upfront. No ability to lex-on-demand.
- **Migration plan:** For streaming, switch to `rustlex` or hand-written lexer. For now, acceptable for game scripts.

## Missing Critical Features

### Lossless Round-Trip
- **Problem:** CST preserves trivia (comments/whitespace) but doesn't preserve exact formatting (e.g., indentation style)
- **Blocks:** Pretty-printer/formatter implementations must guess style or pass through
- **Impact:** Game editor or code generation tools can't guarantee identical formatting round-trip

### Error Recovery Diagnostics
- **Problem:** Parse errors are reported but recovery is opaque. Users don't see which productions attempted or why recovery succeeded/failed
- **Blocks:** IDE error messages and debugging
- **Impact:** Cryptic error messages like "expected token X" without context of what the parser was trying to parse

### Streaming Parser
- **Problem:** Entire source must fit in memory. No ability to parse top-level declarations as they arrive.
- **Blocks:** Real-time editor scenarios, very large codebases
- **Impact:** Editor latency or memory bloat for large projects

## Test Coverage Gaps

### Untested Pattern Forms
- **What's not tested:** Single-element or-patterns (the `.unwrap()` case at line 294)
- **Files:** `writ-parser/tests/parser_tests.rs`
- **Risk:** Unwrap will panic if collector produces single-element vector with None on iteration (logic error)
- **Priority:** High

### Formattable String Edge Cases
- **What's not tested:**
  - Empty strings `$""`
  - Only whitespace `$"   "`
  - Consecutive interpolations `${x}${y}`
  - Escaped braces at boundaries `$"{{x}}"`
  - Very deeply nested expressions `${f(g(h(x)))}`
- **Files:** `writ-parser/tests/parser_tests.rs` (search for FormattableString tests)
- **Risk:** Off-by-one in span calculation, missed interpolations
- **Priority:** High

### Raw String Quote Matching
- **What's not tested:** Quote counts > 6, unbalanced closing quotes
- **Files:** `writ-parser/tests/lexer_tests.rs`
- **Risk:** Lexer hangs or incorrectly closes string
- **Priority:** Medium

### Error Propagation in Interpolations
- **What's not tested:** Malformed expressions in `{}` — currently fall back to `NullLit` with no error
- **Files:** `writ-parser/tests/parser_tests.rs`
- **Risk:** Silent failures; incorrect AST silently produced
- **Priority:** High

### Dialogue Line Continuation
- **What's not tested:** Edge cases with `\ ` at boundaries:
  - Start of dialogue `"\ \n rest"`
  - Before interpolation `"text\ \n {expr}"`
  - Windows line endings
- **Files:** `writ-parser/tests/parser_tests.rs` (DLG tests)
- **Risk:** Span calculation errors, incorrect continuation handling
- **Priority:** Medium

### Recovery Mode Scenarios
- **What's not tested:** Parser recovery when declarations or expressions are malformed
- **Files:** `writ-parser/tests/parser_tests.rs` (no recovery tests found)
- **Risk:** Unknown behavior when parsing broken code; unclear what AST is produced
- **Priority:** Medium

---

*Concerns audit: 2026-02-26*
