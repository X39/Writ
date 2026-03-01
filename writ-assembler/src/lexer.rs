use crate::error::AssembleError;

/// Token kinds produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A dot-prefixed directive: `.module`, `.type`, `.field`, etc.
    Directive(String),
    /// An identifier or instruction mnemonic.
    Ident(String),
    /// Integer literal.
    IntLit(i64),
    /// Float literal.
    FloatLit(f64),
    /// String literal (contents only, no quotes).
    StringLit(String),
    /// Register reference: r0, r1, etc.
    Register(u16),
    /// Label definition: `.foo:` (the name without dot or colon).
    Label(String),
    /// Label reference in instructions: `.foo` (the name without dot).
    LabelRef(String),
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `->`
    Arrow,
    /// `::`
    DoubleColon,
    /// `<`
    LAngle,
    /// `>`
    RAngle,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// Newline (used as statement separator).
    Newline,
    /// End of file.
    Eof,
}

/// A token with its source location.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub col: u32,
}

/// Tokenize source text into a sequence of tokens.
///
/// Collects multiple errors (e.g., unterminated strings) and continues tokenizing.
pub fn tokenize(src: &str) -> Result<Vec<Token>, Vec<AssembleError>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut pos = 0;
    let mut line: u32 = 1;
    let mut col: u32 = 1;

    while pos < len {
        let ch = chars[pos];

        // Skip whitespace (but not newlines)
        if ch == ' ' || ch == '\t' || ch == '\r' {
            pos += 1;
            col += 1;
            continue;
        }

        // Newlines
        if ch == '\n' {
            tokens.push(Token { kind: TokenKind::Newline, line, col });
            pos += 1;
            line += 1;
            col = 1;
            continue;
        }

        // Line comments
        if ch == '/' && pos + 1 < len && chars[pos + 1] == '/' {
            // Skip to end of line
            while pos < len && chars[pos] != '\n' {
                pos += 1;
            }
            continue;
        }

        // String literals
        if ch == '"' {
            let start_line = line;
            let start_col = col;
            pos += 1;
            col += 1;
            let mut s = String::new();
            let mut terminated = false;
            while pos < len {
                let c = chars[pos];
                if c == '\n' {
                    // Unterminated string at newline
                    break;
                }
                if c == '"' {
                    terminated = true;
                    pos += 1;
                    col += 1;
                    break;
                }
                if c == '\\' && pos + 1 < len {
                    pos += 1;
                    col += 1;
                    let esc = chars[pos];
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '0' => s.push('\0'),
                        _ => {
                            errors.push(AssembleError::new(
                                format!("unknown escape sequence '\\{}'", esc),
                                line,
                                col - 1,
                            ));
                            s.push(esc);
                        }
                    }
                    pos += 1;
                    col += 1;
                    continue;
                }
                s.push(c);
                pos += 1;
                col += 1;
            }
            if !terminated {
                errors.push(AssembleError::new("unterminated string literal", start_line, start_col));
            }
            tokens.push(Token { kind: TokenKind::StringLit(s), line: start_line, col: start_col });
            continue;
        }

        // Dot-prefixed: directives, labels, or label references
        if ch == '.' {
            let start_col = col;
            pos += 1;
            col += 1;

            // Collect the identifier after the dot
            let ident_start = pos;
            while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                pos += 1;
                col += 1;
            }

            if pos == ident_start {
                errors.push(AssembleError::new("expected identifier after '.'", line, start_col));
                continue;
            }

            let name: String = chars[ident_start..pos].iter().collect();

            // Check if it's a label definition (followed by ':' but not '::')
            if pos < len && chars[pos] == ':' && !(pos + 1 < len && chars[pos + 1] == ':') {
                pos += 1;
                col += 1;
                tokens.push(Token { kind: TokenKind::Label(name), line, col: start_col });
            } else {
                // Determine if it's a directive or a label reference
                // Known directives list
                let known_directives = [
                    "module", "type", "field", "method", "contract", "impl",
                    "reg", "extern", "global", "regs",
                ];
                let lower = name.to_lowercase();
                if known_directives.contains(&lower.as_str()) {
                    tokens.push(Token {
                        kind: TokenKind::Directive(lower),
                        line,
                        col: start_col,
                    });
                } else {
                    // It's a label reference (used in branch instructions)
                    tokens.push(Token { kind: TokenKind::LabelRef(name), line, col: start_col });
                }
            }
            continue;
        }

        // Symbols
        if ch == '{' {
            tokens.push(Token { kind: TokenKind::OpenBrace, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == '}' {
            tokens.push(Token { kind: TokenKind::CloseBrace, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == '(' {
            tokens.push(Token { kind: TokenKind::OpenParen, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == ')' {
            tokens.push(Token { kind: TokenKind::CloseParen, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == ',' {
            tokens.push(Token { kind: TokenKind::Comma, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == '<' {
            tokens.push(Token { kind: TokenKind::LAngle, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == '>' {
            tokens.push(Token { kind: TokenKind::RAngle, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == '[' {
            tokens.push(Token { kind: TokenKind::OpenBracket, line, col });
            pos += 1;
            col += 1;
            continue;
        }
        if ch == ']' {
            tokens.push(Token { kind: TokenKind::CloseBracket, line, col });
            pos += 1;
            col += 1;
            continue;
        }

        // Arrow (->) or minus (part of negative number handled elsewhere)
        if ch == '-' && pos + 1 < len && chars[pos + 1] == '>' {
            tokens.push(Token { kind: TokenKind::Arrow, line, col });
            pos += 2;
            col += 2;
            continue;
        }

        // Colon or double colon
        if ch == ':' {
            if pos + 1 < len && chars[pos + 1] == ':' {
                tokens.push(Token { kind: TokenKind::DoubleColon, line, col });
                pos += 2;
                col += 2;
            } else {
                tokens.push(Token { kind: TokenKind::Colon, line, col });
                pos += 1;
                col += 1;
            }
            continue;
        }

        // Numeric literals (including negative numbers)
        if ch.is_ascii_digit() || (ch == '-' && pos + 1 < len && chars[pos + 1].is_ascii_digit()) {
            let start_col = col;
            let negative = ch == '-';
            if negative {
                pos += 1;
                col += 1;
            }

            // Check for hex or binary prefix
            if chars[pos] == '0' && pos + 1 < len {
                let next = chars[pos + 1];
                if next == 'x' || next == 'X' {
                    // Hex literal
                    pos += 2;
                    col += 2;
                    let hex_start = pos;
                    while pos < len && (chars[pos].is_ascii_hexdigit() || chars[pos] == '_') {
                        pos += 1;
                        col += 1;
                    }
                    let hex_str: String = chars[hex_start..pos].iter().filter(|c| **c != '_').collect();
                    match i64::from_str_radix(&hex_str, 16) {
                        Ok(v) => {
                            let value = if negative { -v } else { v };
                            tokens.push(Token { kind: TokenKind::IntLit(value), line, col: start_col });
                        }
                        Err(_) => {
                            errors.push(AssembleError::new("invalid hex literal", line, start_col));
                        }
                    }
                    continue;
                }
                if next == 'b' || next == 'B' {
                    // Binary literal
                    pos += 2;
                    col += 2;
                    let bin_start = pos;
                    while pos < len && (chars[pos] == '0' || chars[pos] == '1' || chars[pos] == '_') {
                        pos += 1;
                        col += 1;
                    }
                    let bin_str: String = chars[bin_start..pos].iter().filter(|c| **c != '_').collect();
                    match i64::from_str_radix(&bin_str, 2) {
                        Ok(v) => {
                            let value = if negative { -v } else { v };
                            tokens.push(Token { kind: TokenKind::IntLit(value), line, col: start_col });
                        }
                        Err(_) => {
                            errors.push(AssembleError::new("invalid binary literal", line, start_col));
                        }
                    }
                    continue;
                }
            }

            // Decimal integer or float
            let num_start = pos;
            while pos < len && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
                pos += 1;
                col += 1;
            }

            // Check for float (decimal point)
            let mut is_float = false;
            if pos < len && chars[pos] == '.' && pos + 1 < len && chars[pos + 1].is_ascii_digit() {
                is_float = true;
                pos += 1;
                col += 1;
                while pos < len && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
                    pos += 1;
                    col += 1;
                }
            }

            // Check for exponent
            if pos < len && (chars[pos] == 'e' || chars[pos] == 'E') {
                is_float = true;
                pos += 1;
                col += 1;
                if pos < len && (chars[pos] == '+' || chars[pos] == '-') {
                    pos += 1;
                    col += 1;
                }
                while pos < len && chars[pos].is_ascii_digit() {
                    pos += 1;
                    col += 1;
                }
            }

            let num_str: String = chars[num_start..pos].iter().filter(|c| **c != '_').collect();
            if is_float {
                match num_str.parse::<f64>() {
                    Ok(v) => {
                        let value = if negative { -v } else { v };
                        tokens.push(Token { kind: TokenKind::FloatLit(value), line, col: start_col });
                    }
                    Err(_) => {
                        errors.push(AssembleError::new("invalid float literal", line, start_col));
                    }
                }
            } else {
                match num_str.parse::<i64>() {
                    Ok(v) => {
                        let value = if negative { -v } else { v };
                        tokens.push(Token { kind: TokenKind::IntLit(value), line, col: start_col });
                    }
                    Err(_) => {
                        errors.push(AssembleError::new("invalid integer literal", line, start_col));
                    }
                }
            }
            continue;
        }

        // Identifiers and register references
        if ch.is_alphabetic() || ch == '_' {
            let start_col = col;
            let ident_start = pos;
            while pos < len && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                pos += 1;
                col += 1;
            }
            let ident: String = chars[ident_start..pos].iter().collect();

            // Check for register notation: r followed by digits only
            if ident.starts_with('r') && ident.len() > 1 && ident[1..].chars().all(|c| c.is_ascii_digit()) {
                match ident[1..].parse::<u16>() {
                    Ok(n) => {
                        tokens.push(Token { kind: TokenKind::Register(n), line, col: start_col });
                    }
                    Err(_) => {
                        // Too large for u16, treat as identifier
                        tokens.push(Token { kind: TokenKind::Ident(ident), line, col: start_col });
                    }
                }
            } else {
                tokens.push(Token { kind: TokenKind::Ident(ident), line, col: start_col });
            }
            continue;
        }

        // Unknown character
        errors.push(AssembleError::new(
            format!("unexpected character '{}'", ch),
            line,
            col,
        ));
        pos += 1;
        col += 1;
    }

    // Add EOF
    tokens.push(Token { kind: TokenKind::Eof, line, col });

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_directive() {
        let tokens = tokenize(".module").unwrap();
        assert_eq!(tokens.len(), 2); // Directive + Eof
        assert!(matches!(&tokens[0].kind, TokenKind::Directive(s) if s == "module"));
    }

    #[test]
    fn tokenize_register() {
        let tokens = tokenize("r0 r15").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Register(0)));
        assert!(matches!(tokens[1].kind, TokenKind::Register(15)));
    }

    #[test]
    fn tokenize_label_def_and_ref() {
        let tokens = tokenize(".loop:\n.loop").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Label(s) if s == "loop"));
        assert!(matches!(tokens[1].kind, TokenKind::Newline));
        assert!(matches!(&tokens[2].kind, TokenKind::LabelRef(s) if s == "loop"));
    }

    #[test]
    fn tokenize_string_with_escapes() {
        let tokens = tokenize(r#""hello\nworld""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLit(s) if s == "hello\nworld"));
    }

    #[test]
    fn tokenize_integers() {
        let tokens = tokenize("42 0xFF 0b1010 1_000").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLit(42)));
        assert!(matches!(tokens[1].kind, TokenKind::IntLit(255)));
        assert!(matches!(tokens[2].kind, TokenKind::IntLit(10)));
        assert!(matches!(tokens[3].kind, TokenKind::IntLit(1000)));
    }

    #[test]
    fn tokenize_float() {
        let tokens = tokenize("3.14 1.0e10").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLit(v) if (v - 3.14).abs() < 1e-10));
        assert!(matches!(tokens[1].kind, TokenKind::FloatLit(v) if (v - 1.0e10).abs() < 1.0));
    }

    #[test]
    fn tokenize_symbols() {
        let tokens = tokenize("{ } ( ) , : :: -> < > [ ]").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::OpenBrace));
        assert!(matches!(kinds[1], TokenKind::CloseBrace));
        assert!(matches!(kinds[2], TokenKind::OpenParen));
        assert!(matches!(kinds[3], TokenKind::CloseParen));
        assert!(matches!(kinds[4], TokenKind::Comma));
        assert!(matches!(kinds[5], TokenKind::Colon));
        assert!(matches!(kinds[6], TokenKind::DoubleColon));
        assert!(matches!(kinds[7], TokenKind::Arrow));
        assert!(matches!(kinds[8], TokenKind::LAngle));
        assert!(matches!(kinds[9], TokenKind::RAngle));
        assert!(matches!(kinds[10], TokenKind::OpenBracket));
        assert!(matches!(kinds[11], TokenKind::CloseBracket));
    }

    #[test]
    fn tokenize_comments() {
        let tokens = tokenize("NOP // this is a comment\nRET_VOID").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Ident(s) if s == "NOP"));
        assert!(matches!(tokens[1].kind, TokenKind::Newline));
        assert!(matches!(&tokens[2].kind, TokenKind::Ident(s) if s == "RET_VOID"));
    }

    #[test]
    fn tokenize_unterminated_string_collects_error() {
        let result = tokenize(r#""unterminated"#);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unterminated"));
    }

    #[test]
    fn tokenize_negative_integer() {
        let tokens = tokenize("-42").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLit(-42)));
    }
}
