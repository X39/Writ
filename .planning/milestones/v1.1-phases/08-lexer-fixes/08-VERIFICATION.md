---
phase: 08-lexer-fixes
status: passed
verified: 2026-03-01
verifier: orchestrator
score: 5/5
---

# Phase 8: Lexer Fixes — Verification

## Phase Goal
The lexer correctly validates raw string delimiters, strips leading whitespace dedentation from raw strings, handles unicode escapes inside formattable strings, and rejects invalid escape sequences.

## Requirement Coverage

All 5 phase requirements accounted for:

| Requirement | Plan | Status | Evidence |
|-------------|------|--------|----------|
| LEX-01 | 08-01 | Verified | `raw_string_opening_not_followed_by_newline_is_error` test passes; `raw_string()` callback returns false when no newline follows opening |
| LEX-02 | 08-01 | Verified | `raw_string_closing_not_on_own_line_is_error` test passes; closing delimiter validation scans backwards for whitespace-only prefix |
| LEX-03 | 08-02 | Verified | `dedent_raw_string()` utility strips common prefix, excludes blank lines, strips structural lines. 8 integration tests + 6 unit tests pass |
| LEX-04 | 08-01 | Verified | `formattable_string_unicode_escape_not_interpolation` test passes; `\u{` pattern consumed without incrementing brace_depth |
| LEX-05 | 08-02 | Verified | `process_escapes()` utility rejects `\q`, `\p` with `EscapeError::InvalidEscape`. 11 integration tests + 7 unit tests pass |

## Success Criteria Verification

### 1. Opening `"""` not followed by newline produces error
**Status: PASS**
- `raw_string()` callback in `lexer.rs` checks `bytes[search_start]` after counting extra quotes; returns `false` if not `\n` or `\r\n`
- `formattable_raw_string()` has identical check
- Tests: `raw_string_opening_not_followed_by_newline_is_error`, `raw_string_content_on_opening_line_is_error`, `formattable_raw_string_invalid_opening_is_error`

### 2. Closing `"""` not on its own line produces error
**Status: PASS**
- Both callbacks scan backwards from closing quotes to preceding newline, verify all bytes are space/tab/CR
- Tests: `raw_string_closing_not_on_own_line_is_error`, `formattable_raw_string_invalid_closing_is_error`
- Positive cases: `raw_string_closing_with_only_whitespace_before` (spaces before closing = OK)

### 3. Raw string common leading whitespace prefix stripped
**Status: PASS (via post-lex utility)**
- `dedent_raw_string()` in `string_utils.rs` implements Rust-style dedentation
- Minimum indentation of non-blank lines computed as common prefix
- Structural first/last lines stripped; blank lines excluded from prefix but preserved as empty
- Per 08-CONTEXT.md decisions: token carries verbatim source, utility processes it
- Tests: 8 integration tests (dedent_simple_block through dedent_empty_content) + 6 unit tests

### 4. `\u{XXXX}` treated as unicode escape, not interpolation
**Status: PASS**
- `formattable_string()` callback detects `\u{` pattern and skips past closing `}` without incrementing `brace_depth`
- Tests: `formattable_string_unicode_escape_not_interpolation`, `formattable_string_unicode_escape_with_real_interpolation`

### 5. Invalid escape sequences rejected
**Status: PASS (via post-lex utility)**
- `process_escapes()` in `string_utils.rs` returns `Err(EscapeError::InvalidEscape(c))` for unrecognized escapes
- Per 08-CONTEXT.md decisions: escape validation is a post-lex utility (string token produced, validation done separately)
- Tests: `escapes_invalid_escape_q`, `escapes_invalid_escape_p`, `escapes_trailing_backslash`

## Design Note

Success criteria 3 and 5 are implemented as standalone utility functions rather than inline in the lexer, per the design decisions documented in `08-CONTEXT.md`:
- Raw string tokens carry original verbatim source (CST lossless roundtrip guarantee)
- Dedentation applied via `dedent_raw_string()` callable by parser/lowering/tests
- Escape validation via `process_escapes()` callable by any consumer
- Both are pub re-exported from `writ_parser` crate root

This approach preserves the CST lossless roundtrip property while providing the required validation functionality.

## Test Summary

| Category | Count | Status |
|----------|-------|--------|
| Lexer tests (lexer_tests.rs) | 74 | All pass |
| Parser tests (parser_tests.rs) | 177 | All pass |
| Unit tests (string_utils) | 13 | All pass |
| Doc tests | 2 | All pass |
| Compiler tests | 69 | All pass |
| **Total** | **335** | **All pass** |

## Files Modified

| File | Changes |
|------|---------|
| `writ-parser/src/lexer.rs` | Delimiter validation in raw_string + formattable_raw_string; unicode escape handling in formattable_string |
| `writ-parser/src/string_utils.rs` | New module: dedent_raw_string(), process_escapes(), EscapeError |
| `writ-parser/src/lib.rs` | Added string_utils module and re-exports |
| `writ-parser/tests/lexer_tests.rs` | 32 new tests (13 delimiter validation + 19 string utils) |
| `writ-parser/tests/parser_tests.rs` | 1 test updated (raw_string_basic to valid format) |

---
*Verified: 2026-03-01*
