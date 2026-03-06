//! Control flow expression emission for IL method bodies.
//!
//! Covers: if/else, spawn, defer.

use writ_module::instruction::Instruction;

use crate::check::ir::TypedExpr;
use crate::check::ty::Ty;

use super::super::BodyEmitter;
use super::super::call::pack_args_consecutive;
use super::emit_expr;

/// Emit an if/else expression.
///
/// Allocates a shared result register that both branches MOV into (BUG-04 fix).
/// This ensures the RET instruction always references an initialized register
/// regardless of which branch was taken at runtime.
pub(super) fn emit_if(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    condition: &TypedExpr,
    then_branch: &TypedExpr,
    else_branch: Option<&TypedExpr>,
) -> u16 {
    // Emit condition
    let r_cond = emit_expr(emitter, condition);

    // Allocate a shared result register that both branches MOV into (BUG-04 fix).
    let r_result = emitter.alloc_reg(ty);

    // Create labels
    let else_label = emitter.new_label();
    let end_label = emitter.new_label();

    // BrFalse to else_label — record fixup at current instruction index
    let brf_idx = emitter.instructions.len();
    emitter.emit(Instruction::BrFalse { r_cond, offset: 0 });
    emitter.add_fixup(brf_idx, else_label);

    // Emit then-branch; MOV result into shared register
    let r_then = emit_expr(emitter, then_branch);
    emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_then });

    // Br to end_label — record fixup
    let br_idx = emitter.instructions.len();
    emitter.emit(Instruction::Br { offset: 0 });
    emitter.add_fixup(br_idx, end_label);

    // Mark else label here
    emitter.mark_label_here(else_label);

    // Emit else-branch (or Nop if None); MOV result into shared register
    if let Some(e) = else_branch {
        let r_else = emit_expr(emitter, e);
        emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_else });
    } else {
        emitter.emit(Instruction::Nop);
    }

    // Mark end label here
    emitter.mark_label_here(end_label);

    // Return the shared result register — valid on both then and else paths
    r_result
}

/// Emit a spawn expression.
///
/// `spawn expr` lowers to:
///   1. Emit the inner call expression's arguments
///   2. SPAWN_TASK { r_dst, method_idx, r_base, argc }
///
/// The inner expr must be a Call. method_idx is derived from the call's callee
/// (using the builder's def_token_map, or 0 as placeholder).
pub(super) fn emit_spawn(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    inner: &TypedExpr,
    detached: bool,
) -> u16 {
    let r_dst = emitter.alloc_reg(ty);

    match inner {
        TypedExpr::Call { args, callee_def_id, .. } => {
            // Emit args into consecutive block (BUG-06 fix: skip MOV if already consecutive)
            let arg_regs: Vec<u16> = args.iter().map(|a| emit_expr(emitter, a)).collect();
            let argc = arg_regs.len() as u16;
            let r_base = pack_args_consecutive(emitter, &arg_regs);

            // MC-01 fix: use callee_def_id from the Call node (populated during type checking)
            // instead of extract_callee_def_id_opt which always returned None.
            let method_idx = callee_def_id
                .and_then(|id| emitter.builder.token_for_def(id))
                .map(|t| t.0)
                .unwrap_or(0);

            if detached {
                emitter.emit(Instruction::SpawnDetached { r_dst, method_idx, r_base, argc });
            } else {
                emitter.emit(Instruction::SpawnTask { r_dst, method_idx, r_base, argc });
            }
        }
        _ => {
            // Non-call inner expr: emit it and use a placeholder spawn
            let _ = emit_expr(emitter, inner);
            if detached {
                emitter.emit(Instruction::SpawnDetached { r_dst, method_idx: 0, r_base: 0, argc: 0 });
            } else {
                emitter.emit(Instruction::SpawnTask { r_dst, method_idx: 0, r_base: 0, argc: 0 });
            }
        }
    }

    r_dst
}

/// Emit a defer expression.
///
/// `defer expr` lowers to the following instruction sequence:
///
/// ```text
/// [0] DeferPush { r_dst, method_idx: handler_start_idx }  // registers handler
/// [1] DeferPop                                              // disarms on normal exit
/// [2] Br { offset: N }                                     // skip handler on normal path
/// [3] <handler body instructions>                          // handler code (reached by runtime)
/// [N] DeferEnd                                             // end of handler
/// [N+1] (next instruction — Br target after_handler_label)
/// ```
///
/// The DeferPush.method_idx holds the instruction index of the handler body start
/// (index [3] in the example). On normal exit, DeferPop disarms the defer and
/// Br jumps past the handler. The runtime jumps to method_idx when the defer fires.
///
/// Both the Br skip and DeferPush.method_idx use the label/fixup pipeline so that
/// serialize.rs can convert instruction indices to byte offsets correctly.
pub(super) fn emit_defer(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    let void_ty = Ty(4); // Void
    let r_dst = emitter.alloc_reg(void_ty);

    // Create labels for handler start and post-handler
    let handler_label = emitter.new_label();
    let after_handler_label = emitter.new_label();

    // Emit DeferPush with placeholder method_idx; record index for patching
    let defer_push_idx = emitter.instructions.len();
    emitter.emit(Instruction::DeferPush { r_dst, method_idx: 0 }); // placeholder

    // DeferPop: disarm the defer on normal exit path
    emitter.emit(Instruction::DeferPop);

    // Branch past the handler on normal execution path — use label fixup pipeline
    let br_skip_idx = emitter.instructions.len();
    emitter.emit(Instruction::Br { offset: 0 }); // placeholder
    emitter.add_fixup(br_skip_idx, after_handler_label);

    // Handler starts here — mark the label at the current instruction index
    emitter.mark_label_here(handler_label);

    // Record the handler instruction index for DeferPush patching
    let handler_start_idx = emitter.instructions.len() as u32;

    // Emit the handler body (the deferred expression)
    let _ = emit_expr(emitter, expr);

    // DeferEnd: marks completion of handler execution
    emitter.emit(Instruction::DeferEnd);

    // Mark after_handler_label so the Br skip fixup resolves correctly
    emitter.mark_label_here(after_handler_label);

    // Patch DeferPush with correct handler instruction index.
    // serialize.rs Pass 4 converts this instruction index to a byte offset.
    if let Instruction::DeferPush { method_idx, .. } = &mut emitter.instructions[defer_push_idx] {
        *method_idx = handler_start_idx;
    }

    r_dst
}
