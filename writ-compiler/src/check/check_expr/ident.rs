//! Identifier lookup during type checking.

use chumsky::span::SimpleSpan;

use crate::resolve::def_map::DefKind;
use super::CheckCtx;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;
use writ_diagnostics::{Diagnostic, code};

pub(super) fn check_ident(ctx: &mut CheckCtx, name: &str, span: SimpleSpan) -> TypedExpr {
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
                    // Emit W0006 for deprecated function-as-value references from different files.
                    // (Direct call sites are handled in check_call_with_sig; this covers fn values.)
                    emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
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
                    emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
                    return TypedExpr::Var {
                        ty,
                        span,
                        name: name.to_string(),
                    };
                }
            }
            DefKind::Global => {
                if let Some(&(ty, _)) = ctx.type_env.global_types.get(&def_id) {
                    emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
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
    for privates in ctx.def_map.file_private.values() {
        if let Some(&def_id) = privates.get(name) {
            let entry = ctx.def_map.get_entry(def_id);
            match entry.kind {
                DefKind::Fn | DefKind::ExternFn => {
                    if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                        emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
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
                        emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
                        return TypedExpr::Var {
                            ty,
                            span,
                            name: name.to_string(),
                        };
                    }
                }
                DefKind::Global => {
                    if let Some(&(ty, _)) = ctx.type_env.global_types.get(&def_id) {
                        emit_deprecated_warning_if_cross_file(ctx, def_id, name, span);
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

    // Entity namespace — used for Entity.getOrCreate<T>(), Entity.destroy(), etc.
    // Returns AnyEntity type so member access can resolve static methods.
    if name == "Entity" {
        let entity_ty = ctx.interner.any_entity();
        return TypedExpr::Var {
            ty: entity_ty,
            span,
            name: name.to_string(),
        };
    }

    // Sub-prelude builtin variant constructors.
    // check_ident does not go through ScopeChain -- check by name directly.
    // User-defined symbols in DefMap or local_env shadow these (checked above).
    match name {
        "None" | "Some" => {
            let infer_var = ctx.unify.new_var();
            let infer_ty = ctx.interner.intern(TyKind::Infer(infer_var));
            let opt_ty = ctx.interner.option(infer_ty);
            return TypedExpr::Var {
                ty: opt_ty,
                span,
                name: name.to_string(),
            };
        }
        "Ok" => {
            // Ok(val) constructor: fn(T) -> Result<T, E>
            let ok_infer = ctx.unify.new_var();
            let err_infer = ctx.unify.new_var();
            let ok_ty = ctx.interner.intern(TyKind::Infer(ok_infer));
            let err_ty = ctx.interner.intern(TyKind::Infer(err_infer));
            let res_ty = ctx.interner.result(ok_ty, err_ty);
            let fn_ty = ctx.interner.func(vec![ok_ty], res_ty);
            return TypedExpr::Var {
                ty: fn_ty,
                span,
                name: name.to_string(),
            };
        }
        "Err" => {
            // Err(val) constructor: fn(E) -> Result<T, E>
            let ok_infer = ctx.unify.new_var();
            let err_infer = ctx.unify.new_var();
            let ok_ty = ctx.interner.intern(TyKind::Infer(ok_infer));
            let err_ty = ctx.interner.intern(TyKind::Infer(err_infer));
            let res_ty = ctx.interner.result(ok_ty, err_ty);
            let fn_ty = ctx.interner.func(vec![err_ty], res_ty);
            return TypedExpr::Var {
                ty: fn_ty,
                span,
                name: name.to_string(),
            };
        }
        _ => {}
    }

    // Not found: emit error with poison
    let err_ty = ctx.emit_error(TypeError::UndefinedVariable {
        name: name.to_string(),
        span,
        file: ctx.current_file,
    });
    TypedExpr::Error { ty: err_ty, span }
}

/// Emit a W0006 warning if `def_id` is in deprecated_items and the definition
/// lives in a different file than `ctx.current_file`.
///
/// Called by `check_ident` for non-call ident references (function-as-value,
/// const, global). Direct call sites are handled in `check_call_with_sig`.
fn emit_deprecated_warning_if_cross_file(
    ctx: &mut CheckCtx,
    def_id: crate::resolve::def_map::DefId,
    item_name: &str,
    span: SimpleSpan,
) {
    if let Some(msg) = ctx.type_env.deprecated_items.get(&def_id) {
        let entry = ctx.def_map.get_entry(def_id);
        if entry.file_id != ctx.current_file {
            let warning_msg = if msg.is_empty() {
                format!("`{}` is deprecated", item_name)
            } else {
                format!("`{}` is deprecated: {}", item_name, msg)
            };
            ctx.diags.push(
                Diagnostic::warning(code::W0006, warning_msg)
                    .with_primary(ctx.current_file, span, "deprecated item used here")
                    .build(),
            );
        }
    }
}
