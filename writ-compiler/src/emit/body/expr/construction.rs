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
/// A Range expression evaluates its four fields in declaration order and passes
/// the resulting consecutive register block to one atomic `New` instruction.
///
/// The Range<T> type in writ-runtime has 4 fields (per §1.18):
/// The field order is start, end, start_inclusive, end_inclusive.
pub(super) fn emit_range(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    start: Option<&TypedExpr>,
    end: Option<&TypedExpr>,
    inclusive: bool,
) -> u16 {
    let range_type_idx = emitter.builder.range_type_token();

    // Field: start
    let int_ty = Ty(0); // Int is Ty(0) per TyInterner pre-interned ordering
    let r_start = if let Some(s) = start {
        emit_expr(emitter, s)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    // Field: end
    let r_end = if let Some(e) = end {
        emit_expr(emitter, e)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    // Field: start_inclusive (always true — Writ ranges always include the start)
    let bool_ty = Ty(2); // Bool is Ty(2)
    let r_si = emitter.alloc_reg(bool_ty);
    emitter.emit(Instruction::LoadTrue { r_dst: r_si });
    // Field: end_inclusive (true for ..=, false for ..)
    let r_ei = emitter.alloc_reg(bool_ty);
    if inclusive {
        emitter.emit(Instruction::LoadTrue { r_dst: r_ei });
    } else {
        emitter.emit(Instruction::LoadFalse { r_dst: r_ei });
    }
    let field_regs = [r_start, r_end, r_si, r_ei];
    let r_base = pack_args_consecutive(emitter, &field_regs);
    let r_range = emitter.alloc_reg(ty);
    emitter.emit(Instruction::New {
        r_dst: r_range,
        type_idx: range_type_idx,
        field_count: field_regs.len() as u16,
        r_base,
    });

    r_range
}

/// Emit an array literal. Non-empty arrays use ARRAY_INIT; empty arrays use NEW_ARRAY.
pub(super) fn emit_array_lit(emitter: &mut BodyEmitter<'_>, ty: Ty, elements: &[TypedExpr]) -> u16 {
    let r_dst = emitter.alloc_reg(ty);
    let default_kind = array_default_kind(emitter, ty).operand();

    if elements.is_empty() {
        emitter.emit(Instruction::NewArray {
            r_dst,
            default_kind,
        });
        return r_dst;
    }

    // Non-empty: emit each element, then
    // ARRAY_INIT { r_dst, default_kind, count, r_base }.
    let count = elements.len() as u16;
    let elem_regs: Vec<u16> = elements.iter().map(|e| emit_expr(emitter, e)).collect();

    // BUG-06 fix: use pack_args_consecutive to avoid phantom MOVs when already consecutive
    let r_base = pack_args_consecutive(emitter, &elem_regs);

    emitter.emit(Instruction::ArrayInit {
        r_dst,
        default_kind,
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
/// The type checker normalizes `fields` to exactly one value per declared field
/// in declaration order. Values are evaluated in that order, packed into a
/// consecutive register block, and supplied to the allocation instruction so
/// no partially initialized object can escape.
///
/// Struct/class: NEW { type_idx, field_count, r_base }.
/// Entity: SPAWN_ENTITY { type_idx, field_count, r_base } + INIT_ENTITY.
pub(super) fn emit_new(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    target_def_id: DefId,
    fields: &[(String, TypedExpr)],
) -> u16 {
    let resolved_ty = emitter.interner.resolve_infer(ty);
    let field_regs: Vec<u16> = fields
        .iter()
        .map(|(_, field_expr)| emit_expr(emitter, field_expr))
        .collect();
    let field_count =
        u16::try_from(field_regs.len()).expect("object construction has more than u16::MAX fields");
    let r_base = if field_regs.is_empty() {
        0
    } else {
        pack_args_consecutive(emitter, &field_regs)
    };

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
                field_count,
                r_base,
            });
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
                field_count,
                r_base,
            });
            r_obj
        }
    }
}
