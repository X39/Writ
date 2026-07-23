//! Mutability enforcement for the Writ type checker.
//!
//! A mutable place is a mutable binding/global followed only by mutable fields
//! and/or index/component accesses. Read-only fields stop the path even when
//! the root binding is mutable. Temporaries and call results are not places.
//!
//! Assignment and selected `mut self` calls both use this analysis so direct,
//! overloaded, bound, contract, imported, and built-in mutation paths follow
//! the same rule.

use chumsky::span::SimpleSpan;

use super::check_expr::CheckCtx;
use super::env::{FieldSig, Mutability};
use super::error::TypeError;
use super::ir::TypedExpr;
use super::ty::{Ty, TyKind};
use crate::resolve::def_map::DefKind;

#[derive(Debug)]
enum PlaceMutability {
    Mutable,
    ImmutableBinding { name: String, span: SimpleSpan },
    ImmutableField { name: String, span: SimpleSpan },
    NonPlace,
}

/// Enforce mutability for an assignment target.
pub(crate) fn check_assignment(
    ctx: &mut CheckCtx<'_>,
    target: &TypedExpr,
    assignment_span: SimpleSpan,
) {
    let place = place_mutability(ctx, target);

    if matches!(target, TypedExpr::Var { .. } | TypedExpr::Path { .. }) {
        match place {
            PlaceMutability::Mutable => {}
            PlaceMutability::ImmutableBinding { name, span } => {
                ctx.diags.push(
                    TypeError::ImmutableReassignment {
                        binding_name: name,
                        binding_span: span,
                        assignment_span,
                        file: ctx.current_file,
                    }
                    .into(),
                );
            }
            PlaceMutability::ImmutableField { name, span } => {
                emit_immutable_field(ctx, name, span, assignment_span, "assign to a field");
            }
            PlaceMutability::NonPlace => {
                emit_non_place(ctx, assignment_span, "assign to this value");
            }
        }
        return;
    }

    let mutation_kind = match target {
        TypedExpr::Index { .. } => "perform index assignment",
        _ => "perform field assignment",
    };
    emit_place_error(ctx, place, assignment_span, mutation_kind);
}

/// Require a receiver expression to denote a mutable place before selecting a
/// method whose signature contains `mut self`.
pub(crate) fn check_mutable_receiver(
    ctx: &mut CheckCtx<'_>,
    receiver: &TypedExpr,
    method_name: &str,
    mutation_span: SimpleSpan,
) {
    let place = place_mutability(ctx, receiver);
    emit_place_error(
        ctx,
        place,
        mutation_span,
        &format!("invoke `mut self` method `{method_name}`"),
    );
}

fn emit_place_error(
    ctx: &mut CheckCtx<'_>,
    place: PlaceMutability,
    mutation_span: SimpleSpan,
    mutation_kind: &str,
) {
    match place {
        PlaceMutability::Mutable => {}
        PlaceMutability::ImmutableBinding { name, span } => {
            ctx.diags.push(
                TypeError::ImmutableMutation {
                    binding_name: name,
                    binding_span: span,
                    mutation_span,
                    mutation_kind: mutation_kind.to_string(),
                    file: ctx.current_file,
                }
                .into(),
            );
        }
        PlaceMutability::ImmutableField { name, span } => {
            emit_immutable_field(ctx, name, span, mutation_span, mutation_kind);
        }
        PlaceMutability::NonPlace => emit_non_place(ctx, mutation_span, mutation_kind),
    }
}

fn emit_immutable_field(
    ctx: &mut CheckCtx<'_>,
    field_name: String,
    field_span: SimpleSpan,
    mutation_span: SimpleSpan,
    mutation_kind: &str,
) {
    ctx.diags.push(
        TypeError::ImmutableFieldMutation {
            field_name,
            field_span,
            mutation_span,
            mutation_kind: mutation_kind.to_string(),
            file: ctx.current_file,
        }
        .into(),
    );
}

fn emit_non_place(ctx: &mut CheckCtx<'_>, mutation_span: SimpleSpan, mutation_kind: &str) {
    ctx.diags.push(
        TypeError::NonPlaceMutation {
            mutation_span,
            mutation_kind: mutation_kind.to_string(),
            file: ctx.current_file,
        }
        .into(),
    );
}

fn place_mutability(ctx: &CheckCtx<'_>, expr: &TypedExpr) -> PlaceMutability {
    if ctx.is_error(expr.ty()) {
        // The original type error is more useful than a cascading mutability error.
        return PlaceMutability::Mutable;
    }

    match expr {
        TypedExpr::Var { name, .. } => binding_mutability(ctx, name),
        TypedExpr::SelfRef { .. } => ctx
            .local_env
            .lookup("self")
            .map(|(_, mutability, span)| binding_place("self", mutability, span))
            .unwrap_or(PlaceMutability::NonPlace),
        TypedExpr::Path { segments, .. } => {
            let mut normalized = segments.clone();
            if let Some(first) = normalized.first_mut()
                && let Some(stripped) = first.strip_prefix("::")
            {
                *first = stripped.to_string();
            }
            definition_mutability(ctx, &normalized.join("::"))
        }
        TypedExpr::Field {
            receiver, field, ..
        } => {
            let Some(field_sig) = field_signature(ctx, receiver.ty(), field) else {
                return PlaceMutability::NonPlace;
            };
            if !field_sig.is_mutable {
                return PlaceMutability::ImmutableField {
                    name: field_sig.name.clone(),
                    span: field_sig.span,
                };
            }
            place_mutability(ctx, receiver)
        }
        TypedExpr::ComponentAccess { receiver, .. } | TypedExpr::Index { receiver, .. } => {
            place_mutability(ctx, receiver)
        }
        _ => PlaceMutability::NonPlace,
    }
}

fn binding_mutability(ctx: &CheckCtx<'_>, name: &str) -> PlaceMutability {
    if let Some((_, mutability, span)) = ctx.local_env.lookup(name) {
        return binding_place(name, mutability, span);
    }
    definition_mutability(ctx, name)
}

fn binding_place(name: &str, mutability: Mutability, span: SimpleSpan) -> PlaceMutability {
    match mutability {
        Mutability::Mutable => PlaceMutability::Mutable,
        Mutability::Immutable => PlaceMutability::ImmutableBinding {
            name: name.to_string(),
            span,
        },
    }
}

fn definition_mutability(ctx: &CheckCtx<'_>, name: &str) -> PlaceMutability {
    let def_id = ctx
        .def_map
        .get(name)
        .or_else(|| {
            (!ctx.current_namespace.is_empty())
                .then(|| format!("{}::{name}", ctx.current_namespace))
                .and_then(|fqn| ctx.def_map.get(&fqn))
        })
        .or_else(|| {
            ctx.def_map
                .file_private
                .get(&ctx.current_file)
                .and_then(|definitions| definitions.get(name).copied())
        });

    let Some(def_id) = def_id else {
        return PlaceMutability::NonPlace;
    };
    let entry = ctx.def_map.get_entry(def_id);
    match entry.kind {
        DefKind::Global => {
            let is_mutable = ctx
                .type_env
                .global_types
                .get(&def_id)
                .is_some_and(|(_, is_mutable)| *is_mutable);
            if is_mutable {
                PlaceMutability::Mutable
            } else {
                PlaceMutability::ImmutableBinding {
                    name: entry.name.clone(),
                    span: entry.name_span,
                }
            }
        }
        DefKind::Const => PlaceMutability::ImmutableBinding {
            name: entry.name.clone(),
            span: entry.name_span,
        },
        _ => PlaceMutability::NonPlace,
    }
}

fn field_signature<'env>(
    ctx: &'env CheckCtx<'_>,
    receiver_ty: Ty,
    field_name: &str,
) -> Option<&'env FieldSig> {
    let fields = match ctx.interner.kind(receiver_ty) {
        TyKind::Struct(def_id) | TyKind::Class(def_id) => ctx
            .type_env
            .struct_fields
            .get(def_id)
            .or_else(|| ctx.type_env.component_fields.get(def_id)),
        TyKind::Entity(def_id) => ctx.type_env.entity_fields.get(def_id),
        _ => None,
    }?;
    fields.iter().find(|field| field.name == field_name)
}
