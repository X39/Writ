//! `new` struct/class/entity construction and array literal type checking.

use chumsky::span::SimpleSpan;

use super::super::env::{FieldSig, LocalEnv};
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;
use super::CheckCtx;
use super::check_expr;
use crate::ast::expr::{AstExpr, AstNewField};
use crate::ast::types::AstType;
use writ_diagnostics::{Diagnostic, code};

pub(super) fn check_new_construction(
    ctx: &mut CheckCtx,
    ast_ty: &AstType,
    fields: &[AstNewField],
    span: SimpleSpan,
) -> TypedExpr {
    let resolved_ty = ctx.resolve_ast_type(ast_ty);

    if ctx.is_error(resolved_ty) {
        // Can't resolve the type
        return TypedExpr::Error {
            ty: ctx.interner.error(),
            span,
        };
    }

    // Get the DefId and expected fields
    let (def_id, mut expected_fields) = match ctx.interner.kind(resolved_ty).clone() {
        TyKind::Struct(did) => {
            let fields = ctx
                .type_env
                .struct_fields
                .get(&did)
                .cloned()
                .unwrap_or_default();
            (did, fields)
        }
        TyKind::Class(did) => {
            let fields = ctx
                .type_env
                .struct_fields
                .get(&did)
                .cloned()
                .unwrap_or_default();
            (did, fields)
        }
        TyKind::Entity(did) => {
            let fields = ctx
                .type_env
                .entity_fields
                .get(&did)
                .cloned()
                .unwrap_or_default();
            (did, fields)
        }
        _ => {
            let ty_name = ctx.display_ty(resolved_ty);
            ctx.emit_error(TypeError::TypeMismatch {
                expected: "struct, class, or entity type".to_string(),
                found: ty_name,
                expected_span: span,
                found_span: span,
                file: ctx.current_file,
                help: Some("new requires a struct, class, or entity type".to_string()),
            });
            return TypedExpr::Error {
                ty: ctx.interner.error(),
                span,
            };
        }
    };

    // Generic fields contain GenericParam ordinals in TypeEnv. Substitute the
    // arguments preserved by the resolved nominal instance so construction and
    // later assignment share exactly the same concrete type identity.
    if let Some(type_args) = ctx
        .interner
        .generic_args(resolved_ty)
        .map(|args| args.to_vec())
    {
        for field in &mut expected_fields {
            field.ty = super::super::infer::substitute(field.ty, &type_args, &mut ctx.interner);
        }
    }

    // Emit W0006 if the constructed type is deprecated and defined in a different file.
    if let Some(msg) = ctx.type_env.deprecated_items.get(&def_id) {
        let entry = ctx.def_map.get_entry(def_id);
        if entry.file_id != ctx.current_file {
            let type_name = entry.name.clone();
            let warning_msg = if msg.is_empty() {
                format!("`{}` is deprecated", type_name)
            } else {
                format!("`{}` is deprecated: {}", type_name, msg)
            };
            ctx.diags.push(
                Diagnostic::warning(code::W0006, warning_msg)
                    .with_primary(ctx.current_file, span, "deprecated type constructed here")
                    .build(),
            );
        }
    }

    // Check explicit initializers once, retaining only the first initializer
    // for each known field. The final TypedExpr::New is normalized below.
    let mut explicit_fields: Vec<(String, SimpleSpan, TypedExpr)> = Vec::new();
    for field in fields {
        let typed_value = check_expr(ctx, &field.value);
        let value_ty = typed_value.ty();

        // Find this field in the expected fields
        let field_def = expected_fields
            .iter()
            .find(|field_sig| field_sig.name == field.name);

        if let Some(field_sig) = field_def {
            if let Some((_, first_span, _)) = explicit_fields
                .iter()
                .find(|(name, _, _)| name == &field.name)
            {
                ctx.diags.push(
                    TypeError::DuplicateConstructionField {
                        field_name: field.name.clone(),
                        first_span: *first_span,
                        duplicate_span: field.name_span,
                        file: ctx.current_file,
                    }
                    .into(),
                );
                continue;
            }

            // Check type compatibility
            ctx.check_assignable(
                field_sig.ty,
                value_ty,
                field.name_span,
                typed_value.span(),
                Some(format!("in field `{}`", field.name)),
            );
            explicit_fields.push((field.name.clone(), field.name_span, typed_value));
        } else {
            // Unknown field
            ctx.emit_error(TypeError::UnknownField {
                ty_name: ctx.display_ty(resolved_ty),
                field_name: field.name.clone(),
                span: field.name_span,
                file: ctx.current_file,
            });
        }
    }

    // Produce one initializer per declaration field, in declaration order.
    // Explicit values win over defaults. A construction with a missing value
    // always carries a diagnostic, so successful New nodes are complete.
    let mut typed_fields = Vec::with_capacity(expected_fields.len());
    for field in &expected_fields {
        if let Some(index) = explicit_fields
            .iter()
            .position(|(name, _, _)| name == &field.name)
        {
            let (_, _, value) = explicit_fields.remove(index);
            typed_fields.push((field.name.clone(), value));
            continue;
        }

        if let Some(default) = &field.default {
            let typed_default = check_default_in_declaration_scope(ctx, field, default);
            ctx.check_assignable(
                field.ty,
                typed_default.ty(),
                field.span,
                typed_default.span(),
                Some(format!("in default for field `{}`", field.name)),
            );
            typed_fields.push((field.name.clone(), typed_default));
        } else if field.has_default {
            ctx.diags.push(
                TypeError::UnavailableImportedFieldDefault {
                    type_name: ctx.display_ty(resolved_ty),
                    field_name: field.name.clone(),
                    span,
                    file: ctx.current_file,
                }
                .into(),
            );
        } else {
            ctx.diags.push(
                TypeError::MissingConstructionField {
                    type_name: ctx.display_ty(resolved_ty),
                    field_name: field.name.clone(),
                    span,
                    file: ctx.current_file,
                }
                .into(),
            );
        }
    }

    TypedExpr::New {
        ty: resolved_ty,
        span,
        target_def_id: def_id,
        fields: typed_fields,
    }
}

fn check_default_in_declaration_scope(
    ctx: &mut CheckCtx<'_>,
    field: &FieldSig,
    default: &AstExpr,
) -> TypedExpr {
    let caller_local_env = std::mem::replace(&mut ctx.local_env, LocalEnv::new());
    let caller_self_type = ctx.self_type.take();
    let caller_fn_ret = ctx.current_fn_ret.take();
    let caller_file = std::mem::replace(&mut ctx.current_file, field.decl_file);
    let caller_namespace =
        std::mem::replace(&mut ctx.current_namespace, field.decl_namespace.clone());
    let caller_generics = std::mem::replace(&mut ctx.current_generics, field.decl_generics.clone());

    let typed = check_expr(ctx, default);

    ctx.local_env = caller_local_env;
    ctx.self_type = caller_self_type;
    ctx.current_fn_ret = caller_fn_ret;
    ctx.current_file = caller_file;
    ctx.current_namespace = caller_namespace;
    ctx.current_generics = caller_generics;
    typed
}

pub(super) fn check_array_lit(
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
        if !ctx.is_error(elem_ty)
            && !ctx.is_error(ty)
            && ctx.unify.unify(elem_ty, ty, &mut ctx.interner).is_err()
        {
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

    let array_ty = ctx.interner.array(elem_ty);
    TypedExpr::ArrayLit {
        ty: array_ty,
        span,
        elements: typed_elements,
    }
}
