//! Binary operation emission for IL method bodies.

use writ_module::instruction::Instruction;

use crate::ast::expr::BinaryOp;
use crate::check::ty::{Ty, TyKind};

use super::super::BodyEmitter;
use super::eq::{emit_struct_eq, emit_struct_neq};

/// Emit code for a binary operation. Returns the destination register.
///
/// `ty` is the result type; `operand_ty` is the type of the left/right operands
/// (used for Eq/NotEq dispatch where ty is always Bool).
pub(super) fn emit_binary(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    op: &BinaryOp,
    r_a: u16,
    r_b: u16,
    operand_ty: Ty,
) -> u16 {
    let ty_kind = emitter.interner.kind(ty).clone();

    match op {
        // ── Arithmetic ───────────────────────────────────────────────────────
        BinaryOp::Add => match ty_kind {
            TyKind::Int => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::AddI { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::Float => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::AddF { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::String => {
                // EMIT-20: string + string -> STR_CONCAT
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::StrConcat { r_dst, r_a, r_b });
                r_dst
            }
            _ => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::AddI { r_dst, r_a, r_b });
                r_dst
            }
        },
        BinaryOp::Sub => match ty_kind {
            TyKind::Int => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::SubI { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::Float => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::SubF { r_dst, r_a, r_b });
                r_dst
            }
            _ => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::SubI { r_dst, r_a, r_b });
                r_dst
            }
        },
        BinaryOp::Mul => match ty_kind {
            TyKind::Int => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::MulI { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::Float => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::MulF { r_dst, r_a, r_b });
                r_dst
            }
            _ => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::MulI { r_dst, r_a, r_b });
                r_dst
            }
        },
        BinaryOp::Div => match ty_kind {
            TyKind::Int => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::DivI { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::Float => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::DivF { r_dst, r_a, r_b });
                r_dst
            }
            _ => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::DivI { r_dst, r_a, r_b });
                r_dst
            }
        },
        BinaryOp::Mod => match ty_kind {
            TyKind::Int => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::ModI { r_dst, r_a, r_b });
                r_dst
            }
            TyKind::Float => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::ModF { r_dst, r_a, r_b });
                r_dst
            }
            _ => {
                let r_dst = emitter.alloc_reg(ty);
                emitter.emit(Instruction::ModI { r_dst, r_a, r_b });
                r_dst
            }
        },

        // ── Comparison — Equality ─────────────────────────────────────────────
        BinaryOp::Eq => {
            // Dispatch on the OPERAND type (ty is Bool, the result type).
            let bool_ty = Ty(2); // Bool is Ty(2)
            match emitter.interner.kind(operand_ty).clone() {
                TyKind::Struct(def_id) => emit_struct_eq(emitter, r_a, r_b, def_id),
                TyKind::Float => {
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqF { r_dst, r_a, r_b });
                    r_dst
                }
                TyKind::Bool => {
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqB { r_dst, r_a, r_b });
                    r_dst
                }
                TyKind::String => {
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqS { r_dst, r_a, r_b });
                    r_dst
                }
                _ => {
                    // Int, Class, Entity, Delegate, Array, Enum, Option -- CmpEqI
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqI { r_dst, r_a, r_b });
                    r_dst
                }
            }
        }
        BinaryOp::NotEq => {
            // Dispatch on the OPERAND type (ty is Bool, the result type).
            let bool_ty = Ty(2);
            match emitter.interner.kind(operand_ty).clone() {
                TyKind::Struct(def_id) => emit_struct_neq(emitter, r_a, r_b, def_id),
                TyKind::Float => {
                    let r_cmp = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqF { r_dst: r_cmp, r_a, r_b });
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
                    r_dst
                }
                TyKind::Bool => {
                    let r_cmp = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqB { r_dst: r_cmp, r_a, r_b });
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
                    r_dst
                }
                TyKind::String => {
                    let r_cmp = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqS { r_dst: r_cmp, r_a, r_b });
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
                    r_dst
                }
                _ => {
                    let r_cmp = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::CmpEqI { r_dst: r_cmp, r_a, r_b });
                    let r_dst = emitter.alloc_reg(bool_ty);
                    emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
                    r_dst
                }
            }
        }
        BinaryOp::Lt => {
            let bool_ty = Ty(2);
            let r_dst = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst, r_a, r_b }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst, r_a, r_b }),
            }
            r_dst
        }
        BinaryOp::Gt => {
            // a > b  ≡  b < a
            let bool_ty = Ty(2);
            let r_dst = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst, r_a: r_b, r_b: r_a }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst, r_a: r_b, r_b: r_a }),
            }
            r_dst
        }
        BinaryOp::LtEq => {
            // a <= b  ≡  !(b < a)
            let bool_ty = Ty(2);
            let r_cmp = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst: r_cmp, r_a: r_b, r_b: r_a }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst: r_cmp, r_a: r_b, r_b: r_a }),
            }
            let r_dst = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
            r_dst
        }
        BinaryOp::GtEq => {
            // a >= b  ≡  !(a < b)
            let bool_ty = Ty(2);
            let r_cmp = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst: r_cmp, r_a, r_b }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst: r_cmp, r_a, r_b }),
            }
            let r_dst = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
            r_dst
        }

        // ── Logical ───────────────────────────────────────────────────────────
        BinaryOp::And => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::BitAnd { r_dst, r_a, r_b });
            r_dst
        }
        BinaryOp::Or => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::BitOr { r_dst, r_a, r_b });
            r_dst
        }

        // ── Bitwise ───────────────────────────────────────────────────────────
        BinaryOp::BitAnd => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::BitAnd { r_dst, r_a, r_b });
            r_dst
        }
        BinaryOp::BitOr => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::BitOr { r_dst, r_a, r_b });
            r_dst
        }
        BinaryOp::Shl => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::Shl { r_dst, r_a, r_b });
            r_dst
        }
        BinaryOp::Shr => {
            let r_dst = emitter.alloc_reg(ty);
            emitter.emit(Instruction::Shr { r_dst, r_a, r_b });
            r_dst
        }
    }
}
