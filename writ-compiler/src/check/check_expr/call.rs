//! Function call type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstArg, AstExpr};
use crate::ast::types::AstType;
use crate::resolve::def_map::DefId;
use super::CheckCtx;
use super::check_expr;
use super::{find_fn_def_id, find_fn_candidates};
use super::super::env::FnSig;
use super::super::error::TypeError;
use super::super::infer::instantiate_generic_fn;
use super::super::ir::TypedExpr;
use super::super::ty::{InferVar, TyKind};
use writ_diagnostics::{Diagnostic, code};

pub(super) fn check_call(
    ctx: &mut CheckCtx,
    callee: &AstExpr,
    args: &[AstArg],
    span: SimpleSpan,
) -> TypedExpr {
    // Special case: callee is an Ident that resolves to a function in type_env
    if let AstExpr::Ident { name, span: name_span } = callee {
        // Check if it's a known function by name (with overload resolution)
        if let Some(result) = resolve_overloaded_call(ctx, name, args, span, *name_span) {
            return result;
        }

        // Sub-prelude builtin: `Some(expr)` constructs Option<T> from the argument.
        // Only fires when `Some` is not shadowed by a user-defined function (which is
        // handled above by the overload resolution and early return).
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
                    callee_has_receiver: None,
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
                    callee_has_receiver: None,
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
            if let Some(result) = resolve_overloaded_call(ctx, normalized, args, span, *path_span) {
                return result;
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

    // General case: check callee expression. For an overloaded method, resolve
    // the candidate from the call arguments before collapsing member access to
    // a single function type.
    let typed_callee = if let AstExpr::MemberAccess {
        object,
        field,
        field_span,
        span: member_span,
    } = callee
    {
        let typed_receiver = check_expr(ctx, object);
        if let Some(call) = resolve_overloaded_method_call(
            ctx,
            typed_receiver.clone(),
            field,
            args,
            span,
            *member_span,
        ) {
            return call;
        }
        super::access::check_typed_member_access(
            ctx,
            typed_receiver,
            field,
            *field_span,
            *member_span,
        )
    } else {
        check_expr(ctx, callee)
    };
    let callee_ty = typed_callee.ty();

    if ctx.is_error(callee_ty) {
        let typed_args: Vec<TypedExpr> = args.iter().map(|a| check_expr(ctx, &a.value)).collect();
        return TypedExpr::Call {
            ty: ctx.interner.error(),
            span,
            callee: Box::new(typed_callee),
            args: typed_args,
            callee_def_id: None,
            callee_has_receiver: None,
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
                    callee_has_receiver: None,
                };
            }

            // Check each argument type
            for (i, (arg, &param_ty)) in typed_args.iter().zip(params.iter()).enumerate() {
                let arg_ty = arg.ty();
                ctx.check_assignable(
                    param_ty,
                    arg_ty,
                    span,
                    arg.span(),
                    Some(format!("in argument {}", i + 1)),
                );
            }

            let resolved_ret = ctx.unify.resolve_ty_deep(ret, &mut ctx.interner);

            TypedExpr::Call {
                ty: resolved_ret,
                span,
                callee: Box::new(typed_callee),
                args: typed_args,
                callee_def_id: None,
                callee_has_receiver: None,
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
                callee_has_receiver: None,
            }
        }
    }
}

/// Resolve method calls before ordinary member access erases declaration
/// identity. Keeping the selected declaration's open signature on the callee
/// lets metadata lookup distinguish generic methods even when other modules
/// contribute same-name rows; the call result still uses inferred types.
fn resolve_overloaded_method_call(
    ctx: &mut CheckCtx,
    typed_receiver: TypedExpr,
    method_name: &str,
    args: &[AstArg],
    call_span: SimpleSpan,
    member_span: SimpleSpan,
) -> Option<TypedExpr> {
    let receiver_ty = typed_receiver.ty();
    let receiver_def_id = match ctx.interner.kind(receiver_ty) {
        TyKind::Struct(def_id)
        | TyKind::Class(def_id)
        | TyKind::Entity(def_id)
        | TyKind::Enum(def_id) => *def_id,
        _ => return None,
    };

    // Preserve field-before-method lookup for types that have a field with the
    // same name as an overloaded method set.
    let has_field = ctx
        .type_env
        .struct_fields
        .get(&receiver_def_id)
        .or_else(|| ctx.type_env.entity_fields.get(&receiver_def_id))
        .is_some_and(|fields| fields.iter().any(|(name, _, _)| name == method_name));
    if has_field {
        return None;
    }

    let mut candidates = Vec::new();
    for implementation in ctx.type_env.impl_index.get(&receiver_def_id)? {
        let Some(bindings) = super::super::infer::match_type_pattern(
            implementation.target_ty,
            receiver_ty,
            &ctx.interner,
        ) else {
            continue;
        };
        for (_, signature) in implementation
            .methods
            .iter()
            .filter(|(name, _)| name == method_name)
        {
            candidates.push((
                implementation.contract_def_id.is_none(),
                implementation.impl_def_id,
                implementation.impl_generic_count,
                bindings.clone(),
                signature.clone(),
            ));
        }
    }
    if candidates
        .iter()
        .any(|(is_inherent, _, _, _, _)| *is_inherent)
    {
        candidates.retain(|(is_inherent, _, _, _, _)| *is_inherent);
    }
    if candidates.is_empty() {
        return None;
    }
    let candidate_impl = candidates[0].1;
    if candidates
        .iter()
        .any(|(_, impl_def_id, _, _, _)| *impl_def_id != candidate_impl)
    {
        // Multiple matching impl specializations are an implementation
        // ambiguity (E0125), not an overload ambiguity (E0124). Defer to the
        // ordinary member resolver, which owns that diagnostic.
        return None;
    }

    let typed_args: Vec<TypedExpr> = args
        .iter()
        .map(|arg| check_expr(ctx, &arg.value))
        .collect();
    let matching: Vec<_> = candidates
        .iter()
        .filter_map(|(is_inherent, _, impl_generic_count, receiver_bindings, signature)| {
            let mut bindings = receiver_bindings.clone();
            (signature.params.len() == typed_args.len()
                && typed_args
                    .iter()
                    .zip(&signature.params)
                    .all(|(arg, (_, param_ty))| {
                        ctx.is_error(arg.ty())
                            || ctx.is_error(*param_ty)
                            || super::super::infer::match_type_pattern_with_bindings(
                                *param_ty,
                                arg.ty(),
                                &ctx.interner,
                                &mut bindings,
                            )
                    }))
            .then(|| {
                (
                    *is_inherent,
                    *impl_generic_count,
                    bindings,
                    signature.clone(),
                )
            })
        })
        .collect();

    if matching.is_empty() {
        let error_ty = if let Some((_, _, _, receiver_bindings, same_arity)) = candidates
            .iter()
            .find(|(_, _, _, _, signature)| signature.params.len() == typed_args.len())
        {
            let mut bindings = receiver_bindings.clone();
            let mismatch = typed_args
                .iter()
                .zip(&same_arity.params)
                .find(|(arg, (_, param_ty))| {
                    !ctx.is_error(arg.ty())
                        && !ctx.is_error(*param_ty)
                        && !super::super::infer::match_type_pattern_with_bindings(
                            *param_ty,
                            arg.ty(),
                            &ctx.interner,
                            &mut bindings,
                        )
                });
            if let Some((arg, (_, param_ty))) = mismatch {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: ctx.display_ty(*param_ty),
                    found: ctx.display_ty(arg.ty()),
                    expected_span: member_span,
                    found_span: arg.span(),
                    file: ctx.current_file,
                    help: Some(format!("in call to `{}`", method_name)),
                })
            } else {
                ctx.emit_error(TypeError::NotCallable {
                    ty_name: format!("overload set `{}`", method_name),
                    span: call_span,
                    file: ctx.current_file,
                })
            }
        } else {
            ctx.emit_error(TypeError::ArityMismatch {
                fn_name: method_name.to_string(),
                expected: candidates[0].4.params.len(),
                found: typed_args.len(),
                call_span,
                def_span: member_span,
                file: ctx.current_file,
            })
        };
        return Some(TypedExpr::Call {
            ty: error_ty,
            span: call_span,
            callee: Box::new(TypedExpr::Field {
                ty: error_ty,
                span: member_span,
                receiver: Box::new(typed_receiver),
                field: method_name.to_string(),
            }),
            args: typed_args,
            callee_def_id: None,
            callee_has_receiver: None,
        });
    }

    let (_, impl_generic_count, mut bindings, signature) = match matching.len() {
        1 => matching.into_iter().next().unwrap(),
        count => {
            let error_ty = ctx.emit_error(TypeError::AmbiguousOverload {
                fn_name: method_name.to_string(),
                candidate_count: count,
                call_span,
                file: ctx.current_file,
            });
            return Some(TypedExpr::Call {
                ty: error_ty,
                span: call_span,
                callee: Box::new(TypedExpr::Field {
                    ty: error_ty,
                    span: member_span,
                    receiver: Box::new(typed_receiver),
                    field: method_name.to_string(),
                }),
                args: typed_args,
                callee_def_id: None,
                callee_has_receiver: None,
            });
        }
    };

    let mut infer_vars = Vec::with_capacity(signature.generics.len());
    for method_index in 0..signature.generics.len() {
        let ordinal = impl_generic_count + method_index as u32;
        let var = ctx.unify.new_var();
        let infer_ty = ctx.interner.intern(TyKind::Infer(var));
        if let Some(bound_ty) = bindings
            .iter()
            .find_map(|(index, ty)| (*index == ordinal).then_some(*ty))
        {
            let _ = ctx.unify.unify(infer_ty, bound_ty, &mut ctx.interner);
        }
        if let Some((_, bound_ty)) = bindings.iter_mut().find(|(index, _)| *index == ordinal) {
            *bound_ty = infer_ty;
        } else {
            bindings.push((ordinal, infer_ty));
        }
        infer_vars.push(var);
    }
    let param_tys: Vec<_> = signature
        .params
        .iter()
        .map(|(_, ty)| {
            super::super::infer::substitute_bindings(*ty, &bindings, &mut ctx.interner)
        })
        .collect();
    let ret_ty = super::super::infer::substitute_bindings(
        signature.ret,
        &bindings,
        &mut ctx.interner,
    );
    for (index, (arg, &param_ty)) in typed_args.iter().zip(&param_tys).enumerate() {
        ctx.check_assignable(
            param_ty,
            arg.ty(),
            member_span,
            arg.span(),
            Some(format!("in argument {} of `{}`", index + 1, method_name)),
        );
    }
    let resolved_ret = ctx.unify.resolve_ty_deep(ret_ty, &mut ctx.interner);
    if !signature.generics.is_empty() && !signature.bounds.is_empty() {
        check_contract_bounds(ctx, &signature, &infer_vars, call_span);
    }
    let declaration_params = signature.params.iter().map(|(_, ty)| *ty).collect();
    let callee_ty = ctx.interner.func(declaration_params, signature.ret);

    Some(TypedExpr::Call {
        ty: resolved_ret,
        span: call_span,
        callee: Box::new(TypedExpr::Field {
            ty: callee_ty,
            span: member_span,
            receiver: Box::new(typed_receiver),
            field: method_name.to_string(),
        }),
        args: typed_args,
        callee_def_id: None,
        callee_has_receiver: Some(signature.self_param.is_some()),
    })
}

/// Resolve an overloaded function call by name. Checks all candidates and picks
/// the one whose parameter count and types match the call-site arguments.
///
/// Returns `None` if no candidates are found (so the caller can fall through).
fn resolve_overloaded_call(
    ctx: &mut CheckCtx,
    name: &str,
    args: &[AstArg],
    span: SimpleSpan,
    name_span: SimpleSpan,
) -> Option<TypedExpr> {
    let candidates = find_fn_candidates(ctx, name);
    if candidates.is_empty() {
        return None;
    }

    // Single candidate — fast path (no overload resolution needed)
    if candidates.len() == 1 {
        let def_id = candidates[0];
        if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
            return Some(check_call_with_sig(ctx, name, def_id, sig.clone(), args, span, name_span));
        }
        return None;
    }

    // Multiple candidates — overload resolution.
    // Type-check args once, then match against each candidate's signature.
    let typed_args: Vec<TypedExpr> = args.iter().map(|a| check_expr(ctx, &a.value)).collect();
    let arg_count = typed_args.len();

    let candidate_sigs: Vec<(DefId, FnSig)> = candidates
        .iter()
        .filter_map(|def_id| {
            ctx.type_env
                .fn_sigs
                .get(def_id)
                .cloned()
                .map(|signature| (*def_id, signature))
        })
        .collect();
    let matching: Vec<(DefId, FnSig)> = candidate_sigs
        .iter()
        .filter_map(|(def_id, signature)| {
            let mut bindings = Vec::new();
            (signature.params.len() == arg_count
                && typed_args
                    .iter()
                    .zip(&signature.params)
                    .all(|(arg, (_, param_ty))| {
                        ctx.is_error(arg.ty())
                            || ctx.is_error(*param_ty)
                            || super::super::infer::match_type_pattern_with_bindings(
                                *param_ty,
                                arg.ty(),
                                &ctx.interner,
                                &mut bindings,
                            )
                    }))
            .then(|| (*def_id, signature.clone()))
        })
        .collect();

    match matching.len() {
        0 => {
            let err_ty = if let Some((def_id, same_arity)) = candidate_sigs
                .iter()
                .find(|(_, signature)| signature.params.len() == arg_count)
            {
                let mut bindings = Vec::new();
                let mismatch = typed_args
                    .iter()
                    .zip(&same_arity.params)
                    .find(|(arg, (_, param_ty))| {
                        !ctx.is_error(arg.ty())
                            && !ctx.is_error(*param_ty)
                            && !super::super::infer::match_type_pattern_with_bindings(
                                *param_ty,
                                arg.ty(),
                                &ctx.interner,
                                &mut bindings,
                            )
                    });
                if let Some((arg, (_, param_ty))) = mismatch {
                    let def_span = ctx.def_map.get_entry(*def_id).name_span;
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(*param_ty),
                        found: ctx.display_ty(arg.ty()),
                        expected_span: def_span,
                        found_span: arg.span(),
                        file: ctx.current_file,
                        help: Some(format!("in call to `{}`", name)),
                    })
                } else {
                    ctx.emit_error(TypeError::NotCallable {
                        ty_name: format!("overload set `{}`", name),
                        span,
                        file: ctx.current_file,
                    })
                }
            } else if let Some((def_id, nearest_arity)) = candidate_sigs
                .iter()
                .min_by_key(|(_, signature)| signature.params.len().abs_diff(arg_count))
            {
                ctx.emit_error(TypeError::ArityMismatch {
                    fn_name: name.to_string(),
                    expected: nearest_arity.params.len(),
                    found: arg_count,
                    call_span: span,
                    def_span: ctx.def_map.get_entry(*def_id).name_span,
                    file: ctx.current_file,
                })
            } else {
                return None;
            };
            Some(TypedExpr::Call {
                ty: err_ty,
                span,
                callee: Box::new(TypedExpr::Var {
                    ty: err_ty,
                    span: name_span,
                    name: name.to_string(),
                }),
                args: typed_args,
                callee_def_id: None,
                callee_has_receiver: None,
            })
        }
        1 => {
            // Exactly one match — call it (re-check with proper unification)
            let (def_id, sig) = matching.into_iter().next().unwrap();
            let entry = ctx.def_map.get_entry(def_id);
            let def_span = entry.name_span;
            let (param_tys, ret_ty, infer_vars) =
                instantiate_generic_fn(&sig, &mut ctx.interner, &mut ctx.unify);

            // Unify argument types for real
            for (i, (arg, &param_ty)) in typed_args.iter().zip(param_tys.iter()).enumerate() {
                let arg_ty = arg.ty();
                ctx.check_assignable(
                    param_ty,
                    arg_ty,
                    def_span,
                    arg.span(),
                    Some(format!("in argument {} of `{}`", i + 1, name)),
                );
            }

            let resolved_ret = ctx.unify.resolve_ty_deep(ret_ty, &mut ctx.interner);

            if !sig.generics.is_empty() && !sig.bounds.is_empty() {
                check_contract_bounds(ctx, &sig, &infer_vars, span);
            }

            Some(TypedExpr::Call {
                ty: resolved_ret,
                span,
                callee: Box::new(TypedExpr::Var {
                    ty: ctx.interner.func(param_tys, resolved_ret),
                    span: name_span,
                    name: name.to_string(),
                }),
                args: typed_args,
                callee_def_id: Some(def_id),
                callee_has_receiver: Some(false),
            })
        }
        _ => {
            // Ambiguous — emit error
            let err_ty = ctx.emit_error(TypeError::AmbiguousOverload {
                fn_name: name.to_string(),
                candidate_count: matching.len(),
                call_span: span,
                file: ctx.current_file,
            });
            Some(TypedExpr::Call {
                ty: err_ty,
                span,
                callee: Box::new(TypedExpr::Var {
                    ty: ctx.interner.error(),
                    span: name_span,
                    name: name.to_string(),
                }),
                args: typed_args,
                callee_def_id: None,
                callee_has_receiver: None,
            })
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

    // Emit W0006 if this function is deprecated and called from a different file.
    if let Some(msg) = ctx.type_env.deprecated_items.get(&def_id) {
        if entry.file_id != ctx.current_file {
            let warning_msg = if msg.is_empty() {
                format!("`{}` is deprecated", fn_name)
            } else {
                format!("`{}` is deprecated: {}", fn_name, msg)
            };
            ctx.diags.push(
                Diagnostic::warning(code::W0006, warning_msg)
                    .with_primary(ctx.current_file, name_span, "deprecated item used here")
                    .build(),
            );
        }
    }

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
            callee_has_receiver: None,
        };
    }

    // Check each argument type
    for (i, (arg, &param_ty)) in typed_args.iter().zip(param_tys.iter()).enumerate() {
        let arg_ty = arg.ty();
        ctx.check_assignable(
            param_ty,
            arg_ty,
            def_span,
            arg.span(),
            Some(format!("in argument {} of `{}`", i + 1, fn_name)),
        );
    }

    // Resolve return type (may contain InferVars now resolved)
    let resolved_ret = ctx.unify.resolve_ty_deep(ret_ty, &mut ctx.interner);

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
        callee_has_receiver: Some(false),
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
                TyKind::Struct(did) | TyKind::Class(did) | TyKind::Entity(did) | TyKind::Enum(did) | TyKind::Contract(did) => Some(did),
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
                    let bound_decl_span = if i < sig.bound_decl_spans.len() {
                        sig.bound_decl_spans[i]
                    } else {
                        call_span // fallback: point to call site if no span available
                    };
                    ctx.emit_error(TypeError::UnsatisfiedBound {
                        ty_name: ty_name.clone(),
                        bound_name: contract_name.clone(),
                        call_span,
                        file: ctx.current_file,
                        bound_decl_span,
                        bound_decl_file: sig.fn_file,
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
        let target_ty = ctx.resolve_ast_type(&type_args[0]);
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
                callee_has_receiver: None,
            };
        }
        // If the pair is unsupported, fall through to the error path below
        // (which will call check_expr on callee and emit the unknown-field error)
    }

    // Special case: Entity.getOrCreate<T>() — returns T (specific entity type)
    if let AstExpr::MemberAccess { object, field, field_span: _, span: member_span } = callee
        && field == "getOrCreate"
        && type_args.len() == 1
        && args.is_empty()
    {
        let typed_obj = check_expr(ctx, object);
        let obj_kind = ctx.interner.kind(typed_obj.ty()).clone();
        if matches!(obj_kind, TyKind::AnyEntity) {
            let entity_type = ctx.resolve_ast_type(&type_args[0]);
            let fn_ty = ctx.interner.func(vec![], entity_type);
            let callee_typed = TypedExpr::Field {
                ty: fn_ty,
                span: *member_span,
                receiver: Box::new(typed_obj),
                field: "getOrCreate".to_string(),
            };
            return TypedExpr::Call {
                ty: entity_type,
                span,
                callee: Box::new(callee_typed),
                args: vec![],
                callee_def_id: None,
                callee_has_receiver: None,
            };
        }
    }

    // For generic calls, resolve the callee to get its FnSig
    if let AstExpr::Ident { name, span: name_span } = callee
        && let Some(def_id) = find_fn_def_id(ctx, name)
            && let Some(sig) = ctx.type_env.fn_sigs.get(&def_id).cloned() {
                // Resolve explicit type args
                let explicit_tys: Vec<_> = type_args
                    .iter()
                    .map(|ta| ctx.resolve_ast_type(ta))
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
                        callee_has_receiver: None,
                    };
                }

                // Check each arg type
                for (i, (arg, &param_ty)) in typed_args.iter().zip(param_tys.iter()).enumerate() {
                    let arg_ty = arg.ty();
                    ctx.check_assignable(
                        param_ty,
                        arg_ty,
                        span,
                        arg.span(),
                        Some(format!("in argument {} of `{}`", i + 1, name)),
                    );
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
                    callee_has_receiver: Some(false),
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
        callee_has_receiver: None,
    }
}
