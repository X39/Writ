//! Standalone string processing utilities for raw string dedentation
//! and escape sequence validation/transformation.
//!
//! These functions are independently testable and reusable across the compiler
//! pipeline — callable from lexer, parser, lowering, tests, or any consumer.

use std::fmt;

/// Error type for escape sequence processing.
#[derive(Debug, Clone, PartialEq)]
pub enum EscapeError {
    /// An invalid escape character was found (e.g., `\q`, `\p`).
    InvalidEscape(char),
    /// `\u{}` with no digits.
    EmptyUnicodeEscape,
    /// `\u{...}` contains non-hex characters.
    InvalidUnicodeHex(String),
    /// `\u{...}` has more than 6 hex digits.
    UnicodeTooLong(String),
    /// Trailing backslash at end of string.
    TrailingBackslash,
}

impl fmt::Display for EscapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EscapeError::InvalidEscape(c) => write!(f, "invalid escape sequence: \\{}", c),
            EscapeError::EmptyUnicodeEscape => {
                write!(f, "empty unicode escape: \\u{{}} requires 1-6 hex digits")
            }
            EscapeError::InvalidUnicodeHex(s) => {
                write!(f, "invalid hex digits in unicode escape: \\u{{{}}}", s)
            }
            EscapeError::UnicodeTooLong(s) => {
                write!(
                    f,
                    "unicode escape too long (max 6 hex digits): \\u{{{}}}",
                    s
                )
            }
            EscapeError::TrailingBackslash => write!(f, "trailing backslash at end of string"),
        }
    }
}

/// Strip common leading whitespace from raw string content.
///
/// Given the full content between raw string delimiters (the text matched by
/// the lexer token, from the byte after the opening `"""...\n` through the byte
/// before the closing `\n..."""`), this function:
///
/// 1. Splits into lines
/// 2. Strips the first line (structural -- immediately after opening delimiter)
/// 3. Strips the last line (structural -- the closing delimiter line)
/// 4. Computes common whitespace prefix across non-blank content lines
/// 5. Strips that prefix from all lines (blank lines become empty)
/// 6. Joins with newlines
///
/// Blank/whitespace-only lines are excluded from prefix calculation but preserved
/// (as empty lines) in the output.
///
/// # Examples
///
/// ```
/// use writ_parser::string_utils::dedent_raw_string;
///
/// // Content between """\n and \n""" (structural lines included)
/// let content = "\n    hello\n    world\n    ";
/// assert_eq!(dedent_raw_string(content), "hello\nworld");
/// ```
pub fn dedent_raw_string(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();

    // Need at least 2 lines (first structural + last structural)
    if lines.len() <= 2 {
        return String::new();
    }

    // Strip first line (after opening delimiter) and last line (closing delimiter)
    let content_lines = &lines[1..lines.len() - 1];

    if content_lines.is_empty() {
        return String::new();
    }

    // Find common whitespace prefix among non-blank lines
    let common_prefix = compute_common_prefix(content_lines);

    // Strip prefix from all lines; blank lines become empty
    let result_lines: Vec<&str> = content_lines
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                // Blank/whitespace-only line -> empty in output
                ""
            } else if line.len() >= common_prefix {
                &line[common_prefix..]
            } else {
                line
            }
        })
        .collect();

    result_lines.join("\n")
}

/// Compute the length of the common leading whitespace prefix among non-blank lines.
///
/// Uses character-by-character matching: tab and space are different characters.
/// Only non-blank lines participate in prefix calculation.
fn compute_common_prefix(lines: &[&str]) -> usize {
    let non_blank_lines: Vec<&&str> = lines.iter().filter(|l| !l.trim().is_empty()).collect();

    if non_blank_lines.is_empty() {
        return 0;
    }

    // Start with the leading whitespace of the first non-blank line
    let first = non_blank_lines[0];
    let first_ws: usize = first
        .bytes()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count();

    let mut prefix_len = first_ws;

    // Narrow down by comparing with each subsequent non-blank line
    for line in &non_blank_lines[1..] {
        let line_ws: usize = line
            .bytes()
            .take_while(|b| *b == b' ' || *b == b'\t')
            .count();

        // The common prefix is the shorter of the two, then character-match
        let check_len = prefix_len.min(line_ws);
        let mut matching = 0;
        for i in 0..check_len {
            if first.as_bytes()[i] == line.as_bytes()[i] {
                matching += 1;
            } else {
                break;
            }
        }
        prefix_len = matching;

        if prefix_len == 0 {
            break;
        }
    }

    prefix_len
}

/// Process escape sequences in a string literal, both validating and transforming.
///
/// Recognized escapes: `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\u{XXXX}` (1-6 hex digits).
/// Invalid escapes (e.g., `\q`) return an error.
/// `\u{}` (empty) is rejected. `\u{...}` with non-hex or >6 digits is rejected.
/// Codepoint range validation (0-10FFFF, no surrogates) is deferred to semantic pass.
///
/// # Examples
///
/// ```
/// use writ_parser::string_utils::process_escapes;
///
/// assert_eq!(process_escapes("hello").unwrap(), "hello");
/// assert_eq!(process_escapes("a\\nb").unwrap(), "a\nb");
/// assert_eq!(process_escapes("\\u{41}").unwrap(), "A");
/// assert!(process_escapes("\\q").is_err());
/// ```
pub fn process_escapes(content: &str) -> Result<String, EscapeError> {
    let mut result = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 1;
            if i >= bytes.len() {
                return Err(EscapeError::TrailingBackslash);
            }

            match bytes[i] {
                b'n' => {
                    result.push('\n');
                    i += 1;
                }
                b't' => {
                    result.push('\t');
                    i += 1;
                }
                b'r' => {
                    result.push('\r');
                    i += 1;
                }
                b'0' => {
                    result.push('\0');
                    i += 1;
                }
                b'\\' => {
                    result.push('\\');
                    i += 1;
                }
                b'"' => {
                    result.push('"');
                    i += 1;
                }
                b'u' => {
                    i += 1; // skip 'u'

                    // Expect '{'
                    if i >= bytes.len() || bytes[i] != b'{' {
                        return Err(EscapeError::InvalidEscape('u'));
                    }
                    i += 1; // skip '{'

                    // Collect hex digits until '}'
                    let hex_start = i;
                    while i < bytes.len() && bytes[i] != b'}' {
                        i += 1;
                    }

                    if i >= bytes.len() {
                        // No closing brace found
                        let hex_str = String::from_utf8_lossy(&bytes[hex_start..]).to_string();
                        return Err(EscapeError::InvalidUnicodeHex(hex_str));
                    }

                    let hex_str =
                        std::str::from_utf8(&bytes[hex_start..i]).unwrap_or("").to_string();
                    i += 1; // skip '}'

                    // Validate: must have 1-6 hex digits
                    if hex_str.is_empty() {
                        return Err(EscapeError::EmptyUnicodeEscape);
                    }

                    if hex_str.len() > 6 {
                        return Err(EscapeError::UnicodeTooLong(hex_str));
                    }

                    // Validate all characters are hex digits
                    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(EscapeError::InvalidUnicodeHex(hex_str));
                    }

                    // Parse as u32 and convert to char
                    // (range validation deferred to semantic pass)
                    let codepoint =
                        u32::from_str_radix(&hex_str, 16).map_err(|_| {
                            EscapeError::InvalidUnicodeHex(hex_str.clone())
                        })?;

                    match char::from_u32(codepoint) {
                        Some(c) => result.push(c),
                        None => return Err(EscapeError::InvalidUnicodeHex(hex_str)),
                    }
                }
                other => {
                    // Invalid escape character
                    // Safe to cast since we're dealing with ASCII escape chars
                    return Err(EscapeError::InvalidEscape(other as char));
                }
            }
        } else {
            // Regular character — need to handle multi-byte UTF-8 properly
            // Since we index by bytes but push chars, advance by the char's byte length
            let remaining = &content[i..];
            let ch = remaining.chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================
    // dedent_raw_string unit tests
    // =========================================================

    #[test]
    fn dedent_empty_string() {
        assert_eq!(dedent_raw_string(""), "");
    }

    #[test]
    fn dedent_only_structural_lines() {
        // Just the structural first and last lines, no content
        assert_eq!(dedent_raw_string("\n    "), "");
    }

    #[test]
    fn dedent_simple() {
        let content = "\n    hello\n    world\n    ";
        assert_eq!(dedent_raw_string(content), "hello\nworld");
    }

    #[test]
    fn dedent_mixed_indent() {
        let content = "\n    outer\n        inner\n    ";
        assert_eq!(dedent_raw_string(content), "outer\n    inner");
    }

    #[test]
    fn dedent_blank_lines_excluded() {
        let content = "\n    hello\n\n    world\n    ";
        assert_eq!(dedent_raw_string(content), "hello\n\nworld");
    }

    #[test]
    fn dedent_whitespace_only_line_excluded() {
        let content = "\n    hello\n        \n    world\n    ";
        assert_eq!(dedent_raw_string(content), "hello\n\nworld");
    }

    // =========================================================
    // process_escapes unit tests
    // =========================================================

    #[test]
    fn escapes_no_escapes() {
        assert_eq!(process_escapes("hello world").unwrap(), "hello world");
    }

    #[test]
    fn escapes_newline() {
        assert_eq!(process_escapes("a\\nb").unwrap(), "a\nb");
    }

    #[test]
    fn escapes_tab() {
        assert_eq!(process_escapes("a\\tb").unwrap(), "a\tb");
    }

    #[test]
    fn escapes_unicode_a() {
        assert_eq!(process_escapes("\\u{41}").unwrap(), "A");
    }

    #[test]
    fn escapes_invalid_q() {
        assert!(matches!(
            process_escapes("\\q"),
            Err(EscapeError::InvalidEscape('q'))
        ));
    }

    #[test]
    fn escapes_empty_unicode() {
        assert!(matches!(
            process_escapes("\\u{}"),
            Err(EscapeError::EmptyUnicodeEscape)
        ));
    }

    #[test]
    fn escapes_trailing_backslash() {
        assert!(matches!(
            process_escapes("hello\\"),
            Err(EscapeError::TrailingBackslash)
        ));
    }
}
