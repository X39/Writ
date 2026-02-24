//! Parser module for the Writ language.
//!
//! Converts the logos token stream into chumsky parser input and provides
//! parsers for type expressions, generic parameters, expressions, and statements.
//!
//! The expression parser uses chumsky's `.pratt()` combinator for operator
//! precedence with 13+ levels matching spec Section 17.1. Postfix operators
//! (member access, bracket access, calls, `?`, `!`) are handled via `foldl_with`
//! on the atom, which is then fed into the Pratt parser for prefix and infix
//! operators.
//!
//! Mutual recursion between expressions and statements is achieved through a
//! single `recursive()` call where the recursive reference is a block (delimited
//! list of statements). Both `expr` and `stmt` reference `block` for bodies
//! (if/match/lambda/for/while/atomic), and `stmt` references `expr` for values.

use chumsky::input::{Stream, ValueInput};
use chumsky::pratt::*;
use chumsky::prelude::*;
use chumsky::recovery::{nested_delimiters, skip_then_retry_until};

use crate::cst;
use crate::lexer::Token;

type Span = SimpleSpan;

/// Helper enum for type expression postfix operations.
/// Private to the parser module.
#[derive(Clone)]
enum TypePostfix<'src> {
    Generic(Vec<cst::Spanned<cst::TypeExpr<'src>>>),
    Array,
    Nullable,
}

/// Helper enum for expression postfix chain operations.
/// Used by `foldl_with` to dispatch member access, bracket access, calls,
/// null propagation, and unwrap postfix operators.
#[derive(Clone)]
enum ExprPostfix<'src> {
    /// `.field` or `.method(args)` -- field name, optional args
    MemberOrMethod(cst::Spanned<&'src str>, Option<Vec<cst::Spanned<cst::Arg<'src>>>>),
    /// `[expr]` -- bracket access / indexing
    Bracket(cst::Spanned<cst::Expr<'src>>),
    /// `(args)` -- function call
    Call(Vec<cst::Spanned<cst::Arg<'src>>>),
    /// `?` -- null propagation
    NullPropagate,
    /// `!` -- unwrap
    Unwrap,
    /// `.method<T>(args)` -- generic method call
    GenericMethod(
        cst::Spanned<&'src str>,
        Vec<cst::Spanned<cst::TypeExpr<'src>>>,
        Vec<cst::Spanned<cst::Arg<'src>>>,
    ),
    /// `{ field: value, ... }` -- brace construction
    BraceConstruct(Vec<cst::Spanned<cst::Arg<'src>>>),
}

/// Parse type expressions: simple types, generic types, array types,
/// nullable types, function types, and void.
///
/// This parser is independent of the expression parser (no mutual recursion)
/// and handles all type forms via recursive descent with postfix application.
///
/// Examples:
/// - `int` -> Named("int")
/// - `List<T>` -> Generic(Named("List"), [Named("T")])
/// - `T[]` -> Array(Named("T"))
/// - `T?` -> Nullable(Named("T"))
/// - `fn(int, string) -> bool` -> Func([Named("int"), Named("string")], Some(Named("bool")))
/// - `List<T>[]?` -> Nullable(Array(Generic(Named("List"), [Named("T")])))
pub fn type_expr<'tokens, 'src: 'tokens, I>() -> impl Parser<
    'tokens,
    I,
    cst::Spanned<cst::TypeExpr<'src>>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|type_expr| {
        // Base: simple named type or void
        let named = select! {
            Token::Ident(name) => cst::TypeExpr::Named(name),
            Token::KwVoid => cst::TypeExpr::Void,
        }
        .map_with(|t, e| (t, e.span()));

        // Function type: fn(A, B) -> C
        let fn_type = just(Token::KwFn)
            .ignore_then(
                type_expr
                    .clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .then(
                just(Token::Arrow)
                    .ignore_then(type_expr.clone())
                    .or_not(),
            )
            .map_with(|(params, ret), e| {
                (
                    cst::TypeExpr::Func(params, ret.map(Box::new)),
                    e.span(),
                )
            });

        // Atom: function type or named (try fn_type first since it starts with keyword)
        let atom = fn_type.or(named);

        // Postfix: generics <T, U>, array [], nullable ?
        // Applied left-to-right: Name<T>[]? means ((Name<T>)[])?
        atom.foldl_with(
            choice((
                // Generic arguments: <T, U, V>
                type_expr
                    .clone()
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::Lt), just(Token::Gt))
                    .map(TypePostfix::Generic),
                // Array: []
                just(Token::LBracket)
                    .then(just(Token::RBracket))
                    .to(TypePostfix::Array),
                // Nullable: ?
                just(Token::Question).to(TypePostfix::Nullable),
            ))
            .repeated(),
            |base, postfix, e| match postfix {
                TypePostfix::Generic(args) => {
                    (cst::TypeExpr::Generic(Box::new(base), args), e.span())
                }
                TypePostfix::Array => (cst::TypeExpr::Array(Box::new(base)), e.span()),
                TypePostfix::Nullable => {
                    (cst::TypeExpr::Nullable(Box::new(base)), e.span())
                }
            },
        )
    })
}

/// Parse generic parameter declarations: `<T: Bound + Other, U>`.
///
/// Used at declaration sites (function definitions, struct definitions, etc.)
/// where type parameters with optional bounds are declared.
pub fn generic_params<'tokens, 'src: 'tokens, I>() -> impl Parser<
    'tokens,
    I,
    Vec<cst::Spanned<cst::GenericParam<'src>>>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    let param = select! { Token::Ident(name) => name }
        .map_with(|name, e| (name, e.span()))
        .then(
            just(Token::Colon)
                .ignore_then(
                    type_expr()
                        .separated_by(just(Token::Plus))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .map_with(|((name, name_span), bounds), e| {
            (
                cst::GenericParam {
                    name: (name, name_span),
                    bounds: bounds.unwrap_or_default(),
                },
                e.span(),
            )
        });

    param
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::Lt), just(Token::Gt))
}

/// Parse patterns for match arms and if-let expressions.
///
/// Seven pattern forms per user decision:
/// 1. Literal patterns: 42, "key", true, false, null
/// 2. Wildcard: _
/// 3. Variable binding: x
/// 4. Enum destructuring: Result::Ok(val)
/// 5. Nested destructuring (via recursive enum patterns)
/// 6. Or-patterns: A | B | C
/// 7. Range patterns: 1..=5
fn pattern<'tokens, 'src: 'tokens, I>() -> impl Parser<
    'tokens,
    I,
    cst::Spanned<cst::Pattern<'src>>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone
where
    I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|pattern| {
        // Literal patterns: 42, 3.14, "key", true, false, null
        let literal_pat = select! {
            Token::IntLit(n) => cst::Expr::IntLit(n),
            Token::FloatLit(n) => cst::Expr::FloatLit(n),
            Token::StringLit(s) => cst::Expr::StringLit(s),
            Token::KwTrue => cst::Expr::BoolLit(true),
            Token::KwFalse => cst::Expr::BoolLit(false),
            Token::KwNull => cst::Expr::NullLit,
        }
        .map_with(|e, extra| (cst::Pattern::Literal((e, extra.span())), extra.span()));

        // Wildcard: _ (an identifier token with value "_")
        let wildcard = select! { Token::Ident("_") => cst::Pattern::Wildcard }
            .map_with(|p, e| (p, e.span()));

        // Range pattern: int..=int (only inclusive form for patterns)
        // Must come before literal_pat in choice to try this first
        let range_pat = select! {
            Token::IntLit(n) => cst::Expr::IntLit(n),
        }
        .map_with(|e, extra| (e, extra.span()))
        .then_ignore(just(Token::DotDotEq))
        .then(
            select! {
                Token::IntLit(n) => cst::Expr::IntLit(n),
            }
            .map_with(|e, extra| (e, extra.span())),
        )
        .map_with(|(lo, hi), e| {
            (
                cst::Pattern::Range(Box::new(lo), cst::RangeKind::Inclusive, Box::new(hi)),
                e.span(),
            )
        });

        // Enum destructuring: Path::Variant or Path::Variant(patterns)
        // e.g., Result::Ok(val), QuestStatus::InProgress(step)
        // Requires at least 2 path segments separated by ::
        let enum_destruct = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .separated_by(just(Token::ColonColon))
            .at_least(2)
            .collect::<Vec<_>>()
            .then(
                pattern
                    .clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .or_not(),
            )
            .map_with(|(path, params), e| {
                (
                    cst::Pattern::EnumDestructure(path, params.unwrap_or_default()),
                    e.span(),
                )
            });

        // Variable binding: name (any identifier that's not _)
        // Must come after enum_destruct and wildcard in choice
        let variable = select! { Token::Ident(name) => name }
            .map_with(|name, e| (cst::Pattern::Variable(name), e.span()));

        // Single pattern (before or-pattern)
        // Order matters: try range first (int..=int), then wildcard, literal, enum, variable
        let single = choice((
            range_pat,
            wildcard,
            literal_pat,
            enum_destruct,
            variable,
        ));

        // Or-pattern: A | B | C
        // Uses the Pipe token for separator
        single
            .clone()
            .separated_by(just(Token::Pipe))
            .at_least(1)
            .collect::<Vec<_>>()
            .map_with(|pats: Vec<cst::Spanned<cst::Pattern<'src>>>, e| {
                if pats.len() == 1 {
                    pats.into_iter().next().unwrap()
                } else {
                    (cst::Pattern::Or(pats), e.span())
                }
            })
    })
}

/// Parse a formattable string token into text and expression segments.
///
/// Splits the opaque content at `{`/`}` boundaries, respecting `{{`/`}}` escapes
/// and brace nesting. Expression segments are re-lexed and re-parsed using
/// the full `parse()` entry point (wrapped as expression extraction).
fn parse_formattable_string<'src>(
    src: &'src str,
    token_span: Span,
    is_raw: bool,
) -> Vec<cst::Spanned<cst::StringSegment<'src>>> {
    // Strip the prefix ($" or $""") and suffix (" or """)
    let prefix_len = if is_raw {
        // $""" -- but may have extra quotes for N-quote raw strings
        // Count the actual opening quotes after $
        let after_dollar = &src[token_span.start + 1..token_span.end];
        let opening_quotes = after_dollar.bytes().take_while(|&b| b == b'"').count();
        1 + opening_quotes // $ + opening quotes
    } else {
        2 // $"
    };
    let suffix_len = if is_raw {
        prefix_len - 1 // Same number of quotes as opening, minus the $
    } else {
        1 // "
    };

    let content_start = token_span.start + prefix_len;
    let content_end = token_span.end - suffix_len;

    if content_start >= content_end {
        return Vec::new();
    }

    let content = &src[content_start..content_end];
    let bytes = content.as_bytes();
    let mut segments = Vec::new();
    let mut pos = 0;
    let mut text_start = 0;

    while pos < bytes.len() {
        if pos + 1 < bytes.len() && bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            // Escaped brace: {{ -> literal {
            // Include both chars as text, they represent a literal {
            pos += 2;
        } else if bytes[pos] == b'{' {
            // Flush accumulated text before this interpolation
            if pos > text_start {
                let text = &content[text_start..pos];
                let span = SimpleSpan::from(
                    (content_start + text_start)..(content_start + pos),
                );
                segments.push((cst::StringSegment::Text(text), span));
            }

            // Start of expression interpolation
            let expr_start = pos + 1;
            let mut depth = 1u32;
            pos += 1;
            while pos < bytes.len() && depth > 0 {
                if bytes[pos] == b'{' {
                    depth += 1;
                } else if bytes[pos] == b'}' {
                    depth -= 1;
                }
                if depth > 0 {
                    pos += 1;
                }
            }
            let expr_text = &content[expr_start..pos];
            let expr_src_offset = content_start + expr_start;

            // Parse the expression using the standalone helper
            if let Some(parsed_expr) = parse_expr_from_source(expr_text) {
                // Adjust the parsed expression's span by adding base offset
                let adjusted = (
                    parsed_expr.0,
                    SimpleSpan::from(
                        (expr_src_offset + parsed_expr.1.start)
                            ..(expr_src_offset + parsed_expr.1.end),
                    ),
                );
                let seg_span = SimpleSpan::from(
                    (content_start + expr_start - 1)
                        ..(content_start + pos + 1),
                );
                segments.push((
                    cst::StringSegment::Expr(Box::new(adjusted)),
                    seg_span,
                ));
            }

            // Skip closing }
            if pos < bytes.len() {
                pos += 1;
            }
            text_start = pos;
        } else if pos + 1 < bytes.len() && bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
            // Escaped brace: }} -> literal }
            pos += 2;
        } else {
            pos += 1;
        }
    }

    // Flush remaining text
    if pos > text_start {
        let text = &content[text_start..pos];
        let span =
            SimpleSpan::from((content_start + text_start)..(content_start + pos));
        segments.push((cst::StringSegment::Text(text), span));
    }

    segments
}

/// Split dialogue text into text and `{expr}` interpolation segments.
///
/// Operates on a raw source span `src[text_start..text_end]`, producing
/// `DlgTextSegment` variants. The algorithm mirrors `parse_formattable_string`
/// but works on arbitrary byte offsets into `src` rather than token-delimited
/// strings.
///
/// Handles:
/// - `{{` and `}}` as escaped literal braces (raw text passthrough)
/// - `{expr}` interpolation with brace-depth tracking
/// - `\` + newline line continuation: flushes preceding text, emits a single
///   space as a separate `Text(" ")` segment, then resumes after skipping
///   leading whitespace on the continuation line (per DLG-03)
/// - `\` + `\r\n` (Windows line endings) line continuation
fn split_dlg_text_segments<'src>(
    src: &'src str,
    text_start: usize,
    text_end: usize,
) -> Vec<cst::Spanned<cst::DlgTextSegment<'src>>> {
    let content = &src[text_start..text_end];
    let bytes = content.as_bytes();
    let mut segments = Vec::new();
    let mut pos = 0;
    let mut seg_start = 0;

    while pos < bytes.len() {
        if pos + 1 < bytes.len() && bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            // Escaped brace: {{ -> literal { (raw text passthrough)
            pos += 2;
        } else if bytes[pos] == b'{' {
            // Flush accumulated text before this interpolation
            if pos > seg_start {
                let text = &content[seg_start..pos];
                let span = SimpleSpan::from(
                    (text_start + seg_start)..(text_start + pos),
                );
                segments.push((cst::DlgTextSegment::Text(text), span));
            }

            // Start of expression interpolation
            let expr_start = pos + 1;
            let mut depth = 1u32;
            pos += 1;
            while pos < bytes.len() && depth > 0 {
                if bytes[pos] == b'{' {
                    depth += 1;
                } else if bytes[pos] == b'}' {
                    depth -= 1;
                }
                if depth > 0 {
                    pos += 1;
                }
            }
            let expr_text = &content[expr_start..pos];
            let expr_src_offset = text_start + expr_start;

            // Parse the expression using the standalone helper
            if let Some(parsed_expr) = parse_expr_from_source(expr_text) {
                let adjusted = (
                    parsed_expr.0,
                    SimpleSpan::from(
                        (expr_src_offset + parsed_expr.1.start)
                            ..(expr_src_offset + parsed_expr.1.end),
                    ),
                );
                let seg_span = SimpleSpan::from(
                    (text_start + expr_start - 1)..(text_start + pos + 1),
                );
                segments.push((
                    cst::DlgTextSegment::Expr(Box::new(adjusted)),
                    seg_span,
                ));
            }

            // Skip closing }
            if pos < bytes.len() {
                pos += 1;
            }
            seg_start = pos;
        } else if pos + 1 < bytes.len() && bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
            // Escaped brace: }} -> literal } (raw text passthrough)
            pos += 2;
        } else if bytes[pos] == b'\\' && pos + 1 < bytes.len()
            && (bytes[pos + 1] == b'\n'
                || (bytes[pos + 1] == b'\r'
                    && pos + 2 < bytes.len()
                    && bytes[pos + 2] == b'\n'))
        {
            // Line continuation: `\` at EOL joins with single space (DLG-03).
            //
            // Strategy: flush text before `\` as one segment, emit a separate
            // `Text(" ")` segment for the joining space (using a static str
            // which coerces to &'src str via lifetime subtyping), then let
            // the continuation text become the next natural segment.

            // Flush text before the backslash
            if pos > seg_start {
                let text = &content[seg_start..pos];
                let span = SimpleSpan::from(
                    (text_start + seg_start)..(text_start + pos),
                );
                segments.push((cst::DlgTextSegment::Text(text), span));
            }

            // The span for the joining space covers the `\<newline>` sequence
            let continuation_start = pos;

            // Skip backslash + newline
            if bytes[pos + 1] == b'\r' {
                pos += 3; // skip \ \r \n
            } else {
                pos += 2; // skip \ \n
            }

            // Skip leading whitespace on continuation line
            while pos < bytes.len()
                && (bytes[pos] == b' ' || bytes[pos] == b'\t')
            {
                pos += 1;
            }

            // Emit a single space segment covering the continuation span
            let space_span = SimpleSpan::from(
                (text_start + continuation_start)..(text_start + pos),
            );
            segments.push((cst::DlgTextSegment::Text(" "), space_span));

            seg_start = pos;
        } else {
            pos += 1;
        }
    }

    // Flush remaining text
    if pos > seg_start {
        let text = &content[seg_start..pos];
        let span = SimpleSpan::from(
            (text_start + seg_start)..(text_start + pos),
        );
        segments.push((cst::DlgTextSegment::Text(text), span));
    }

    segments
}

/// Parse a single expression from a token stream.
///
/// This is a standalone helper used by `parse_formattable_string` to parse
/// expression segments inside `{}` delimiters. It wraps the expression text
/// as `expr;` and uses the full `parse()` pipeline, then extracts the expression.
fn parse_expr_from_source<'src>(
    expr_src: &'src str,
) -> Option<cst::Spanned<cst::Expr<'src>>> {
    // Wrap as an expression statement so parse() can handle it
    // Since parse() works on &'src str, we need the lifetime to match.
    // We can't easily create a new string with 'src lifetime, so instead
    // we parse the token stream directly using the program_parser.
    let tokens = crate::lexer::lex(expr_src);
    let non_trivia: Vec<(Token<'src>, Span)> = tokens
        .into_iter()
        .filter(|(t, _)| {
            !matches!(
                t,
                Token::Whitespace | Token::LineComment | Token::BlockComment
            )
        })
        .collect();

    if non_trivia.is_empty() {
        return None;
    }

    let eoi = Span::from(expr_src.len()..expr_src.len());
    let token_stream =
        Stream::from_iter(non_trivia).map(eoi, |(t, s): (_, _)| (t, s));

    // Parse using program_parser with the expression source (no formattable
    // strings inside interpolation need the src parameter, so empty is fine
    // for the recursive case -- but we pass the actual source for correctness)
    let (output, _errors) = program_parser(expr_src)
        .parse(token_stream)
        .into_output_errors();

    // Extract the expression from the first item (which should be Item::Stmt(Stmt::Expr(...)))
    output.and_then(|items| {
        items.into_iter().next().and_then(|(item, span)| match item {
            cst::Item::Stmt((cst::Stmt::Expr(expr), _)) => Some(expr),
            _ => Some((cst::Expr::NullLit, span)), // fallback, shouldn't happen
        })
    })
}

/// Parse the complete program: a list of items (declarations and statements)
/// with mutual recursion between expressions, statements, and declarations.
///
/// The top-level recursive reference is the item list. Block bodies (used
/// by expressions like if/match/lambda) extract statements from items.
///
/// Expression atoms include block-bodied forms: if/else, if-let, match, block
/// expressions, lambdas, and concurrency prefix keywords.
///
/// Statement forms: let/let mut, for, while, break, continue, return, atomic,
/// and expression statements (with semicolon rules per user decisions).
///
/// Declaration forms: fn, namespace, using, const, global mut, dlg (and
/// struct, enum, contract, impl, entity, component, extern to be added in
/// Plans 02-03).
///
/// The `src` parameter provides access to the original source text for
/// formattable string interpolation parsing.
pub fn program_parser<'src, I>(
    src: &'src str,
) -> impl Parser<
    'src,
    I,
    Vec<cst::Spanned<cst::Item<'src>>>,
    extra::Err<Rich<'src, Token<'src>, Span>>,
> + Clone + 'src
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span>,
{
    recursive(|items: Recursive<
        dyn Parser<'src, I, Vec<cst::Spanned<cst::Item<'src>>>, _> + '_,
    >| {
        // =============================================================
        // Block: { items* } -> extracts stmts from items
        // The shared reference for mutual recursion.
        // Inside blocks only statements appear (wrapped as Item::Stmt).
        // =============================================================
        let block = items
            .clone()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(|item_list: Vec<cst::Spanned<cst::Item<'src>>>| {
                item_list
                    .into_iter()
                    .filter_map(|(item, _span)| match item {
                        cst::Item::Stmt(s) => Some(s),
                        // Declarations in block context: wrap as DlgDecl-like stmt
                        // or ignore. In practice, blocks only parse stmts.
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });

        // =============================================================
        // Expression parser (uses block for if/match/lambda bodies)
        // =============================================================

        // We build the expression parser as a recursive parser that also
        // references the block for block-bodied expression forms.
        let expr = {
            // We need a recursive expr for self-referential expression forms
            // (e.g., nested expressions in binary ops, calls, etc.)
            recursive(|expr: Recursive<
                dyn Parser<'src, I, cst::Spanned<cst::Expr<'src>>, _> + '_,
            >| {
                // =============================================
                // Atoms: base expressions consumed by the Pratt parser
                // =============================================

                // Literals
                let literal = select! {
                    Token::IntLit(n) => cst::Expr::IntLit(n),
                    Token::FloatLit(n) => cst::Expr::FloatLit(n),
                    Token::StringLit(s) => cst::Expr::StringLit(s),
                    Token::KwTrue => cst::Expr::BoolLit(true),
                    Token::KwFalse => cst::Expr::BoolLit(false),
                    Token::KwNull => cst::Expr::NullLit,
                    Token::KwSelf => cst::Expr::SelfLit,
                }
                .map_with(|e, extra| (e, extra.span()));

                // Raw string literal: """...""" (no payload in token, extract from source)
                let raw_string_lit = select! { Token::RawStringLit => () }
                    .map_with(move |_, e| {
                        let span: Span = e.span();
                        let text = &src[span.start..span.end];
                        (cst::Expr::StringLit(text), span)
                    });

                // Identifier or contextual keyword usable as identifier
                // Some keywords (entity, component, use, on) can appear as
                // identifiers in expression context.
                let ident_token = select! {
                    Token::Ident(name) => name,
                    Token::KwEntity => "entity",
                    Token::KwComponent => "component",
                    Token::KwUse => "use",
                    Token::KwOn => "on",
                };

                // Identifier (may be start of path a::b::c or root path ::a::b)
                let ident_or_path = just(Token::ColonColon).or_not()
                    .then(
                        ident_token
                            .map_with(|name, e| (name, e.span()))
                            .separated_by(just(Token::ColonColon))
                            .at_least(1)
                            .collect::<Vec<_>>(),
                    )
                    .map_with(|(root_prefix, segments): (Option<_>, Vec<cst::Spanned<&'src str>>), e| {
                        if root_prefix.is_none() && segments.len() == 1 {
                            (cst::Expr::Ident(segments[0].0), e.span())
                        } else {
                            // Root prefix (::) is represented as an empty first segment
                            // or we just use Path with segments (root distinguished by context)
                            (cst::Expr::Path(segments), e.span())
                        }
                    });

                // Parenthesized expression: (expr)
                let paren_expr = expr
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .map_with(|inner: cst::Spanned<cst::Expr<'src>>, e| (inner.0, e.span()));

                // Array literal: [expr, expr, ...]
                let array_lit = expr
                    .clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map_with(|items, e| (cst::Expr::ArrayLit(items), e.span()));

                // Half-open and full range prefixes: ..expr, ..=expr, ..
                let range_prefix = choice((
                    just(Token::DotDotEq)
                        .ignore_then(expr.clone())
                        .map_with(|rhs, e| {
                            (
                                cst::Expr::Range(
                                    None,
                                    cst::RangeKind::Inclusive,
                                    Some(Box::new(rhs)),
                                ),
                                e.span(),
                            )
                        }),
                    just(Token::DotDot)
                        .ignore_then(expr.clone().or_not())
                        .map_with(|rhs, e| {
                            (
                                cst::Expr::Range(
                                    None,
                                    cst::RangeKind::Exclusive,
                                    rhs.map(Box::new),
                                ),
                                e.span(),
                            )
                        }),
                ));

                // =============================================
                // Block-bodied expression atoms (Plan 03)
                // =============================================

                // Block expression: { stmts }
                let block_expr = block
                    .clone()
                    .map_with(|body, e| (cst::Expr::Block(body), e.span()));

                // If/else expression (also handles if-let)
                // Uses recursive definition for else-if chains
                let if_expr = recursive(|if_expr: Recursive<
                    dyn Parser<'src, I, cst::Spanned<cst::Expr<'src>>, _> + '_,
                >| {
                    // if let Pattern = expr { block } [else { block } | else if ...]
                    let if_let = just(Token::KwIf)
                        .ignore_then(just(Token::KwLet))
                        .ignore_then(pattern())
                        .then_ignore(just(Token::Eq))
                        .then(expr.clone())
                        .then(block.clone())
                        .then(
                            just(Token::KwElse)
                                .ignore_then(
                                    if_expr
                                        .clone()
                                        .or(block.clone().map_with(|b, e| {
                                            (cst::Expr::Block(b), e.span())
                                        })),
                                )
                                .or_not(),
                        )
                        .map_with(|(((pat, val), then_b), else_b), e| {
                            (
                                cst::Expr::IfLet {
                                    pattern: Box::new(pat),
                                    value: Box::new(val),
                                    then_block: then_b,
                                    else_block: else_b.map(Box::new),
                                },
                                e.span(),
                            )
                        });

                    // if expr { block } [else { block } | else if ...]
                    let if_cond = just(Token::KwIf)
                        .ignore_then(expr.clone())
                        .then(block.clone())
                        .then(
                            just(Token::KwElse)
                                .ignore_then(
                                    if_expr
                                        .clone()
                                        .or(block.clone().map_with(|b, e| {
                                            (cst::Expr::Block(b), e.span())
                                        })),
                                )
                                .or_not(),
                        )
                        .map_with(|((cond, then_b), else_b), e| {
                            (
                                cst::Expr::If {
                                    condition: Box::new(cond),
                                    then_block: then_b,
                                    else_block: else_b.map(Box::new),
                                },
                                e.span(),
                            )
                        });

                    // Try if-let first (starts with `if let`), then if-cond
                    if_let.or(if_cond)
                });

                // Match expression: match expr { pattern => { body }, ... }
                let match_arm = pattern()
                    .then_ignore(just(Token::FatArrow))
                    .then(block.clone())
                    .map_with(|(pat, body), e| {
                        (
                            cst::MatchArm {
                                pattern: pat,
                                body,
                            },
                            e.span(),
                        )
                    });

                let match_expr = just(Token::KwMatch)
                    .ignore_then(expr.clone())
                    .then(
                        match_arm
                            .separated_by(just(Token::Comma).or_not())
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LBrace), just(Token::RBrace)),
                    )
                    .map_with(|(scrutinee, arms), e| {
                        (
                            cst::Expr::Match {
                                scrutinee: Box::new(scrutinee),
                                arms,
                            },
                            e.span(),
                        )
                    });

                // Lambda: fn(params) [-> type] { body }
                let lambda_param = select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span()))
                    .then(just(Token::Colon).ignore_then(type_expr()).or_not())
                    .map_with(|(name, ty), e| {
                        (cst::LambdaParam { name, ty }, e.span())
                    });

                let lambda = just(Token::KwFn)
                    .ignore_then(
                        lambda_param
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LParen), just(Token::RParen)),
                    )
                    .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
                    .then(block.clone())
                    .map_with(|((params, ret_type), body), e| {
                        (
                            cst::Expr::Lambda {
                                params,
                                return_type: ret_type.map(Box::new),
                                body,
                            },
                            e.span(),
                        )
                    });

                // Concurrency prefix keywords: spawn, detached, join, cancel, defer, try
                let spawn_expr = just(Token::KwSpawn)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Spawn(Box::new(e)), extra.span()));
                let detached_expr = just(Token::KwDetached)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Detached(Box::new(e)), extra.span()));
                let join_expr = just(Token::KwJoin)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Join(Box::new(e)), extra.span()));
                let cancel_expr = just(Token::KwCancel)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Cancel(Box::new(e)), extra.span()));
                let defer_expr = just(Token::KwDefer)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Defer(Box::new(e)), extra.span()));
                let try_expr = just(Token::KwTry)
                    .ignore_then(expr.clone())
                    .map_with(|e, extra| (cst::Expr::Try(Box::new(e)), extra.span()));

                // Formattable string: $"Hello {name}!" or $"""Hello {name}!"""
                // Parse the opaque token by splitting into text/expr segments
                let formattable_string = select! {
                    Token::FormattableStringLit => false,
                    Token::FormattableRawStringLit => true,
                }
                .map_with(move |is_raw, e| {
                    let span: Span = e.span();
                    let segments = parse_formattable_string(src, span, is_raw);
                    if is_raw {
                        (cst::Expr::FormattableRawString(segments), span)
                    } else {
                        (cst::Expr::FormattableString(segments), span)
                    }
                });

                // Generic call: f<T>(args) -- disambiguation from comparison
                // Try parsing ident<type_args>(call_args) as a unit.
                // If it fails, chumsky backtracks and ident_or_path handles it.
                // Named arg for generic_call (same pattern as regular call args)
                let gc_arg = select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span()))
                    .then_ignore(just(Token::Colon))
                    .or_not()
                    .then(expr.clone())
                    .map_with(|(name, value), e| {
                        (cst::Arg { name, value }, e.span())
                    });

                let generic_call = select! { Token::Ident(name) => name }
                    .map_with(|name, e| (name, e.span()))
                    .then(
                        type_expr()
                            .separated_by(just(Token::Comma))
                            .at_least(1)
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::Lt), just(Token::Gt)),
                    )
                    .then(
                        // Require immediate ( after > to disambiguate from comparison
                        gc_arg
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LParen), just(Token::RParen)),
                    )
                    .map_with(|((name, type_args), call_args), e| {
                        (
                            cst::Expr::GenericCall(
                                Box::new((cst::Expr::Ident(name.0), name.1)),
                                type_args,
                                call_args,
                            ),
                            e.span(),
                        )
                    });

                // Atom: all expression forms that can start an expression
                // Order matters: try keyword-prefixed atoms first, then
                // block-bodied, then simple atoms.
                // Generic call must come before ident_or_path so chumsky
                // tries the f<T>(args) parse first.
                let atom = choice((
                    // Concurrency prefix keywords
                    spawn_expr,
                    detached_expr,
                    join_expr,
                    cancel_expr,
                    defer_expr,
                    try_expr,
                    // Block-bodied expressions
                    if_expr,
                    match_expr,
                    lambda,
                    block_expr,
                    // Formattable strings
                    formattable_string,
                    // Simple atoms
                    literal,
                    raw_string_lit,
                    array_lit,
                    range_prefix,
                    paren_expr,
                    // Generic call before ident to try f<T>(args) first
                    generic_call,
                    ident_or_path,
                ))
                .labelled("expression");

                // =============================================
                // Argument list for calls: (a, b, name: val)
                // =============================================

                let arg = select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span()))
                    .then_ignore(just(Token::Colon))
                    .or_not()
                    .then(expr.clone())
                    .map_with(|(name, value), e| {
                        (cst::Arg { name, value }, e.span())
                    });

                let args = arg
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen));

                // =============================================
                // Postfix chain: member access, bracket access, calls, ?, !
                // =============================================

                let postfix_chain = atom.foldl_with(
                    choice((
                        // Generic method call: .method<T>(args) -- before regular member access
                        just(Token::Dot)
                            .ignore_then(
                                select! { Token::Ident(name) => name }
                                    .map_with(|name, e| (name, e.span())),
                            )
                            .then(
                                type_expr()
                                    .separated_by(just(Token::Comma))
                                    .at_least(1)
                                    .collect::<Vec<_>>()
                                    .delimited_by(just(Token::Lt), just(Token::Gt)),
                            )
                            .then(args.clone())
                            .map(|((field, type_args), call_args)| {
                                ExprPostfix::GenericMethod(field, type_args, call_args)
                            }),
                        // Member access: .field or .method(args)
                        just(Token::Dot)
                            .ignore_then(
                                select! { Token::Ident(name) => name }
                                    .map_with(|name, e| (name, e.span())),
                            )
                            .then(args.clone().or_not())
                            .map(|(field, maybe_args)| {
                                ExprPostfix::MemberOrMethod(field, maybe_args)
                            }),
                        // Bracket access: [expr]
                        expr.clone()
                            .delimited_by(just(Token::LBracket), just(Token::RBracket))
                            .map(ExprPostfix::Bracket),
                        // Call: (args)
                        args.clone().map(ExprPostfix::Call),
                        // Brace construction: { field: value, ... }
                        // Named args required (at least name: expr) to disambiguate from block
                        select! { Token::Ident(name) => name }
                            .map_with(|n, e| (n, e.span()))
                            .then_ignore(just(Token::Colon))
                            .then(expr.clone())
                            .map_with(|(name, value), e| {
                                (cst::Arg { name: Some(name), value }, e.span())
                            })
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LBrace), just(Token::RBrace))
                            .map(ExprPostfix::BraceConstruct),
                        // Postfix ?: null propagation
                        just(Token::Question).to(ExprPostfix::NullPropagate),
                        // Postfix !: unwrap
                        just(Token::Bang).to(ExprPostfix::Unwrap),
                    ))
                    .repeated(),
                    |left: cst::Spanned<cst::Expr<'src>>,
                     postfix: ExprPostfix<'src>,
                     e| {
                        let span = e.span();
                        match postfix {
                            ExprPostfix::MemberOrMethod(field, None) => {
                                (cst::Expr::MemberAccess(Box::new(left), field), span)
                            }
                            ExprPostfix::MemberOrMethod(field, Some(a)) => {
                                let member_span =
                                    SimpleSpan::from(left.1.start..field.1.end);
                                let member = (
                                    cst::Expr::MemberAccess(Box::new(left), field),
                                    member_span,
                                );
                                (cst::Expr::Call(Box::new(member), a), span)
                            }
                            ExprPostfix::Bracket(inner) => (
                                cst::Expr::BracketAccess(
                                    Box::new(left),
                                    Box::new(inner),
                                ),
                                span,
                            ),
                            ExprPostfix::GenericMethod(field, type_args, call_args) => {
                                let member_span =
                                    SimpleSpan::from(left.1.start..field.1.end);
                                let member = (
                                    cst::Expr::MemberAccess(Box::new(left), field),
                                    member_span,
                                );
                                (
                                    cst::Expr::GenericCall(
                                        Box::new(member),
                                        type_args,
                                        call_args,
                                    ),
                                    span,
                                )
                            }
                            ExprPostfix::Call(a) => {
                                (cst::Expr::Call(Box::new(left), a), span)
                            }
                            ExprPostfix::BraceConstruct(a) => {
                                (cst::Expr::Call(Box::new(left), a), span)
                            }
                            ExprPostfix::NullPropagate => (
                                cst::Expr::UnaryPostfix(
                                    Box::new(left),
                                    cst::PostfixOp::NullPropagate,
                                ),
                                span,
                            ),
                            ExprPostfix::Unwrap => (
                                cst::Expr::UnaryPostfix(
                                    Box::new(left),
                                    cst::PostfixOp::Unwrap,
                                ),
                                span,
                            ),
                        }
                    },
                );

                // =============================================
                // Pratt parser for prefix and infix operators
                // =============================================

                let pratt_expr = postfix_chain
                    .pratt((
                        // --- Prefix operators (highest binding) ---
                        prefix(
                            14,
                            just(Token::Minus),
                            |_, rhs: cst::Spanned<cst::Expr<'src>>, e| {
                                (
                                    cst::Expr::UnaryPrefix(
                                        cst::PrefixOp::Neg,
                                        Box::new(rhs),
                                    ),
                                    e.span(),
                                )
                            },
                        ),
                        prefix(14, just(Token::Bang), |_, rhs, e| {
                            (
                                cst::Expr::UnaryPrefix(
                                    cst::PrefixOp::Not,
                                    Box::new(rhs),
                                ),
                                e.span(),
                            )
                        }),
                        prefix(14, just(Token::Caret), |_, rhs, e| {
                            (cst::Expr::FromEnd(Box::new(rhs)), e.span())
                        }),
                        // --- Multiplicative: *, /, % ---
                        infix(left(12), just(Token::Star), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Mul,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(12), just(Token::Slash), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Div,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(12), just(Token::Percent), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Mod,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Additive: +, - ---
                        infix(left(11), just(Token::Plus), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Add,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(11), just(Token::Minus), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Sub,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // NOTE: Range (.., ..=) is handled OUTSIDE the Pratt
                        // parser to support half-open forms like `5..` where
                        // the RHS is optional.
                        // --- Comparison: <, >, <=, >= ---
                        infix(left(8), just(Token::Lt), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Lt,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(8), just(Token::Gt), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Gt,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(8), just(Token::LtEq), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::LtEq,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(8), just(Token::GtEq), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::GtEq,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Equality: ==, != ---
                        infix(left(7), just(Token::EqEq), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Eq,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(left(7), just(Token::BangEq), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::NotEq,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Bitwise AND: & ---
                        infix(left(6), just(Token::Amp), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::BitAnd,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Bitwise OR: | ---
                        infix(left(5), just(Token::Pipe), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::BitOr,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Logical AND: && ---
                        infix(left(4), just(Token::AmpAmp), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::And,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Logical OR: || ---
                        infix(left(3), just(Token::PipePipe), |l, _, r, e| {
                            (
                                cst::Expr::Binary(
                                    Box::new(l),
                                    cst::BinaryOp::Or,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        // --- Assignment: =, +=, -=, *=, /=, %= ---
                        infix(right(2), just(Token::Eq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::Assign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(right(2), just(Token::PlusEq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::AddAssign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(right(2), just(Token::MinusEq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::SubAssign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(right(2), just(Token::StarEq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::MulAssign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(right(2), just(Token::SlashEq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::DivAssign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                        infix(right(2), just(Token::PercentEq), |l, _, r, e| {
                            (
                                cst::Expr::Assign(
                                    Box::new(l),
                                    cst::AssignOp::ModAssign,
                                    Box::new(r),
                                ),
                                e.span(),
                            )
                        }),
                    ));

                // =============================================
                // Range expressions: handled outside Pratt to allow
                // half-open forms like `5..` where RHS is optional.
                // After parsing the base expression via Pratt, check
                // for an optional trailing `..` or `..=`.
                // =============================================
                let range_suffix = choice((
                    just(Token::DotDotEq).to(cst::RangeKind::Inclusive),
                    just(Token::DotDot).to(cst::RangeKind::Exclusive),
                ))
                .then(pratt_expr.clone().or_not());

                pratt_expr
                    .then(range_suffix.or_not())
                    .map_with(
                        |pair: (
                            cst::Spanned<cst::Expr<'src>>,
                            Option<(
                                cst::RangeKind,
                                Option<cst::Spanned<cst::Expr<'src>>>,
                            )>,
                        ),
                         e| {
                            let (lhs, range) = pair;
                            if let Some((kind, rhs)) = range {
                                (
                                    cst::Expr::Range(
                                        Some(Box::new(lhs)),
                                        kind,
                                        rhs.map(Box::new),
                                    ),
                                    e.span(),
                                )
                            } else {
                                lhs
                            }
                        },
                    )
                    .boxed()
            })
        };

        // =============================================================
        // Dialogue parsers (uses expr, block for code escapes;
        // recursive dlg_body for nested dialogue blocks)
        // =============================================================

        // The dialogue parsers are defined within program_parser's
        // recursive() scope so they can access expr, stmts, and block
        // for mutual recursion between dialogue and code.

        // Build a single-statement parser for $ statement; inside dialogue.
        // This handles `let` declarations and expression statements with ;.
        let dlg_code_stmt = {
            let let_s = just(Token::KwLet)
                .ignore_then(just(Token::KwMut).or_not())
                .then(
                    select! { Token::Ident(name) => name }
                        .map_with(|n, e| (n, e.span())),
                )
                .then(
                    just(Token::Colon).ignore_then(type_expr()).or_not(),
                )
                .then_ignore(just(Token::Eq))
                .then(expr.clone())
                .then_ignore(just(Token::Semi))
                .map_with(|(((mutable, name), ty), value), e| {
                    (
                        cst::Stmt::Let {
                            mutable: mutable.is_some(),
                            name,
                            ty,
                            value,
                        },
                        e.span(),
                    )
                });
            let expr_s = expr
                .clone()
                .then_ignore(just(Token::Semi))
                .map_with(|expression, e| {
                    (cst::Stmt::Expr(expression), e.span())
                });
            let atomic_s = just(Token::KwAtomic)
                .ignore_then(block.clone())
                .map_with(|body, e| {
                    (cst::Stmt::Atomic(body), e.span())
                });
            choice((let_s, atomic_s, expr_s))
        };

        let dlg_body = recursive(
            |dlg_lines: Recursive<
                dyn Parser<
                        'src,
                        I,
                        Vec<cst::Spanned<cst::DlgLine<'src>>>,
                        _,
                    > + '_,
            >| {
                // --- Localization key parser: #ident ---
                let loc_key = just(Token::Hash)
                    .ignore_then(
                        select! { Token::Ident(name) => name }
                            .map_with(|n, e| (n, e.span())),
                    );

                // --- Transition parser: -> name or -> name(args) ---
                let transition = just(Token::Arrow)
                    .ignore_then(
                        select! { Token::Ident(name) => name }
                            .map_with(|n, e| (n, e.span())),
                    )
                    .then(
                        expr.clone()
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(
                                just(Token::LParen),
                                just(Token::RParen),
                            )
                            .or_not(),
                    )
                    .map_with(|(target, args), e| {
                        (
                            cst::DlgLine::Transition((
                                cst::DlgTransition { target, args },
                                e.span(),
                            )),
                            e.span(),
                        )
                    });

                // --- Dialogue if parser (after $ if consumed) ---
                // Recursive for else-if chains
                let dlg_if = recursive(
                    |dlg_if_rec: Recursive<
                        dyn Parser<
                                'src,
                                I,
                                cst::Spanned<cst::DlgIf<'src>>,
                                _,
                            > + '_,
                    >| {
                        expr.clone()
                            .then(
                                dlg_lines
                                    .clone()
                                    .delimited_by(
                                        just(Token::LBrace),
                                        just(Token::RBrace),
                                    ),
                            )
                            .then(
                                just(Token::KwElse)
                                    .ignore_then(choice((
                                        // else if ...
                                        just(Token::KwIf)
                                            .ignore_then(dlg_if_rec.clone())
                                            .map_with(|inner, e| {
                                                (
                                                    cst::DlgElse::ElseIf(
                                                        inner.0,
                                                    ),
                                                    e.span(),
                                                )
                                            }),
                                        // else { ... }
                                        dlg_lines
                                            .clone()
                                            .delimited_by(
                                                just(Token::LBrace),
                                                just(Token::RBrace),
                                            )
                                            .map_with(|lines, e| {
                                                (
                                                    cst::DlgElse::Else(lines),
                                                    e.span(),
                                                )
                                            }),
                                    )))
                                    .or_not(),
                            )
                            .map_with(
                                |((condition, then_block), else_block), e| {
                                    (
                                        cst::DlgIf {
                                            condition: Box::new(condition),
                                            then_block,
                                            else_block: else_block
                                                .map(Box::new),
                                        },
                                        e.span(),
                                    )
                                },
                            )
                    },
                );

                // --- Dialogue match parser (after $ match consumed) ---
                let dlg_match_arm = pattern()
                    .then_ignore(just(Token::FatArrow))
                    .then(
                        dlg_lines.clone().delimited_by(
                            just(Token::LBrace),
                            just(Token::RBrace),
                        ),
                    )
                    .map_with(|(pat, body), e| {
                        (
                            cst::DlgMatchArm {
                                pattern: pat,
                                body,
                            },
                            e.span(),
                        )
                    });

                let dlg_match = expr
                    .clone()
                    .then(
                        dlg_match_arm
                            .separated_by(just(Token::Comma).or_not())
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(
                                just(Token::LBrace),
                                just(Token::RBrace),
                            ),
                    )
                    .map_with(|(scrutinee, arms), e| {
                        (
                            cst::DlgMatch {
                                scrutinee: Box::new(scrutinee),
                                arms,
                            },
                            e.span(),
                        )
                    });

                // --- Dialogue choice parser (after $ choice consumed) ---
                let choice_arm = select! { Token::StringLit(s) => s }
                    .map_with(|s, e| (s, e.span()))
                    .then(loc_key.clone().or_not())
                    .then(
                        dlg_lines.clone().delimited_by(
                            just(Token::LBrace),
                            just(Token::RBrace),
                        ),
                    )
                    .map_with(|((label, loc_key), body), e| {
                        (
                            cst::DlgChoiceArm {
                                label,
                                loc_key,
                                body,
                            },
                            e.span(),
                        )
                    });

                let dlg_choice = choice_arm
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace))
                    .map_with(|arms, e| {
                        (cst::DlgChoice { arms }, e.span())
                    });

                // --- Dollar escape parser ---
                // $ if | $ match | $ choice | $ { block } | $ stmt;
                let dlg_escape = just(Token::Dollar).ignore_then(
                    choice((
                        // $ if condition { dlg } [else { dlg }]
                        just(Token::KwIf)
                            .ignore_then(dlg_if)
                            .map_with(|dif, e| {
                                (cst::DlgLine::If(dif), e.span())
                            }),
                        // $ match expr { arms }
                        just(Token::KwMatch)
                            .ignore_then(dlg_match)
                            .map_with(|dm, e| {
                                (cst::DlgLine::Match(dm), e.span())
                            }),
                        // $ choice { arms }
                        // "choice" is Ident("choice"), not a keyword
                        select! { Token::Ident("choice") => () }
                            .ignore_then(dlg_choice)
                            .map_with(|dc, e| {
                                (cst::DlgLine::Choice(dc), e.span())
                            }),
                        // $ { block } -- code block escape
                        items
                            .clone()
                            .delimited_by(
                                just(Token::LBrace),
                                just(Token::RBrace),
                            )
                            .map(|item_list: Vec<cst::Spanned<cst::Item<'src>>>| {
                                item_list
                                    .into_iter()
                                    .filter_map(|(item, _)| match item {
                                        cst::Item::Stmt(s) => Some(s),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .map_with(|body, e| {
                                (
                                    cst::DlgLine::CodeEscape((
                                        cst::DlgEscape::Block(body),
                                        e.span(),
                                    )),
                                    e.span(),
                                )
                            }),
                        // $ statement; -- single code statement
                        // (catch-all for $, must be last)
                        dlg_code_stmt.clone().map_with(|s, e| {
                            (
                                cst::DlgLine::CodeEscape((
                                    cst::DlgEscape::Statement(Box::new(s)),
                                    e.span(),
                                )),
                                e.span(),
                            )
                        }),
                    )),
                );

                // --- Speaker lines and text lines ---
                //
                // Dialogue text uses span-based source slicing:
                // 1. Consume non-sigil tokens to determine text boundaries
                // 2. Use first..last token spans to slice src
                // 3. Pass to split_dlg_text_segments for {expr} interpolation
                //
                // Dialogue sigils (start a new dialogue line):
                //   @ (speaker), $ (escape), -> (transition), } (block end)
                //
                // Non-sigil tokens are consumed as "text" tokens. The actual
                // text content is extracted from src using the token spans.

                // Match any token that is NOT a dialogue sigil.
                // Sigils at brace depth 0: @ $ -> }
                //
                // Brace-aware: when `{` is encountered inside text,
                // consume tokens until the matching `}` (tracking
                // depth) so that `{expr}` interpolation braces don't
                // terminate text collection prematurely.
                let is_sigil_no_rbrace = select! {
                    Token::At => (),
                    Token::Dollar => (),
                    Token::Arrow => (),
                };

                // Balanced brace group: { ... } consumed as a unit.
                // Collects LBrace, all inner tokens (recursively for
                // nested braces), and the matching RBrace.
                let brace_group = recursive(
                    |brace_group: Recursive<
                        dyn Parser<
                                'src,
                                I,
                                Vec<(Token<'src>, Span)>,
                                _,
                            > + '_,
                    >| {
                        just(Token::LBrace)
                            .map_with(|t: Token<'src>, e| (t, e.span()))
                            .then(
                                choice((
                                    // Nested brace group
                                    brace_group.clone(),
                                    // Any single token except LBrace/RBrace
                                    any()
                                        .and_is(
                                            just(Token::LBrace)
                                                .or(just(Token::RBrace))
                                                .not(),
                                        )
                                        .map_with(|t: Token<'src>, e| {
                                            vec![(t, e.span())]
                                        }),
                                ))
                                .repeated()
                                .collect::<Vec<_>>(),
                            )
                            .then(
                                just(Token::RBrace)
                                    .map_with(|t: Token<'src>, e| (t, e.span())),
                            )
                            .map(|((open, inner_groups), close)| {
                                let mut tokens = vec![open];
                                for group in inner_groups {
                                    tokens.extend(group);
                                }
                                tokens.push(close);
                                tokens
                            })
                    },
                );

                // A single non-sigil token (not @, $, ->, }, or {)
                let non_sigil_single = any()
                    .and_is(is_sigil_no_rbrace.not())
                    .and_is(just(Token::RBrace).not())
                    .and_is(just(Token::LBrace).not())
                    .map_with(|t: Token<'src>, e| vec![(t, e.span())]);

                // A "text token unit" is either a balanced brace group
                // or a single non-sigil token
                let text_token_unit = brace_group.or(non_sigil_single);

                // Collect a sequence of text token units, then flatten
                let text_tokens = text_token_unit
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<Vec<(Token<'src>, Span)>>>()
                    .map(|groups| {
                        let mut all = Vec::new();
                        for group in groups {
                            all.extend(group);
                        }
                        all
                    });

                // Helper closure: extract text segments and optional loc_key
                // from a sequence of (Token, Span) pairs.
                // If the last two tokens are Hash + Ident, they form a loc_key.
                // The text is extracted from src[first_start..text_end] and
                // passed to split_dlg_text_segments.
                //
                // Returns (segments, loc_key).
                let extract_text_and_loc_key = move |tokens: &[(Token<'src>, Span)]|
                 -> (
                    Vec<cst::Spanned<cst::DlgTextSegment<'src>>>,
                    Option<cst::Spanned<&'src str>>,
                ) {
                    if tokens.is_empty() {
                        return (Vec::new(), None);
                    }

                    // Check for trailing #key: last two tokens = Hash + Ident
                    let (text_tokens_slice, loc_key) = if tokens.len() >= 2 {
                        let second_last = &tokens[tokens.len() - 2];
                        let last = &tokens[tokens.len() - 1];
                        if matches!(second_last.0, Token::Hash) {
                            if let Token::Ident(name) = last.0 {
                                (
                                    &tokens[..tokens.len() - 2],
                                    Some((name, last.1)),
                                )
                            } else {
                                (tokens, None)
                            }
                        } else {
                            (tokens, None)
                        }
                    } else {
                        (tokens, None)
                    };

                    if text_tokens_slice.is_empty() {
                        return (Vec::new(), loc_key);
                    }

                    // Determine the text span from first to last token
                    let text_start = text_tokens_slice[0].1.start;
                    let text_end =
                        text_tokens_slice[text_tokens_slice.len() - 1].1.end;

                    let segments =
                        split_dlg_text_segments(src, text_start, text_end);

                    (segments, loc_key)
                };

                // Speaker line: @ speaker [text] [#key]
                //
                // After @speaker, if non-sigil tokens follow, they are text
                // content. The text is extracted using span slicing.
                // In the sigil-delimited model, @speaker followed by text on
                // subsequent lines (before next sigil) merges into SpeakerLine.
                let speaker_line = just(Token::At)
                    .ignore_then(
                        select! { Token::Ident(name) => name }
                            .map_with(|n, e| (n, e.span())),
                    )
                    .then(text_tokens.clone().or_not())
                    .map_with(
                        move |(speaker, maybe_tokens), e| match maybe_tokens {
                            None => {
                                // No text tokens -- standalone speaker tag
                                (
                                    cst::DlgLine::SpeakerTag(speaker),
                                    e.span(),
                                )
                            }
                            Some(tokens) => {
                                let (text, loc_key) =
                                    extract_text_and_loc_key(&tokens);
                                (
                                    cst::DlgLine::SpeakerLine {
                                        speaker,
                                        text,
                                        loc_key,
                                    },
                                    e.span(),
                                )
                            }
                        },
                    );

                // Text line: any sequence of non-sigil tokens not starting
                // with @ (caught by speaker_line), $ (caught by dlg_escape),
                // or -> (caught by transition).
                let text_line = text_tokens
                    .clone()
                    .map_with(
                        move |tokens: Vec<(Token<'src>, Span)>, e| {
                            let (text, loc_key) =
                                extract_text_and_loc_key(&tokens);
                            (
                                cst::DlgLine::TextLine { text, loc_key },
                                e.span(),
                            )
                        },
                    );

                // --- Dialogue line: one of the dialogue forms ---
                // Order: speaker (@), escape ($), transition (->), text
                let dlg_line = choice((
                    speaker_line,
                    dlg_escape,
                    transition,
                    text_line,
                ));

                dlg_line.repeated().collect::<Vec<_>>()
            },
        );

        // --- dlg declaration parser ---
        // dlg name[(params)] { dlg_body }

        let dlg_param = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .map_with(|(name, ty), e| (cst::Param { name, ty }, e.span()));

        let dlg_param_list = dlg_param
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let dlg_decl = just(Token::KwDlg)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(dlg_param_list.or_not())
            .then(
                dlg_body
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with(|((name, params), body), e| {
                (
                    cst::DlgDecl {
                        name,
                        params,
                        body,
                    },
                    e.span(),
                )
            });

        // =============================================================
        // Statement parser (uses expr for values, block for bodies)
        // =============================================================

        // let [mut] name [: type] = expr;
        let let_stmt = just(Token::KwLet)
            .ignore_then(just(Token::KwMut).or_not())
            .then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(just(Token::Colon).ignore_then(type_expr()).or_not())
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .then_ignore(just(Token::Semi))
            .map_with(|(((mutable, name), ty), value), e| {
                (
                    cst::Stmt::Let {
                        mutable: mutable.is_some(),
                        name,
                        ty,
                        value,
                    },
                    e.span(),
                )
            });

        // for name in expr { body }
        let for_stmt = just(Token::KwFor)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then_ignore(just(Token::KwIn))
            .then(expr.clone())
            .then(block.clone())
            .map_with(|((binding, iterable), body), e| {
                (
                    cst::Stmt::For {
                        binding,
                        iterable,
                        body,
                    },
                    e.span(),
                )
            });

        // while expr { body }
        let while_stmt = just(Token::KwWhile)
            .ignore_then(expr.clone())
            .then(block.clone())
            .map_with(|(condition, body), e| {
                (cst::Stmt::While { condition, body }, e.span())
            });

        // break [expr];
        let break_stmt = just(Token::KwBreak)
            .ignore_then(expr.clone().or_not())
            .then_ignore(just(Token::Semi))
            .map_with(|value, e| (cst::Stmt::Break(value), e.span()));

        // continue;
        let continue_stmt = just(Token::KwContinue)
            .ignore_then(just(Token::Semi))
            .to(cst::Stmt::Continue)
            .map_with(|s, e| (s, e.span()));

        // return [expr];
        let return_stmt = just(Token::KwReturn)
            .ignore_then(expr.clone().or_not())
            .then_ignore(just(Token::Semi))
            .map_with(|value, e| (cst::Stmt::Return(value), e.span()));

        // atomic { body }
        let atomic_stmt = just(Token::KwAtomic)
            .ignore_then(block.clone())
            .map_with(|body, e| (cst::Stmt::Atomic(body), e.span()));

        // Expression statement: expr; or expr (without ; for block-bodied)
        // Block-bodied constructs (if, match, for, while, blocks, lambdas)
        // don't require trailing semicolons.
        let expr_stmt = expr
            .clone()
            .then(just(Token::Semi).or_not())
            .map_with(|(expression, _semi), e| {
                (cst::Stmt::Expr(expression), e.span())
            });

        // Transition statement: -> target(args);
        // Used in on handlers and code contexts
        let transition_stmt = just(Token::Arrow)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .or_not(),
            )
            .then_ignore(just(Token::Semi).or_not())
            .map_with(|(target, args), e| {
                (
                    cst::Stmt::Transition((
                        cst::DlgTransition { target, args },
                        e.span(),
                    )),
                    e.span(),
                )
            });

        // Statement: try all specific forms first, then fall back to expr stmt.
        let stmt = choice((
            let_stmt,
            for_stmt,
            while_stmt,
            break_stmt,
            continue_stmt,
            return_stmt,
            atomic_stmt,
            transition_stmt,
            expr_stmt,
        ))
        .labelled("statement");

        // =============================================================
        // Declaration parsers (Phase 4)
        // =============================================================

        // --- Visibility parser ---
        let visibility = choice((
            just(Token::KwPub).to(cst::Visibility::Pub),
            just(Token::KwPriv).to(cst::Visibility::Priv),
        ));

        // --- Attribute parser ---
        // Single attribute argument: named (ident: expr) or positional (expr)
        let attr_arg = choice((
            // Named arg: ident: expr
            select! { Token::Ident(name) => name }
                .map_with(|n, e| (n, e.span()))
                .then_ignore(just(Token::Colon))
                .then(expr.clone())
                .map_with(|(name, value), e| {
                    (cst::AttrArg::Named(name, value), e.span())
                }),
            // Positional arg: expr
            expr.clone().map_with(|value, e| {
                (cst::AttrArg::Positional(value), e.span())
            }),
        ));

        // Single attribute: Name or Name(args)
        let single_attr = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .then(
                attr_arg
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .or_not(),
            )
            .map(|(name, args)| cst::Attribute {
                name,
                args: args.unwrap_or_default(),
            });

        // Attribute block: [Attr1, Attr2(arg)]
        let attr_block = single_attr
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map_with(|attrs, e| (attrs, e.span()));

        // Stacked attributes: [Attr1] [Attr2] ...
        let attrs = attr_block.repeated().collect::<Vec<_>>();

        // --- Parameter list parser (reusable) ---
        // Accept contextual keywords (entity, component, use, on) as param names
        let fn_param = select! {
                Token::Ident(name) => name,
                Token::KwEntity => "entity",
                Token::KwComponent => "component",
                Token::KwUse => "use",
                Token::KwOn => "on",
            }
            .map_with(|n, e| (n, e.span()))
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .map_with(|(name, ty), e| (cst::Param { name, ty }, e.span()));

        let fn_param_list = fn_param
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        // --- fn_decl parser ---
        // fn name [<generics>] (params) [-> type] { body }
        let fn_decl = just(Token::KwFn)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(fn_param_list.clone())
            .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
            .then(block.clone())
            .map(|((((name, generics), params), return_type), body)| {
                cst::FnDecl {
                    attrs: Vec::new(),
                    vis: None,
                    name,
                    generics,
                    params,
                    return_type,
                    body,
                }
            });

        // --- Qualified name parser (for namespace/using) ---
        let qualified_name = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .separated_by(just(Token::ColonColon))
            .at_least(1)
            .collect::<Vec<_>>();

        // --- namespace_decl parser ---
        // Two forms:
        // 1. Declarative: namespace qualified::name ;
        // 2. Block: namespace name { items }
        let namespace_decl = just(Token::KwNamespace)
            .ignore_then(qualified_name.clone())
            .then(choice((
                // Block form: { items }
                items.clone()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace))
                    .map(Some),
                // Declarative form: ;
                just(Token::Semi).map(|_| None),
            )))
            .map_with(|(name, maybe_body), e| {
                let decl = match maybe_body {
                    Some(body) => cst::NamespaceDecl::Block(name, body),
                    None => cst::NamespaceDecl::Declarative(name),
                };
                (
                    cst::Item::Namespace((decl, e.span())),
                    e.span(),
                )
            });

        // --- using_decl parser ---
        // using [alias =] qualified::name ;
        let using_decl = just(Token::KwUsing)
            .ignore_then(
                // Try alias = form first (backtracking)
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span()))
                    .then_ignore(just(Token::Eq))
                    .or_not(),
            )
            .then(qualified_name.clone())
            .then_ignore(just(Token::Semi))
            .map_with(|(alias, path), e| {
                (
                    cst::Item::Using((
                        cst::UsingDecl { alias, path },
                        e.span(),
                    )),
                    e.span(),
                )
            });

        // --- const_decl parser ---
        // const name : type = expr ;
        let const_decl = just(Token::KwConst)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .then_ignore(just(Token::Semi))
            .map(|((name, ty), value)| cst::ConstDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                ty,
                value,
            });

        // --- global_decl parser ---
        // global mut name : type = expr ;
        let global_decl = just(Token::KwGlobal)
            .ignore_then(just(Token::KwMut))
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .then_ignore(just(Token::Eq))
            .then(expr.clone())
            .then_ignore(just(Token::Semi))
            .map(|((name, ty), value)| cst::GlobalDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                ty,
                value,
            });

        // --- struct_decl parser ---
        // [vis] struct Name [<generics>] { [vis] field: type [= default], ... }
        let struct_field = visibility.clone().or_not()
            .then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .then(
                just(Token::Eq)
                    .ignore_then(expr.clone())
                    .or_not(),
            )
            .map_with(|(((vis, name), ty), default), e| {
                (
                    cst::StructField { vis, name, ty, default },
                    e.span(),
                )
            });

        let struct_decl = just(Token::KwStruct)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(
                struct_field.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|((name, generics), fields)| cst::StructDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                generics,
                fields,
            });

        // --- enum_decl parser ---
        // [vis] enum Name [<generics>] { Variant, Variant(name: type, ...), ... }
        let enum_variant = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .then(fn_param_list.clone().or_not())
            .map_with(|(name, fields), e| {
                (
                    cst::EnumVariant { name, fields },
                    e.span(),
                )
            });

        let enum_decl = just(Token::KwEnum)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(
                enum_variant
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|((name, generics), variants)| cst::EnumDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                generics,
                variants,
            });

        // --- fn_sig parser (function signature, no body) ---
        // fn name [<generics>] (params) [-> type] ;
        let fn_sig = just(Token::KwFn)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(fn_param_list.clone())
            .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
            .then_ignore(just(Token::Semi))
            .map(|(((name, generics), params), return_type)| cst::FnSig {
                attrs: Vec::new(),
                vis: None,
                name,
                generics,
                params,
                return_type,
            });

        // --- contract_decl parser ---
        // [vis] contract Name [<generics>] { fn_sig; ... }
        let contract_member = fn_sig.clone()
            .map_with(|sig, e| {
                (cst::ContractMember::FnSig(sig), e.span())
            });

        let contract_decl = just(Token::KwContract)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(
                contract_member
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|((name, generics), members)| cst::ContractDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                generics,
                members,
            });

        // --- Operator symbol parser ---
        // CRITICAL: Match []= before [] (longest match first)
        let op_symbol = choice((
            just(Token::LBracket)
                .then(just(Token::RBracket))
                .then(just(Token::Eq))
                .to(cst::OpSymbol::IndexSet),
            just(Token::LBracket)
                .then(just(Token::RBracket))
                .to(cst::OpSymbol::Index),
            just(Token::Plus).to(cst::OpSymbol::Add),
            just(Token::Minus).to(cst::OpSymbol::Sub),
            just(Token::Star).to(cst::OpSymbol::Mul),
            just(Token::Slash).to(cst::OpSymbol::Div),
            just(Token::Percent).to(cst::OpSymbol::Mod),
            just(Token::EqEq).to(cst::OpSymbol::Eq),
            just(Token::Lt).to(cst::OpSymbol::Lt),
            just(Token::Bang).to(cst::OpSymbol::Not),
        ))
        .map_with(|sym, e| (sym, e.span()));

        // --- op_decl parser ---
        // [vis] operator SYMBOL (params) [-> type] { body }
        let op_decl = visibility.clone().or_not()
            .then_ignore(select! { Token::Ident("operator") => () })
            .then(op_symbol)
            .then(fn_param_list.clone())
            .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
            .then(block.clone())
            .map_with(|((((vis, symbol), params), return_type), body), e| {
                (
                    cst::ImplMember::Op((
                        cst::OpDecl {
                            vis,
                            symbol,
                            params,
                            return_type,
                            body,
                        },
                        e.span(),
                    )),
                    e.span(),
                )
            });

        // --- impl member: fn or operator ---
        let impl_fn_member = visibility.clone().or_not()
            .then(fn_decl.clone())
            .map_with(|(vis, mut fd), e| {
                fd.vis = vis;
                (cst::ImplMember::Fn((fd, e.span())), e.span())
            });

        let impl_member = choice((
            op_decl,
            impl_fn_member,
        ));

        // --- impl_decl parser ---
        // impl [Contract for] Type { members }
        // Strategy: try "contract for type" form first via backtracking
        let impl_decl = just(Token::KwImpl)
            .ignore_then(
                // Try: type_expr KwFor type_expr (contract-for form)
                type_expr()
                    .then_ignore(just(Token::KwFor))
                    .then(type_expr())
                    .map(|(contract, target)| (Some(contract), target))
                    .or(
                        // Fallback: just type_expr (plain impl)
                        type_expr().map(|target| (None, target))
                    ),
            )
            .then(
                impl_member
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|((contract, target), members)| cst::ImplDecl {
                contract,
                target,
                members,
            });

        // --- Entity member parsers ---

        // use Component { field: expr, ... },
        let use_field = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .then_ignore(just(Token::Colon))
            .then(expr.clone())
            .map_with(|(name, value), e| {
                (cst::UseField { name, value }, e.span())
            });

        let entity_use = just(Token::KwUse)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(
                use_field
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .then_ignore(just(Token::Comma).or_not())
            .map_with(|(component, fields), e| {
                (cst::EntityMember::Use { component, fields }, e.span())
            });

        // on event [(params)] { body }
        let entity_on = just(Token::KwOn)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(fn_param_list.clone().or_not())
            .then(block.clone())
            .map_with(|((event, params), body), e| {
                (cst::EntityMember::On { event, params, body }, e.span())
            });

        // [vis] fn name(...) block -- entity method
        let entity_fn = visibility.clone().or_not()
            .then(fn_decl.clone())
            .map_with(|(vis, mut fd), e| {
                fd.vis = vis;
                (cst::EntityMember::Fn((fd, e.span())), e.span())
            });

        // [vis] name: type [= default], -- property (catch-all, must be LAST)
        let entity_property = visibility.clone().or_not()
            .then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then_ignore(just(Token::Colon))
            .then(type_expr())
            .then(
                just(Token::Eq)
                    .ignore_then(expr.clone())
                    .or_not(),
            )
            .then_ignore(just(Token::Comma).or_not())
            .map_with(|(((vis, name), ty), default), e| {
                (cst::EntityMember::Property { vis, name, ty, default }, e.span())
            });

        let entity_member = choice((
            entity_use,
            entity_on,
            entity_fn,
            entity_property,
        ));

        // --- entity_decl parser ---
        // [vis] entity Name { members }
        let entity_decl = just(Token::KwEntity)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(
                entity_member
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|(name, members)| cst::EntityDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                members,
            });

        // --- Component member parsers ---
        // [vis] fn name(...) block -- method
        let component_fn = visibility.clone().or_not()
            .then(fn_decl.clone())
            .map_with(|(vis, mut fd), e| {
                fd.vis = vis;
                (cst::ComponentMember::Fn((fd, e.span())), e.span())
            });

        // [vis] name: type [= default], -- field
        let component_field = struct_field.clone()
            .then_ignore(just(Token::Comma))
            .map_with(|(sf, sf_span), e| {
                (cst::ComponentMember::Field((sf, sf_span)), e.span())
            });

        let component_member = choice((
            component_fn,
            component_field.clone(),
        ));

        // --- component_decl parser ---
        // [vis] component Name { members }
        let component_decl = just(Token::KwComponent)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(
                component_member
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|(name, members)| cst::ComponentDecl {
                attrs: Vec::new(),
                vis: None,
                name,
                members,
            });

        // --- extern_decl parser ---
        // extern fn ...; | extern struct ... { } | extern component ... { }
        let extern_fn = just(Token::KwFn)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(fn_param_list.clone())
            .then(just(Token::Arrow).ignore_then(type_expr()).or_not())
            .then_ignore(just(Token::Semi))
            .map_with(|(((name, generics), params), return_type), e| {
                cst::ExternDecl::Fn((
                    cst::FnSig {
                        attrs: Vec::new(),
                        vis: None,
                        name,
                        generics,
                        params,
                        return_type,
                    },
                    e.span(),
                ))
            });

        let extern_struct = just(Token::KwStruct)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(generic_params().or_not())
            .then(
                struct_field.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with(|((name, generics), fields), e| {
                cst::ExternDecl::Struct((
                    cst::StructDecl {
                        attrs: Vec::new(),
                        vis: None,
                        name,
                        generics,
                        fields,
                    },
                    e.span(),
                ))
            });

        let extern_component = just(Token::KwComponent)
            .ignore_then(
                select! { Token::Ident(name) => name }
                    .map_with(|n, e| (n, e.span())),
            )
            .then(
                component_field.clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with(|(name, members), e| {
                cst::ExternDecl::Component((
                    cst::ComponentDecl {
                        attrs: Vec::new(),
                        vis: None,
                        name,
                        members,
                    },
                    e.span(),
                ))
            });

        let extern_decl = just(Token::KwExtern)
            .ignore_then(choice((
                extern_fn,
                extern_struct,
                extern_component,
            )));

        // =============================================================
        // Top-level item parser: declarations first, then stmt fallback
        // =============================================================

        // Declarations that accept attrs + vis prefix
        let attrs_vis_decl = attrs.clone()
            .then(visibility.clone().or_not())
            .then(choice((
                // fn -> Item::Fn
                fn_decl.map_with(|fd, e| {
                    (cst::Item::Fn((fd, e.span())), e.span())
                }),
                // const -> Item::Const
                const_decl.map_with(|cd, e| {
                    (cst::Item::Const((cd, e.span())), e.span())
                }),
                // global -> Item::Global
                global_decl.map_with(|gd, e| {
                    (cst::Item::Global((gd, e.span())), e.span())
                }),
                // struct -> Item::Struct
                struct_decl.map_with(|sd, e| {
                    (cst::Item::Struct((sd, e.span())), e.span())
                }),
                // enum -> Item::Enum
                enum_decl.map_with(|ed, e| {
                    (cst::Item::Enum((ed, e.span())), e.span())
                }),
                // contract -> Item::Contract
                contract_decl.map_with(|cd, e| {
                    (cst::Item::Contract((cd, e.span())), e.span())
                }),
                // entity -> Item::Entity
                entity_decl.map_with(|ed, e| {
                    (cst::Item::Entity((ed, e.span())), e.span())
                }),
                // component -> Item::Component
                component_decl.map_with(|cd, e| {
                    (cst::Item::Component((cd, e.span())), e.span())
                }),
                // dlg -> Item::Dlg
                dlg_decl.map_with(|dd, e| {
                    (cst::Item::Dlg(dd), e.span())
                }),
            )))
            .map_with(|((attr_list, vis), (mut item, _inner_span)), e| {
                // Attach attrs and vis to the inner declaration
                match &mut item {
                    cst::Item::Fn((fd, _)) => {
                        fd.attrs = attr_list;
                        fd.vis = vis;
                    }
                    cst::Item::Const((cd, _)) => {
                        cd.attrs = attr_list;
                        cd.vis = vis;
                    }
                    cst::Item::Global((gd, _)) => {
                        gd.attrs = attr_list;
                        gd.vis = vis;
                    }
                    cst::Item::Struct((sd, _)) => {
                        sd.attrs = attr_list;
                        sd.vis = vis;
                    }
                    cst::Item::Enum((ed, _)) => {
                        ed.attrs = attr_list;
                        ed.vis = vis;
                    }
                    cst::Item::Contract((cd, _)) => {
                        cd.attrs = attr_list;
                        cd.vis = vis;
                    }
                    cst::Item::Entity((ed, _)) => {
                        ed.attrs = attr_list;
                        ed.vis = vis;
                    }
                    cst::Item::Component((cd, _)) => {
                        cd.attrs = attr_list;
                        cd.vis = vis;
                    }
                    cst::Item::Dlg(_) => {
                        // DlgDecl doesn't have attrs/vis fields
                        // (vis was already consumed above)
                    }
                    _ => {}
                }
                (item, e.span())
            });

        // Statement fallback -> Item::Stmt
        let stmt_item = stmt.map_with(|s, e| {
            (cst::Item::Stmt(s), e.span())
        });

        // impl_decl -> Item::Impl (no attrs/vis prefix; impl keyword is unique)
        let impl_item = impl_decl.map_with(|id, e| {
            (cst::Item::Impl((id, e.span())), e.span())
        });

        // extern_decl -> Item::Extern (with optional attrs prefix)
        let extern_item = attrs.clone()
            .then(extern_decl)
            .map_with(|(attr_list, mut ed), e| {
                // Attach attrs to the inner declaration if it's an Fn
                match &mut ed {
                    cst::ExternDecl::Fn((sig, _)) => {
                        sig.attrs = attr_list;
                    }
                    cst::ExternDecl::Struct((sd, _)) => {
                        sd.attrs = attr_list;
                    }
                    cst::ExternDecl::Component((cd, _)) => {
                        cd.attrs = attr_list;
                    }
                }
                (cst::Item::Extern((ed, e.span())), e.span())
            });

        // Top-level item: try declarations first, then statement
        let item = choice((
            namespace_decl,
            using_decl,
            impl_item,
            extern_item,
            attrs_vis_decl,
            stmt_item,
        ))
        .labelled("declaration");

        // The recursive reference: a list of items with recovery.
        // Item-level recovery uses skip_then_retry_until to skip bad tokens
        // and retry. The .boxed() prevents type-level stack overflow
        // in the deeply recursive parser structure.
        // Item-level recovery: when an item fails to parse,
        // skip tokens (consuming balanced brace groups) and retry.
        // Sentinel: RBrace (stops inside blocks) or end of input.
        // The balanced-brace skip ensures broken fn/struct bodies
        // are consumed as a unit, allowing recovery to reach the
        // next declaration. .boxed() prevents type-level stack overflow.
        item
            .recover_with(skip_then_retry_until(
                nested_delimiters(
                    Token::LBrace,
                    Token::RBrace,
                    [(Token::LParen, Token::RParen), (Token::LBracket, Token::RBracket)],
                    |span| {
                        (cst::Item::Stmt((cst::Stmt::Expr((cst::Expr::Error, span)), span)), span)
                    },
                )
                .ignored()
                .or(any().ignored()),
                just(Token::RBrace).ignored().or(end()),
            ))
            .boxed()
            .repeated()
            .collect::<Vec<_>>()
    })
}

/// Parse a Writ source string, returning parsed items and errors.
///
/// Converts the token stream from the lexer into a CST by:
/// 1. Lexing the source into tokens
/// 2. Filtering trivia (whitespace, comments) for parser input
/// 3. Running the program parser to produce items
///
/// Returns `Vec<Spanned<Item>>` where each item is either a declaration
/// (fn, struct, enum, etc.) or a statement wrapped in `Item::Stmt`.
///
/// Trivia is preserved in the raw token stream for full-fidelity CST
/// reconstruction (Phase 5). For Phase 2, filtering before parsing is correct.
pub fn parse<'src>(
    src: &'src str,
) -> (
    Option<Vec<cst::Spanned<cst::Item<'src>>>>,
    Vec<Rich<'static, Token<'src>, Span>>,
) {
    let tokens = crate::lexer::lex(src);
    let eoi = Span::from(src.len()..src.len());
    // Filter out trivia tokens -- the parser works with non-trivia tokens only.
    let non_trivia: Vec<(Token<'src>, Span)> = tokens
        .into_iter()
        .filter(|(t, _)| {
            !matches!(
                t,
                Token::Whitespace | Token::LineComment | Token::BlockComment
            )
        })
        .collect();
    let token_stream =
        Stream::from_iter(non_trivia).map(eoi, |(t, s): (_, _)| (t, s));
    program_parser(src).parse(token_stream).into_output_errors()
}
