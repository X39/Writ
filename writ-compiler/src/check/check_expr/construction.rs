//! `new` struct/class/entity construction and array literal type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstExpr, AstNewField};
use crate::ast::types::AstType;
use super::CheckCtx;
use super::check_expr;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;

pub(super) fn check_new_construction(
    ctx: &mut CheckCtx,
    ast_ty: &AstType,
    fields: &[AstNewField],
    span: SimpleSpan,
) -> TypedExpr {
    let generic_map = rustc_hash::FxHashMap::default();
    let resolved_ty = super::super::env::resolve_ast_type_with_file(ast_ty, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file);

    if ctx.is_error(resolved_ty) {
        // Can't resolve the type
        return TypedExpr::Error {
            ty: ctx.interner.error(),
            span,
        };
    }

    // Get the DefId and expected fields
    let (def_id, expected_fields) = match ctx.interner.kind(resolved_ty).clone() {
        TyKind::Struct(did) => {
            let fields = ctx.type_env.struct_fields.get(&did).cloned().unwrap_or_default();
            (did, fields)
        }
        TyKind::Class(did) => {
            let fields = ctx.type_env.struct_fields.get(&did).cloned().unwrap_or_default();
            (did, fields)
        }
        TyKind::Entity(did) => {
            let fields = ctx.type_env.entity_fields.get(&did).cloned().unwrap_or_default();
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

    // Check each provided field
    let mut typed_fields = Vec::new();
    let mut provided_names = Vec::new();
    for field in fields {
        let typed_value = check_expr(ctx, &field.value);
        let value_ty = typed_value.ty();

        // Find this field in the expected fields
        let field_def = expected_fields.iter().find(|(name, _, _)| name == &field.name);

        if let Some((_name, expected_ty, _fspan)) = field_def {
            // Check type compatibility
            if !ctx.is_error(value_ty) && !ctx.is_error(*expected_ty)
                && ctx.unify.unify(*expected_ty, value_ty, &mut ctx.interner).is_err() {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(*expected_ty),
                        found: ctx.display_ty(value_ty),
                        expected_span: field.name_span,
                        found_span: typed_value.span(),
                        file: ctx.current_file,
                        help: Some(format!("in field `{}`", field.name)),
                    });
                }
        } else {
            // Unknown field
            ctx.emit_error(TypeError::UnknownField {
                ty_name: ctx.display_ty(resolved_ty),
                field_name: field.name.clone(),
                span: field.name_span,
                file: ctx.current_file,
            });
        }

        provided_names.push(field.name.clone());
        typed_fields.push((field.name.clone(), typed_value));
    }

    // Check for missing required fields
    for (fname, _, _) in &expected_fields {
        if !provided_names.iter().any(|n| n == fname) {
            ctx.diags.push(TypeError::MissingConstructionField {
                type_name: ctx.display_ty(resolved_ty),
                field_name: fname.clone(),
                span,
                file: ctx.current_file,
            }.into());
        }
    }

    TypedExpr::New {
        ty: resolved_ty,
        span,
        target_def_id: def_id,
        fields: typed_fields,
    }
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
        if !ctx.is_error(elem_ty) && !ctx.is_error(ty)
            && ctx.unify.unify(elem_ty, ty, &mut ctx.interner).is_err() {
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
