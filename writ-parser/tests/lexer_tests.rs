use writ_parser::{lex, Token};

// =============================================================
// Lossless Roundtrip Tests (LEX-02, INTG-02)
// =============================================================

#[test]
fn lossless_roundtrip_comments() {
    let src = include_str!("cases/01_comments.writ");
    let tokens = lex(src);
    let reconstructed: String = tokens
        .iter()
        .map(|(_, span)| &src[span.start..span.end])
        .collect();
    assert_eq!(
        src, reconstructed,
        "Lossless roundtrip failed for 01_comments.writ"
    );
}

#[test]
fn lossless_roundtrip_string_literals() {
    let src = include_str!("cases/02_string_literals.writ");
    let tokens = lex(src);
    let reconstructed: String = tokens
        .iter()
        .map(|(_, span)| &src[span.start..span.end])
        .collect();
    assert_eq!(
        src, reconstructed,
        "Lossless roundtrip failed for 02_string_literals.writ"
    );
}

// =============================================================
// No Error Tokens in Reference Files
// =============================================================

#[test]
fn no_error_tokens_in_comments_writ() {
    let src = include_str!("cases/01_comments.writ");
    let tokens = lex(src);
    let errors: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "Found {} error tokens in 01_comments.writ. First error at byte offset {:?}: '{}'",
        errors.len(),
        errors.first().map(|(_, s)| (s.start, s.end)),
        errors
            .first()
            .map(|(_, s)| &src[s.start..s.end])
            .unwrap_or("")
    );
}

#[test]
fn no_error_tokens_in_string_literals_writ() {
    let src = include_str!("cases/02_string_literals.writ");
    let tokens = lex(src);
    let errors: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "Found {} error tokens in 02_string_literals.writ. First error at byte offset {:?}: '{}'",
        errors.len(),
        errors.first().map(|(_, s)| (s.start, s.end)),
        errors
            .first()
            .map(|(_, s)| &src[s.start..s.end])
            .unwrap_or("")
    );
}

// =============================================================
// Comment Tests (LEX-03)
// =============================================================

#[test]
fn single_line_comment() {
    let tokens = lex("// hello world");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::LineComment));
}

#[test]
fn block_comment_simple() {
    let tokens = lex("/* hello */");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::BlockComment));
}

#[test]
fn nested_block_comment() {
    // User decision: block comments nest (Rust-style)
    let tokens = lex("/* outer /* inner */ still outer */");
    assert_eq!(
        tokens.len(),
        1,
        "Nested block comment should be a single token, got {} tokens: {:?}",
        tokens.len(),
        tokens
    );
    assert!(matches!(tokens[0].0, Token::BlockComment));
}

#[test]
fn deeply_nested_block_comment() {
    let tokens = lex("/* a /* b /* c */ d */ e */");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::BlockComment));
}

#[test]
fn empty_line_comment() {
    let tokens = lex("//");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::LineComment));
}

#[test]
fn empty_block_comment() {
    let tokens = lex("/* */");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::BlockComment));
}

#[test]
fn comments_in_01_comments_writ() {
    let src = include_str!("cases/01_comments.writ");
    let tokens = lex(src);
    let comment_tokens: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::LineComment | Token::BlockComment))
        .collect();
    // 01_comments.writ has many comments:
    // single-line, multi-line, trailing, preceding, nested-style content, empty //, empty /* */
    assert!(
        comment_tokens.len() >= 8,
        "Expected at least 8 comment tokens, got {}",
        comment_tokens.len()
    );
}

// =============================================================
// String Literal Tests (STR-01, STR-03)
// =============================================================

#[test]
fn basic_string_literal() {
    let tokens = lex(r#""hello""#);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(non_ws.len(), 1);
    assert!(matches!(non_ws[0].0, Token::StringLit(_)));
}

#[test]
fn basic_string_with_escapes() {
    let tokens = lex(r#""hello\nworld""#);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(non_ws.len(), 1);
    assert!(matches!(non_ws[0].0, Token::StringLit(_)));
}

#[test]
fn basic_string_with_escaped_quote() {
    let tokens = lex(r#""She said \"hello\"""#);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(non_ws.len(), 1);
    assert!(matches!(non_ws[0].0, Token::StringLit(_)));
}

#[test]
fn empty_string() {
    let tokens = lex(r#""""#);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(non_ws.len(), 1);
    assert!(matches!(non_ws[0].0, Token::StringLit(_)));
}

#[test]
fn raw_string_triple_quotes() {
    let src = "\"\"\"hello world\"\"\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Triple-quote raw string should be a single token, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::RawStringLit));
}

#[test]
fn raw_string_four_quotes() {
    // """" contains """ inside """"
    let src = "\"\"\"\"contains \"\"\" inside\"\"\"\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Four-quote raw string should be a single token, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::RawStringLit));
}

#[test]
fn raw_string_five_quotes() {
    // """"" contains """ and """" inside """""
    let src = "\"\"\"\"\"contains \"\"\" and \"\"\"\" inside\"\"\"\"\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Five-quote raw string should be a single token, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::RawStringLit));
}

#[test]
fn formattable_string() {
    let src = "$\"hello {name}!\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Formattable string should be a single token in Phase 1, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::FormattableStringLit));
}

#[test]
fn formattable_string_with_nested_braces() {
    // $"JSON: {{\"key\": \"{name}\"}}"
    let src = "$\"JSON: {{\\\"key\\\": \\\"{name}\\\"}}\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Formattable string with nested braces should be a single token, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::FormattableStringLit));
}

#[test]
fn formattable_raw_string() {
    let src = "$\"\"\"\nhello {name}\n\"\"\"";
    let tokens = lex(src);
    let non_ws: Vec<_> = tokens
        .iter()
        .filter(|(t, _)| !matches!(t, Token::Whitespace))
        .collect();
    assert_eq!(
        non_ws.len(),
        1,
        "Formattable raw string should be a single token, got {:?}",
        non_ws
    );
    assert!(matches!(non_ws[0].0, Token::FormattableRawStringLit));
}

// =============================================================
// Numeric Literal Tests (LEX-04)
// =============================================================

#[test]
fn integer_literal() {
    let tokens = lex("42");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::IntLit("42")));
}

#[test]
fn hex_literal() {
    let tokens = lex("0xFF");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::HexLit("0xFF")));
}

#[test]
fn hex_literal_uppercase() {
    let tokens = lex("0XFF");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::HexLit("0XFF")));
}

#[test]
fn binary_literal() {
    let tokens = lex("0b1010");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::BinLit("0b1010")));
}

#[test]
fn float_literal() {
    let tokens = lex("3.14");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::FloatLit("3.14")));
}

#[test]
fn scientific_notation() {
    let tokens = lex("1.5e10");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::FloatLit("1.5e10")));
}

#[test]
fn scientific_notation_negative_exponent() {
    let tokens = lex("3.0e-8");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::FloatLit("3.0e-8")));
}

#[test]
fn underscore_separator_integer() {
    let tokens = lex("1_000_000");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::IntLit("1_000_000")));
}

#[test]
fn underscore_separator_hex() {
    let tokens = lex("0xFF_FF");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::HexLit("0xFF_FF")));
}

#[test]
fn underscore_separator_float() {
    let tokens = lex("1_000.5");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::FloatLit("1_000.5")));
}

// =============================================================
// Boolean and Null Literal Tests (LEX-05)
// =============================================================

#[test]
fn bool_true() {
    let tokens = lex("true");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::KwTrue));
}

#[test]
fn bool_false() {
    let tokens = lex("false");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::KwFalse));
}

#[test]
fn null_literal() {
    let tokens = lex("null");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::KwNull));
}

// =============================================================
// Keyword vs Identifier Priority (LEX-01)
// =============================================================

#[test]
fn keyword_fn_not_identifier() {
    let tokens = lex("fn");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::KwFn));
}

#[test]
fn identifier_fn_prefix() {
    // "fn_name" should be an identifier, not KwFn + _name
    let tokens = lex("fn_name");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].0, Token::Ident("fn_name")));
}

#[test]
fn all_keywords_recognized() {
    let keywords = vec![
        ("fn", Token::KwFn),
        ("dlg", Token::KwDlg),
        ("struct", Token::KwStruct),
        ("enum", Token::KwEnum),
        ("contract", Token::KwContract),
        ("impl", Token::KwImpl),
        ("entity", Token::KwEntity),
        ("component", Token::KwComponent),
        ("namespace", Token::KwNamespace),
        ("extern", Token::KwExtern),
        ("using", Token::KwUsing),
        ("pub", Token::KwPub),
        ("priv", Token::KwPriv),
        ("let", Token::KwLet),
        ("mut", Token::KwMut),
        ("const", Token::KwConst),
        ("global", Token::KwGlobal),
        ("if", Token::KwIf),
        ("else", Token::KwElse),
        ("match", Token::KwMatch),
        ("for", Token::KwFor),
        ("while", Token::KwWhile),
        ("in", Token::KwIn),
        ("return", Token::KwReturn),
        ("break", Token::KwBreak),
        ("continue", Token::KwContinue),
        ("spawn", Token::KwSpawn),
        ("detached", Token::KwDetached),
        ("join", Token::KwJoin),
        ("cancel", Token::KwCancel),
        ("defer", Token::KwDefer),
        ("try", Token::KwTry),
        ("void", Token::KwVoid),
        ("true", Token::KwTrue),
        ("false", Token::KwFalse),
        ("null", Token::KwNull),
        ("self", Token::KwSelf),
        ("use", Token::KwUse),
        ("on", Token::KwOn),
        ("atomic", Token::KwAtomic),
    ];

    for (text, expected) in keywords {
        let tokens = lex(text);
        assert_eq!(
            tokens.len(),
            1,
            "Expected 1 token for '{}', got {}",
            text,
            tokens.len()
        );
        assert_eq!(
            tokens[0].0, expected,
            "Keyword '{}' should produce {:?}, got {:?}",
            text, expected, tokens[0].0
        );
    }
}

// =============================================================
// Operator and Sigil Tests (LEX-01)
// =============================================================

#[test]
fn multi_char_operators() {
    assert!(matches!(lex("->")[0].0, Token::Arrow));
    assert!(matches!(lex("::")[0].0, Token::ColonColon));
    assert!(matches!(lex("..=")[0].0, Token::DotDotEq));
    assert!(matches!(lex("..")[0].0, Token::DotDot));
    assert!(matches!(lex("==")[0].0, Token::EqEq));
    assert!(matches!(lex("!=")[0].0, Token::BangEq));
    assert!(matches!(lex("<=")[0].0, Token::LtEq));
    assert!(matches!(lex(">=")[0].0, Token::GtEq));
    assert!(matches!(lex("&&")[0].0, Token::AmpAmp));
    assert!(matches!(lex("||")[0].0, Token::PipePipe));
    assert!(matches!(lex("+=")[0].0, Token::PlusEq));
    assert!(matches!(lex("-=")[0].0, Token::MinusEq));
    assert!(matches!(lex("*=")[0].0, Token::StarEq));
    assert!(matches!(lex("/=")[0].0, Token::SlashEq));
    assert!(matches!(lex("%=")[0].0, Token::PercentEq));
}

#[test]
fn single_char_operators_and_sigils() {
    assert!(matches!(lex("=")[0].0, Token::Eq));
    assert!(matches!(lex("+")[0].0, Token::Plus));
    assert!(matches!(lex("-")[0].0, Token::Minus));
    assert!(matches!(lex("*")[0].0, Token::Star));
    assert!(matches!(lex("/")[0].0, Token::Slash));
    assert!(matches!(lex("%")[0].0, Token::Percent));
    assert!(matches!(lex("<")[0].0, Token::Lt));
    assert!(matches!(lex(">")[0].0, Token::Gt));
    assert!(matches!(lex("!")[0].0, Token::Bang));
    assert!(matches!(lex("?")[0].0, Token::Question));
    assert!(matches!(lex("^")[0].0, Token::Caret));
    assert!(matches!(lex("@")[0].0, Token::At));
    assert!(matches!(lex("#")[0].0, Token::Hash));
    assert!(matches!(lex(".")[0].0, Token::Dot));
    assert!(matches!(lex(",")[0].0, Token::Comma));
    assert!(matches!(lex(":")[0].0, Token::Colon));
    assert!(matches!(lex(";")[0].0, Token::Semi));
    assert!(matches!(lex("(")[0].0, Token::LParen));
    assert!(matches!(lex(")")[0].0, Token::RParen));
    assert!(matches!(lex("{")[0].0, Token::LBrace));
    assert!(matches!(lex("}")[0].0, Token::RBrace));
    assert!(matches!(lex("[")[0].0, Token::LBracket));
    assert!(matches!(lex("]")[0].0, Token::RBracket));
}

// =============================================================
// Whitespace Preservation Test (LEX-02)
// =============================================================

#[test]
fn whitespace_preserved() {
    let tokens = lex("fn  main");
    assert_eq!(tokens.len(), 3);
    assert!(matches!(tokens[0].0, Token::KwFn));
    assert!(matches!(tokens[1].0, Token::Whitespace));
    assert!(matches!(tokens[2].0, Token::Ident("main")));
}

#[test]
fn newlines_preserved() {
    let tokens = lex("fn\nmain");
    assert_eq!(tokens.len(), 3);
    assert!(matches!(tokens[0].0, Token::KwFn));
    assert!(matches!(tokens[1].0, Token::Whitespace));
    assert!(matches!(tokens[2].0, Token::Ident("main")));
}

// =============================================================
// Span Correctness Tests (INTG-02)
// =============================================================

#[test]
fn spans_cover_exact_source_text() {
    let src = "fn main() { }";
    let tokens = lex(src);
    for (tok, span) in &tokens {
        let text = &src[span.start..span.end];
        match tok {
            Token::KwFn => assert_eq!(text, "fn"),
            Token::Ident("main") => assert_eq!(text, "main"),
            Token::LParen => assert_eq!(text, "("),
            Token::RParen => assert_eq!(text, ")"),
            Token::LBrace => assert_eq!(text, "{"),
            Token::RBrace => assert_eq!(text, "}"),
            Token::Whitespace => assert!(text.chars().all(|c| c.is_whitespace())),
            _ => {}
        }
    }
}
