//! Literal value emission for IL method bodies.

use writ_module::instruction::Instruction;

use crate::check::ir::TypedLiteral;
use crate::check::ty::Ty;

use super::super::BodyEmitter;

/// Emit code for a literal value. Returns the destination register.
pub(super) fn emit_literal(emitter: &mut BodyEmitter<'_>, ty: Ty, value: &TypedLiteral) -> u16 {
    match value {
        TypedLiteral::Int(v) => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::LoadInt { r_dst, value: *v });
            r_dst
        }
        TypedLiteral::Float(v) => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::LoadFloat { r_dst, value: *v });
            r_dst
        }
        TypedLiteral::Bool(true) => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::LoadTrue { r_dst });
            r_dst
        }
        TypedLiteral::Bool(false) => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::LoadFalse { r_dst });
            r_dst
        }
        TypedLiteral::String(s) => {
            // String interning: BodyEmitter holds &'a ModuleBuilder (immutable), so
            // we cannot call string_heap.intern() directly during body emission.
            // Instead, record the string and instruction index in pending_strings.
            // The caller (emit_all_bodies or emit_bodies) will intern the strings
            // and patch the LoadString instructions with correct string_idx values.
            let r_dst = emitter.alloc_reg(ty);
            let instr_idx = emitter.instructions.len();
            emitter.emit(Instruction::LoadString { r_dst, string_idx: 0 }); // placeholder
            emitter.pending_strings.push((instr_idx, s.clone()));
            r_dst
        }
    }
}
