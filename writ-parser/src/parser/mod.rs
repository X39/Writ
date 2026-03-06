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

use chumsky::prelude::*;

use crate::cst;

pub(crate) type Span = SimpleSpan;

/// Helper enum for type expression postfix operations.
/// Private to the parser module.
#[derive(Clone)]
pub(super) enum TypePostfix<'src> {
    Generic(Vec<cst::Spanned<cst::TypeExpr<'src>>>),
    Array,
    Nullable,
}

/// Helper enum for expression postfix chain operations.
/// Used by `foldl_with` to dispatch member access, bracket access, calls,
/// null propagation, and unwrap postfix operators.
#[derive(Clone)]
pub(super) enum ExprPostfix<'src> {
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
}

pub mod generic_params;
pub mod type_expr;
mod pattern;
mod program;

pub use generic_params::generic_params;
pub use program::{parse, program_parser};
pub use type_expr::type_expr;
