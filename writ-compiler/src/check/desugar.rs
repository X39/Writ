//! Desugaring of ?/!/try operators to typed Match expressions.
//!
//! These operators are desugared during type checking so that the output
//! TypedAst contains only Match expressions, not raw UnaryPostfix or Try nodes.
//!
//! Since Option and Result are prelude types without user-defined DefIds,
//! the desugared Match arms use Wildcard patterns with the variant name
//! stored as metadata. The codegen layer recognizes these patterns by
//! the match structure (two arms on Option/Result types).

use chumsky::span::SimpleSpan;

use super::check_expr::{check_expr, CheckCtx};
use super::error::TypeError;
use super::ir::*;
use super::ty::TyKind;
use crate::ast::expr::AstExpr;

/// Desugar `expr?` (null propagation) on Option<T>.
///
/// Produces: `match expr { Some(val) => val, None => return None }`
pub fn desugar_question(ctx: &mut CheckCtx, inner_expr: &AstExpr, span: SimpleSpan) -> TypedExpr {
    let typed_inner = check_expr(ctx, inner_expr);
    let inner_ty = typed_inner.ty();

    // Poison propagation
    if ctx.is_error(inner_ty) {
        return TypedExpr::Match {
            ty: ctx.interner.error(),
            span,
            scrutinee: Box::new(typed_inner),
            arms: Vec::new(),
        };
    }

    match ctx.interner.kind(inner_ty).clone() {
        TyKind::Option(value_ty) => {
            // Verify enclosing function returns Option<_>
            if let Some(ret_ty) = ctx.current_fn_ret {
                match ctx.interner.kind(ret_ty).clone() {
                    TyKind::Option(_) => {
                        // Valid context: desugar to match
                        build_option_question_match(ctx, typed_inner, value_ty, ret_ty, span)
                    }
                    _ => {
                        let err_ty = ctx.emit_error(TypeError::QuestionWrongContext {
                            expected: "Option<_>".to_string(),
                            actual: ctx.display_ty(ret_ty),
                            span,
                            file: ctx.current_file,
                        });
                        TypedExpr::Error { ty: err_ty, span }
                    }
                }
            } else {
                let err_ty = ctx.emit_error(TypeError::QuestionWrongContext {
                    expected: "Option<_>".to_string(),
                    actual: "void".to_string(),
                    span,
                    file: ctx.current_file,
                });
                TypedExpr::Error { ty: err_ty, span }
            }
        }
        _ => {
            let err_ty = ctx.emit_error(TypeError::QuestionOnNonOption {
                found_ty: ctx.display_ty(inner_ty),
                span,
                file: ctx.current_file,
            });
            TypedExpr::Error { ty: err_ty, span }
        }
    }
}

/// Desugar `expr!` (unwrap) on Option<T> or Result<T,E>.
///
/// Option: `match expr { Some(val) => val, None => <crash> }`
/// Result: `match expr { Ok(val) => val, Err(_) => <crash> }`
pub fn desugar_unwrap(ctx: &mut CheckCtx, inner_expr: &AstExpr, span: SimpleSpan) -> TypedExpr {
    let typed_inner = check_expr(ctx, inner_expr);
    let inner_ty = typed_inner.ty();

    if ctx.is_error(inner_ty) {
        return TypedExpr::Match {
            ty: ctx.interner.error(),
            span,
            scrutinee: Box::new(typed_inner),
            arms: Vec::new(),
        };
    }

    match ctx.interner.kind(inner_ty).clone() {
        TyKind::Option(value_ty) => {
            build_unwrap_match(ctx, typed_inner, value_ty, span)
        }
        TyKind::Result(ok_ty, _err_ty) => {
            build_unwrap_match(ctx, typed_inner, ok_ty, span)
        }
        _ => {
            let err_ty = ctx.emit_error(TypeError::QuestionOnNonOption {
                found_ty: ctx.display_ty(inner_ty),
                span,
                file: ctx.current_file,
            });
            TypedExpr::Error { ty: err_ty, span }
        }
    }
}

/// Desugar `try expr` on Result<T,E>.
///
/// Produces: `match expr { Ok(val) => val, Err(e) => return Err(e) }`
pub fn desugar_try(ctx: &mut CheckCtx, inner_expr: &AstExpr, span: SimpleSpan) -> TypedExpr {
    let typed_inner = check_expr(ctx, inner_expr);
    let inner_ty = typed_inner.ty();

    if ctx.is_error(inner_ty) {
        return TypedExpr::Match {
            ty: ctx.interner.error(),
            span,
            scrutinee: Box::new(typed_inner),
            arms: Vec::new(),
        };
    }

    match ctx.interner.kind(inner_ty).clone() {
        TyKind::Result(ok_ty, err_ty) => {
            // Verify enclosing function returns Result<_, E>
            if let Some(ret_ty) = ctx.current_fn_ret {
                match ctx.interner.kind(ret_ty).clone() {
                    TyKind::Result(_, ret_err_ty) => {
                        // Check error types are compatible
                        if !ctx.is_error(err_ty) && !ctx.is_error(ret_err_ty) {
                            let _ = ctx.unify.unify(err_ty, ret_err_ty, &mut ctx.interner);
                        }
                        build_try_match(ctx, typed_inner, ok_ty, ret_ty, span)
                    }
                    _ => {
                        let err = ctx.emit_error(TypeError::TryOnNonResult {
                            found_ty: format!(
                                "enclosing function returns `{}`, not Result",
                                ctx.display_ty(ret_ty)
                            ),
                            span,
                            file: ctx.current_file,
                        });
                        TypedExpr::Error { ty: err, span }
                    }
                }
            } else {
                let err = ctx.emit_error(TypeError::TryOnNonResult {
                    found_ty: "try used outside function".to_string(),
                    span,
                    file: ctx.current_file,
                });
                TypedExpr::Error { ty: err, span }
            }
        }
        _ => {
            let err_ty = ctx.emit_error(TypeError::TryOnNonResult {
                found_ty: ctx.display_ty(inner_ty),
                span,
                file: ctx.current_file,
            });
            TypedExpr::Error { ty: err_ty, span }
        }
    }
}

// ============================================================================
// Match builders
// ============================================================================

fn build_option_question_match(
    ctx: &mut CheckCtx,
    scrutinee: TypedExpr,
    value_ty: super::ty::Ty,
    ret_ty: super::ty::Ty,
    span: SimpleSpan,
) -> TypedExpr {
    // Arm 1: Some(val) => val (represented as Variable binding)
    let some_arm = TypedArm {
        pattern: TypedPattern::Variable {
            name: "__some_val".to_string(),
            ty: value_ty,
            span,
        },
        body: TypedExpr::Var {
            ty: value_ty,
            span,
            name: "__some_val".to_string(),
        },
        span,
    };

    // Arm 2: None => return None (wildcard/fallback)
    let none_arm = TypedArm {
        pattern: TypedPattern::Wildcard { span },
        body: TypedExpr::Return {
            ty: ctx.interner.void(),
            span,
            value: Some(Box::new(TypedExpr::Path {
                ty: ret_ty,
                span,
                segments: vec!["None".to_string()],
            })),
        },
        span,
    };

    TypedExpr::Match {
        ty: value_ty,
        span,
        scrutinee: Box::new(scrutinee),
        arms: vec![some_arm, none_arm],
    }
}

fn build_unwrap_match(
    _ctx: &mut CheckCtx,
    scrutinee: TypedExpr,
    value_ty: super::ty::Ty,
    span: SimpleSpan,
) -> TypedExpr {
    // Arm 1: Some/Ok(val) => val
    let success_arm = TypedArm {
        pattern: TypedPattern::Variable {
            name: "__unwrap_val".to_string(),
            ty: value_ty,
            span,
        },
        body: TypedExpr::Var {
            ty: value_ty,
            span,
            name: "__unwrap_val".to_string(),
        },
        span,
    };

    // Arm 2: None/Err => crash (represented as Error node)
    let crash_arm = TypedArm {
        pattern: TypedPattern::Wildcard { span },
        body: TypedExpr::Error {
            ty: value_ty, // crash produces expected type for continuity
            span,
        },
        span,
    };

    TypedExpr::Match {
        ty: value_ty,
        span,
        scrutinee: Box::new(scrutinee),
        arms: vec![success_arm, crash_arm],
    }
}

fn build_try_match(
    ctx: &mut CheckCtx,
    scrutinee: TypedExpr,
    ok_ty: super::ty::Ty,
    ret_ty: super::ty::Ty,
    span: SimpleSpan,
) -> TypedExpr {
    // Arm 1: Ok(val) => val
    let ok_arm = TypedArm {
        pattern: TypedPattern::Variable {
            name: "__ok_val".to_string(),
            ty: ok_ty,
            span,
        },
        body: TypedExpr::Var {
            ty: ok_ty,
            span,
            name: "__ok_val".to_string(),
        },
        span,
    };

    // Arm 2: Err(e) => return Err(e)
    let err_arm = TypedArm {
        pattern: TypedPattern::Wildcard { span },
        body: TypedExpr::Return {
            ty: ctx.interner.void(),
            span,
            value: Some(Box::new(TypedExpr::Path {
                ty: ret_ty,
                span,
                segments: vec!["Err".to_string()],
            })),
        },
        span,
    };

    TypedExpr::Match {
        ty: ok_ty,
        span,
        scrutinee: Box::new(scrutinee),
        arms: vec![ok_arm, err_arm],
    }
}
