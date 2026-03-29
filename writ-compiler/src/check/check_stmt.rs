//! Statement type checking.

use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;

use crate::ast::expr::AstExpr;

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

                // Contract assignability: if annotation resolves to a contract type,
                // check that the inferred concrete type implements the contract.
                // Otherwise perform normal unification.
                if let TyKind::Contract(contract_def_id) = ctx.interner.kind(annotated_ty).clone() {
                    // Skip assignability check if inferred type is Error (poison propagation)
                    if !ctx.is_error(inferred_ty) {
                        let concrete_def_id = match ctx.interner.kind(inferred_ty).clone() {
                            TyKind::Struct(did) | TyKind::Class(did) | TyKind::Entity(did) => Some(did),
                            TyKind::Contract(did) if did == contract_def_id => None, // same contract, already valid
                            _ => {
                                // Primitive or other type cannot implement a contract
                                let contract_entry = ctx.def_map.get_entry(contract_def_id);
                                ctx.emit_error(TypeError::MissingContractImpl {
                                    ty_name: ctx.display_ty(inferred_ty),
                                    contract_name: contract_entry.name.clone(),
                                    span: *name_span,
                                    file: ctx.current_file,
                                    suggestion: format!(
                                        "add `impl {} for {}`",
                                        contract_entry.name,
                                        ctx.display_ty(inferred_ty)
                                    ),
                                });
                                None
                            }
                        };
                        if let Some(concrete_did) = concrete_def_id {
                            let satisfies = ctx.type_env.impl_index.get(&concrete_did)
                                .map(|impls| impls.iter().any(|e| e.contract_def_id == Some(contract_def_id)))
                                .unwrap_or(false);
                            if !satisfies {
                                let contract_entry = ctx.def_map.get_entry(contract_def_id);
                                ctx.emit_error(TypeError::MissingContractImpl {
                                    ty_name: ctx.display_ty(inferred_ty),
                                    contract_name: contract_entry.name.clone(),
                                    span: *name_span,
                                    file: ctx.current_file,
                                    suggestion: format!(
                                        "add `impl {} for {}`",
                                        contract_entry.name,
                                        ctx.display_ty(inferred_ty)
                                    ),
                                });
                            }
                        }
                    }
                } else if !ctx.is_error(annotated_ty) && !ctx.is_error(inferred_ty)
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

                    // Contract assignability: if the declared return type is a contract,
                    // check that the concrete value type implements the contract rather
                    // than doing plain unification (which would fail for concrete→contract).
                    if let TyKind::Contract(contract_def_id) = ctx.interner.kind(ret_ty).clone() {
                        if !ctx.is_error(val_ty) {
                            let concrete_def_id = match ctx.interner.kind(val_ty).clone() {
                                TyKind::Struct(did) | TyKind::Class(did) | TyKind::Entity(did) => Some(did),
                                TyKind::Contract(did) if did == contract_def_id => None, // same contract, valid
                                _ => {
                                    let contract_entry = ctx.def_map.get_entry(contract_def_id);
                                    ctx.emit_error(TypeError::MissingContractImpl {
                                        ty_name: ctx.display_ty(val_ty),
                                        contract_name: contract_entry.name.clone(),
                                        span: tv.span(),
                                        file: ctx.current_file,
                                        suggestion: format!(
                                            "add `impl {} for {}`",
                                            contract_entry.name,
                                            ctx.display_ty(val_ty)
                                        ),
                                    });
                                    None
                                }
                            };
                            if let Some(concrete_did) = concrete_def_id {
                                let satisfies = ctx.type_env.impl_index.get(&concrete_did)
                                    .map(|impls| impls.iter().any(|e| e.contract_def_id == Some(contract_def_id)))
                                    .unwrap_or(false);
                                if !satisfies {
                                    let contract_entry = ctx.def_map.get_entry(contract_def_id);
                                    ctx.emit_error(TypeError::MissingContractImpl {
                                        ty_name: ctx.display_ty(val_ty),
                                        contract_name: contract_entry.name.clone(),
                                        span: tv.span(),
                                        file: ctx.current_file,
                                        suggestion: format!(
                                            "add `impl {} for {}`",
                                            contract_entry.name,
                                            ctx.display_ty(val_ty)
                                        ),
                                    });
                                }
                            }
                        }
                    } else if !ctx.is_error(val_ty) && !ctx.is_error(ret_ty)
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
            // Warn if iterable is `[start..end]` — likely meant `start..end`
            if let AstExpr::ArrayLit { elements, .. } = iterable {
                if elements.iter().any(|e| matches!(e, AstExpr::Range { .. })) {
                    ctx.diags.push(
                        writ_diagnostics::Diagnostic::warning(
                            writ_diagnostics::code::W0005,
                            "array literal containing a range in for loop — did you mean `for i in start..end`?",
                        )
                        .with_primary(ctx.current_file, *span, "this iterates over the array, not the range")
                        .with_help("remove the brackets to iterate the range directly: `for i in 2..n`")
                        .build(),
                    );
                }
            }

            let typed_iterable = check_expr(ctx, iterable);

            // Detect whether the iterable is a class type implementing Iterable<T>.
            // Since Iterable and Iterator are prelude contracts, they have no DefId in the
            // user module's DefMap. Detection uses method-name matching on impl entries.
            let mut _class_iterable: Option<crate::resolve::def_map::DefId> = None;

            let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
                TyKind::Array(elem) => elem,
                TyKind::Class(class_def_id) => {
                    // Check if this class has an impl entry containing an "iterator" method.
                    // Iterable<T> is a prelude contract (no DefId), so we match by method name.
                    let has_iterator_method = ctx.type_env.impl_index.get(&class_def_id)
                        .map(|impls| impls.iter().any(|entry| {
                            entry.methods.iter().any(|(name, _)| name == "iterator")
                        }))
                        .unwrap_or(false);

                    if has_iterator_method {
                        _class_iterable = Some(class_def_id);
                        // Element type: create a fresh InferVar. The loop body constrains it
                        // (e.g. `sum = sum + x` will unify x with int).
                        // Full generic specialization tracking is deferred (Phase 119+).
                        let var = ctx.unify.new_var();
                        ctx.interner.intern(TyKind::Infer(var))
                    } else {
                        ctx.emit_error(crate::check::error::TypeError::NotIterable {
                            ty_name: ctx.display_ty(typed_iterable.ty()),
                            span: *span,
                            file: ctx.current_file,
                        })
                    }
                }
                _ => {
                    // Check if the iterable is a Range expression -- ranges iterate as int
                    if matches!(&typed_iterable, crate::check::ir::TypedExpr::Range { .. }) {
                        ctx.interner.int()
                    } else {
                        ctx.emit_error(crate::check::error::TypeError::NotIterable {
                            ty_name: ctx.display_ty(typed_iterable.ty()),
                            span: *span,
                            file: ctx.current_file,
                        })
                    }
                }
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
                // Contract DefIds: None for prelude contracts (Iterable, Iterator have no
                // user-module DefId). The emitter uses type_ref_token_by_name as a fallback.
                // class_iterable is Some when this is a class Iterable for-in loop.
                iterable_contract_def_id: None,
                iterator_contract_def_id: None,
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
