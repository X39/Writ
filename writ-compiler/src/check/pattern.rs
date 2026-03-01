//! Pattern checking and exhaustiveness verification.

use chumsky::span::SimpleSpan;

use super::check_expr::CheckCtx;
use super::error::TypeError;
use super::ir::{TypedArm, TypedPattern};
use super::ty::TyKind;

/// Check that a match expression is exhaustive for enum types.
///
/// For enum scrutinees, verifies all variants are covered.
/// A wildcard or variable pattern covers all remaining variants.
pub fn check_exhaustiveness(
    ctx: &mut CheckCtx,
    scrutinee_ty: super::ty::Ty,
    arms: &[TypedArm],
    match_span: SimpleSpan,
) {
    if ctx.is_error(scrutinee_ty) {
        return;
    }

    match ctx.interner.kind(scrutinee_ty).clone() {
        TyKind::Enum(def_id) => {
            // Get all variant names
            let all_variants: Vec<String> = ctx
                .type_env
                .enum_variants
                .get(&def_id)
                .map(|vs| vs.iter().map(|v| v.name.clone()).collect())
                .unwrap_or_default();

            if all_variants.is_empty() {
                return;
            }

            // Collect covered variants
            let mut has_wildcard = false;
            let mut covered = Vec::new();

            for arm in arms {
                collect_covered_variants(&arm.pattern, &mut covered, &mut has_wildcard);
            }

            if has_wildcard {
                return; // Wildcard covers everything
            }

            // Find missing variants
            let missing: Vec<String> = all_variants
                .iter()
                .filter(|v| !covered.iter().any(|c| c == *v))
                .cloned()
                .collect();

            if !missing.is_empty() {
                ctx.diags.push(
                    TypeError::NonExhaustiveMatch {
                        missing_variants: missing,
                        match_span,
                        file: ctx.current_file,
                    }
                    .into(),
                );
            }
        }
        TyKind::Bool => {
            // Check both true and false are covered
            let mut has_true = false;
            let mut has_false = false;
            let mut has_wildcard = false;

            for arm in arms {
                check_bool_coverage(&arm.pattern, &mut has_true, &mut has_false, &mut has_wildcard);
            }

            if !has_wildcard && (!has_true || !has_false) {
                let mut missing = Vec::new();
                if !has_true {
                    missing.push("true".to_string());
                }
                if !has_false {
                    missing.push("false".to_string());
                }
                ctx.diags.push(
                    TypeError::NonExhaustiveMatch {
                        missing_variants: missing,
                        match_span,
                        file: ctx.current_file,
                    }
                    .into(),
                );
            }
        }
        _ => {
            // Non-enum matches are less strict; we don't enforce exhaustiveness
            // for int, string, etc. since they have infinite domains.
            // Just verify at least one arm exists (already handled by the
            // empty arms check above).
        }
    }
}

fn collect_covered_variants(pattern: &TypedPattern, covered: &mut Vec<String>, has_wildcard: &mut bool) {
    match pattern {
        TypedPattern::Wildcard { .. } | TypedPattern::Variable { .. } => {
            *has_wildcard = true;
        }
        TypedPattern::EnumVariant { variant_name, .. } => {
            covered.push(variant_name.clone());
        }
        TypedPattern::Or { patterns, .. } => {
            for p in patterns {
                collect_covered_variants(p, covered, has_wildcard);
            }
        }
        TypedPattern::Literal { .. } | TypedPattern::Range { .. } => {}
    }
}

fn check_bool_coverage(
    pattern: &TypedPattern,
    has_true: &mut bool,
    has_false: &mut bool,
    has_wildcard: &mut bool,
) {
    match pattern {
        TypedPattern::Wildcard { .. } | TypedPattern::Variable { .. } => {
            *has_wildcard = true;
        }
        TypedPattern::Literal { value, .. } => {
            if let super::ir::TypedLiteral::Bool(b) = value {
                if *b {
                    *has_true = true;
                } else {
                    *has_false = true;
                }
            }
        }
        TypedPattern::Or { patterns, .. } => {
            for p in patterns {
                check_bool_coverage(p, has_true, has_false, has_wildcard);
            }
        }
        _ => {}
    }
}

/// Check if a pattern is a catch-all (wildcard or variable binding).
#[allow(dead_code)]
fn pattern_is_exhaustive(pattern: &TypedPattern) -> bool {
    matches!(pattern, TypedPattern::Wildcard { .. } | TypedPattern::Variable { .. })
}
