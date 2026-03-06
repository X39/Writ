use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::cst;
use crate::lexer::Token;

use super::Span;

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
pub(super) fn pattern<'tokens, 'src: 'tokens, I>() -> impl Parser<
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

        // Single-segment enum destructure: Variant(patterns)
        // e.g., Some(v), Ok(val) -- single identifier with mandatory parenthesised args.
        // This is distinct from a variable binding because it MUST have the arg list.
        let enum_destruct_single = select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span()))
            .then(
                pattern
                    .clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map_with(|(name_spanned, params), e| {
                (
                    cst::Pattern::EnumDestructure(vec![name_spanned], params),
                    e.span(),
                )
            });

        // Variable binding: name (any identifier that's not _)
        // Must come after enum_destruct and wildcard in choice
        let variable = select! { Token::Ident(name) => name }
            .map_with(|name, e| (cst::Pattern::Variable(name), e.span()));

        // Single pattern (before or-pattern)
        // Order matters: try range first (int..=int), then wildcard, literal, enum (multi then
        // single), variable. enum_destruct_single must come before variable so `Some(v)` is
        // parsed as destructure rather than as a variable named "Some" followed by a call.
        let single = choice((
            range_pat,
            wildcard,
            literal_pat,
            enum_destruct,
            enum_destruct_single,
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
