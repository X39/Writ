use chumsky::span::SimpleSpan;
use logos::Logos;

/// Logos callback for nested block comments.
/// Tracks nesting depth: `/* /* inner */ outer */` is one comment.
fn nested_block_comment<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> bool {
    let mut depth: u32 = 1;
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            if depth == 0 {
                lex.bump(i + 2);
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false // unclosed comment
}

/// Logos callback for raw strings with N-quote delimiter matching.
/// The lexer has already matched `"""`. This callback counts any additional
/// opening quotes and scans for the matching closing delimiter.
///
/// Validation rules (spec v0.4):
/// - Opening delimiter must be immediately followed by a newline (`\n` or `\r\n`).
/// - Closing delimiter must appear on its own line (only whitespace before it).
fn raw_string<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> bool {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();

    // Count additional quotes beyond the initial """
    let extra_quotes = bytes.iter().take_while(|&&b| b == b'"').count();
    let total_quotes = 3 + extra_quotes; // 3 from the token pattern + extras

    // Build closing delimiter pattern
    let search_start = extra_quotes;

    if search_start >= bytes.len() {
        // Nothing after the opening quotes
        return false;
    }

    // Opening delimiter must be followed by newline
    match bytes[search_start] {
        b'\n' => {}
        b'\r' if search_start + 1 < bytes.len() && bytes[search_start + 1] == b'\n' => {}
        _ => return false, // opening delimiter not followed by newline
    }

    // Scan for exactly `total_quotes` consecutive quotes not followed by another quote
    let mut i = search_start;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // Count consecutive quotes at this position
            let quote_start = i;
            while i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
            let quote_count = i - quote_start;

            // Check if we found exactly the right number of closing quotes
            if quote_count == total_quotes {
                // Validate: closing delimiter must be on its own line
                // Scan backwards from quote_start to find the preceding newline
                let mut j = quote_start;
                while j > search_start && bytes[j - 1] != b'\n' {
                    j -= 1;
                }
                // Everything from j..quote_start must be whitespace
                let prefix = &bytes[j..quote_start];
                if !prefix.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
                    return false; // closing delimiter not on its own line
                }

                lex.bump(i);
                return true;
            }
            // If we found more quotes than needed, check if the first `total_quotes`
            // form a valid close (they don't, because they're followed by more quotes)
            // If fewer quotes than needed, just continue scanning
        } else {
            i += 1;
        }
    }
    false // unclosed raw string
}

/// Logos callback for formattable strings: `$"..."` with `{expr}` interpolation.
/// The lexer has already matched `$"`. Scans for closing `"` while tracking brace depth.
/// In Phase 1, the entire string (including interpolation) is captured as one opaque token.
///
/// Unicode escapes (`\u{XXXX}`) are recognized and skipped without incrementing
/// brace depth, so they are not treated as interpolation slots.
fn formattable_string<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> bool {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();
    let mut i = 0;
    let mut brace_depth: u32 = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1; // skip the backslash
                if i < bytes.len() {
                    if bytes[i] == b'u' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                        // \u{...} unicode escape — skip past the closing }
                        // Do NOT increment brace_depth
                        i += 2; // skip 'u' and '{'
                        while i < bytes.len() && bytes[i] != b'}' {
                            i += 1;
                        }
                        if i < bytes.len() {
                            i += 1; // skip '}'
                        }
                    } else {
                        i += 1; // skip the escaped character
                    }
                }
            }
            b'{' => {
                brace_depth += 1;
                i += 1;
            }
            b'}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                i += 1;
            }
            b'"' if brace_depth == 0 => {
                // Found closing quote
                lex.bump(i + 1);
                return true;
            }
            _ => {
                i += 1;
            }
        }
    }
    false // unclosed formattable string
}

/// Logos callback for formattable raw strings: `$"""..."""` with interpolation.
/// The lexer has already matched `$"""`. Counts additional quotes and scans for
/// matching closing delimiter while allowing `{expr}` interpolation.
///
/// Validation rules (spec v0.4):
/// - Opening delimiter must be immediately followed by a newline (`\n` or `\r\n`).
/// - Closing delimiter must appear on its own line (only whitespace before it).
fn formattable_raw_string<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> bool {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();

    // Count additional quotes beyond the initial """
    let extra_quotes = bytes.iter().take_while(|&&b| b == b'"').count();
    let total_quotes = 3 + extra_quotes;

    let search_start = extra_quotes;
    if search_start >= bytes.len() {
        return false;
    }

    // Opening delimiter must be followed by newline
    match bytes[search_start] {
        b'\n' => {}
        b'\r' if search_start + 1 < bytes.len() && bytes[search_start + 1] == b'\n' => {}
        _ => return false, // opening delimiter not followed by newline
    }

    // Scan for exactly `total_quotes` consecutive quotes
    let mut i = search_start;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let quote_start = i;
            while i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
            let quote_count = i - quote_start;
            if quote_count == total_quotes {
                // Validate: closing delimiter must be on its own line
                let mut j = quote_start;
                while j > search_start && bytes[j - 1] != b'\n' {
                    j -= 1;
                }
                let prefix = &bytes[j..quote_start];
                if !prefix.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
                    return false; // closing delimiter not on its own line
                }

                lex.bump(i);
                return true;
            }
        } else {
            i += 1;
        }
    }
    false // unclosed formattable raw string
}

/// All Writ language tokens. Trivia (whitespace, comments) are preserved as
/// explicit variants for full-fidelity CST construction.
#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token<'src> {
    // =========================================================
    // Trivia (NOT skipped -- full fidelity)
    // =========================================================
    #[regex(r"[ \t\r\n]+")]
    Whitespace,

    #[regex(r"//[^\n]*", allow_greedy = true)]
    LineComment,

    #[token("/*", nested_block_comment)]
    BlockComment,

    // =========================================================
    // Keywords — Declaration
    // =========================================================
    #[token("fn")]
    KwFn,
    #[token("dlg")]
    KwDlg,
    #[token("struct")]
    KwStruct,
    #[token("enum")]
    KwEnum,
    #[token("contract")]
    KwContract,
    #[token("impl")]
    KwImpl,
    #[token("entity")]
    KwEntity,
    #[token("component")]
    KwComponent,
    #[token("namespace")]
    KwNamespace,
    #[token("extern")]
    KwExtern,
    #[token("using")]
    KwUsing,
    #[token("new")]
    KwNew,

    // =========================================================
    // Keywords — Visibility
    // =========================================================
    #[token("pub")]
    KwPub,
    #[token("priv")]
    KwPriv,

    // =========================================================
    // Keywords — Variables
    // =========================================================
    #[token("let")]
    KwLet,
    #[token("mut")]
    KwMut,
    #[token("const")]
    KwConst,
    #[token("global")]
    KwGlobal,

    // =========================================================
    // Keywords — Control flow
    // =========================================================
    #[token("if")]
    KwIf,
    #[token("else")]
    KwElse,
    #[token("match")]
    KwMatch,
    #[token("for")]
    KwFor,
    #[token("while")]
    KwWhile,
    #[token("in")]
    KwIn,
    #[token("return")]
    KwReturn,
    #[token("break")]
    KwBreak,
    #[token("continue")]
    KwContinue,

    // =========================================================
    // Keywords — Concurrency
    // =========================================================
    #[token("spawn")]
    KwSpawn,
    #[token("detached")]
    KwDetached,
    #[token("join")]
    KwJoin,
    #[token("cancel")]
    KwCancel,
    #[token("defer")]
    KwDefer,

    // =========================================================
    // Keywords — Error handling
    // =========================================================
    #[token("try")]
    KwTry,

    // =========================================================
    // Keywords — Types
    // =========================================================
    #[token("void")]
    KwVoid,

    // =========================================================
    // Keywords — Values
    // =========================================================
    #[token("true")]
    KwTrue,
    #[token("false")]
    KwFalse,
    #[token("null")]
    KwNull,
    #[token("self")]
    KwSelf,

    // =========================================================
    // Keywords — Entity
    // =========================================================
    #[token("use")]
    KwUse,
    #[token("on")]
    KwOn,

    // =========================================================
    // Keywords — Concurrency (globals)
    // =========================================================
    #[token("atomic")]
    KwAtomic,

    // =========================================================
    // String Literals
    // =========================================================
    /// Formattable raw string: $"""...""" (must precede raw string and formattable string)
    #[token("$\"\"\"", formattable_raw_string)]
    FormattableRawStringLit,

    /// Raw string: """...""" (must precede basic string)
    #[token("\"\"\"", raw_string)]
    RawStringLit,

    /// Formattable string: $"..." (must precede basic string and dollar)
    #[token("$\"", formattable_string)]
    FormattableStringLit,

    /// Basic string: "..." with escape sequences
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLit(&'src str),

    // =========================================================
    // Numeric Literals
    // =========================================================
    /// Float literal with optional scientific notation
    /// Must be defined before IntLit to ensure `3.14` matches as float
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+\-]?[0-9][0-9_]*)?")]
    FloatLit(&'src str),

    /// Hex literal: 0x... or 0X...
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*")]
    HexLit(&'src str),

    /// Binary literal: 0b... or 0B...
    #[regex(r"0[bB][01][01_]*")]
    BinLit(&'src str),

    /// Decimal integer literal with optional underscore separators
    #[regex(r"[0-9][0-9_]*")]
    IntLit(&'src str),

    // =========================================================
    // Identifiers (after keywords, so keywords take priority)
    // =========================================================
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident(&'src str),

    // =========================================================
    // Multi-character operators (before single-character)
    // =========================================================
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("::")]
    ColonColon,
    #[token("..=")]
    DotDotEq,
    #[token("..")]
    DotDot,

    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AmpAmp,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("||")]
    PipePipe,

    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,

    // =========================================================
    // Single-character operators
    // =========================================================
    #[token("=")]
    Eq,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("!")]
    Bang,
    #[token("?")]
    Question,
    #[token("^")]
    Caret,

    // =========================================================
    // Sigils
    // =========================================================
    #[token("@")]
    At,
    #[token("$")]
    Dollar,
    #[token("#")]
    Hash,

    // =========================================================
    // Punctuation
    // =========================================================
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,

    // =========================================================
    // Delimiters
    // =========================================================
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // =========================================================
    // Error sentinel
    // =========================================================
    Error,
}

/// Lex a Writ source string into a vector of (Token, SimpleSpan) pairs.
/// Every byte of the source is covered by exactly one token span.
/// Trivia (whitespace, comments) is preserved as explicit token variants.
pub fn lex(src: &str) -> Vec<(Token<'_>, SimpleSpan)> {
    Token::lexer(src)
        .spanned()
        .map(|(tok, span)| {
            let tok = tok.unwrap_or(Token::Error);
            (tok, SimpleSpan::from(span))
        })
        .collect()
}
