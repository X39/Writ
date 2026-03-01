//! Statement emission for IL method bodies.
//!
//! `emit_stmt` dispatches on TypedStmt variants. Variants deferred to later
//! plans (For, Atomic) emit a Nop placeholder.

use writ_module::instruction::Instruction;

use crate::check::ir::TypedStmt;
use crate::check::ty::TyKind;

use super::BodyEmitter;
use super::expr::emit_expr;

/// Emit code for a TypedStmt.
pub fn emit_stmt(emitter: &mut BodyEmitter<'_>, stmt: &TypedStmt) {
    match stmt {
        // ── Let binding ───────────────────────────────────────────────────────
        TypedStmt::Let { name, ty: _, value, .. } => {
            // Emit the value expression into a register.
            let r_val = emit_expr(emitter, value);
            // The value register IS the local's register — no MOV needed.
            // Insert into locals map so future Var references find it.
            emitter.locals.insert(name.clone(), r_val);
            // Record debug info: register r_val corresponds to source variable `name`.
            // start_pc = instruction count at this point (byte offset computed in serialize.rs).
            // end_pc = u32::MAX sentinel meaning "live until end of body" (clamped in serialize.rs).
            let start_pc = emitter.instructions.len() as u32;
            emitter.debug_locals.push((r_val, name.clone(), start_pc, u32::MAX));
        }

        // ── Bare expression ───────────────────────────────────────────────────
        TypedStmt::Expr { expr, .. } => {
            // Emit for side-effects; discard the result register.
            let _ = emit_expr(emitter, expr);
        }

        // ── While loop ────────────────────────────────────────────────────────
        TypedStmt::While { condition, body, .. } => {
            // Allocate labels
            let loop_start = emitter.new_label();
            let loop_end = emitter.new_label();
            let continue_lbl = loop_start; // continue jumps back to start

            emitter.push_loop(loop_end, continue_lbl);

            // Mark loop start
            emitter.mark_label_here(loop_start);

            // Emit condition
            let r_cond = emit_expr(emitter, condition);

            // BrFalse to loop_end
            let brf_idx = emitter.instructions.len();
            emitter.emit(Instruction::BrFalse { r_cond, offset: 0 });
            emitter.add_fixup(brf_idx, loop_end);

            // Emit body
            for s in body {
                emit_stmt(emitter, s);
            }

            // Br back to loop_start
            let br_idx = emitter.instructions.len();
            emitter.emit(Instruction::Br { offset: 0 });
            emitter.add_fixup(br_idx, loop_start);

            // Mark loop_end
            emitter.mark_label_here(loop_end);

            emitter.pop_loop();
        }

        // ── For loop ──────────────────────────────────────────────────────────
        TypedStmt::For { binding, binding_ty, iterable, body, .. } => {
            emit_for_loop(emitter, binding, *binding_ty, iterable, body);
        }

        // ── Return ────────────────────────────────────────────────────────────
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                // EMIT-24: Tail-call optimization — Return(Call(...)) emits TailCall.
                // Delegate to emit_expr which handles the Return variant including
                // the tail-call detection pattern.
                use crate::check::ir::TypedExpr;
                if let TypedExpr::Call { callee, args, callee_def_id, .. } = v {
                    let _ = super::expr::emit_tail_call(emitter, callee, args, *callee_def_id);
                } else {
                    let r_src = emit_expr(emitter, v);
                    emitter.emit(Instruction::Ret { r_src });
                }
            } else {
                emitter.emit(Instruction::RetVoid);
            }
        }

        // ── Break ─────────────────────────────────────────────────────────────
        TypedStmt::Break { value, .. } => {
            // Emit value if present (loop-with-value, not common in Writ but safe)
            if let Some(v) = value {
                let _ = emit_expr(emitter, v);
            }
            let break_lbl = emitter.break_label();
            let br_idx = emitter.instructions.len();
            emitter.emit(Instruction::Br { offset: 0 });
            emitter.add_fixup(br_idx, break_lbl);
        }

        // ── Continue ──────────────────────────────────────────────────────────
        TypedStmt::Continue { .. } => {
            let continue_lbl = emitter.continue_label();
            let br_idx = emitter.instructions.len();
            emitter.emit(Instruction::Br { offset: 0 });
            emitter.add_fixup(br_idx, continue_lbl);
        }

        // ── Atomic block ──────────────────────────────────────────────────────
        TypedStmt::Atomic { body, .. } => {
            // TODO: Plan 03 — ATOMIC_BEGIN/END (EMIT-16)
            emitter.emit(Instruction::AtomicBegin);
            for s in body {
                emit_stmt(emitter, s);
            }
            emitter.emit(Instruction::AtomicEnd);
        }

        // ── Error (should never reach codegen) ────────────────────────────────
        TypedStmt::Error { .. } => {
            panic!("TypedStmt::Error reached codegen — pre-pass should have aborted");
        }
    }
}

// ─── For loop emission ────────────────────────────────────────────────────────

/// Emit a for loop over an iterable.
///
/// For arrays: emit a counter loop with ARRAY_LEN + ARRAY_LOAD per iteration.
/// The binding variable is bound to the loaded element in each iteration.
///
/// Pattern:
/// ```text
/// r_arr   = emit iterable
/// r_len   = ARRAY_LEN r_arr
/// r_iter  = LOAD_INT 0
/// loop_start:
///   r_cond = CMP_LT r_iter, r_len
///   BR_FALSE r_cond, loop_end
///   r_elem  = ARRAY_LOAD r_arr, r_iter
///   ... body (binding=r_elem) ...
///   r_one  = LOAD_INT 1
///   r_iter = ADD_I r_iter, r_one
///   BR loop_start
/// loop_end:
/// ```
fn emit_for_loop(
    emitter: &mut super::BodyEmitter<'_>,
    binding: &str,
    binding_ty: crate::check::ty::Ty,
    iterable: &crate::check::ir::TypedExpr,
    body: &[crate::check::ir::TypedStmt],
) {
    let iter_ty = iterable.ty();
    // Pre-interned primitives: Int=Ty(0), Float=Ty(1), Bool=Ty(2), String=Ty(3), Void=Ty(4)
    // (see TyInterner::new() fixed ordering, same convention as alloc_void_reg)
    let int_ty = crate::check::ty::Ty(0);
    let bool_ty = crate::check::ty::Ty(2);

    match emitter.interner.kind(iter_ty).clone() {
        TyKind::Array(_elem_ty) => {
            // Array iteration via index counter loop
            let r_arr = emit_expr(emitter, iterable);
            let r_len = emitter.alloc_reg(int_ty);
            emitter.emit(Instruction::ArrayLen { r_dst: r_len, r_arr });

            // Initialize counter to 0
            let r_iter = emitter.alloc_reg(int_ty);
            emitter.emit(Instruction::LoadInt { r_dst: r_iter, value: 0 });

            // Labels
            let loop_start = emitter.new_label();
            let loop_end = emitter.new_label();
            emitter.push_loop(loop_end, loop_start);

            emitter.mark_label_here(loop_start);

            // CmpLtI r_cond, r_iter, r_len
            let r_cond = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpLtI { r_dst: r_cond, r_a: r_iter, r_b: r_len });

            // BrFalse r_cond, loop_end
            let brf_idx = emitter.instructions.len();
            emitter.emit(Instruction::BrFalse { r_cond, offset: 0 });
            emitter.add_fixup(brf_idx, loop_end);

            // Load element: ARRAY_LOAD r_elem, r_arr, r_iter
            let r_elem = emitter.alloc_reg(binding_ty);
            emitter.emit(Instruction::ArrayLoad { r_dst: r_elem, r_arr, r_idx: r_iter });

            // Bind element to loop variable
            emitter.locals.insert(binding.to_string(), r_elem);

            // Emit body
            for stmt in body {
                emit_stmt(emitter, stmt);
            }

            // Increment counter: r_one = 1, r_iter = AddI(r_iter, r_one)
            let r_one = emitter.alloc_reg(int_ty);
            emitter.emit(Instruction::LoadInt { r_dst: r_one, value: 1 });
            emitter.emit(Instruction::AddI { r_dst: r_iter, r_a: r_iter, r_b: r_one });

            // Br loop_start
            let br_idx = emitter.instructions.len();
            emitter.emit(Instruction::Br { offset: 0 });
            emitter.add_fixup(br_idx, loop_start);

            emitter.mark_label_here(loop_end);
            emitter.pop_loop();
        }
        _ => {
            // Non-array iterables: emit Nop (future: Range iteration, iterator protocol)
            let _ = emit_expr(emitter, iterable);
            emitter.emit(Instruction::Nop);
        }
    }
}
