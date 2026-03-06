//! Function call type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstArg, AstExpr};
use crate::ast::types::AstType;
use crate::resolve::def_map::DefId;
use super::CheckCtx;
use super::check_expr;
use super::find_fn_def_id;
use super::super::env::FnSig;
use super::super::error::TypeError;
use super::super::infer::instantiate_generic_fn;
use super::super::ir::TypedExpr;
use super::super::ty::{InferVar, TyKind};

pub(super) fn check_call(
    ctx: &mut CheckCtx,
    callee: &AstExpr,
    args: &[AstArg],
    span: SimpleSpan,
) -> TypedExpr {
    // Special case: callee is an Ident that resolves to a function in type_env
    if let AstExpr::Ident { name, span: name_span } = callee {
        // Check if it's a known function by name
        if let Some(def_id) = find_fn_def_id(ctx, name)
            && let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                return check_call_with_sig(ctx, name, def_id, sig.clone(), args, span, *name_span);
            }

        // Sub-prelude builtin: `Some(expr)` constructs Option<T> from the argument.
        // Only fires when `Some` is not shadowed by a user-defined function (which is
        // handled above by the find_fn_def_id fast-path and early return).
        if name == "Some" {
            let typed_args: Vec<TypedExpr> =
                args.iter().map(|a| check_expr(ctx, &a.value)).collect();
            if typed_args.len() == 1 {
                let inner_ty = typed_args[0].ty();
                let opt_ty = ctx.interner.option(inner_ty);
                return TypedExpr::Call {
                    ty: opt_ty,
                    span,
                    callee: Box::new(TypedExpr::Var {
                        ty: opt_ty,
                        span: *name_span,
                        name: "Some".to_string(),
                    }),
                    args: typed_args,
                    callee_def_id: None,
                };
            } else {
                let err_ty = ctx.emit_error(TypeError::ArityMismatch {
                    fn_name: "Some".to_string(),
                    expected: 1,
                    found: typed_args.len(),
                    call_span: span,
                    def_span: *name_span,
                    file: ctx.current_file,
                });
                return TypedExpr::Call {
                    ty: err_ty,
                    span,
                    callee: Box::new(TypedExpr::Var {
                        ty: ctx.interner.error(),
                        span: *name_span,
                        name: "Some".to_string(),
                    }),
                    args: typed_args,
                    callee_def_id: None,
                };
            }
        }
    }

    // Special case: callee is a root-qualified single-segment Path (e.g. `::log`).
    // lower/expr.rs encodes `::log` as Path { segments: ["::log"] }, so after stripping
    // the leading `::` we can resolve it exactly like the Ident fast-path — this ensures
    // `callee_def_id` is set (enabling CALL_EXTERN for ExternFn callees instead of CALL_INDIRECT).
    if let AstExpr::Path { segments, span: path_span } = callee
        && segments.len() == 1 {
            let raw = &segments[0];
            let normalized = raw.strip_prefix("::").unwrap_or(raw.as_str());
            if let Some(def_id) = find_fn_def_id(ctx, normalized)
                && let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                    return check_call_with_sig(
                        ctx,
                        normalized,
                        def_id,
                        sig.clone(),
                        args,
                        span,
                        *path_span,
                    );
                }
        }

    // Special case: two-segment log namespace call — `log::debug(msg)` or `::log::debug(msg)`.
    // lower/expr.rs encodes `::log::debug` as Path { segments: ["::log", "debug"] }
    // (leading "::" on first segment only).  Strip the prefix, join as FQN, look up the
    // synthetic ExternFn DefId injected by inject_log_namespace.
    if let AstExpr::Path { segments, span: path_span } = callee
        && segments.len() == 2 {
            let first = segments[0].strip_prefix("::").unwrap_or(segments[0].as_str());
            let second = segments[1].as_str();
            if first == "log" {
                let fqn = format!("log::{}", second);
                if let Some(def_id) = ctx.def_map.get(&fqn)
                    && let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                        return check_call_with_sig(
                            ctx,
                            &fqn,
                            def_id,
                            sig.clone(),
                            args,
                            span,
                            *path_span,
                        );
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
                if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty)
                    && ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner).is_err() {
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

pub(super) fn check_call_with_sig(
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
        if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty)
            && ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner).is_err() {
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
pub(super) fn check_contract_bounds(
    ctx: &mut CheckCtx,
    sig: &FnSig,
    infer_vars: &[InferVar],
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
                TyKind::Struct(did) | TyKind::Class(did) | TyKind::Entity(did) | TyKind::Enum(did) => Some(did),
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

pub(super) fn check_generic_call(
    ctx: &mut CheckCtx,
    callee: &AstExpr,
    type_args: &[AstType],
    args: &[AstArg],
    span: SimpleSpan,
) -> TypedExpr {
    // Special case: `expr.into<T>()` — primitive type conversion via Into<T> contract.
    //
    // The fmt_string lowering generates GenericCall { callee: MemberAccess { field: "into" },
    // type_args: [T] } for interpolated expressions. We desugar this here to the
    // `field: "into_<T>"` sentinel pattern that the emitter already handles (builtins.rs).
    //
    // Supported pairs: int/float/bool/string -> string, int -> float, float -> int.
    if let AstExpr::MemberAccess { object, field, field_span: _, span: member_span } = callee
        && field == "into"
        && type_args.len() == 1
        && args.is_empty()
    {
        let typed_obj = check_expr(ctx, object);
        let src_kind = ctx.interner.kind(typed_obj.ty()).clone();
        let generic_map = rustc_hash::FxHashMap::default();
        let target_ty = super::super::env::resolve_ast_type_with_file(
            &type_args[0], ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file,
        );
        let target_kind = ctx.interner.kind(target_ty).clone();

        // Build the sentinel field name from (src_kind, target_kind)
        let sentinel: Option<(&str, _)> = match (&src_kind, &target_kind) {
            (TyKind::Int, TyKind::String) => Some(("into_string", target_ty)),
            (TyKind::Float, TyKind::String) => Some(("into_string", target_ty)),
            (TyKind::Bool, TyKind::String) => Some(("into_string", target_ty)),
            (TyKind::String, TyKind::String) => Some(("into_string", target_ty)),
            (TyKind::Int, TyKind::Float) => Some(("into_float", target_ty)),
            (TyKind::Float, TyKind::Int) => Some(("into_int", target_ty)),
            (TyKind::String, TyKind::Int) => Some(("into_int", target_ty)),
            (TyKind::String, TyKind::Float) => Some(("into_float", target_ty)),
            (TyKind::String, TyKind::Bool) => Some(("into_bool", target_ty)),
            _ => None,
        };

        if let Some((field_name, ret_ty)) = sentinel {
            let fn_ty = ctx.interner.func(vec![], ret_ty);
            let callee_typed = TypedExpr::Field {
                ty: fn_ty,
                span: *member_span,
                receiver: Box::new(typed_obj),
                field: field_name.to_string(),
            };
            return TypedExpr::Call {
                ty: ret_ty,
                span,
                callee: Box::new(callee_typed),
                args: vec![],
                callee_def_id: None,
            };
        }
        // If the pair is unsupported, fall through to the error path below
        // (which will call check_expr on callee and emit the unknown-field error)
    }

    // For generic calls, resolve the callee to get its FnSig
    if let AstExpr::Ident { name, span: name_span } = callee
        && let Some(def_id) = find_fn_def_id(ctx, name)
            && let Some(sig) = ctx.type_env.fn_sigs.get(&def_id).cloned() {
                // Resolve explicit type args
                let generic_map = rustc_hash::FxHashMap::default();
                let explicit_tys: Vec<_> = type_args
                    .iter()
                    .map(|ta| super::super::env::resolve_ast_type_with_file(ta, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file))
                    .collect();

                // Build substitution from explicit type args
                let subst = explicit_tys;

                // Substitute into param types
                let param_tys: Vec<_> = sig
                    .params
                    .iter()
                    .map(|(_, ty)| super::super::infer::substitute(*ty, &subst, &mut ctx.interner))
                    .collect();
                let ret_ty = super::super::infer::substitute(sig.ret, &subst, &mut ctx.interner);

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
                    if !ctx.is_error(arg_ty) && !ctx.is_error(param_ty)
                        && ctx.unify.unify(param_ty, arg_ty, &mut ctx.interner).is_err() {
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
