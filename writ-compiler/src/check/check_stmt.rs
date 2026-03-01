//! Statement type checking.

use crate::ast::stmt::AstStmt;

use super::check_expr::{check_expr, CheckCtx};
use super::env::{Mutability, resolve_ast_type};
use super::error::TypeError;
use super::ir::*;
use super::ty::TyKind;

/// Type-check a statement, returning a TypedStmt.
pub fn check_stmt(ctx: &mut CheckCtx, stmt: &AstStmt) -> TypedStmt {
    match stmt {
        AstStmt::Let {
            mutable,
            name,
            name_span,
            ty,
            value,
            span,
        } => {
            let typed_value = check_expr(ctx, value);
            let inferred_ty = typed_value.ty();

            // If type annotation present, check compatibility
            let final_ty = if let Some(annotation) = ty {
                let generic_map = rustc_hash::FxHashMap::default();
                let annotated_ty =
                    resolve_ast_type(annotation, ctx.def_map, &mut ctx.interner, &generic_map);

                if !ctx.is_error(annotated_ty) && !ctx.is_error(inferred_ty) {
                    if let Err(_) = ctx.unify.unify(annotated_ty, inferred_ty, &mut ctx.interner) {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: ctx.display_ty(annotated_ty),
                            found: ctx.display_ty(inferred_ty),
                            expected_span: *name_span,
                            found_span: typed_value.span(),
                            file: ctx.current_file,
                            help: None,
                        });
                    }
                }
                annotated_ty
            } else {
                inferred_ty
            };

            // Define in local environment
            let mutability = if *mutable {
                Mutability::Mutable
            } else {
                Mutability::Immutable
            };
            ctx.local_env
                .define(name.clone(), final_ty, mutability, *name_span);

            TypedStmt::Let {
                name: name.clone(),
                name_span: *name_span,
                ty: final_ty,
                mutable: *mutable,
                value: typed_value,
                span: *span,
            }
        }

        AstStmt::Expr { expr, span } => {
            let typed_expr = check_expr(ctx, expr);
            TypedStmt::Expr {
                expr: typed_expr,
                span: *span,
            }
        }

        AstStmt::Return { value, span } => {
            let typed_value = value.as_ref().map(|v| check_expr(ctx, v));

            if let Some(ret_ty) = ctx.current_fn_ret {
                if let Some(ref tv) = typed_value {
                    let val_ty = tv.ty();
                    if !ctx.is_error(val_ty) && !ctx.is_error(ret_ty) {
                        if let Err(_) = ctx.unify.unify(ret_ty, val_ty, &mut ctx.interner) {
                            ctx.emit_error(TypeError::TypeMismatch {
                                expected: ctx.display_ty(ret_ty),
                                found: ctx.display_ty(val_ty),
                                expected_span: *span,
                                found_span: tv.span(),
                                file: ctx.current_file,
                                help: Some("return value type must match function return type".to_string()),
                            });
                        }
                    }
                } else {
                    // Return with no value: check function returns void
                    let void_ty = ctx.interner.void();
                    if !ctx.is_error(ret_ty) && ret_ty != void_ty {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: ctx.display_ty(ret_ty),
                            found: "void".to_string(),
                            expected_span: *span,
                            found_span: *span,
                            file: ctx.current_file,
                            help: Some("function expects a return value".to_string()),
                        });
                    }
                }
            }

            TypedStmt::Return {
                value: typed_value,
                span: *span,
            }
        }

        // Stubs for later plans
        AstStmt::For {
            binding,
            binding_span,
            iterable,
            body,
            span,
        } => {
            let typed_iterable = check_expr(ctx, iterable);
            let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
                TyKind::Array(elem) => elem,
                _ => ctx.interner.error(),
            };

            ctx.local_env.push_scope();
            ctx.local_env.define(
                binding.clone(),
                elem_ty,
                Mutability::Immutable,
                *binding_span,
            );

            let typed_body: Vec<TypedStmt> = body.iter().map(|s| check_stmt(ctx, s)).collect();
            ctx.local_env.pop_scope();

            TypedStmt::For {
                binding: binding.clone(),
                binding_span: *binding_span,
                binding_ty: elem_ty,
                mutable: false,
                iterable: typed_iterable,
                body: typed_body,
                span: *span,
            }
        }

        AstStmt::While {
            condition,
            body,
            span,
        } => {
            let typed_cond = check_expr(ctx, condition);
            let typed_body: Vec<TypedStmt> = body.iter().map(|s| check_stmt(ctx, s)).collect();
            TypedStmt::While {
                condition: typed_cond,
                body: typed_body,
                span: *span,
            }
        }

        AstStmt::Break { value, span } => {
            let typed_value = value.as_ref().map(|v| check_expr(ctx, v));
            TypedStmt::Break {
                value: typed_value,
                span: *span,
            }
        }

        AstStmt::Continue { span } => TypedStmt::Continue { span: *span },

        AstStmt::Atomic { body, span } => {
            let typed_body: Vec<TypedStmt> = body.iter().map(|s| check_stmt(ctx, s)).collect();
            TypedStmt::Atomic {
                body: typed_body,
                span: *span,
            }
        }

        AstStmt::Error { span } => TypedStmt::Error { span: *span },
    }
}
