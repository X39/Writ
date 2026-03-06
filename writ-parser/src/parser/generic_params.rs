use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::cst;
use crate::lexer::Token;

use super::Span;
use super::type_expr::type_expr;

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
