use writ_parser::cst::{Spanned, Stmt};
use crate::ast::expr::{AstExpr, AstArg};
use crate::ast::stmt::AstStmt;
use crate::lower::context::LoweringContext;
use crate::lower::expr::lower_expr;
use crate::lower::optional::lower_type;

/// Folds a CST `Stmt` into a lowered `AstStmt`.
///
/// Calls `lower_expr` on all expression sub-nodes and `lower_type` on
/// any type annotation sub-nodes.
///
/// `Stmt::Transition` lowers to `AstStmt::Return` with the target as a `Call` expression.
pub fn lower_stmt(spanned: Spanned<Stmt<'_>>, ctx: &mut LoweringContext) -> AstStmt {
    let (stmt, span) = spanned;
    match stmt {
        Stmt::Let {
            mutable,
            name: (name_str, name_span),
            ty,
            value,
        } => AstStmt::Let {
            mutable,
            name: name_str.to_string(),
            name_span,
            ty: ty.map(lower_type),
            value: lower_expr(value, ctx),
            span,
        },

        Stmt::Expr(inner) => AstStmt::Expr {
            expr: lower_expr(inner, ctx),
            span,
        },

        Stmt::For {
            binding: (b, b_span),
            iterable,
            body,
        } => AstStmt::For {
            binding: b.to_string(),
            binding_span: b_span,
            iterable: lower_expr(iterable, ctx),
            body: body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
            span,
        },

        Stmt::While { condition, body } => AstStmt::While {
            condition: lower_expr(condition, ctx),
            body: body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
            span,
        },

        Stmt::Break(val) => AstStmt::Break {
            value: val.map(|v| lower_expr(v, ctx)),
            span,
        },

        Stmt::Continue => AstStmt::Continue { span },

        Stmt::Return(val) => AstStmt::Return {
            value: val.map(|v| lower_expr(v, ctx)),
            span,
        },

        Stmt::Atomic(body) => AstStmt::Atomic {
            body: body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
            span,
        },

        Stmt::Transition((trans, trans_span)) => AstStmt::Return {
            value: Some(AstExpr::Call {
                callee: Box::new(AstExpr::Ident {
                    name: trans.target.0.to_string(),
                    span: trans.target.1,
                }),
                args: trans
                    .args
                    .unwrap_or_default()
                    .into_iter()
                    .map(|e| AstArg {
                        name: None,
                        value: lower_expr(e, ctx),
                        span: trans_span,
                    })
                    .collect(),
                span: trans_span,
            }),
            span,
        },
    }
}
