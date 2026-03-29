//! Statement emission for IL method bodies.
//!
//! `emit_stmt` dispatches on TypedStmt variants. Variants deferred to later
//! plans (For, Atomic) emit a Nop placeholder.

use writ_module::instruction::Instruction;

use crate::check::ir::TypedStmt;
use crate::check::ty::TyKind;
use crate::emit::collect::{ITERABLE_CONTRACT_TOKEN, ITERATOR_CONTRACT_TOKEN};

use super::BodyEmitter;
use super::call::pack_args_consecutive;
use super::expr::emit_expr;

/// Extract the source span from any TypedStmt variant.
fn stmt_span(stmt: &TypedStmt) -> chumsky::span::SimpleSpan {
    match stmt {
        TypedStmt::Let { span, .. }
        | TypedStmt::Expr { span, .. }
        | TypedStmt::For { span, .. }
        | TypedStmt::While { span, .. }
        | TypedStmt::Break { span, .. }
        | TypedStmt::Continue { span }
        | TypedStmt::Return { span, .. }
        | TypedStmt::Atomic { span, .. }
        | TypedStmt::Error { span } => *span,
    }
}

/// Emit code for a TypedStmt.
pub fn emit_stmt(emitter: &mut BodyEmitter<'_>, stmt: &TypedStmt) {
    // Push a source span entry at the current instruction index for this statement.
    // This enables PREP-01 line/col resolution in disassembler output.
    let span = stmt_span(stmt);
    let instr_idx = emitter.instructions.len() as u32;
    emitter.source_spans.push((instr_idx, span));

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
    use crate::check::ir::TypedExpr;

    // Check for Range iterable FIRST (before type-based dispatch).
    // Range expressions have ty=int (no TyKind::Range exists), so we must
    // match on the expression variant, not the type.
    if let TypedExpr::Range { start, end, inclusive, .. } = iterable {
        emit_for_range(emitter, binding, binding_ty, start.as_deref(), end.as_deref(), *inclusive, body);
        return;
    }

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
        TyKind::Class(_class_def_id) => {
            // ── Iterator protocol desugaring for class Iterable<T> ───────────
            //
            // Desugars `for x in collection` into:
            //   r_collection = emit collection
            //   r_iter = CALL_VIRT r_collection, Iterable.iterator(), argc=1
            //   loop_start:
            //     r_next = CALL_VIRT r_iter, Iterator.next(), argc=1
            //     r_is_none = IS_NONE r_next
            //     BR_TRUE r_is_none, loop_end
            //     r_elem = UNWRAP r_next
            //     <bind r_elem to loop var>
            //     <body>
            //     BR loop_start
            //   loop_end:

            // Void is used as the register type for opaque reference values
            // (the iterator object and the T? option) since the type interner
            // is immutable at emit time and can't intern new compound types.
            let void_ty = crate::check::ty::Ty(4);

            // 1. Emit the collection expression.
            let r_collection = emit_expr(emitter, iterable);

            // 2. CALL_VIRT: r_iter = collection.iterator()
            //    Iterable contract token is spec-locked at virtual module row 14.
            let iterable_contract_idx = ITERABLE_CONTRACT_TOKEN.0;
            // slot=0: "iterator" is the first (and only) method of Iterable<T>
            let iterable_slot: u16 = 0;

            let r_iter = emitter.alloc_reg(void_ty);
            let r_base_call_iter = pack_args_consecutive(emitter, &[r_collection]);
            emitter.emit(Instruction::CallVirt {
                r_dst: r_iter,
                r_obj: r_collection,
                contract_idx: iterable_contract_idx,
                slot: iterable_slot,
                r_base: r_base_call_iter,
                argc: 1,
            });

            // 3. Set up labels.
            let loop_start = emitter.new_label();
            let loop_end = emitter.new_label();
            emitter.push_loop(loop_end, loop_start);

            emitter.mark_label_here(loop_start);

            // 4. CALL_VIRT: r_next = iter.next()
            //    Iterator contract token is spec-locked at virtual module row 15.
            let iterator_contract_idx = ITERATOR_CONTRACT_TOKEN.0;
            // slot=0: "next" is the first (and only) method of Iterator<T>
            let iterator_slot: u16 = 0;

            let r_next = emitter.alloc_reg(void_ty);
            let r_base_call_next = pack_args_consecutive(emitter, &[r_iter]);
            emitter.emit(Instruction::CallVirt {
                r_dst: r_next,
                r_obj: r_iter,
                contract_idx: iterator_contract_idx,
                slot: iterator_slot,
                r_base: r_base_call_next,
                argc: 1,
            });

            // 5. IS_NONE: check if next() returned null.
            let r_is_none = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::IsNone { r_dst: r_is_none, r_opt: r_next });

            // 6. BR_TRUE r_is_none, loop_end
            let brt_idx = emitter.instructions.len();
            emitter.emit(Instruction::BrTrue { r_cond: r_is_none, offset: 0 });
            emitter.add_fixup(brt_idx, loop_end);

            // 7. UNWRAP: r_elem = unwrap r_next
            let r_elem = emitter.alloc_reg(binding_ty);
            emitter.emit(Instruction::Unwrap { r_dst: r_elem, r_opt: r_next });

            // 8. Bind element to loop variable.
            emitter.locals.insert(binding.to_string(), r_elem);

            // 9. Emit body.
            for stmt in body {
                emit_stmt(emitter, stmt);
            }

            // 10. BR loop_start
            let br_idx = emitter.instructions.len();
            emitter.emit(Instruction::Br { offset: 0 });
            emitter.add_fixup(br_idx, loop_start);

            emitter.mark_label_here(loop_end);
            emitter.pop_loop();
        }
        _ => {
            // Unknown iterable type — emit Nop (should not reach here after check_stmt
            // emits a NotIterable error and returns the Error poison type).
            let _ = emit_expr(emitter, iterable);
            emitter.emit(Instruction::Nop);
        }
    }
}

// ─── Range loop emission ─────────────────────────────────────────────────────

/// Emit a for loop over a range expression.
///
/// Pattern for exclusive range (start..end):
/// ```text
/// r_iter  = emit start (or LOAD_INT 0 if None)
/// r_end   = emit end
/// loop_start:
///   r_cond = CMP_LT_I r_iter, r_end     // exclusive: i < end
///   BR_FALSE r_cond, loop_end
///   ... body (binding = r_iter) ...
///   r_one  = LOAD_INT 1
///   r_iter = ADD_I r_iter, r_one
///   BR loop_start
/// loop_end:
/// ```
///
/// For inclusive range (start..=end), use end+1 with CMP_LT_I (no CMP_LE_I in the VM).
fn emit_for_range(
    emitter: &mut super::BodyEmitter<'_>,
    binding: &str,
    _binding_ty: crate::check::ty::Ty,
    start: Option<&crate::check::ir::TypedExpr>,
    end: Option<&crate::check::ir::TypedExpr>,
    inclusive: bool,
    body: &[crate::check::ir::TypedStmt],
) {
    let int_ty = crate::check::ty::Ty(0);
    let bool_ty = crate::check::ty::Ty(2);

    // Emit start value (default to 0 if not specified)
    let r_iter = if let Some(s) = start {
        emit_expr(emitter, s)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };

    // Emit end value (if no end, this is an open range -- emit Nop and return)
    let r_end = if let Some(e) = end {
        emit_expr(emitter, e)
    } else {
        // Open-ended range in for loop -- not meaningful, emit Nop
        emitter.emit(Instruction::Nop);
        return;
    };

    // For inclusive ranges, add 1 to end and use CmpLtI (no CmpLeI in the VM).
    // Exclusive: i < end. Inclusive: i < end+1 (equivalent to i <= end).
    let r_limit = if inclusive {
        let r_one = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r_one, value: 1 });
        let r_lim = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::AddI { r_dst: r_lim, r_a: r_end, r_b: r_one });
        r_lim
    } else {
        r_end
    };

    // Labels
    let loop_start = emitter.new_label();
    let loop_end = emitter.new_label();
    emitter.push_loop(loop_end, loop_start);

    emitter.mark_label_here(loop_start);

    // Condition: r_iter < r_limit
    let r_cond = emitter.alloc_reg(bool_ty);
    emitter.emit(Instruction::CmpLtI { r_dst: r_cond, r_a: r_iter, r_b: r_limit });

    // BrFalse to loop_end
    let brf_idx = emitter.instructions.len();
    emitter.emit(Instruction::BrFalse { r_cond, offset: 0 });
    emitter.add_fixup(brf_idx, loop_end);

    // Bind the iterator register as the loop variable
    emitter.locals.insert(binding.to_string(), r_iter);

    // Emit body
    for stmt in body {
        emit_stmt(emitter, stmt);
    }

    // Increment: r_iter = r_iter + 1
    let r_one = emitter.alloc_reg(int_ty);
    emitter.emit(Instruction::LoadInt { r_dst: r_one, value: 1 });
    emitter.emit(Instruction::AddI { r_dst: r_iter, r_a: r_iter, r_b: r_one });

    // Branch back to loop_start
    let br_idx = emitter.instructions.len();
    emitter.emit(Instruction::Br { offset: 0 });
    emitter.add_fixup(br_idx, loop_start);

    emitter.mark_label_here(loop_end);
    emitter.pop_loop();
}
