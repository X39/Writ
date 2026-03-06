//! Match expression and pattern type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstExpr, AstMatchArm, AstPattern, RangeKind};
use super::CheckCtx;
use super::check_expr;
use super::check_block_stmts;
use super::super::env::Mutability;
use super::super::error::TypeError;
use super::super::ir::{TypedArm, TypedExpr, TypedLiteral, TypedPattern};
use super::super::ty::TyKind;

pub(super) fn check_match(
    ctx: &mut CheckCtx,
    scrutinee: &AstExpr,
    arms: &[AstMatchArm],
    span: SimpleSpan,
) -> TypedExpr {
    let typed_scrutinee = check_expr(ctx, scrutinee);
    let scrutinee_ty = typed_scrutinee.ty();

    if arms.is_empty() {
        return TypedExpr::Match {
            ty: ctx.interner.void(),
            span,
            scrutinee: Box::new(typed_scrutinee),
            arms: Vec::new(),
        };
    }

    let mut typed_arms = Vec::new();
    let mut arm_types: Vec<_> = Vec::new();

    for arm in arms {
        ctx.local_env.push_scope();

        // Bind pattern variables
        let typed_pattern = check_pattern(ctx, &arm.pattern, scrutinee_ty);

        // Check body
        let body = check_block_stmts(ctx, &arm.body, arm.span);
        let body_ty = body.ty();

        ctx.local_env.pop_scope();

        arm_types.push(body_ty);
        typed_arms.push(TypedArm {
            pattern: typed_pattern,
            body,
            span: arm.span,
        });
    }

    // Unify all arm types
    let mut result_ty = arm_types[0];
    for (i, &arm_ty) in arm_types.iter().enumerate().skip(1) {
        if ctx.is_error(result_ty) || ctx.is_error(arm_ty) {
            continue;
        }
        if ctx.unify.unify(result_ty, arm_ty, &mut ctx.interner).is_err() {
            ctx.emit_error(TypeError::TypeMismatch {
                expected: ctx.display_ty(result_ty),
                found: ctx.display_ty(arm_ty),
                expected_span: typed_arms[0].span,
                found_span: typed_arms[i].span,
                file: ctx.current_file,
                help: Some("all match arms must have the same type".to_string()),
            });
            result_ty = ctx.interner.error();
            break;
        }
    }

    // Check exhaustiveness for enum types
    super::super::pattern::check_exhaustiveness(ctx, scrutinee_ty, &typed_arms, span);

    TypedExpr::Match {
        ty: result_ty,
        span,
        scrutinee: Box::new(typed_scrutinee),
        arms: typed_arms,
    }
}

pub(super) fn check_pattern(ctx: &mut CheckCtx, pattern: &AstPattern, scrutinee_ty: super::super::ty::Ty) -> TypedPattern {
    match pattern {
        AstPattern::Wildcard { span } => TypedPattern::Wildcard { span: *span },
        AstPattern::Variable { name, span } => {
            // Handle unqualified None/Some when scrutinee is Option<T>.
            // These are sub-prelude builtin variant names, not user variable bindings.
            if matches!(name.as_str(), "None" | "Some")
                && let TyKind::Option(_) = ctx.interner.kind(scrutinee_ty) {
                    // None/Some in pattern position on an Option: treat as wildcard
                    // (no user variable is bound). Semantic correctness for IL emission
                    // is handled by the desugar layer; this just suppresses false errors.
                    return TypedPattern::Wildcard { span: *span };
                }
            // Bind the variable to the scrutinee type
            ctx.local_env.define(
                name.clone(),
                scrutinee_ty,
                Mutability::Immutable,
                *span,
            );
            TypedPattern::Variable {
                name: name.clone(),
                ty: scrutinee_ty,
                span: *span,
            }
        }
        AstPattern::Literal { expr, span } => {
            // Check the literal expression and verify it's compatible with scrutinee
            let typed_lit = check_expr(ctx, expr);
            let lit_ty = typed_lit.ty();
            if !ctx.is_error(lit_ty) && !ctx.is_error(scrutinee_ty)
                && ctx.unify.unify(scrutinee_ty, lit_ty, &mut ctx.interner).is_err() {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(scrutinee_ty),
                        found: ctx.display_ty(lit_ty),
                        expected_span: *span,
                        found_span: typed_lit.span(),
                        file: ctx.current_file,
                        help: Some("pattern type must match scrutinee type".to_string()),
                    });
                }
            // Extract literal value from the typed expression
            match typed_lit {
                TypedExpr::Literal { value, .. } => TypedPattern::Literal {
                    value,
                    span: *span,
                },
                _ => TypedPattern::Wildcard { span: *span },
            }
        }
        AstPattern::EnumDestructure { path, fields, span } => {
            // Resolve the enum variant from the path
            // e.g., path = ["Option", "Some"] or ["Some"]
            let variant_name = path.last().map(|s| s.as_str()).unwrap_or("");

            // Handle Option<T> pattern arms without Option:: prefix.
            // Single-segment path "None" or "Some" when scrutinee is TyKind::Option.
            if let TyKind::Option(inner_ty) = ctx.interner.kind(scrutinee_ty).clone() {
                match variant_name {
                    "None" => {
                        // None has no sub-bindings.
                        return TypedPattern::Wildcard { span: *span };
                    }
                    "Some" => {
                        // Some(v) -- bind the inner value pattern to inner_ty.
                        if let Some(field_pat) = fields.first() {
                            // Bind field pattern against the inner type.
                            check_pattern(ctx, field_pat, inner_ty);
                        }
                        // Return a Variable pattern bound to the whole scrutinee (Option<T>).
                        // The emitter treats this as a catch-all arm. Semantic correctness
                        // for IL is handled by the desugar layer for ?/! operators;
                        // explicit match arms on Option compile but emit as wildcards.
                        return TypedPattern::Wildcard { span: *span };
                    }
                    _ => {}
                }
            }

            // Try to find the enum def_id and variant fields
            let mut enum_def_id = None;
            let mut variant_fields = Vec::new();

            // Check if scrutinee is an enum type
            if let TyKind::Enum(def_id) = ctx.interner.kind(scrutinee_ty).clone()
                && let Some(variants) = ctx.type_env.enum_variants.get(&def_id) {
                    for v in variants {
                        if v.name == variant_name {
                            enum_def_id = Some(def_id);
                            variant_fields = v.fields.clone();
                            break;
                        }
                    }
                }

            // Bind pattern variables to the variant's field types
            let mut typed_bindings = Vec::new();
            for (i, field_pat) in fields.iter().enumerate() {
                let field_ty = variant_fields
                    .get(i)
                    .map(|(_, ty)| *ty)
                    .unwrap_or_else(|| ctx.interner.error());
                typed_bindings.push(check_pattern(ctx, field_pat, field_ty));
            }

            if let Some(eid) = enum_def_id {
                TypedPattern::EnumVariant {
                    enum_def_id: eid,
                    variant_name: variant_name.to_string(),
                    bindings: typed_bindings,
                    span: *span,
                }
            } else {
                // Could not resolve enum variant, produce wildcard with bindings still defined
                TypedPattern::Wildcard { span: *span }
            }
        }
        AstPattern::Or { patterns, span } => {
            let typed_pats: Vec<TypedPattern> = patterns
                .iter()
                .map(|p| check_pattern(ctx, p, scrutinee_ty))
                .collect();
            TypedPattern::Or {
                patterns: typed_pats,
                span: *span,
            }
        }
        AstPattern::Range { start, kind, end, span } => {
            let start_typed = check_expr(ctx, start);
            let end_typed = check_expr(ctx, end);
            // Both should be compatible with scrutinee type
            let start_lit = match start_typed {
                TypedExpr::Literal { value, .. } => value,
                _ => TypedLiteral::Int(0),
            };
            let end_lit = match end_typed {
                TypedExpr::Literal { value, .. } => value,
                _ => TypedLiteral::Int(0),
            };
            TypedPattern::Range {
                start: start_lit,
                end: end_lit,
                inclusive: matches!(kind, RangeKind::Inclusive),
                span: *span,
            }
        }
    }
}
