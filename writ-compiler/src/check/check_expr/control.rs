//! Control flow type checking (if expressions, blocks).

use chumsky::span::SimpleSpan;

use crate::ast::expr::AstExpr;
use super::CheckCtx;
use super::check_expr;
use super::check_block_stmts;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;

pub(super) fn check_if(
    ctx: &mut CheckCtx,
    condition: &AstExpr,
    then_block: &[crate::ast::stmt::AstStmt],
    else_block: Option<&AstExpr>,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_cond = check_expr(ctx, condition);
    let cond_ty = typed_cond.ty();
    let bool_ty = ctx.interner.bool_ty();

    if !ctx.is_error(cond_ty) && cond_ty != bool_ty {
        ctx.emit_error(TypeError::TypeMismatch {
            expected: "bool".to_string(),
            found: ctx.display_ty(cond_ty),
            expected_span: typed_cond.span(),
            found_span: typed_cond.span(),
            file: ctx.current_file,
            help: Some("if condition must be bool".to_string()),
        });
    }

    // Check then block
    let then_typed = check_block_stmts(ctx, then_block, span);
    let then_ty = then_typed.ty();

    // Check else block
    if let Some(else_expr) = else_block {
        let else_typed = check_expr(ctx, else_expr);
        let else_ty = else_typed.ty();

        // Unify branch types
        let result_ty = if ctx.is_error(then_ty) || ctx.is_error(else_ty) {
            ctx.interner.error()
        } else if ctx.unify.unify(then_ty, else_ty, &mut ctx.interner).is_err() {
            ctx.emit_error(TypeError::TypeMismatch {
                expected: ctx.display_ty(then_ty),
                found: ctx.display_ty(else_ty),
                expected_span: then_typed.span(),
                found_span: else_typed.span(),
                file: ctx.current_file,
                help: Some("if/else branches must have the same type".to_string()),
            })
        } else {
            then_ty
        };

        TypedExpr::If {
            ty: result_ty,
            span,
            condition: Box::new(typed_cond),
            then_branch: Box::new(then_typed),
            else_branch: Some(Box::new(else_typed)),
        }
    } else {
        // No else: type is void
        TypedExpr::If {
            ty: ctx.interner.void(),
            span,
            condition: Box::new(typed_cond),
            then_branch: Box::new(then_typed),
            else_branch: None,
        }
    }
}
