//! Structural equality emission for IL method bodies.
//!
//! Handles field-by-field equality and inequality for value-type structs.

use writ_module::instruction::Instruction;

use crate::check::ty::{Ty, TyKind};
use crate::resolve::def_map::DefId;

use super::super::BodyEmitter;

/// Emit field-by-field structural equality for value-type structs.
///
/// Returns a register containing bool: true if all fields are equal.
/// For each field: GetField both operands, compare with type-appropriate CmpEq, AND results.
/// Empty struct (no fields) returns LoadBool { val: true }.
/// Reference-type fields (class, entity, delegate, array) use CmpEqI (pointer identity).
/// Nested value-struct fields recurse into emit_struct_eq.
pub(super) fn emit_struct_eq(
    emitter: &mut BodyEmitter<'_>,
    r_a: u16,
    r_b: u16,
    def_id: DefId,
) -> u16 {
    let bool_ty = Ty(2); // Bool is Ty(2)
    let fields: Vec<(String, Ty)> = emitter
        .struct_field_types
        .get(&def_id)
        .cloned()
        .unwrap_or_default();

    if fields.is_empty() {
        // Empty struct: structurally equal by definition.
        let r_dst = emitter.alloc_reg(bool_ty);
        emitter.emit(Instruction::LoadTrue { r_dst });
        return r_dst;
    }

    let mut r_result: Option<u16> = None;
    for (field_name, field_ty) in &fields {
        let field_idx = emitter
            .builder
            .field_token_by_name(def_id, field_name)
            .unwrap_or(0);

        let r_fa = emitter.alloc_reg(*field_ty);
        emitter.emit(Instruction::GetField { r_dst: r_fa, r_obj: r_a, field_idx });
        let r_fb = emitter.alloc_reg(*field_ty);
        emitter.emit(Instruction::GetField { r_dst: r_fb, r_obj: r_b, field_idx });

        let r_field_eq = emit_field_eq(emitter, r_fa, r_fb, *field_ty);

        r_result = Some(match r_result {
            None => r_field_eq,
            Some(r_prev) => {
                let r_and = emitter.alloc_reg(bool_ty);
                emitter.emit(Instruction::BitAnd { r_dst: r_and, r_a: r_prev, r_b: r_field_eq });
                r_and
            }
        });
    }
    r_result.unwrap()
}

/// Emit field-by-field structural inequality for value-type structs.
///
/// Returns a register containing bool: true if ANY field differs (short-circuit OR).
/// For each field: GetField, CmpEq, NOT. Accumulate with BitOr.
/// Empty struct (no fields) returns LoadBool { val: false } (empty structs are always equal).
pub(super) fn emit_struct_neq(
    emitter: &mut BodyEmitter<'_>,
    r_a: u16,
    r_b: u16,
    def_id: DefId,
) -> u16 {
    let bool_ty = Ty(2);
    let fields: Vec<(String, Ty)> = emitter
        .struct_field_types
        .get(&def_id)
        .cloned()
        .unwrap_or_default();

    if fields.is_empty() {
        // Empty struct: structurally equal, so != is always false.
        let r_dst = emitter.alloc_reg(bool_ty);
        emitter.emit(Instruction::LoadFalse { r_dst });
        return r_dst;
    }

    let mut r_result: Option<u16> = None;
    for (field_name, field_ty) in &fields {
        let field_idx = emitter
            .builder
            .field_token_by_name(def_id, field_name)
            .unwrap_or(0);

        let r_fa = emitter.alloc_reg(*field_ty);
        emitter.emit(Instruction::GetField { r_dst: r_fa, r_obj: r_a, field_idx });
        let r_fb = emitter.alloc_reg(*field_ty);
        emitter.emit(Instruction::GetField { r_dst: r_fb, r_obj: r_b, field_idx });

        // Compare equal, then NOT to get "this field differs"
        let r_field_eq = emit_field_eq(emitter, r_fa, r_fb, *field_ty);
        let r_field_neq = emitter.alloc_reg(bool_ty);
        emitter.emit(Instruction::Not { r_dst: r_field_neq, r_src: r_field_eq });

        r_result = Some(match r_result {
            None => r_field_neq,
            Some(r_prev) => {
                let r_or = emitter.alloc_reg(bool_ty);
                emitter.emit(Instruction::BitOr { r_dst: r_or, r_a: r_prev, r_b: r_field_neq });
                r_or
            }
        });
    }
    r_result.unwrap()
}

/// Compare two field values for equality based on their type.
///
/// Returns a register containing bool (true = equal).
/// Dispatches to the correct CmpEq variant per field type:
/// - Struct(nested) -> recursive emit_struct_eq
/// - Float -> CmpEqF
/// - Bool -> CmpEqB
/// - String -> CmpEqS
/// - All others (Int, Class, Entity, Delegate, Array, Enum, Option) -> CmpEqI
fn emit_field_eq(
    emitter: &mut BodyEmitter<'_>,
    r_a: u16,
    r_b: u16,
    field_ty: Ty,
) -> u16 {
    let bool_ty = Ty(2);
    let kind = emitter.interner.kind(field_ty).clone();
    match kind {
        TyKind::Struct(nested_def_id) => {
            // Nested value-type struct: recurse for field-by-field equality.
            emit_struct_eq(emitter, r_a, r_b, nested_def_id)
        }
        TyKind::Float => {
            let r = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqF { r_dst: r, r_a, r_b });
            r
        }
        TyKind::Bool => {
            let r = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqB { r_dst: r, r_a, r_b });
            r
        }
        TyKind::String => {
            let r = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqS { r_dst: r, r_a, r_b });
            r
        }
        _ => {
            // Int, Class, Entity, Delegate, Array, Enum, Option — pointer/value identity.
            let r = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqI { r_dst: r, r_a, r_b });
            r
        }
    }
}
