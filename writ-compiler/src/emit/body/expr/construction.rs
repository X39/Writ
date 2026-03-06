//! Object construction expression emission for IL method bodies.
//!
//! Covers: Range<T>, array literals, struct/entity construction.

use writ_module::instruction::Instruction;

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
///   field 0: start (T)
///   field 1: end (T)
///   field 2: start_inclusive (Bool) — always true in Writ syntax
///   field 3: end_inclusive (Bool)   — true for ..=, false for ..
pub(super) fn emit_range(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    start: Option<&TypedExpr>,
    end: Option<&TypedExpr>,
    inclusive: bool,
) -> u16 {
    let range_type_idx = emitter.builder.range_type_token();
    let r_range = emitter.alloc_reg(ty);
    emitter.emit(Instruction::New { r_dst: r_range, type_idx: range_type_idx });

    // Field 0: start
    let int_ty = Ty(0); // Int is Ty(0) per TyInterner pre-interned ordering
    let r_start = if let Some(s) = start {
        emit_expr(emitter, s)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 0, r_val: r_start });

    // Field 1: end
    let r_end = if let Some(e) = end {
        emit_expr(emitter, e)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 1, r_val: r_end });

    // Field 2: start_inclusive (always true — Writ ranges always include the start)
    let bool_ty = Ty(2); // Bool is Ty(2)
    let r_si = emitter.alloc_reg(bool_ty);
    emitter.emit(Instruction::LoadTrue { r_dst: r_si });
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 2, r_val: r_si });

    // Field 3: end_inclusive (true for ..=, false for ..)
    let r_ei = emitter.alloc_reg(bool_ty);
    if inclusive {
        emitter.emit(Instruction::LoadTrue { r_dst: r_ei });
    } else {
        emitter.emit(Instruction::LoadFalse { r_dst: r_ei });
    }
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 3, r_val: r_ei });

    r_range
}

/// Emit an array literal. Non-empty arrays use ARRAY_INIT; empty arrays use NEW_ARRAY.
pub(super) fn emit_array_lit(emitter: &mut BodyEmitter<'_>, ty: Ty, elements: &[TypedExpr]) -> u16 {
    let r_dst = emitter.alloc_reg(ty);

    if elements.is_empty() {
        // Empty array: NewArray { r_dst, elem_type: 0 }
        // Element type token is 0 (deferred to Plan 04 full wiring)
        emitter.emit(Instruction::NewArray { r_dst, elem_type: 0 });
        return r_dst;
    }

    // Non-empty: emit each element, then ARRAY_INIT { r_dst, elem_type, count, r_base }
    let count = elements.len() as u16;
    let elem_regs: Vec<u16> = elements.iter().map(|e| emit_expr(emitter, e)).collect();

    // BUG-06 fix: use pack_args_consecutive to avoid phantom MOVs when already consecutive
    let r_base = pack_args_consecutive(emitter, &elem_regs);

    // elem_type token: 0 as placeholder (Plan 04 will wire real type sigs)
    emitter.emit(Instruction::ArrayInit { r_dst, elem_type: 0, count, r_base });
    r_dst
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
    let type_idx = emitter
        .builder
        .token_for_def(target_def_id)
        .map(|t| t.0)
        .unwrap_or(0);

    match emitter.interner.kind(ty) {
        TyKind::Entity(_) => {
            // EMIT-11: Entity construction sequence per spec §2.16.7
            let r_entity = emitter.alloc_reg(ty);
            emitter.emit(Instruction::SpawnEntity { r_dst: r_entity, type_idx });
            // ONLY explicitly-provided fields get SET_FIELD
            for (field_name, field_expr) in fields {
                let r_val = emit_expr(emitter, field_expr);
                let field_idx = emitter
                    .builder
                    .field_token_by_name(target_def_id, field_name)
                    .unwrap_or(0);
                emitter.emit(Instruction::SetField { r_obj: r_entity, field_idx, r_val });
            }
            emitter.emit(Instruction::InitEntity { r_entity });
            r_entity
        }
        _ => {
            // EMIT-10: Struct construction
            let r_obj = emitter.alloc_reg(ty);
            emitter.emit(Instruction::New { r_dst: r_obj, type_idx });
            for (field_name, field_expr) in fields {
                let r_val = emit_expr(emitter, field_expr);
                let field_idx = emitter
                    .builder
                    .field_token_by_name(target_def_id, field_name)
                    .unwrap_or(0);
                emitter.emit(Instruction::SetField { r_obj, field_idx, r_val });
            }
            r_obj
        }
    }
}
