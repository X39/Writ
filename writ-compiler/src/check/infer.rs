//! Type inference helpers: ResolvedType -> Ty conversion, generic instantiation.

use super::env::FnSig;
use super::ty::{InferVar, Ty, TyInterner, TyKind};
use super::unify::UnifyCtx;

/// Create fresh InferVars for a generic function's type parameters,
/// then substitute them into the param types and return type.
pub fn instantiate_generic_fn(
    fn_sig: &FnSig,
    interner: &mut TyInterner,
    unify: &mut UnifyCtx,
) -> (Vec<Ty>, Ty, Vec<InferVar>) {
    if fn_sig.generics.is_empty() {
        // Non-generic: return params and ret as-is
        let param_tys: Vec<Ty> = fn_sig.params.iter().map(|(_, ty)| *ty).collect();
        return (param_tys, fn_sig.ret, Vec::new());
    }

    // Create fresh InferVars for each generic param
    let infer_vars: Vec<InferVar> = fn_sig
        .generics
        .iter()
        .map(|_| unify.new_var())
        .collect();

    // Build substitution map: GenericParam(i) -> Infer(var_i)
    let subst: Vec<Ty> = infer_vars
        .iter()
        .map(|var| interner.intern(TyKind::Infer(*var)))
        .collect();

    // Substitute into param types
    let param_tys: Vec<Ty> = fn_sig
        .params
        .iter()
        .map(|(_, ty)| substitute(*ty, &subst, interner))
        .collect();

    // Substitute into return type
    let ret = substitute(fn_sig.ret, &subst, interner);

    (param_tys, ret, infer_vars)
}

/// Substitute GenericParam(i) with subst[i] in a type.
pub fn substitute(ty: Ty, subst: &[Ty], interner: &mut TyInterner) -> Ty {
    match interner.full_kind(ty).clone() {
        TyKind::GenericParam(idx) => {
            if (idx as usize) < subst.len() {
                subst[idx as usize]
            } else {
                ty
            }
        }
        TyKind::Array(elem) => {
            let new_elem = substitute(elem, subst, interner);
            interner.array(new_elem)
        }
        TyKind::Option(inner) => {
            let new_inner = substitute(inner, subst, interner);
            interner.option(new_inner)
        }
        TyKind::Result(ok, err) => {
            let new_ok = substitute(ok, subst, interner);
            let new_err = substitute(err, subst, interner);
            interner.result(new_ok, new_err)
        }
        TyKind::TaskHandle(inner) => {
            let new_inner = substitute(inner, subst, interner);
            interner.task_handle(new_inner)
        }
        TyKind::GenericInstance {
            base,
            namespace,
            name,
            args,
        } => {
            let new_base = substitute(base, subst, interner);
            let new_args = args
                .into_iter()
                .map(|arg| substitute(arg, subst, interner))
                .collect();
            interner.generic_instance(new_base, namespace, name, new_args)
        }
        TyKind::Func { params, ret } => {
            let new_params: Vec<Ty> = params
                .iter()
                .map(|p| substitute(*p, subst, interner))
                .collect();
            let new_ret = substitute(ret, subst, interner);
            interner.func(new_params, new_ret)
        }
        // All other types don't contain generics
        _ => ty,
    }
}

/// Match a possibly-generic implementation pattern against a concrete type.
///
/// Generic parameters in `pattern` bind by ordinal. Repeated occurrences must
/// bind to the same structural type, so a pattern such as `Pair<T, T>` does not
/// match `Pair<int, string>`.
pub fn match_type_pattern(
    pattern: Ty,
    concrete: Ty,
    interner: &TyInterner,
) -> Option<Vec<(u32, Ty)>> {
    let mut bindings = Vec::new();
    if match_type_pattern_with_bindings(pattern, concrete, interner, &mut bindings) {
        Some(bindings)
    } else {
        None
    }
}

/// Continue matching a type pattern while preserving bindings established by
/// another part of the same implementation (for example, its target type).
pub fn match_type_pattern_with_bindings(
    pattern: Ty,
    concrete: Ty,
    interner: &TyInterner,
    bindings: &mut Vec<(u32, Ty)>,
) -> bool {
    if pattern == concrete {
        return true;
    }

    match (interner.full_kind(pattern), interner.full_kind(concrete)) {
        (TyKind::GenericParam(ordinal), _) => {
            if let Some((_, bound)) = bindings.iter().find(|(idx, _)| idx == ordinal) {
                types_equal_strict(*bound, concrete, interner)
            } else {
                bindings.push((*ordinal, concrete));
                true
            }
        }
        (
            TyKind::GenericInstance {
                base: pattern_base,
                args: pattern_args,
                ..
            },
            TyKind::GenericInstance {
                base: concrete_base,
                args: concrete_args,
                ..
            },
        ) => {
            pattern_args.len() == concrete_args.len()
                && types_equal_strict(*pattern_base, *concrete_base, interner)
                && pattern_args.iter().zip(concrete_args).all(|(pattern_arg, concrete_arg)| {
                    match_type_pattern_with_bindings(
                        *pattern_arg,
                        *concrete_arg,
                        interner,
                        bindings,
                    )
                })
        }
        (TyKind::Array(pattern), TyKind::Array(concrete))
        | (TyKind::Option(pattern), TyKind::Option(concrete))
        | (TyKind::TaskHandle(pattern), TyKind::TaskHandle(concrete))
        | (TyKind::ReflectionType(pattern), TyKind::ReflectionType(concrete)) => {
            match_type_pattern_with_bindings(*pattern, *concrete, interner, bindings)
        }
        (
            TyKind::Result(pattern_ok, pattern_err),
            TyKind::Result(concrete_ok, concrete_err),
        ) => {
            match_type_pattern_with_bindings(*pattern_ok, *concrete_ok, interner, bindings)
                && match_type_pattern_with_bindings(
                    *pattern_err,
                    *concrete_err,
                    interner,
                    bindings,
                )
        }
        (
            TyKind::Func {
                params: pattern_params,
                ret: pattern_ret,
            },
            TyKind::Func {
                params: concrete_params,
                ret: concrete_ret,
            },
        ) => {
            pattern_params.len() == concrete_params.len()
                && pattern_params.iter().zip(concrete_params).all(
                    |(pattern_param, concrete_param)| {
                        match_type_pattern_with_bindings(
                            *pattern_param,
                            *concrete_param,
                            interner,
                            bindings,
                        )
                    },
                )
                && match_type_pattern_with_bindings(
                    *pattern_ret,
                    *concrete_ret,
                    interner,
                    bindings,
                )
        }
        _ => false,
    }
}

/// Substitute only the generic ordinals bound while matching an impl pattern.
pub fn substitute_bindings(
    ty: Ty,
    bindings: &[(u32, Ty)],
    interner: &mut TyInterner,
) -> Ty {
    match interner.full_kind(ty).clone() {
        TyKind::GenericParam(ordinal) => bindings
            .iter()
            .find_map(|(idx, ty)| (*idx == ordinal).then_some(*ty))
            .unwrap_or(ty),
        TyKind::Array(elem) => {
            let elem = substitute_bindings(elem, bindings, interner);
            interner.array(elem)
        }
        TyKind::Option(inner) => {
            let inner = substitute_bindings(inner, bindings, interner);
            interner.option(inner)
        }
        TyKind::Result(ok, err) => {
            let ok = substitute_bindings(ok, bindings, interner);
            let err = substitute_bindings(err, bindings, interner);
            interner.result(ok, err)
        }
        TyKind::TaskHandle(inner) => {
            let inner = substitute_bindings(inner, bindings, interner);
            interner.task_handle(inner)
        }
        TyKind::ReflectionType(inner) => {
            let inner = substitute_bindings(inner, bindings, interner);
            interner.reflection_type(inner)
        }
        TyKind::GenericInstance {
            base,
            namespace,
            name,
            args,
        } => {
            let base = substitute_bindings(base, bindings, interner);
            let args = args
                .into_iter()
                .map(|arg| substitute_bindings(arg, bindings, interner))
                .collect();
            interner.generic_instance(base, namespace, name, args)
        }
        TyKind::Func { params, ret } => {
            let params = params
                .into_iter()
                .map(|param| substitute_bindings(param, bindings, interner))
                .collect();
            let ret = substitute_bindings(ret, bindings, interner);
            interner.func(params, ret)
        }
        _ => ty,
    }
}

/// Structural equality that does not treat `GenericParam` as a wildcard.
pub fn types_equal_strict(a: Ty, b: Ty, interner: &TyInterner) -> bool {
    if a == b {
        return true;
    }

    match (interner.full_kind(a), interner.full_kind(b)) {
        (
            TyKind::GenericInstance {
                base: a_base,
                args: a_args,
                ..
            },
            TyKind::GenericInstance {
                base: b_base,
                args: b_args,
                ..
            },
        ) => {
            a_args.len() == b_args.len()
                && types_equal_strict(*a_base, *b_base, interner)
                && a_args
                    .iter()
                    .zip(b_args)
                    .all(|(a_arg, b_arg)| types_equal_strict(*a_arg, *b_arg, interner))
        }
        (TyKind::Array(a), TyKind::Array(b))
        | (TyKind::Option(a), TyKind::Option(b))
        | (TyKind::TaskHandle(a), TyKind::TaskHandle(b))
        | (TyKind::ReflectionType(a), TyKind::ReflectionType(b)) => {
            types_equal_strict(*a, *b, interner)
        }
        (TyKind::Result(a_ok, a_err), TyKind::Result(b_ok, b_err)) => {
            types_equal_strict(*a_ok, *b_ok, interner)
                && types_equal_strict(*a_err, *b_err, interner)
        }
        (
            TyKind::Func {
                params: a_params,
                ret: a_ret,
            },
            TyKind::Func {
                params: b_params,
                ret: b_ret,
            },
        ) => {
            a_params.len() == b_params.len()
                && a_params
                    .iter()
                    .zip(b_params)
                    .all(|(a_param, b_param)| {
                        types_equal_strict(*a_param, *b_param, interner)
                    })
                && types_equal_strict(*a_ret, *b_ret, interner)
        }
        _ => false,
    }
}
