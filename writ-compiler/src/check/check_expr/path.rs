//! Qualified path resolution during type checking.

use chumsky::span::SimpleSpan;

use crate::resolve::def_map::DefKind;
use super::CheckCtx;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;

pub(super) fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    // Normalize root-qualified paths: lower/expr.rs encodes `::log` as
    // Path { segments: ["::log"] } (leading "::" prepended to first segment).
    // Strip the prefix before DefMap lookup so "::log" resolves to "log".
    let normalized_segments: Vec<String> = {
        let mut segs = segments.to_vec();
        if let Some(first) = segs.first_mut()
            && let Some(stripped) = first.strip_prefix("::") {
                *first = stripped.to_string();
            }
        segs
    };
    let fqn = normalized_segments.join("::");
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

    // Handle Option::None and Option::Some as constructor expressions.
    if normalized_segments.len() == 2 && normalized_segments[0] == "Option" {
        match normalized_segments[1].as_str() {
            "None" => {
                let infer_var = ctx.unify.new_var();
                let infer_ty = ctx.interner.intern(TyKind::Infer(infer_var));
                let opt_ty = ctx.interner.option(infer_ty);
                return TypedExpr::Var {
                    ty: opt_ty,
                    span,
                    name: "None".to_string(),
                };
            }
            "Some" => {
                // Return a function type fn(T) -> Option<T> for use as a callee.
                let infer_var = ctx.unify.new_var();
                let infer_ty = ctx.interner.intern(TyKind::Infer(infer_var));
                let opt_ty = ctx.interner.option(infer_ty);
                let fn_ty = ctx.interner.func(vec![infer_ty], opt_ty);
                return TypedExpr::Var {
                    ty: fn_ty,
                    span,
                    name: "Some".to_string(),
                };
            }
            _ => {}
        }
    }

    // Try enum variant path like `Direction::North`
    if normalized_segments.len() == 2 {
        let enum_name = &normalized_segments[0];
        let variant_name = &normalized_segments[1];

        // Look up enum by name: first in by_fqn (pub), then in file_private
        let mut enum_def_id_opt = ctx.def_map.get(enum_name);
        if enum_def_id_opt.is_none() {
            for privates in ctx.def_map.file_private.values() {
                if let Some(&def_id) = privates.get(enum_name.as_str()) {
                    let entry = ctx.def_map.get_entry(def_id);
                    if matches!(entry.kind, DefKind::Enum) {
                        enum_def_id_opt = Some(def_id);
                        break;
                    }
                }
            }
        }

        if let Some(enum_def_id) = enum_def_id_opt {
            let entry = ctx.def_map.get_entry(enum_def_id);
            if matches!(entry.kind, DefKind::Enum)
                && let Some(variants) = ctx.type_env.enum_variants.get(&enum_def_id)
                    && let Some(variant_idx) = variants.iter().position(|v| v.name == *variant_name) {
                        let enum_ty = ctx.interner.intern(TyKind::Enum(enum_def_id));
                        // Unit variant: emit the tag index as an int literal typed as the enum.
                        return TypedExpr::Literal {
                            ty: enum_ty,
                            span,
                            value: super::super::ir::TypedLiteral::Int(variant_idx as i64),
                        };
                    }
        }
    }

    // Truly unresolved: emit error
    let err_ty = ctx.emit_error(TypeError::UndefinedVariable {
        name: segments.join("::"),
        span,
        file: ctx.current_file,
    });
    TypedExpr::Error { ty: err_ty, span }
}
