//! String expression emission for IL method bodies.
//!
//! Handles multi-part string concatenation optimization via StrBuild.

use writ_module::instruction::Instruction;

use crate::ast::expr::BinaryOp;
use crate::check::ir::TypedExpr;
use crate::check::ty::{Ty, TyInterner, TyKind};

use super::super::BodyEmitter;
use super::super::call::pack_args_consecutive;
use super::emit_expr;

/// Attempt to collect a left-associative string Add chain of 3+ parts.
///
/// Format strings are lowered by fmt_string.rs to left-associative Binary(Add) trees
/// where every node has TyKind::String. A chain `a + b + c` becomes:
/// `Binary(Add, Binary(Add, a, b), c)`
///
/// Returns `Some(parts)` if the expression is a string Add chain of 3+ leaf nodes,
/// or `None` if it's a 2-part chain or not a string Add at all.
pub(super) fn try_collect_str_build_parts<'a>(
    expr: &'a TypedExpr,
    interner: &TyInterner,
) -> Option<Vec<&'a TypedExpr>> {
    if let TypedExpr::Binary { op, ty, .. } = expr
        && *op == BinaryOp::Add
            && matches!(interner.kind(*ty), TyKind::String) {
                let mut parts = Vec::new();
                collect_string_chain(expr, interner, &mut parts);
                if parts.len() >= 3 {
                    return Some(parts);
                }
            }
    None
}

/// Recursively collect leaf nodes from a left-associative string Add chain.
fn collect_string_chain<'a>(
    expr: &'a TypedExpr,
    interner: &TyInterner,
    parts: &mut Vec<&'a TypedExpr>,
) {
    match expr {
        TypedExpr::Binary { left, op, right, ty, .. }
            if *op == BinaryOp::Add && matches!(interner.kind(*ty), TyKind::String) =>
        {
            // Recurse left (may be another string Add), push right leaf
            collect_string_chain(left, interner, parts);
            parts.push(right);
        }
        _ => {
            // Leaf node (literal, var, etc.)
            parts.push(expr);
        }
    }
}

/// Emit StrBuild for a 3+ part string concatenation chain.
///
/// Parts are emitted into consecutive registers starting at r_base, then
/// StrBuild { r_dst, count, r_base } is emitted. This replaces nested StrConcat.
pub(super) fn emit_str_build(emitter: &mut BodyEmitter<'_>, ty: Ty, parts: &[&TypedExpr]) -> u16 {
    // Emit each part expression
    let part_regs: Vec<u16> = parts.iter().map(|p| emit_expr(emitter, p)).collect();
    let count = part_regs.len() as u16;

    // BUG-06 fix: pack into consecutive block, skipping MOV if already consecutive
    let r_base = pack_args_consecutive(emitter, &part_regs);

    let r_dst = emitter.alloc_reg(ty);
    emitter.emit(Instruction::StrBuild { r_dst, count, r_base });
    r_dst
}
