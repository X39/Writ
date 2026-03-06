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
    match interner.kind(ty).clone() {
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
