//! Object construction expression emission for IL method bodies.
//!
//! Covers: Range<T>, array literals, struct/entity construction.

use writ_module::instruction::{ArrayDefaultKind, Instruction};

use crate::check::ir::TypedExpr;
use crate::check::ty::{Ty, TyKind};
use crate::resolve::def_map::DefId;

use super::super::BodyEmitter;
use super::super::call::pack_args_consecutive;
use super::emit_expr;

/// Emit a Range<T> construction sequence.
///
/// A Range expression lowers to a struct construction sequence:
/// New { r_dst: r_range, type_idx: range_type_idx }
/// followed by 4 SetField instructions for start, end, start_inclusive, end_inclusive.
///
/// The Range<T> type in writ-runtime has 4 fields (per §1.18):
/// Each SET_FIELD uses the imported field's table-6 FieldRef token.
pub(super) fn emit_range(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    start: Option<&TypedExpr>,
    end: Option<&TypedExpr>,
    inclusive: bool,
) -> u16 {
    let range_type_idx = emitter.builder.range_type_token();
    let r_range = emitter.alloc_reg(ty);
    emitter.emit(Instruction::New {
        r_dst: r_range,
        type_idx: range_type_idx,
    });
    let range_field = |name: &str| {
        emitter
            .builder
            .imported_field_token_by_type_name("Range", name)
            .unwrap_or_else(|| panic!("writ-runtime Range field `{name}` has no FieldRef token"))
    };
    let start_field = range_field("start");
    let end_field = range_field("end");
    let start_inclusive_field = range_field("start_inclusive");
    let end_inclusive_field = range_field("end_inclusive");

    // Field: start
    let int_ty = Ty(0); // Int is Ty(0) per TyInterner pre-interned ordering
    let r_start = if let Some(s) = start {
        emit_expr(emitter, s)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField {
        r_obj: r_range,
        field_token: start_field,
        r_val: r_start,
    });

    // Field: end
    let r_end = if let Some(e) = end {
        emit_expr(emitter, e)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField {
        r_obj: r_range,
        field_token: end_field,
        r_val: r_end,
    });

    // Field: start_inclusive (always true — Writ ranges always include the start)
    let bool_ty = Ty(2); // Bool is Ty(2)
    let r_si = emitter.alloc_reg(bool_ty);
    emitter.emit(Instruction::LoadTrue { r_dst: r_si });
    emitter.emit(Instruction::SetField {
        r_obj: r_range,
        field_token: start_inclusive_field,
        r_val: r_si,
    });

    // Field: end_inclusive (true for ..=, false for ..)
    let r_ei = emitter.alloc_reg(bool_ty);
    if inclusive {
        emitter.emit(Instruction::LoadTrue { r_dst: r_ei });
    } else {
        emitter.emit(Instruction::LoadFalse { r_dst: r_ei });
    }
    emitter.emit(Instruction::SetField {
        r_obj: r_range,
        field_token: end_inclusive_field,
        r_val: r_ei,
    });

    r_range
}

/// Emit an array literal. Non-empty arrays use ARRAY_INIT; empty arrays use NEW_ARRAY.
pub(super) fn emit_array_lit(emitter: &mut BodyEmitter<'_>, ty: Ty, elements: &[TypedExpr]) -> u16 {
    let r_dst = emitter.alloc_reg(ty);
    let elem_type = array_default_kind(emitter, ty).operand();

    if elements.is_empty() {
        emitter.emit(Instruction::NewArray { r_dst, elem_type });
        return r_dst;
    }

    // Non-empty: emit each element, then ARRAY_INIT { r_dst, elem_type, count, r_base }
    let count = elements.len() as u16;
    let elem_regs: Vec<u16> = elements.iter().map(|e| emit_expr(emitter, e)).collect();

    // BUG-06 fix: use pack_args_consecutive to avoid phantom MOVs when already consecutive
    let r_base = pack_args_consecutive(emitter, &elem_regs);

    emitter.emit(Instruction::ArrayInit {
        r_dst,
        elem_type,
        count,
        r_base,
    });
    r_dst
}

fn array_default_kind(emitter: &BodyEmitter<'_>, array_ty: Ty) -> ArrayDefaultKind {
    let array_ty = emitter.interner.resolve_infer(array_ty);
    let TyKind::Array(elem_ty) = emitter.interner.kind(array_ty) else {
        return ArrayDefaultKind::Unavailable;
    };
    let elem_ty = emitter.interner.resolve_infer(*elem_ty);
    match emitter.interner.kind(elem_ty) {
        TyKind::Int => ArrayDefaultKind::Int,
        TyKind::Float => ArrayDefaultKind::Float,
        TyKind::Bool => ArrayDefaultKind::Bool,
        TyKind::String => ArrayDefaultKind::String,
        TyKind::Class(_)
        | TyKind::Entity(_)
        | TyKind::AnyEntity
        | TyKind::Enum(_)
        | TyKind::Contract(_)
        | TyKind::Array(_)
        | TyKind::Func { .. }
        | TyKind::Option(_)
        | TyKind::Result(_, _)
        | TyKind::TaskHandle(_)
        | TyKind::ReflectionType(_) => ArrayDefaultKind::NullReference,
        TyKind::Void
        | TyKind::Struct(_)
        | TyKind::GenericParam(_)
        | TyKind::Infer(_)
        | TyKind::Error => ArrayDefaultKind::Unavailable,
        TyKind::GenericInstance { .. } => {
            unreachable!("TyInterner::kind exposes a generic instance's base kind")
        }
    }
}

/// Emit a struct or entity construction sequence.
///
/// Struct: NEW { type_idx } + SET_FIELD per explicit field.
/// Entity: SPAWN_ENTITY { type_idx } + SET_FIELD(explicit fields only) + INIT_ENTITY.
///
/// Entity default field values do NOT generate SET_FIELD (spec §2.16.7).
pub(super) fn emit_new(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    target_def_id: DefId,
    fields: &[(String, TypedExpr)],
) -> u16 {
    let resolved_ty = emitter.interner.resolve_infer(ty);

    match emitter.interner.kind(resolved_ty) {
        TyKind::Entity(_) => {
            // EntityRegistry and lifecycle hooks store current-module TypeDef
            // rows. Keep SPAWN_ENTITY nominal until those paths carry an
            // explicit specialization descriptor of their own.
            let type_idx = emitter
                .builder
                .token_for_def(target_def_id)
                .map(|token| token.0)
                .unwrap_or(0);
            // EMIT-11: Entity construction sequence per spec §2.16.7
            let r_entity = emitter.alloc_reg(ty);
            emitter.emit(Instruction::SpawnEntity {
                r_dst: r_entity,
                type_idx,
            });
            // ONLY explicitly-provided fields get SET_FIELD
            for (field_name, field_expr) in fields {
                let r_val = emit_expr(emitter, field_expr);
                let field_token = emitter
                    .builder
                    .field_token_by_name(target_def_id, field_name)
                    .unwrap_or_else(|| {
                        panic!("checked entity field `{field_name}` has no metadata operand")
                    });
                emitter.emit(Instruction::SetField {
                    r_obj: r_entity,
                    field_token,
                    r_val,
                });
            }
            emitter.emit(Instruction::InitEntity { r_entity });
            r_entity
        }
        _ => {
            let type_idx = emitter
                .builder
                .type_spec_token_for_encoded_ty(resolved_ty, emitter.interner)
                .or_else(|| emitter.builder.token_for_def(target_def_id))
                .map(|token| token.0)
                .unwrap_or(0);
            // EMIT-10: Struct construction
            let r_obj = emitter.alloc_reg(ty);
            emitter.emit(Instruction::New {
                r_dst: r_obj,
                type_idx,
            });
            for (field_name, field_expr) in fields {
                let r_val = emit_expr(emitter, field_expr);
                let field_token = emitter
                    .builder
                    .field_token_by_name(target_def_id, field_name)
                    .unwrap_or_else(|| {
                        panic!("checked construction field `{field_name}` has no metadata operand")
                    });
                emitter.emit(Instruction::SetField {
                    r_obj,
                    field_token,
                    r_val,
                });
            }
            r_obj
        }
    }
}
