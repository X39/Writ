//! Call instruction emission for IL method bodies.
//!
//! Handles the four call variants:
//! - CALL: direct calls to free functions and concrete-receiver method calls (EMIT-27 specialization)
//! - CALL_VIRT: virtual dispatch for contract-dispatched calls on generic receivers
//! - CALL_EXTERN: calls to extern functions
//! - CALL_INDIRECT: calls to delegate/Func typed callees
//!
//! Also handles argument packing (EMIT-09): arguments must be placed in consecutive
//! registers r_base..r_base+argc-1 before the call instruction.
//!
//! Boxing/unboxing (EMIT-21): BOX is emitted when passing value types (Int/Float/Bool/Enum)
//! to generic parameters; UNBOX is emitted on return when expecting value type from generic return.

use writ_module::instruction::Instruction;

use crate::check::ir::TypedExpr;
use crate::check::ty::{Ty, TyKind};
use crate::resolve::def_map::DefId;

use super::BodyEmitter;
use super::expr::emit_expr;

/// The kind of call dispatch to use for a given call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// Direct call to a known method (free function or concrete-receiver method).
    /// EMIT-27: When the receiver's static type is TyKind::Struct or TyKind::Entity,
    /// CALL_VIRT is specialized to CALL.
    Direct,
    /// Virtual dispatch via a contract slot (generic receiver or explicit contract method).
    /// `slot` is the vtable slot index from the ContractMethod table.
    Virtual { slot: u16 },
    /// Call to an extern function.
    Extern,
    /// Indirect call through a delegate (callee is a Func-typed value).
    Indirect,
}

/// Emit a CALL/CALL_VIRT/CALL_EXTERN instruction sequence.
///
/// This is the main entry point for call dispatch from expr.rs.
/// The caller determines the CallKind by analysing the callee expression;
/// the emitter then packs arguments and emits the appropriate instruction.
///
/// Returns the destination register holding the call result.
pub fn emit_call(
    emitter: &mut BodyEmitter<'_>,
    call_expr: &TypedExpr,
    callee_def_id: DefId,
    kind: CallKind,
) -> u16 {
    let (ty, args) = match call_expr {
        TypedExpr::Call { ty, args, .. } => (*ty, args),
        _ => unreachable!("emit_call called on non-Call expr"),
    };

    let r_dst = emitter.alloc_reg(ty);

    // ── Argument packing (EMIT-09 / Pitfall 7) ───────────────────────────────
    //
    // All arguments must be in consecutive registers r_base..r_base+argc-1.
    // 1. Emit each argument, collecting the result registers.
    // 2. Allocate a consecutive block (r_base is the first).
    // 3. MOV each arg into its slot if not already consecutive.

    let arg_regs: Vec<u16> = args.iter().map(|arg| emit_expr(emitter, arg)).collect();
    let argc = arg_regs.len() as u16;

    // BUG-06 fix: use pack_args_consecutive to avoid phantom MOVs when args
    // are already in consecutive registers (the common case for simple calls).
    let r_base = pack_args_consecutive(emitter, &arg_regs);

    // ── Emit the call instruction ─────────────────────────────────────────────
    match kind {
        CallKind::Direct => {
            let method_idx = emitter
                .builder
                .token_for_def(callee_def_id)
                .map(|t| t.0)
                .unwrap_or(0); // 0 = unresolved (e.g., builtin or missing token)
            emitter.emit(Instruction::Call {
                r_dst,
                method_idx,
                r_base,
                argc,
            });
        }
        CallKind::Virtual { slot } => {
            // CALL_VIRT: receiver is r_base (implicit self), remaining args follow.
            // The spec layout: r_obj = receiver, r_base = first actual arg, argc = n-1
            let r_obj = r_base;
            let r_args_base = if argc > 0 { r_base + 1 } else { r_base };
            let n_args = if argc > 0 { argc - 1 } else { 0 };
            // FIX-02: Resolve the contract token for this virtual call site.
            // If the callee DefId has a registered impl-method-to-contract mapping
            // (populated by register_impl_method_contract during collection), emit
            // the contract's MetadataToken.0 as contract_idx. The runtime's CALL_VIRT
            // handler will use this to reconstruct the type_args_hash for dispatch lookup.
            // Falls back to 0 when no mapping is available (legacy path).
            let contract_idx: u32 = emitter
                .builder
                .contract_token_for_method_def_id(callee_def_id)
                .map(|t| t.0)
                .unwrap_or(0);
            emitter.emit(Instruction::CallVirt {
                r_dst,
                r_obj,
                contract_idx,
                slot,
                r_base: r_args_base,
                argc: n_args,
            });
        }
        CallKind::Extern => {
            let extern_idx = emitter
                .builder
                .token_for_def(callee_def_id)
                .map(|t| t.0)
                .unwrap_or(0);
            emitter.emit(Instruction::CallExtern {
                r_dst,
                extern_idx,
                r_base,
                argc,
            });
        }
        CallKind::Indirect => {
            // For indirect calls, the callee is a delegate register.
            // The plan calls emit_call_indirect separately; this path is a fallback.
            // We need the callee's register — use r_base as placeholder.
            emitter.emit(Instruction::CallIndirect {
                r_dst,
                r_delegate: r_base,
                r_base,
                argc,
            });
        }
    }

    r_dst
}

/// Emit a CALL_INDIRECT instruction for delegate/Func typed callees.
///
/// - `r_delegate`: the register holding the delegate (Func-typed) value.
/// Returns the destination register.
pub fn emit_call_indirect(
    emitter: &mut BodyEmitter<'_>,
    call_expr: &TypedExpr,
    r_delegate: u16,
) -> u16 {
    let (ty, args) = match call_expr {
        TypedExpr::Call { ty, args, .. } => (*ty, args),
        _ => unreachable!("emit_call_indirect called on non-Call expr"),
    };

    let r_dst = emitter.alloc_reg(ty);

    // Pack arguments into consecutive block (BUG-06 fix: skip MOV if already consecutive).
    let arg_regs: Vec<u16> = args.iter().map(|arg| emit_expr(emitter, arg)).collect();
    let argc = arg_regs.len() as u16;
    let r_base = pack_args_consecutive(emitter, &arg_regs);

    emitter.emit(Instruction::CallIndirect {
        r_dst,
        r_delegate,
        r_base,
        argc,
    });

    r_dst
}

/// Emit a BOX instruction if the argument type is a value type (Int/Float/Bool/Enum)
/// and the target parameter type is a GenericParam.
///
/// Returns the (possibly boxed) register to use at the call site.
pub fn emit_box_if_needed(
    emitter: &mut BodyEmitter<'_>,
    r_val: u16,
    arg_ty: Ty,
    param_ty: Ty,
) -> u16 {
    let needs_box = is_value_type(emitter, arg_ty) && is_generic_param(emitter, param_ty);
    if needs_box {
        let r_boxed = emitter.alloc_reg(param_ty);
        emitter.emit(Instruction::Box { r_dst: r_boxed, r_val });
        r_boxed
    } else {
        r_val
    }
}

/// Emit an UNBOX instruction if the call result is a GenericParam but the call site
/// expects a concrete value type.
///
/// Returns the (possibly unboxed) register.
pub fn emit_unbox_if_needed(
    emitter: &mut BodyEmitter<'_>,
    r_boxed: u16,
    declared_ret_ty: Ty,
    expected_ty: Ty,
) -> u16 {
    let needs_unbox = is_generic_param(emitter, declared_ret_ty) && is_value_type(emitter, expected_ty);
    if needs_unbox {
        let r_unboxed = emitter.alloc_reg(expected_ty);
        emitter.emit(Instruction::Unbox { r_dst: r_unboxed, r_boxed });
        r_unboxed
    } else {
        r_boxed
    }
}

// ─── Callee analysis ──────────────────────────────────────────────────────────

/// Determine the CallKind for a TypedExpr::Call by examining the callee.
///
/// This is the primary dispatch decision tree:
/// - Callee is a Var with the callee_def_id pointing to ExternFn -> Extern
/// - Callee is a Field on a generic receiver with contract method -> Virtual(slot)
/// - Callee is a Field on a concrete (Struct/Entity) receiver -> Direct (EMIT-27)
/// - Callee has TyKind::Func -> Indirect (handled separately by emit_call_indirect)
/// - Default: Direct
pub fn analyze_callee(
    emitter: &BodyEmitter<'_>,
    call_expr: &TypedExpr,
    callee_def_id: DefId,
) -> CallKind {
    let callee = match call_expr {
        TypedExpr::Call { callee, .. } => callee.as_ref(),
        _ => return CallKind::Direct,
    };

    // Check if callee type is Func -> CALL_INDIRECT
    let callee_ty = callee.ty();
    if matches!(emitter.interner.kind(callee_ty), TyKind::Func { .. }) {
        return CallKind::Indirect;
    }

    // Check if callee is a Field access
    if let TypedExpr::Field { receiver, .. } = callee {
        let recv_ty = receiver.ty();
        match emitter.interner.kind(recv_ty) {
            TyKind::Struct(_) | TyKind::Entity(_) => {
                // Concrete receiver -> EMIT-27: specialize to CALL
                return CallKind::Direct;
            }
            TyKind::GenericParam(_) => {
                // Generic receiver -> CALL_VIRT
                // Look up the vtable slot from the builder's contract method table.
                let slot = emitter
                    .builder
                    .contract_method_slot_by_def_id(callee_def_id)
                    .unwrap_or(0);
                return CallKind::Virtual { slot };
            }
            _ => {
                // Default: direct call
                return CallKind::Direct;
            }
        }
    }

    // Check if the DefId maps to an ExternDef token
    if let Some(token) = emitter.builder.token_for_def(callee_def_id) {
        use crate::emit::metadata::TableId;
        if token.table() == TableId::ExternDef {
            return CallKind::Extern;
        }
    }

    CallKind::Direct
}

// ─── Argument packing helper ──────────────────────────────────────────────────

/// Pack argument registers into a consecutive block starting at r_base.
///
/// If `arg_regs` are already consecutive (i.e., [N, N+1, N+2, ...]), returns
/// their existing base register with no MOV instructions emitted (BUG-06 fix).
/// Otherwise, allocates a new consecutive block and emits MOV instructions to
/// pack each argument into its slot.
///
/// This is the canonical argument packing implementation used by all call sites
/// (emit_call, emit_call_indirect, inline Call arm, emit_tail_call, emit_spawn,
/// emit_array_lit, emit_str_build).
pub fn pack_args_consecutive(emitter: &mut BodyEmitter<'_>, arg_regs: &[u16]) -> u16 {
    let argc = arg_regs.len() as u16;
    if argc == 0 {
        return emitter.regs.next();
    }
    let first = arg_regs[0];
    let already_consecutive = arg_regs
        .iter()
        .enumerate()
        .all(|(i, &r)| r == first + i as u16);
    if already_consecutive {
        return first;
    }
    // Need to repack into a new consecutive block.
    let r_block_start = emitter.regs.next();
    for i in 0..argc {
        let slot_reg = r_block_start + i;
        let arg_reg = arg_regs[i as usize];
        let arg_ty = emitter.regs.type_of(arg_reg);
        let allocated = emitter.regs.alloc(arg_ty);
        debug_assert_eq!(
            allocated, slot_reg,
            "consecutive register allocation should produce r_block_start + i"
        );
        emitter.emit(Instruction::Mov { r_dst: allocated, r_src: arg_reg });
    }
    r_block_start
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns true if the type is a value type (Int, Float, Bool, or Enum).
fn is_value_type(emitter: &BodyEmitter<'_>, ty: Ty) -> bool {
    matches!(
        emitter.interner.kind(ty),
        TyKind::Int | TyKind::Float | TyKind::Bool | TyKind::Enum(_)
    )
}

/// Returns true if the type is a GenericParam.
fn is_generic_param(emitter: &BodyEmitter<'_>, ty: Ty) -> bool {
    matches!(emitter.interner.kind(ty), TyKind::GenericParam(_))
}
