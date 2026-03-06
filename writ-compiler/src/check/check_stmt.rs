//! Statement type checking.

use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;

use super::check_expr::{check_expr, CheckCtx};
use super::env::{Mutability, resolve_ast_type_with_file};
use super::error::TypeError;
use super::ir::TypedStmt;
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
            let (final_ty, ann_span, ann_def_id) = if let Some(annotation) = ty {
                let generic_map = rustc_hash::FxHashMap::default();
                let annotated_ty =
                    resolve_ast_type_with_file(annotation, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file);

                if !ctx.is_error(annotated_ty) && !ctx.is_error(inferred_ty)
                    && ctx.unify.unify(annotated_ty, inferred_ty, &mut ctx.interner).is_err() {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: ctx.display_ty(annotated_ty),
                            found: ctx.display_ty(inferred_ty),
                            expected_span: *name_span,
                            found_span: typed_value.span(),
                            file: ctx.current_file,
                            help: None,
                        });
                    }

                // Capture annotation span and DefId for LSP go-to-def support
                let span_of_ann = match annotation {
                    AstType::Named { span, .. }
                    | AstType::Generic { span, .. }
                    | AstType::Array { span, .. }
                    | AstType::Func { span, .. }
                    | AstType::Void { span, .. } => Some(*span),
                };
                let def_id_of_ann = match annotation {
                    AstType::Named { name: type_name, .. } => {
                        // Try public FQN first (works for non-namespaced projects and
                        // FQN type names already prefixed by the resolver).
                        ctx.def_map.get(type_name).or_else(|| {
                            // Try namespace-prefixed FQN (required for namespaced projects
                            // where the type_name is a simple identifier like `MyStruct`
                            // but the DefMap stores it as `mymod::MyStruct`).
                            if !ctx.current_namespace.is_empty() {
                                let fqn = format!("{}::{}", ctx.current_namespace, type_name);
                                ctx.def_map.get(&fqn)
                            } else {
                                None
                            }
                        }).or_else(|| {
                            // Try file-private definitions (struct/enum without `pub`).
                            ctx.def_map.file_private
                                .get(&ctx.current_file)
                                .and_then(|privs| privs.get(type_name.as_str()).copied())
                        })
                    }
                    _ => None,
                };

                (annotated_ty, span_of_ann, def_id_of_ann)
            } else {
                // Detect bare `None` without type annotation.
                // check_ident injects None as Option<InferVar>. If no annotation constrains
                // the infer var, it stays unresolved -- emit a specific error.
                if let TyKind::Option(inner) = ctx.interner.kind(inferred_ty).clone()
                    && let TyKind::Infer(var) = ctx.interner.kind(inner).clone()
                        && ctx.unify.resolve(var).is_none() {
                            ctx.emit_error(TypeError::NoneWithoutAnnotation {
                                span: typed_value.span(),
                                file: ctx.current_file,
                            });
                        }
                (inferred_ty, None, None)
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
                type_ann_span: ann_span,
                type_ann_def_id: ann_def_id,
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
                    if !ctx.is_error(val_ty) && !ctx.is_error(ret_ty)
                        && ctx.unify.unify(ret_ty, val_ty, &mut ctx.interner).is_err() {
                            ctx.emit_error(TypeError::TypeMismatch {
                                expected: ctx.display_ty(ret_ty),
                                found: ctx.display_ty(val_ty),
                                expected_span: *span,
                                found_span: tv.span(),
                                file: ctx.current_file,
                                help: Some("return value type must match function return type".to_string()),
                            });
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
