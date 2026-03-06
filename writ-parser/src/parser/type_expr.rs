use chumsky::input::ValueInput;
use chumsky::prelude::*;

use crate::cst;
use crate::lexer::Token;

use super::{Span, TypePostfix};

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
        // Void type (keyword, must be tried before ident-based paths)
        let void_type = select! {
            Token::KwVoid => cst::TypeExpr::Void,
        }
        .map_with(|t, e| (t, e.span()));

        // Ident token for type paths
        let ident_token_for_type = select! {
            Token::Ident(name) => name,
        };

        // Qualified or named type: [::] ident (:: ident)*
        // Single-segment → Named, multi-segment or rooted → Qualified
        let named_or_qualified = just(Token::ColonColon).or_not()
            .then(
                ident_token_for_type
                    .map_with(|name, e| (name, e.span()))
                    .separated_by(just(Token::ColonColon))
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map_with(|(root_prefix, segments): (Option<_>, Vec<cst::Spanned<&'src str>>), e| {
                if root_prefix.is_none() && segments.len() == 1 {
                    (cst::TypeExpr::Named(segments[0].0), e.span())
                } else {
                    (cst::TypeExpr::Qualified {
                        segments,
                        rooted: root_prefix.is_some(),
                    }, e.span())
                }
            });

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

        // Atom: function type, void, or named/qualified path
        let atom = fn_type.or(void_type).or(named_or_qualified);

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
