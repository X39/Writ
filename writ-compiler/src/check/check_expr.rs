//! Expression type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::*;
use crate::ast::types::AstType;
use crate::resolve::def_map::{DefId, DefKind, DefMap};

use super::env::{FnSig, LocalEnv, TypeEnv};
use super::error::TypeError;
use super::infer::instantiate_generic_fn;
use super::ir::*;
use super::ty::{Ty, TyInterner, TyKind};
use super::unify::UnifyCtx;
use writ_diagnostics::{Diagnostic, FileId};

/// Central checking context threaded through all checking functions.
pub struct CheckCtx<'def> {
    pub interner: TyInterner,
    pub diags: Vec<Diagnostic>,
    pub def_map: &'def DefMap,
    pub type_env: &'def TypeEnv,
    pub unify: UnifyCtx,
    pub local_env: LocalEnv,
    pub current_fn_ret: Option<Ty>,
    pub current_file: FileId,
    pub self_type: Option<Ty>,
}

impl CheckCtx<'_> {
    /// Check if a type is the poison/error type.
    pub fn is_error(&self, ty: Ty) -> bool {
        matches!(self.interner.kind(ty), TyKind::Error)
    }

    /// Emit a type error and return the Error poison type.
    pub fn emit_error(&mut self, err: TypeError) -> Ty {
        self.diags.push(err.into());
        self.interner.error()
    }

    /// Format a type for display in error messages.
    pub fn display_ty(&self, ty: Ty) -> String {
        self.interner.display(ty)
    }
}

/// Type-check an expression, returning a TypedExpr.
pub fn check_expr(ctx: &mut CheckCtx, expr: &AstExpr) -> TypedExpr {
    match expr {
        AstExpr::IntLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.int(),
            span: *span,
            value: TypedLiteral::Int(*value),
        },
        AstExpr::FloatLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.float(),
            span: *span,
            value: TypedLiteral::Float(*value),
        },
        AstExpr::StringLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.string_ty(),
            span: *span,
            value: TypedLiteral::String(value.clone()),
        },
        AstExpr::BoolLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.bool_ty(),
            span: *span,
            value: TypedLiteral::Bool(*value),
        },

        AstExpr::Ident { name, span } => check_ident(ctx, name, *span),

        AstExpr::Path { segments, span } => check_path(ctx, segments, *span),

        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => check_binary(ctx, left, op, right, *span),

        AstExpr::UnaryPrefix { op, expr, span } => check_unary_prefix(ctx, op, expr, *span),

        AstExpr::UnaryPostfix { expr: inner, op, span } => {
            match op {
                PostfixOp::NullPropagate => {
                    super::desugar::desugar_question(ctx, inner, *span)
                }
                PostfixOp::Unwrap => {
                    super::desugar::desugar_unwrap(ctx, inner, *span)
                }
            }
        }

        AstExpr::Call { callee, args, span } => check_call(ctx, callee, args, *span),

        AstExpr::GenericCall {
            callee,
            type_args,
            args,
            span,
        } => check_generic_call(ctx, callee, type_args, args, *span),

        AstExpr::If {
            condition,
            then_block,
            else_block,
            span,
        } => check_if(ctx, condition, then_block, else_block.as_deref(), *span),

        AstExpr::Block { stmts, span } => check_block(ctx, stmts, *span),

        AstExpr::MemberAccess { object, field, field_span, span } => {
            check_member_access(ctx, object, field, *field_span, *span)
        },
        AstExpr::BracketAccess { object, index, span } => {
            check_bracket_access(ctx, object, index, *span)
        },
        AstExpr::SelfLit { span } => {
            if let Some(self_ty) = ctx.self_type {
                TypedExpr::SelfRef { ty: self_ty, span: *span }
            } else {
                let err_ty = ctx.emit_error(TypeError::UndefinedVariable {
                    name: "self".to_string(),
                    span: *span,
                    file: ctx.current_file,
                });
                TypedExpr::Error { ty: err_ty, span: *span }
            }
        },
        AstExpr::Match { scrutinee, arms, span } => {
            check_match(ctx, scrutinee, arms, *span)
        },
        AstExpr::IfLet { pattern, value, then_block, else_block, span } => {
            let typed_value = check_expr(ctx, value);
            let value_ty = typed_value.ty();

            // Check then block with pattern bindings
            ctx.local_env.push_scope();
            let _typed_pattern = check_pattern(ctx, pattern, value_ty);
            let then_typed = check_block_stmts(ctx, then_block, *span);
            let then_ty = then_typed.ty();
            ctx.local_env.pop_scope();

            // Check else block
            if let Some(else_expr) = else_block {
                let else_typed = check_expr(ctx, else_expr);
                let else_ty = else_typed.ty();

                let result_ty = if ctx.is_error(then_ty) || ctx.is_error(else_ty) {
                    ctx.interner.error()
                } else if let Err(_) = ctx.unify.unify(then_ty, else_ty, &mut ctx.interner) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(then_ty),
                        found: ctx.display_ty(else_ty),
                        expected_span: then_typed.span(),
                        found_span: else_typed.span(),
                        file: ctx.current_file,
                        help: Some("if-let branches must have the same type".to_string()),
                    })
                } else {
                    then_ty
                };

                TypedExpr::If {
                    ty: result_ty,
                    span: *span,
                    condition: Box::new(typed_value),
                    then_branch: Box::new(then_typed),
                    else_branch: Some(Box::new(else_typed)),
                }
            } else {
                TypedExpr::If {
                    ty: ctx.interner.void(),
                    span: *span,
                    condition: Box::new(typed_value),
                    then_branch: Box::new(then_typed),
                    else_branch: None,
                }
            }
        },
        AstExpr::Lambda { params, return_type, body, span } => {
            check_lambda(ctx, params, return_type.as_deref(), body, *span)
        },
        AstExpr::Spawn { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let task_ty = ctx.interner.task_handle(inner_ty);
            TypedExpr::Spawn {
                ty: task_ty,
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::SpawnDetached { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            TypedExpr::SpawnDetached {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Join { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let result_ty = if ctx.is_error(inner_ty) {
                ctx.interner.error()
            } else {
                match ctx.interner.kind(inner_ty).clone() {
                    TyKind::TaskHandle(inner) => inner,
                    _ => {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: "TaskHandle<T>".to_string(),
                            found: ctx.display_ty(inner_ty),
                            expected_span: *span,
                            found_span: typed_inner.span(),
                            file: ctx.current_file,
                            help: Some("join requires a TaskHandle".to_string()),
                        })
                    }
                }
            };
            TypedExpr::Join {
                ty: result_ty,
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Cancel { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            if !ctx.is_error(inner_ty) {
                if !matches!(ctx.interner.kind(inner_ty), TyKind::TaskHandle(_)) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: "TaskHandle<T>".to_string(),
                        found: ctx.display_ty(inner_ty),
                        expected_span: *span,
                        found_span: typed_inner.span(),
                        file: ctx.current_file,
                        help: Some("cancel requires a TaskHandle".to_string()),
                    });
                }
            }
            TypedExpr::Cancel {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Defer { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            TypedExpr::Defer {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Try { expr: inner, span } => {
            super::desugar::desugar_try(ctx, inner, *span)
        },
        AstExpr::New { ty: ast_ty, fields, span } => {
            check_new_construction(ctx, ast_ty, fields, *span)
        },
        AstExpr::ArrayLit { elements, span } => {
            check_array_lit(ctx, elements, *span)
        },
        AstExpr::Assign { target, value, span } => {
            let typed_target = check_expr(ctx, target);
            let typed_value = check_expr(ctx, value);
            let target_ty = typed_target.ty();
            let value_ty = typed_value.ty();

            if !ctx.is_error(target_ty) && !ctx.is_error(value_ty) {
                if let Err(_) = ctx.unify.unify(target_ty, value_ty, &mut ctx.interner) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(target_ty),
                        found: ctx.display_ty(value_ty),
                        expected_span: typed_target.span(),
                        found_span: typed_value.span(),
                        file: ctx.current_file,
                        help: None,
                    });
                }
            }

            // Check mutability of the assignment target
            check_assignment_mutability(ctx, &typed_target, *span);

            TypedExpr::Assign {
                ty: ctx.interner.void(),
                span: *span,
                target: Box::new(typed_target),
                value: Box::new(typed_value),
            }
        }
        AstExpr::Range { start, kind: _, end, span } => {
            let typed_start = start.as_ref().map(|s| check_expr(ctx, s));
            let typed_end = end.as_ref().map(|e| check_expr(ctx, e));

            // If both present, unify their types
            if let (Some(s), Some(e)) = (&typed_start, &typed_end) {
                let s_ty = s.ty();
                let e_ty = e.ty();
                if !ctx.is_error(s_ty) && !ctx.is_error(e_ty) {
                    let _ = ctx.unify.unify(s_ty, e_ty, &mut ctx.interner);
                }
            }

            // Range type is int for now (simplification; proper Range<T> is a runtime type)
            let range_ty = ctx.interner.int();

            TypedExpr::Range {
                ty: range_ty,
                span: *span,
                start: typed_start.map(Box::new),
                end: typed_end.map(Box::new),
                inclusive: false,
            }
        },
        AstExpr::FromEnd { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let int_ty = ctx.interner.int();
            if !ctx.is_error(inner_ty) && inner_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(inner_ty),
                    expected_span: *span,
                    found_span: typed_inner.span(),
                    file: ctx.current_file,
                    help: Some("from-end index requires int".to_string()),
                });
            }
            // FromEnd produces an int (the index value)
            TypedExpr::UnaryPrefix {
                ty: int_ty,
                span: *span,
                op: PrefixOp::FromEnd,
                expr: Box::new(typed_inner),
            }
        },

        AstExpr::Error { span } => TypedExpr::Error {
            ty: ctx.interner.error(),
            span: *span,
        },
    }
}

fn check_ident(ctx: &mut CheckCtx, name: &str, span: SimpleSpan) -> TypedExpr {
    // First check local environment
    if let Some((ty, _mutability, _binding_span)) = ctx.local_env.lookup(name) {
        return TypedExpr::Var {
            ty,
            span,
            name: name.to_string(),
        };
    }

    // Check DefMap for constants, globals, functions
    if let Some(def_id) = ctx.def_map.get(name) {
        let entry = ctx.def_map.get_entry(def_id);
        match entry.kind {
            DefKind::Fn | DefKind::ExternFn => {
                if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                    let ty = ctx.interner.func(
                        sig.params.iter().map(|(_, t)| *t).collect(),
                        sig.ret,
                    );
                    return TypedExpr::Var {
                        ty,
                        span,
                        name: name.to_string(),
                    };
                }
            }
            DefKind::Const => {
                if let Some(&ty) = ctx.type_env.const_types.get(&def_id) {
                    return TypedExpr::Var {
                        ty,
                        span,
                        name: name.to_string(),
                    };
                }
            }
            DefKind::Global => {
                if let Some(&(ty, _)) = ctx.type_env.global_types.get(&def_id) {
                    return TypedExpr::Var {
                        ty,
                        span,
                        name: name.to_string(),
                    };
                }
            }
            _ => {}
        }
    }

    // Also check by FQN with namespace prefixes - look in file-private scope
    for (_file_id, privates) in &ctx.def_map.file_private {
        if let Some(&def_id) = privates.get(name) {
            let entry = ctx.def_map.get_entry(def_id);
            match entry.kind {
                DefKind::Fn | DefKind::ExternFn => {
                    if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                        let ty = ctx.interner.func(
                            sig.params.iter().map(|(_, t)| *t).collect(),
                            sig.ret,
                        );
                        return TypedExpr::Var {
                            ty,
                            span,
                            name: name.to_string(),
                        };
                    }
                }
                DefKind::Const => {
                    if let Some(&ty) = ctx.type_env.const_types.get(&def_id) {
                        return TypedExpr::Var {
                            ty,
                            span,
                            name: name.to_string(),
                        };
                    }
                }
                DefKind::Global => {
                    if let Some(&(ty, _)) = ctx.type_env.global_types.get(&def_id) {
                        return TypedExpr::Var {
                            ty,
                            span,
                            name: name.to_string(),
                        };
                    }
                }
                _ => {}
            }
        }
    }

    // Not found: emit error with poison
    let err_ty = ctx.emit_error(TypeError::UndefinedVariable {
        name: name.to_string(),
        span,
        file: ctx.current_file,
    });
    TypedExpr::Error { ty: err_ty, span }
}

fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    // Try to resolve as a fully qualified name
    let fqn = segments.join("::");
    if let Some(def_id) = ctx.def_map.get(&fqn) {
        let entry = ctx.def_map.get_entry(def_id);
        match entry.kind {
            DefKind::Fn | DefKind::ExternFn => {
                if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                    let ty = ctx.interner.func(
                        sig.params.iter().map(|(_, t)| *t).collect(),
                        sig.ret,
                    );
                    return TypedExpr::Path {
                        ty,
                        span,
                        segments: segments.to_vec(),
                    };
                }
            }
            DefKind::Const => {
                if let Some(&ty) = ctx.type_env.const_types.get(&def_id) {
                    return TypedExpr::Path {
                        ty,
                        span,
                        segments: segments.to_vec(),
                    };
                }
            }
            _ => {}
        }
    }

    // Could be an enum variant path like `Direction::North`
    // Stub for now
    TypedExpr::Path {
        ty: ctx.interner.error(),
        span,
        segments: segments.to_vec(),
    }
}

fn check_binary(
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
            if let Err(_) = ctx.unify.unify(left_ty, right_ty, &mut ctx.interner) {
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

fn check_unary_prefix(
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

fn check_call(
    ctx: &mut CheckCtx,
    callee: &AstExpr,
    args: &[AstArg],
    span: SimpleSpan,
) -> TypedExpr {
    // Special case: callee is an Ident that resolves to a function in type_env
    if let AstExpr::Ident { name, span: name_span } = callee {
        // Check if it's a known function by name
        if let Some(def_id) = find_fn_def_id(ctx, name) {
            if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                return check_call_with_sig(ctx, name, def_id, sig.clone(), args, span, *name_span);
            }
        }
    }

    // General case: check callee expression
    let typed_callee = check_expr(ctx, callee);
    let callee_ty = typed_callee.ty();

    if ctx.is_error(callee_ty) {
        let typed_args: Vec<TypedExpr> = args.iter().map(|a| check_expr(ctx, &a.value)).collect();
        return TypedExpr::Call {
            ty: ctx.interner.error(),
            span,
            callee: Box::new(typed_callee),
            args: typed_args,
            callee_def_id: None,
        };
    }

    match ctx.interner.kind(callee_ty).clone() {
        TyKind::Func { params, ret } => {
            let typed_args: Vec<TypedExpr> =
                args.iter().map(|a| check_expr(ctx, &a.value)).collect();

            // Check arity
            if typed_args.len() != params.len() {
                ctx.emit_error(TypeError::ArityMismatch {
                    fn_name: "<function value>".to_string(),
                    expected: params.len(),
                    found: typed_args.len(),
                    call_span: span,
                    def_span: typed_callee.span(),
                    file: ctx.current_file,
                });
                return TypedExpr::Call {
                    ty: ctx.interner.error(),
                    span,
                    callee: Box::new(typed_callee),
                    args: typed_args,
                    callee_def_id: None,
                };
            }

            // Check each argument type
            for (i, (arg, &param_ty)) in typed_args.iter().zip(params.iter()).enumerate() {
                let arg_ty = arg.ty();
                if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty) {
                    if let Err(_) = ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner) {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: ctx.display_ty(param_ty),
                            found: ctx.display_ty(arg_ty),
                            expected_span: span,
                            found_span: arg.span(),
                            file: ctx.current_file,
                            help: Some(format!("in argument {}", i + 1)),
                        });
                    }
                }
            }

            TypedExpr::Call {
                ty: ret,
                span,
                callee: Box::new(typed_callee),
                args: typed_args,
                callee_def_id: None,
            }
        }
        _ => {
            let typed_args: Vec<TypedExpr> =
                args.iter().map(|a| check_expr(ctx, &a.value)).collect();
            let err_ty = ctx.emit_error(TypeError::NotCallable {
                ty_name: ctx.display_ty(callee_ty),
                span: typed_callee.span(),
                file: ctx.current_file,
            });
            TypedExpr::Call {
                ty: err_ty,
                span,
                callee: Box::new(typed_callee),
                args: typed_args,
                callee_def_id: None,
            }
        }
    }
}

fn check_call_with_sig(
    ctx: &mut CheckCtx,
    fn_name: &str,
    def_id: DefId,
    sig: FnSig,
    args: &[AstArg],
    span: SimpleSpan,
    name_span: SimpleSpan,
) -> TypedExpr {
    let entry = ctx.def_map.get_entry(def_id);
    let def_span = entry.name_span;

    // Instantiate generics
    let (param_tys, ret_ty, infer_vars) =
        instantiate_generic_fn(&sig, &mut ctx.interner, &mut ctx.unify);

    let typed_args: Vec<TypedExpr> = args.iter().map(|a| check_expr(ctx, &a.value)).collect();

    // Adjust expected arity: skip self_param if present
    let expected_arity = param_tys.len();
    if typed_args.len() != expected_arity {
        ctx.emit_error(TypeError::ArityMismatch {
            fn_name: fn_name.to_string(),
            expected: expected_arity,
            found: typed_args.len(),
            call_span: span,
            def_span,
            file: ctx.current_file,
        });
        return TypedExpr::Call {
            ty: ctx.interner.error(),
            span,
            callee: Box::new(TypedExpr::Var {
                ty: ctx.interner.func(param_tys, ret_ty),
                span: name_span,
                name: fn_name.to_string(),
            }),
            args: typed_args,
            callee_def_id: None,
        };
    }

    // Check each argument type
    for (i, (arg, &param_ty)) in typed_args.iter().zip(param_tys.iter()).enumerate() {
        let arg_ty = arg.ty();
        if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty) {
            if let Err(_) = ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner) {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: ctx.display_ty(param_ty),
                    found: ctx.display_ty(arg_ty),
                    expected_span: def_span,
                    found_span: arg.span(),
                    file: ctx.current_file,
                    help: Some(format!("in argument {} of `{}`", i + 1, fn_name)),
                });
            }
        }
    }

    // Resolve return type (may contain InferVars now resolved)
    let resolved_ret = ctx.unify.resolve_ty(ret_ty, &ctx.interner);

    // Check contract bounds on resolved generic parameters
    if !sig.generics.is_empty() && !sig.bounds.is_empty() {
        check_contract_bounds(ctx, &sig, &infer_vars, span);
    }

    TypedExpr::Call {
        ty: resolved_ret,
        span,
        callee: Box::new(TypedExpr::Var {
            ty: ctx.interner.func(param_tys, resolved_ret),
            span: name_span,
            name: fn_name.to_string(),
        }),
        args: typed_args,
        callee_def_id: Some(def_id),
    }
}

/// Check contract bounds after generic type argument inference.
fn check_contract_bounds(
    ctx: &mut CheckCtx,
    sig: &FnSig,
    infer_vars: &[super::ty::InferVar],
    call_span: SimpleSpan,
) {
    for (i, bounds) in sig.bounds.iter().enumerate() {
        if bounds.is_empty() {
            continue;
        }

        // Resolve the infer var to a concrete type
        let resolved_ty = if i < infer_vars.len() {
            ctx.unify.resolve(infer_vars[i])
        } else {
            None
        };

        if let Some(concrete_ty) = resolved_ty {
            // Get the DefId of the concrete type to look up in impl_index
            let concrete_def_id = match ctx.interner.kind(concrete_ty).clone() {
                TyKind::Struct(did) | TyKind::Entity(did) | TyKind::Enum(did) => Some(did),
                _ => None,
            };

            for &bound_contract_id in bounds {
                let bound_entry = ctx.def_map.get_entry(bound_contract_id);
                let contract_name = bound_entry.name.clone();

                // Check if the concrete type has an impl for this contract
                let satisfies_bound = if let Some(did) = concrete_def_id {
                    ctx.type_env
                        .impl_index
                        .get(&did)
                        .map(|impls| {
                            impls.iter().any(|entry| {
                                entry.contract_def_id == Some(bound_contract_id)
                            })
                        })
                        .unwrap_or(false)
                } else {
                    // Primitive types: check built-in implementations
                    // For now, primitives don't satisfy any contract bounds
                    false
                };

                if !satisfies_bound {
                    let ty_name = ctx.display_ty(concrete_ty);
                    ctx.emit_error(TypeError::UnsatisfiedBound {
                        ty_name: ty_name.clone(),
                        bound_name: contract_name.clone(),
                        call_span,
                        file: ctx.current_file,
                    });
                }
            }
        }
    }
}

fn check_generic_call(
    ctx: &mut CheckCtx,
    callee: &AstExpr,
    type_args: &[AstType],
    args: &[AstArg],
    span: SimpleSpan,
) -> TypedExpr {
    // For generic calls, resolve the callee to get its FnSig
    if let AstExpr::Ident { name, span: name_span } = callee {
        if let Some(def_id) = find_fn_def_id(ctx, name) {
            if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id).cloned() {
                // Resolve explicit type args
                let generic_map = rustc_hash::FxHashMap::default();
                let explicit_tys: Vec<Ty> = type_args
                    .iter()
                    .map(|ta| super::env::resolve_ast_type(ta, ctx.def_map, &mut ctx.interner, &generic_map))
                    .collect();

                // Build substitution from explicit type args
                let subst = explicit_tys;

                // Substitute into param types
                let param_tys: Vec<Ty> = sig
                    .params
                    .iter()
                    .map(|(_, ty)| super::infer::substitute(*ty, &subst, &mut ctx.interner))
                    .collect();
                let ret_ty = super::infer::substitute(sig.ret, &subst, &mut ctx.interner);

                let typed_args: Vec<TypedExpr> =
                    args.iter().map(|a| check_expr(ctx, &a.value)).collect();

                // Check arity
                if typed_args.len() != param_tys.len() {
                    let entry = ctx.def_map.get_entry(def_id);
                    ctx.emit_error(TypeError::ArityMismatch {
                        fn_name: name.to_string(),
                        expected: param_tys.len(),
                        found: typed_args.len(),
                        call_span: span,
                        def_span: entry.name_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Call {
                        ty: ctx.interner.error(),
                        span,
                        callee: Box::new(TypedExpr::Var {
                            ty: ctx.interner.error(),
                            span: *name_span,
                            name: name.to_string(),
                        }),
                        args: typed_args,
                        callee_def_id: None,
                    };
                }

                // Check each arg type
                for (i, (arg, &param_ty)) in typed_args.iter().zip(param_tys.iter()).enumerate() {
                    let arg_ty = arg.ty();
                    if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty) {
                        if let Err(_) = ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner) {
                            ctx.emit_error(TypeError::TypeMismatch {
                                expected: ctx.display_ty(param_ty),
                                found: ctx.display_ty(arg_ty),
                                expected_span: span,
                                found_span: arg.span(),
                                file: ctx.current_file,
                                help: Some(format!("in argument {} of `{}`", i + 1, name)),
                            });
                        }
                    }
                }

                return TypedExpr::Call {
                    ty: ret_ty,
                    span,
                    callee: Box::new(TypedExpr::Var {
                        ty: ctx.interner.func(param_tys, ret_ty),
                        span: *name_span,
                        name: name.to_string(),
                    }),
                    args: typed_args,
                    callee_def_id: Some(def_id),
                };
            }
        }
    }

    // Fallback: check args but return error
    let typed_args: Vec<TypedExpr> = args.iter().map(|a| check_expr(ctx, &a.value)).collect();
    TypedExpr::Call {
        ty: ctx.interner.error(),
        span,
        callee: Box::new(check_expr(ctx, callee)),
        args: typed_args,
        callee_def_id: None,
    }
}

fn check_if(
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
        } else if let Err(_) = ctx.unify.unify(then_ty, else_ty, &mut ctx.interner) {
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

fn check_block(
    ctx: &mut CheckCtx,
    stmts: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    check_block_stmts(ctx, stmts, span)
}

/// Check a list of statements as a block expression.
/// The type of the block is the type of the last expression-statement (if it's an Expr without
/// semicolon), otherwise Void.
pub fn check_block_stmts(
    ctx: &mut CheckCtx,
    stmts: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    ctx.local_env.push_scope();

    let mut typed_stmts = Vec::new();
    for stmt in stmts {
        typed_stmts.push(super::check_stmt::check_stmt(ctx, stmt));
    }

    ctx.local_env.pop_scope();

    // Block type: type of last Expr statement, or void
    let block_ty = typed_stmts
        .last()
        .and_then(|s| match s {
            TypedStmt::Expr { expr, .. } => Some(expr.ty()),
            TypedStmt::Return { .. } => Some(ctx.interner.void()),
            _ => None,
        })
        .unwrap_or_else(|| ctx.interner.void());

    TypedExpr::Block {
        ty: block_ty,
        span,
        stmts: typed_stmts,
        tail: None,
    }
}

// =============================================================================
// Member access (field and method resolution)
// =============================================================================

fn check_member_access(
    ctx: &mut CheckCtx,
    object: &AstExpr,
    field: &str,
    field_span: SimpleSpan,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_obj = check_expr(ctx, object);
    let obj_ty = typed_obj.ty();

    // Poison propagation
    if ctx.is_error(obj_ty) {
        return TypedExpr::Field {
            ty: ctx.interner.error(),
            span,
            receiver: Box::new(typed_obj),
            field: field.to_string(),
        };
    }

    let kind = ctx.interner.kind(obj_ty).clone();
    match kind {
        TyKind::Struct(def_id) | TyKind::Entity(def_id) => {
            // Look up in struct_fields or entity_fields
            let fields = if matches!(kind, TyKind::Struct(_)) {
                ctx.type_env.struct_fields.get(&def_id)
            } else {
                ctx.type_env.entity_fields.get(&def_id)
            };

            if let Some(field_list) = fields {
                for (fname, fty, _fspan) in field_list {
                    if fname == field {
                        return TypedExpr::Field {
                            ty: *fty,
                            span,
                            receiver: Box::new(typed_obj),
                            field: field.to_string(),
                        };
                    }
                }
            }

            // Check impl_index for methods
            if let Some(impls) = ctx.type_env.impl_index.get(&def_id) {
                for impl_entry in impls {
                    for (method_name, method_sig) in &impl_entry.methods {
                        if method_name == field {
                            // Method access: build a Func type for the method
                            let param_tys: Vec<Ty> = method_sig.params.iter().map(|(_, t)| *t).collect();
                            let fn_ty = ctx.interner.func(param_tys, method_sig.ret);
                            return TypedExpr::Field {
                                ty: fn_ty,
                                span,
                                receiver: Box::new(typed_obj),
                                field: field.to_string(),
                            };
                        }
                    }
                }
            }

            // Not found
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Enum(def_id) => {
            // Check for associated methods via impl_index
            if let Some(impls) = ctx.type_env.impl_index.get(&def_id) {
                for impl_entry in impls {
                    for (method_name, method_sig) in &impl_entry.methods {
                        if method_name == field {
                            let param_tys: Vec<Ty> = method_sig.params.iter().map(|(_, t)| *t).collect();
                            let fn_ty = ctx.interner.func(param_tys, method_sig.ret);
                            return TypedExpr::Field {
                                ty: fn_ty,
                                span,
                                receiver: Box::new(typed_obj),
                                field: field.to_string(),
                            };
                        }
                    }
                }
            }

            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        _ => {
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
    }
}

// =============================================================================
// Bracket access (component access and array indexing)
// =============================================================================

fn check_bracket_access(
    ctx: &mut CheckCtx,
    object: &AstExpr,
    index: &AstExpr,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_obj = check_expr(ctx, object);
    let obj_ty = typed_obj.ty();

    // Poison propagation
    if ctx.is_error(obj_ty) {
        let typed_index = check_expr(ctx, index);
        return TypedExpr::Index {
            ty: ctx.interner.error(),
            span,
            receiver: Box::new(typed_obj),
            index: Box::new(typed_index),
        };
    }

    let kind = ctx.interner.kind(obj_ty).clone();
    match kind {
        TyKind::Array(elem_ty) => {
            // Array indexing: index must be int
            let typed_index = check_expr(ctx, index);
            let index_ty = typed_index.ty();
            let int_ty = ctx.interner.int();

            if !ctx.is_error(index_ty) && index_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(index_ty),
                    expected_span: span,
                    found_span: typed_index.span(),
                    file: ctx.current_file,
                    help: Some("array index must be int".to_string()),
                });
            }

            TypedExpr::Index {
                ty: elem_ty,
                span,
                receiver: Box::new(typed_obj),
                index: Box::new(typed_index),
            }
        }
        TyKind::Entity(def_id) => {
            // Component access: entity[ComponentName]
            // The index should be an identifier naming the component
            let component_name = match index {
                AstExpr::Ident { name, .. } => Some(name.as_str()),
                AstExpr::Path { segments, .. } if segments.len() == 1 => Some(segments[0].as_str()),
                _ => None,
            };

            if let Some(comp_name) = component_name {
                // Check if the entity has this component declared (guaranteed access)
                let has_component = ctx.type_env.entity_components
                    .get(&def_id)
                    .map(|comps| comps.iter().any(|c| c == comp_name))
                    .unwrap_or(false);

                // Look up the component's type by searching for its DefId
                let comp_def_id = ctx.def_map.get(comp_name);
                let comp_ty = comp_def_id.and_then(|did| {
                    let entry = ctx.def_map.get_entry(did);
                    if matches!(entry.kind, DefKind::Component | DefKind::ExternComponent) {
                        Some(ctx.interner.intern(TyKind::Struct(did)))
                    } else {
                        None
                    }
                });

                if let Some(component_ty) = comp_ty {
                    let _typed_index = check_expr(ctx, index);
                    if has_component {
                        // Guaranteed access: return the component type directly
                        TypedExpr::ComponentAccess {
                            ty: component_ty,
                            span,
                            receiver: Box::new(typed_obj),
                            component: comp_name.to_string(),
                        }
                    } else {
                        // Optional access: wrap in Option
                        let opt_ty = ctx.interner.option(component_ty);
                        TypedExpr::ComponentAccess {
                            ty: opt_ty,
                            span,
                            receiver: Box::new(typed_obj),
                            component: comp_name.to_string(),
                        }
                    }
                } else {
                    let _typed_index = check_expr(ctx, index);
                    TypedExpr::ComponentAccess {
                        ty: ctx.interner.error(),
                        span,
                        receiver: Box::new(typed_obj),
                        component: comp_name.to_string(),
                    }
                }
            } else {
                // Not a component access identifier
                let typed_index = check_expr(ctx, index);
                TypedExpr::Index {
                    ty: ctx.interner.error(),
                    span,
                    receiver: Box::new(typed_obj),
                    index: Box::new(typed_index),
                }
            }
        }
        _ => {
            // Not indexable
            let typed_index = check_expr(ctx, index);
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::TypeMismatch {
                expected: "array or entity".to_string(),
                found: ty_name,
                expected_span: span,
                found_span: typed_obj.span(),
                file: ctx.current_file,
                help: Some("bracket access requires an array or entity type".to_string()),
            });
            TypedExpr::Index {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                index: Box::new(typed_index),
            }
        }
    }
}

// =============================================================================
// Match expression
// =============================================================================

fn check_match(
    ctx: &mut CheckCtx,
    scrutinee: &AstExpr,
    arms: &[AstMatchArm],
    span: SimpleSpan,
) -> TypedExpr {
    let typed_scrutinee = check_expr(ctx, scrutinee);
    let scrutinee_ty = typed_scrutinee.ty();

    if arms.is_empty() {
        return TypedExpr::Match {
            ty: ctx.interner.void(),
            span,
            scrutinee: Box::new(typed_scrutinee),
            arms: Vec::new(),
        };
    }

    let mut typed_arms = Vec::new();
    let mut arm_types: Vec<Ty> = Vec::new();

    for arm in arms {
        ctx.local_env.push_scope();

        // Bind pattern variables
        let typed_pattern = check_pattern(ctx, &arm.pattern, scrutinee_ty);

        // Check body
        let body = super::check_expr::check_block_stmts(ctx, &arm.body, arm.span);
        let body_ty = body.ty();

        ctx.local_env.pop_scope();

        arm_types.push(body_ty);
        typed_arms.push(TypedArm {
            pattern: typed_pattern,
            body,
            span: arm.span,
        });
    }

    // Unify all arm types
    let mut result_ty = arm_types[0];
    for (i, &arm_ty) in arm_types.iter().enumerate().skip(1) {
        if ctx.is_error(result_ty) || ctx.is_error(arm_ty) {
            continue;
        }
        if let Err(_) = ctx.unify.unify(result_ty, arm_ty, &mut ctx.interner) {
            ctx.emit_error(TypeError::TypeMismatch {
                expected: ctx.display_ty(result_ty),
                found: ctx.display_ty(arm_ty),
                expected_span: typed_arms[0].span,
                found_span: typed_arms[i].span,
                file: ctx.current_file,
                help: Some("all match arms must have the same type".to_string()),
            });
            result_ty = ctx.interner.error();
            break;
        }
    }

    // Check exhaustiveness for enum types
    super::pattern::check_exhaustiveness(ctx, scrutinee_ty, &typed_arms, span);

    TypedExpr::Match {
        ty: result_ty,
        span,
        scrutinee: Box::new(typed_scrutinee),
        arms: typed_arms,
    }
}

fn check_pattern(ctx: &mut CheckCtx, pattern: &AstPattern, scrutinee_ty: Ty) -> TypedPattern {
    match pattern {
        AstPattern::Wildcard { span } => TypedPattern::Wildcard { span: *span },
        AstPattern::Variable { name, span } => {
            // Bind the variable to the scrutinee type
            ctx.local_env.define(
                name.clone(),
                scrutinee_ty,
                super::env::Mutability::Immutable,
                *span,
            );
            TypedPattern::Variable {
                name: name.clone(),
                ty: scrutinee_ty,
                span: *span,
            }
        }
        AstPattern::Literal { expr, span } => {
            // Check the literal expression and verify it's compatible with scrutinee
            let typed_lit = check_expr(ctx, expr);
            let lit_ty = typed_lit.ty();
            if !ctx.is_error(lit_ty) && !ctx.is_error(scrutinee_ty) {
                if let Err(_) = ctx.unify.unify(scrutinee_ty, lit_ty, &mut ctx.interner) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(scrutinee_ty),
                        found: ctx.display_ty(lit_ty),
                        expected_span: *span,
                        found_span: typed_lit.span(),
                        file: ctx.current_file,
                        help: Some("pattern type must match scrutinee type".to_string()),
                    });
                }
            }
            // Extract literal value from the typed expression
            match typed_lit {
                TypedExpr::Literal { value, .. } => TypedPattern::Literal {
                    value,
                    span: *span,
                },
                _ => TypedPattern::Wildcard { span: *span },
            }
        }
        AstPattern::EnumDestructure { path, fields, span } => {
            // Resolve the enum variant from the path
            // e.g., path = ["Option", "Some"] or ["Some"]
            let variant_name = path.last().map(|s| s.as_str()).unwrap_or("");

            // Try to find the enum def_id and variant fields
            let mut enum_def_id = None;
            let mut variant_fields = Vec::new();

            // Check if scrutinee is an enum type
            if let TyKind::Enum(def_id) = ctx.interner.kind(scrutinee_ty).clone() {
                if let Some(variants) = ctx.type_env.enum_variants.get(&def_id) {
                    for v in variants {
                        if v.name == variant_name {
                            enum_def_id = Some(def_id);
                            variant_fields = v.fields.clone();
                            break;
                        }
                    }
                }
            }

            // Bind pattern variables to the variant's field types
            let mut typed_bindings = Vec::new();
            for (i, field_pat) in fields.iter().enumerate() {
                let field_ty = variant_fields
                    .get(i)
                    .map(|(_, ty)| *ty)
                    .unwrap_or_else(|| ctx.interner.error());
                typed_bindings.push(check_pattern(ctx, field_pat, field_ty));
            }

            if let Some(eid) = enum_def_id {
                TypedPattern::EnumVariant {
                    enum_def_id: eid,
                    variant_name: variant_name.to_string(),
                    bindings: typed_bindings,
                    span: *span,
                }
            } else {
                // Could not resolve enum variant, produce wildcard with bindings still defined
                TypedPattern::Wildcard { span: *span }
            }
        }
        AstPattern::Or { patterns, span } => {
            let typed_pats: Vec<TypedPattern> = patterns
                .iter()
                .map(|p| check_pattern(ctx, p, scrutinee_ty))
                .collect();
            TypedPattern::Or {
                patterns: typed_pats,
                span: *span,
            }
        }
        AstPattern::Range { start, kind, end, span } => {
            let start_typed = check_expr(ctx, start);
            let end_typed = check_expr(ctx, end);
            // Both should be compatible with scrutinee type
            let start_lit = match start_typed {
                TypedExpr::Literal { value, .. } => value,
                _ => TypedLiteral::Int(0),
            };
            let end_lit = match end_typed {
                TypedExpr::Literal { value, .. } => value,
                _ => TypedLiteral::Int(0),
            };
            TypedPattern::Range {
                start: start_lit,
                end: end_lit,
                inclusive: matches!(kind, crate::ast::expr::RangeKind::Inclusive),
                span: *span,
            }
        }
    }
}

// =============================================================================
// Lambda / closure checking
// =============================================================================

fn check_lambda(
    ctx: &mut CheckCtx,
    params: &[AstLambdaParam],
    return_type: Option<&AstType>,
    body: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    let generic_map = rustc_hash::FxHashMap::default();

    // Resolve parameter types
    let mut param_tys = Vec::new();
    let mut param_names = Vec::new();
    for p in params {
        let ty = if let Some(ref annotation) = p.ty {
            super::env::resolve_ast_type(annotation, ctx.def_map, &mut ctx.interner, &generic_map)
        } else {
            // No annotation: create an inference variable
            let var = ctx.unify.new_var();
            ctx.interner.intern(TyKind::Infer(var))
        };
        param_tys.push(ty);
        param_names.push(p.name.clone());
    }

    // Resolve return type
    let ret_ty = if let Some(rt) = return_type {
        super::env::resolve_ast_type(rt, ctx.def_map, &mut ctx.interner, &generic_map)
    } else {
        ctx.interner.void()
    };

    // Set up context for body checking
    let old_ret = ctx.current_fn_ret;
    ctx.current_fn_ret = Some(ret_ty);
    ctx.local_env.push_scope();

    // Define params in scope
    for (name, ty) in param_names.iter().zip(param_tys.iter()) {
        ctx.local_env.define(
            name.clone(),
            *ty,
            super::env::Mutability::Immutable,
            span,
        );
    }

    // Check body
    let typed_body = check_block_stmts(ctx, body, span);

    ctx.local_env.pop_scope();
    ctx.current_fn_ret = old_ret;

    // Build captures list (simplified: any outer variables referenced in the body
    // would be tracked here, but for now we produce an empty list since we don't
    // have a capture tracking mechanism in LocalEnv yet)
    let captures = Vec::new();

    // Build function type
    let func_ty = ctx.interner.func(param_tys.clone(), ret_ty);

    let typed_params: Vec<(String, super::ty::Ty)> = param_names
        .into_iter()
        .zip(param_tys.into_iter())
        .collect();

    TypedExpr::Lambda {
        ty: func_ty,
        span,
        params: typed_params,
        ret_ty,
        captures,
        body: Box::new(typed_body),
    }
}

// =============================================================================
// New construction checking
// =============================================================================

fn check_new_construction(
    ctx: &mut CheckCtx,
    ast_ty: &AstType,
    fields: &[AstNewField],
    span: SimpleSpan,
) -> TypedExpr {
    let generic_map = rustc_hash::FxHashMap::default();
    let resolved_ty = super::env::resolve_ast_type(ast_ty, ctx.def_map, &mut ctx.interner, &generic_map);

    if ctx.is_error(resolved_ty) {
        // Can't resolve the type
        return TypedExpr::Error {
            ty: ctx.interner.error(),
            span,
        };
    }

    // Get the DefId and expected fields
    let (def_id, expected_fields) = match ctx.interner.kind(resolved_ty).clone() {
        TyKind::Struct(did) => {
            let fields = ctx.type_env.struct_fields.get(&did).cloned().unwrap_or_default();
            (did, fields)
        }
        TyKind::Entity(did) => {
            let fields = ctx.type_env.entity_fields.get(&did).cloned().unwrap_or_default();
            (did, fields)
        }
        _ => {
            let ty_name = ctx.display_ty(resolved_ty);
            ctx.emit_error(TypeError::TypeMismatch {
                expected: "struct or entity type".to_string(),
                found: ty_name,
                expected_span: span,
                found_span: span,
                file: ctx.current_file,
                help: Some("new requires a struct or entity type".to_string()),
            });
            return TypedExpr::Error {
                ty: ctx.interner.error(),
                span,
            };
        }
    };

    // Check each provided field
    let mut typed_fields = Vec::new();
    let mut provided_names = Vec::new();
    for field in fields {
        let typed_value = check_expr(ctx, &field.value);
        let value_ty = typed_value.ty();

        // Find this field in the expected fields
        let field_def = expected_fields.iter().find(|(name, _, _)| name == &field.name);

        if let Some((_name, expected_ty, _fspan)) = field_def {
            // Check type compatibility
            if !ctx.is_error(value_ty) && !ctx.is_error(*expected_ty) {
                if let Err(_) = ctx.unify.unify(*expected_ty, value_ty, &mut ctx.interner) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(*expected_ty),
                        found: ctx.display_ty(value_ty),
                        expected_span: field.name_span,
                        found_span: typed_value.span(),
                        file: ctx.current_file,
                        help: Some(format!("in field `{}`", field.name)),
                    });
                }
            }
        } else {
            // Unknown field
            ctx.emit_error(TypeError::UnknownField {
                ty_name: ctx.display_ty(resolved_ty),
                field_name: field.name.clone(),
                span: field.name_span,
                file: ctx.current_file,
            });
        }

        provided_names.push(field.name.clone());
        typed_fields.push((field.name.clone(), typed_value));
    }

    // Check for missing required fields
    for (fname, _, _) in &expected_fields {
        if !provided_names.iter().any(|n| n == fname) {
            ctx.diags.push(TypeError::MissingConstructionField {
                type_name: ctx.display_ty(resolved_ty),
                field_name: fname.clone(),
                span,
                file: ctx.current_file,
            }.into());
        }
    }

    TypedExpr::New {
        ty: resolved_ty,
        span,
        target_def_id: def_id,
        fields: typed_fields,
    }
}

// =============================================================================
// Array literal checking
// =============================================================================

fn check_array_lit(
    ctx: &mut CheckCtx,
    elements: &[AstExpr],
    span: SimpleSpan,
) -> TypedExpr {
    if elements.is_empty() {
        // Empty array: infer element type later
        let var = ctx.unify.new_var();
        let elem_ty = ctx.interner.intern(TyKind::Infer(var));
        let array_ty = ctx.interner.array(elem_ty);
        return TypedExpr::ArrayLit {
            ty: array_ty,
            span,
            elements: Vec::new(),
        };
    }

    let typed_elements: Vec<TypedExpr> = elements.iter().map(|e| check_expr(ctx, e)).collect();

    // Unify all element types
    let first_ty = typed_elements[0].ty();
    let mut elem_ty = first_ty;
    for (i, te) in typed_elements.iter().enumerate().skip(1) {
        let ty = te.ty();
        if !ctx.is_error(elem_ty) && !ctx.is_error(ty) {
            if let Err(_) = ctx.unify.unify(elem_ty, ty, &mut ctx.interner) {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: ctx.display_ty(elem_ty),
                    found: ctx.display_ty(ty),
                    expected_span: typed_elements[0].span(),
                    found_span: te.span(),
                    file: ctx.current_file,
                    help: Some(format!("array element {} has different type", i)),
                });
                elem_ty = ctx.interner.error();
                break;
            }
        }
    }

    let array_ty = ctx.interner.array(elem_ty);
    TypedExpr::ArrayLit {
        ty: array_ty,
        span,
        elements: typed_elements,
    }
}

// =============================================================================
// Mutability checking helpers
// =============================================================================

/// Check whether an assignment target is mutable. If not, emit an error.
pub fn check_assignment_mutability(ctx: &mut CheckCtx, target: &TypedExpr, assignment_span: SimpleSpan) {
    if let Some((name, mutability, binding_span)) = find_root_binding(target, &ctx.local_env) {
        if mutability == super::env::Mutability::Immutable {
            // Determine if this is a simple reassignment or a field mutation
            match target {
                TypedExpr::Var { .. } => {
                    ctx.diags.push(TypeError::ImmutableReassignment {
                        binding_name: name,
                        binding_span,
                        assignment_span,
                        file: ctx.current_file,
                    }.into());
                }
                TypedExpr::Field { .. } | TypedExpr::Index { .. } => {
                    ctx.diags.push(TypeError::ImmutableMutation {
                        binding_name: name,
                        binding_span,
                        mutation_span: assignment_span,
                        mutation_kind: "field assignment".to_string(),
                        file: ctx.current_file,
                    }.into());
                }
                _ => {}
            }
        }
    }
}

/// Walk a TypedExpr to find its root variable binding.
fn find_root_binding(expr: &TypedExpr, local_env: &LocalEnv) -> Option<(String, super::env::Mutability, SimpleSpan)> {
    match expr {
        TypedExpr::Var { name, .. } => {
            local_env.lookup(name).map(|(_, m, sp)| (name.clone(), m, sp))
        }
        TypedExpr::SelfRef { .. } => {
            local_env.lookup("self").map(|(_, m, sp)| ("self".to_string(), m, sp))
        }
        TypedExpr::Field { receiver, .. } => find_root_binding(receiver, local_env),
        TypedExpr::Index { receiver, .. } => find_root_binding(receiver, local_env),
        _ => None,
    }
}

/// Find a function DefId by name, checking both FQN and file-private scopes.
fn find_fn_def_id(ctx: &CheckCtx, name: &str) -> Option<DefId> {
    // Check by simple name in by_fqn (for single-namespace programs)
    if let Some(def_id) = ctx.def_map.get(name) {
        let entry = ctx.def_map.get_entry(def_id);
        if matches!(entry.kind, DefKind::Fn | DefKind::ExternFn) {
            return Some(def_id);
        }
    }

    // Check file-private
    for (_file_id, privates) in &ctx.def_map.file_private {
        if let Some(&def_id) = privates.get(name) {
            let entry = ctx.def_map.get_entry(def_id);
            if matches!(entry.kind, DefKind::Fn | DefKind::ExternFn) {
                return Some(def_id);
            }
        }
    }

    // Check all FQN entries that end with this name
    for (fqn, &def_id) in &ctx.def_map.by_fqn {
        if fqn.ends_with(&format!("::{}", name)) || fqn == name {
            let entry = ctx.def_map.get_entry(def_id);
            if matches!(entry.kind, DefKind::Fn | DefKind::ExternFn) {
                return Some(def_id);
            }
        }
    }

    None
}
