//! Binary and unary-prefix operator type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstExpr, BinaryOp, PrefixOp};
use super::CheckCtx;
use super::check_expr;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;

pub(super) fn check_binary(
    ctx: &mut CheckCtx,
    left: &AstExpr,
    op: &BinaryOp,
    right: &AstExpr,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_left = check_expr(ctx, left);
    let typed_right = check_expr(ctx, right);
    let left_ty = typed_left.ty();
    let right_ty = typed_right.ty();

    // Poison propagation
    if ctx.is_error(left_ty) || ctx.is_error(right_ty) {
        return TypedExpr::Binary {
            ty: ctx.interner.error(),
            span,
            left: Box::new(typed_left),
            op: op.clone(),
            right: Box::new(typed_right),
        };
    }

    let result_ty = match op {
        // Arithmetic: both same numeric, result same type
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            let left_kind = ctx.interner.kind(left_ty).clone();
            let right_kind = ctx.interner.kind(right_ty).clone();
            match (&left_kind, &right_kind) {
                (TyKind::Int, TyKind::Int) => ctx.interner.int(),
                (TyKind::Float, TyKind::Float) => ctx.interner.float(),
                // String concatenation for +
                (TyKind::String, TyKind::String) if matches!(op, BinaryOp::Add) => {
                    ctx.interner.string_ty()
                }
                _ => {
                    let op_str = match op {
                        BinaryOp::Add => "+",
                        BinaryOp::Sub => "-",
                        BinaryOp::Mul => "*",
                        BinaryOp::Div => "/",
                        BinaryOp::Mod => "%",
                        _ => unreachable!(),
                    };
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(left_ty),
                        found: ctx.display_ty(right_ty),
                        expected_span: typed_left.span(),
                        found_span: typed_right.span(),
                        file: ctx.current_file,
                        help: Some(format!("operator `{}` requires matching numeric types", op_str)),
                    })
                }
            }
        }

        // Comparison: same type, result bool
        BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::Gt
        | BinaryOp::LtEq | BinaryOp::GtEq => {
            if ctx.unify.unify(left_ty, right_ty, &mut ctx.interner).is_err() {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: ctx.display_ty(left_ty),
                    found: ctx.display_ty(right_ty),
                    expected_span: typed_left.span(),
                    found_span: typed_right.span(),
                    file: ctx.current_file,
                    help: Some("comparison requires matching types".to_string()),
                })
            } else {
                ctx.interner.bool_ty()
            }
        }

        // Logical: both bool, result bool
        BinaryOp::And | BinaryOp::Or => {
            let bool_ty = ctx.interner.bool_ty();
            if left_ty != bool_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: ctx.display_ty(left_ty),
                    expected_span: typed_left.span(),
                    found_span: typed_left.span(),
                    file: ctx.current_file,
                    help: None,
                })
            } else if right_ty != bool_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: ctx.display_ty(right_ty),
                    expected_span: typed_right.span(),
                    found_span: typed_right.span(),
                    file: ctx.current_file,
                    help: None,
                })
            } else {
                bool_ty
            }
        }

        // Bitwise: both int, result int
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::Shl | BinaryOp::Shr => {
            let int_ty = ctx.interner.int();
            if left_ty != int_ty || right_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(if left_ty != int_ty { left_ty } else { right_ty }),
                    expected_span: typed_left.span(),
                    found_span: if left_ty != int_ty {
                        typed_left.span()
                    } else {
                        typed_right.span()
                    },
                    file: ctx.current_file,
                    help: Some("bitwise operators require int operands".to_string()),
                })
            } else {
                int_ty
            }
        }
    };

    TypedExpr::Binary {
        ty: result_ty,
        span,
        left: Box::new(typed_left),
        op: op.clone(),
        right: Box::new(typed_right),
    }
}

pub(super) fn check_unary_prefix(
    ctx: &mut CheckCtx,
    op: &PrefixOp,
    expr: &AstExpr,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_expr = check_expr(ctx, expr);
    let inner_ty = typed_expr.ty();

    if ctx.is_error(inner_ty) {
        return TypedExpr::UnaryPrefix {
            ty: ctx.interner.error(),
            span,
            op: op.clone(),
            expr: Box::new(typed_expr),
        };
    }

    let result_ty = match op {
        PrefixOp::Neg => {
            match ctx.interner.kind(inner_ty) {
                TyKind::Int => ctx.interner.int(),
                TyKind::Float => ctx.interner.float(),
                _ => ctx.emit_error(TypeError::TypeMismatch {
                    expected: "numeric type".to_string(),
                    found: ctx.display_ty(inner_ty),
                    expected_span: span,
                    found_span: typed_expr.span(),
                    file: ctx.current_file,
                    help: Some("negation requires int or float".to_string()),
                }),
            }
        }
        PrefixOp::Not => {
            let bool_ty = ctx.interner.bool_ty();
            if inner_ty != bool_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: ctx.display_ty(inner_ty),
                    expected_span: span,
                    found_span: typed_expr.span(),
                    file: ctx.current_file,
                    help: None,
                })
            } else {
                bool_ty
            }
        }
        PrefixOp::FromEnd => {
            // ^expr: from-end indexing, inner must be int
            let int_ty = ctx.interner.int();
            if inner_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(inner_ty),
                    expected_span: span,
                    found_span: typed_expr.span(),
                    file: ctx.current_file,
                    help: None,
                })
            } else {
                int_ty
            }
        }
    };

    TypedExpr::UnaryPrefix {
        ty: result_ty,
        span,
        op: op.clone(),
        expr: Box::new(typed_expr),
    }
}
