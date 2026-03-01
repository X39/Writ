//! Expression emission for IL method bodies.
//!
//! `emit_expr` dispatches on TypedExpr variants and returns the destination
//! register containing the result. Variants not handled in this plan (Match,
//! Lambda, etc.) emit a Nop placeholder and return a void register.

use writ_module::instruction::Instruction;

use crate::ast::expr::{BinaryOp, PrefixOp};
use crate::check::ir::{TypedExpr, TypedLiteral};
use crate::check::ty::{Ty, TyKind};

use super::BodyEmitter;
use super::call::{emit_call_indirect, pack_args_consecutive};

/// Emit code for a TypedExpr. Returns the destination register.
///
/// For expressions with no meaningful value (void returns, placeholders),
/// a void register is still allocated and returned to satisfy the invariant
/// that every emit_expr call returns a register.
pub fn emit_expr(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    // StrBuild optimization: detect 3+ part string concatenation chains BEFORE
    // emitting sub-expressions. This must be done here to have access to the
    // original TypedExpr nodes needed for chain collection.
    if let Some(parts) = try_collect_str_build_parts(expr, emitter.interner) {
        return emit_str_build(emitter, expr.ty(), &parts);
    }

    match expr {
        // ── Literals ──────────────────────────────────────────────────────────
        TypedExpr::Literal { ty, value, .. } => emit_literal(emitter, *ty, value),

        // ── Variable / Self ───────────────────────────────────────────────────
        TypedExpr::Var { name, ty, .. } => {
            // Return the register directly if we know it, otherwise alloc a new one.
            // For now, look up in locals map; if not found (e.g., forward ref), alloc.
            if let Some(&reg) = emitter.locals.get(name) {
                reg
            } else {
                // Unresolved var: alloc a new register (shouldn't happen post-typecheck)
                emitter.alloc_reg(*ty)
            }
        }

        TypedExpr::SelfRef { .. } => {
            // self is always in r0
            // Return r0 directly if it exists, otherwise it means we're outside a method
            0
        }

        // ── Binary operations ──────────────────────────────────────────────────
        TypedExpr::Binary { left, op, right, ty, .. } => {
            let r_a = emit_expr(emitter, left);
            let r_b = emit_expr(emitter, right);
            emit_binary(emitter, *ty, op, r_a, r_b)
        }

        // ── Unary prefix ──────────────────────────────────────────────────────
        TypedExpr::UnaryPrefix { op, expr: inner, ty, .. } => {
            let r_src = emit_expr(emitter, inner);
            let r_dst = emitter.alloc_reg(*ty);
            match op {
                PrefixOp::Neg => {
                    match emitter.interner.kind(*ty) {
                        TyKind::Int => emitter.emit(Instruction::NegI { r_dst, r_src }),
                        TyKind::Float => emitter.emit(Instruction::NegF { r_dst, r_src }),
                        _ => emitter.emit(Instruction::NegI { r_dst, r_src }),
                    }
                }
                PrefixOp::Not => {
                    emitter.emit(Instruction::Not { r_dst, r_src });
                }
                PrefixOp::FromEnd => {
                    // ^ prefix — stub, handled in Plan 02 (array operations)
                    emitter.emit(Instruction::Nop);
                }
            }
            r_dst
        }

        // ── If expression ─────────────────────────────────────────────────────
        TypedExpr::If { condition, then_branch, else_branch, ty, .. } => {
            emit_if(emitter, *ty, condition, then_branch, else_branch.as_deref())
        }

        // ── Block expression ──────────────────────────────────────────────────
        TypedExpr::Block { stmts, tail, ty, .. } => {
            use super::stmt::emit_stmt;
            // BUG-10 fix: the typechecker always sets tail=None and puts the
            // final expression as the last TypedStmt::Expr in stmts. Detect
            // that pattern and return the final expression's register instead
            // of allocating a fresh void register.
            //
            // Explicit tail takes priority (forward-compatible if typechecker
            // ever starts setting tail directly).
            if let Some(tail_expr) = tail {
                for stmt in stmts {
                    emit_stmt(emitter, stmt);
                }
                emit_expr(emitter, tail_expr)
            } else if let Some((last, rest)) = stmts.split_last() {
                // Check if the last statement is a bare expression (value-producing).
                if let crate::check::ir::TypedStmt::Expr { expr: last_expr, .. } = last {
                    for stmt in rest {
                        emit_stmt(emitter, stmt);
                    }
                    // Return the register of the final expression — this is the
                    // block's value (BUG-10 fix: was returning alloc_void_reg).
                    emit_expr(emitter, last_expr)
                } else {
                    // Last stmt is a Let/While/For/etc — block is void.
                    for stmt in stmts {
                        emit_stmt(emitter, stmt);
                    }
                    // BUG-16 fix: skip register allocation for void blocks — the
                    // caller emits RetVoid without using the register, so allocating
                    // a void register here produces a spurious .reg r0 void in the IL.
                    if *ty == Ty(4) { 0 } else { emitter.alloc_void_reg() }
                }
            } else {
                // Empty block — void.
                // BUG-16 fix: skip register allocation for void blocks — the
                // caller emits RetVoid without using the register, so allocating
                // a void register here produces a spurious .reg r0 void in the IL.
                if *ty == Ty(4) { 0 } else { emitter.alloc_void_reg() }
            }
        }

        // ── Assignment ────────────────────────────────────────────────────────
        TypedExpr::Assign { target, value, ty, .. } => {
            let r_val = emit_expr(emitter, value);
            match target.as_ref() {
                TypedExpr::Var { name, .. } => {
                    if let Some(&r_dst) = emitter.locals.get(name) {
                        emitter.emit(Instruction::Mov { r_dst, r_src: r_val });
                        r_dst
                    } else {
                        // New assignment target not in locals — treat as alloc
                        let r_dst = emitter.alloc_reg(*ty);
                        emitter.locals.insert(name.clone(), r_dst);
                        emitter.emit(Instruction::Mov { r_dst, r_src: r_val });
                        r_dst
                    }
                }
                TypedExpr::Field { receiver, field, .. } => {
                    // Emit receiver, then SET_FIELD
                    let r_obj = emit_expr(emitter, receiver);
                    let receiver_def_id = extract_type_def_id(emitter, receiver.ty());
                    let field_idx = if let Some(def_id) = receiver_def_id {
                        emitter.builder.field_token_by_name(def_id, field).unwrap_or(0)
                    } else {
                        0
                    };
                    emitter.emit(Instruction::SetField { r_obj, field_idx, r_val });
                    r_val
                }
                TypedExpr::Index { receiver, index, .. } => {
                    // Array index write: ARRAY_STORE { r_arr, r_idx, r_val }
                    let r_arr = emit_expr(emitter, receiver);
                    let r_idx = emit_expr(emitter, index);
                    emitter.emit(Instruction::ArrayStore { r_arr, r_idx, r_val });
                    r_val
                }
                _ => {
                    emitter.emit(Instruction::Nop);
                    r_val
                }
            }
        }

        // ── Return expression ──────────────────────────────────────────────────
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                // EMIT-24: Tail-call optimization — Return(Call(...)) emits TailCall
                // instead of Call + Ret. This is required for dialogue transitions
                // which produce recursive state machine patterns.
                if let TypedExpr::Call { callee, args, callee_def_id, .. } = v.as_ref() {
                    return emit_tail_call(emitter, callee, args, *callee_def_id);
                }
                let r_src = emit_expr(emitter, v);
                emitter.emit(Instruction::Ret { r_src });
            } else {
                emitter.emit(Instruction::RetVoid);
            }
            emitter.alloc_void_reg()
        }

        // ── Path (treat like Var) ──────────────────────────────────────────────
        TypedExpr::Path { segments, ty, .. } => {
            let name = segments.last().cloned().unwrap_or_default();
            if let Some(&reg) = emitter.locals.get(&name) {
                reg
            } else {
                emitter.alloc_reg(*ty)
            }
        }

        // ── Error (should never reach codegen after pre-pass) ─────────────────
        TypedExpr::Error { .. } => {
            panic!("TypedExpr::Error reached codegen — pre-pass should have aborted");
        }

        // ── Call dispatch (EMIT-09, EMIT-21, EMIT-27) ─────────────────────────
        TypedExpr::Call { callee, ty, callee_def_id, .. } => {
            let callee_ty = callee.ty();

            // ── Built-in shortcut: Option/Result/Array methods ────────────────
            // Before standard dispatch, check if this is a built-in method call
            // that should emit a dedicated instruction (not CALL).
            if let Some(r) = try_emit_builtin_method(emitter, expr) {
                return r;
            }

            // BUG-07 fix: only use CALL_INDIRECT for genuine delegate/closure
            // calls where callee_def_id is None. When callee_def_id is Some(_),
            // the callee is a statically-known named function and must use the
            // direct/extern/virtual dispatch path below regardless of callee type.
            let is_static_call = callee_def_id.is_some();
            if !is_static_call && matches!(emitter.interner.kind(callee_ty), TyKind::Func { .. }) {
                let r_delegate = emit_expr(emitter, callee);
                emit_call_indirect(emitter, expr, r_delegate)
            } else {
                // MC-01 fix: use the DefId stored directly in callee_def_id (populated by
                // check_call_with_sig and check_generic_call during type checking).
                // Previously extract_callee_def_id_opt always returned None, causing
                // method_idx=0 for all CALL/CALL_VIRT/TailCall instructions on this path.
                let maybe_def_id = *callee_def_id;

                // Use a synthetic "empty" DefId for the analyze_callee dispatch.
                // Since the full pipeline provides the real DefId, this is fine for now.
                // analyze_callee primarily uses the receiver type (available), not the DefId.
                let kind = match callee.as_ref() {
                    TypedExpr::Field { receiver, .. } => {
                        // Dispatch based on receiver's concrete/generic type
                        match emitter.interner.kind(receiver.ty()) {
                            TyKind::Struct(_) | TyKind::Entity(_) => {
                                super::call::CallKind::Direct
                            }
                            TyKind::GenericParam(_) => {
                                super::call::CallKind::Virtual { slot: 0 }
                            }
                            _ => super::call::CallKind::Direct,
                        }
                    }
                    _ => {
                        // Check if callee_def_id maps to an ExternDef token (BUG-05 fix).
                        // If the callee resolves to an ExternDef entry, emit CALL_EXTERN.
                        let is_extern = maybe_def_id
                            .and_then(|id| emitter.builder.token_for_def(id))
                            .map(|t| {
                                use crate::emit::metadata::TableId;
                                t.table() == TableId::ExternDef
                            })
                            .unwrap_or(false);
                        if is_extern {
                            super::call::CallKind::Extern
                        } else {
                            super::call::CallKind::Direct
                        }
                    }
                };

                // Emit the call with a placeholder DefId (method_idx will be 0 unless
                // the builder has a token registered for `maybe_def_id`).
                let r_dst_call = emitter.alloc_reg(*ty);

                // Emit args in consecutive block (BUG-06 fix: skip MOV if already consecutive)
                let TypedExpr::Call { args, .. } = expr else { unreachable!() };
                let arg_regs: Vec<u16> = args.iter().map(|arg| emit_expr(emitter, arg)).collect();
                let argc = arg_regs.len() as u16;
                let r_base = pack_args_consecutive(emitter, &arg_regs);

                let method_idx = maybe_def_id
                    .and_then(|id| emitter.builder.token_for_def(id))
                    .map(|t| t.0)
                    .unwrap_or(0);

                match kind {
                    super::call::CallKind::Direct => {
                        emitter.emit(Instruction::Call { r_dst: r_dst_call, method_idx, r_base, argc });
                    }
                    super::call::CallKind::Virtual { slot } => {
                        let r_obj = r_base;
                        let r_args_base = if argc > 0 { r_base + 1 } else { r_base };
                        let n_args = argc.saturating_sub(1);
                        // FIX-02: Resolve contract token via the callee's DefId if available.
                        // When maybe_def_id is Some and has a registered impl-method-to-contract
                        // mapping, emit the real contract token instead of 0. Falls back to 0
                        // when no DefId is available (current pipeline: extract_callee_def_id_opt
                        // returns None until BodyEmitter gains DefMap access).
                        let contract_idx: u32 = maybe_def_id
                            .and_then(|id| emitter.builder.contract_token_for_method_def_id(id))
                            .map(|t| t.0)
                            .unwrap_or(0);
                        emitter.emit(Instruction::CallVirt { r_dst: r_dst_call, r_obj, contract_idx, slot, r_base: r_args_base, argc: n_args });
                    }
                    super::call::CallKind::Extern => {
                        emitter.emit(Instruction::CallExtern { r_dst: r_dst_call, extern_idx: method_idx, r_base, argc });
                    }
                    super::call::CallKind::Indirect => {
                        let r_delegate = emitter.regs.next().saturating_sub(1);
                        emitter.emit(Instruction::CallIndirect { r_dst: r_dst_call, r_delegate, r_base, argc });
                    }
                }
                r_dst_call
            }
        }

        // ── Field access (GET_FIELD) ───────────────────────────────────────────
        TypedExpr::Field { receiver, field, ty, .. } => {
            let r_obj = emit_expr(emitter, receiver);
            let receiver_def_id = extract_type_def_id(emitter, receiver.ty());
            let field_idx = if let Some(def_id) = receiver_def_id {
                emitter.builder.field_token_by_name(def_id, field).unwrap_or(0)
            } else {
                0
            };
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::GetField { r_dst, r_obj, field_idx });
            r_dst
        }

        // ── Component access (GET_COMPONENT) ──────────────────────────────────
        TypedExpr::ComponentAccess { receiver, ty, .. } => {
            let r_entity = emit_expr(emitter, receiver);
            // Resolve the component type token from the component's ty (TyKind::Struct(def_id))
            let comp_idx = extract_type_def_id(emitter, *ty)
                .and_then(|def_id| emitter.builder.token_for_def(def_id))
                .map(|t| t.0)
                .unwrap_or(0);
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::GetComponent { r_dst, r_entity, comp_type_idx: comp_idx });
            r_dst
        }

        // ── Index access — ARRAY_LOAD ─────────────────────────────────────────
        TypedExpr::Index { ty, receiver, index, .. } => {
            let r_arr = emit_expr(emitter, receiver);
            let r_idx = emit_expr(emitter, index);
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::ArrayLoad { r_dst, r_arr, r_idx });
            r_dst
        }

        // ── Match — enum/option/result pattern lowering (EMIT-17, EMIT-23) ───
        TypedExpr::Match { .. } => {
            super::patterns::emit_match(emitter, expr)
        }

        // ── Lambda — closure/delegate lowering (EMIT-14) ─────────────────────
        TypedExpr::Lambda { ty, captures, .. } => {
            let mut counter = emitter.lambda_counter;
            let r = super::closure::emit_lambda(emitter, captures, &mut counter, *ty);
            emitter.lambda_counter = counter;
            r
        }

        // ── Object construction (EMIT-10, EMIT-11) ────────────────────────────
        TypedExpr::New { ty, target_def_id, fields, .. } => {
            emit_new(emitter, *ty, *target_def_id, fields)
        }
        TypedExpr::ArrayLit { ty, elements, .. } => {
            emit_array_lit(emitter, *ty, elements)
        }
        TypedExpr::Range { ty, start, end, inclusive, .. } => {
            emit_range(emitter, *ty, start.as_deref(), end.as_deref(), *inclusive)
        }
        // ── Spawn — SPAWN_TASK (EMIT-15) ──────────────────────────────────────
        TypedExpr::Spawn { ty, expr: inner, .. } => {
            emit_spawn(emitter, *ty, inner, false)
        }
        // ── SpawnDetached — SPAWN_DETACHED (EMIT-15) ──────────────────────────
        TypedExpr::SpawnDetached { ty, expr: inner, .. } => {
            emit_spawn(emitter, *ty, inner, true)
        }
        // ── Join — JOIN (EMIT-15) ──────────────────────────────────────────────
        TypedExpr::Join { ty, expr: inner, .. } => {
            let r_task = emit_expr(emitter, inner);
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::Join { r_dst, r_task });
            r_dst
        }
        // ── Cancel — CANCEL (EMIT-15) ──────────────────────────────────────────
        TypedExpr::Cancel { expr: inner, .. } => {
            let r_task = emit_expr(emitter, inner);
            emitter.emit(Instruction::Cancel { r_task });
            emitter.alloc_void_reg()
        }
        // ── Defer — DEFER_PUSH/POP/END (EMIT-15) ─────────────────────────────
        TypedExpr::Defer { expr: inner, .. } => {
            emit_defer(emitter, inner)
        }
    }
}

// ─── Literal emission ────────────────────────────────────────────────────────

fn emit_literal(emitter: &mut BodyEmitter<'_>, ty: crate::check::ty::Ty, value: &TypedLiteral) -> u16 {
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

// ─── Binary emission ────────────────────────────────────────────────────────

fn emit_binary(
    emitter: &mut BodyEmitter<'_>,
    ty: crate::check::ty::Ty,
    op: &BinaryOp,
    r_a: u16,
    r_b: u16,
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
            // Result is always bool, but we need to know the operand type.
            // The operand type is carried on left/right, not on ty (which is Bool).
            // We use r_a's allocated type from the interner; for now use ty_kind of
            // the result (Bool). We need the operand kind — which we infer from
            // the instruction's register types. Since we don't have that here,
            // we fall back to checking the result type and use a convention:
            // Eq on Bool result always means the operands are compared via CmpEqI/F/B/S.
            // We need to pass the OPERAND type, not result type.
            // Solution: the operand type is in left.ty() / right.ty(). The caller passes
            // ty=result type (Bool). We need to store operand type somewhere.
            // Workaround: We accept that the Binary variant already resolved the correct
            // comparison instruction selection above the call. The caller should use
            // emit_binary_with_operand_type. For now, emit CmpEqI as default.
            // This will be fixed by passing operand_ty separately when needed.
            let bool_ty = crate::check::ty::Ty(2); // Bool is Ty(2)
            let r_dst = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqI { r_dst, r_a, r_b });
            r_dst
        }
        BinaryOp::NotEq => {
            let bool_ty = crate::check::ty::Ty(2);
            let r_cmp = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::CmpEqI { r_dst: r_cmp, r_a, r_b });
            let r_dst = emitter.alloc_reg(bool_ty);
            emitter.emit(Instruction::Not { r_dst, r_src: r_cmp });
            r_dst
        }
        BinaryOp::Lt => {
            let bool_ty = crate::check::ty::Ty(2);
            let r_dst = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst, r_a, r_b }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst, r_a, r_b }),
            }
            r_dst
        }
        BinaryOp::Gt => {
            // a > b  ≡  b < a
            let bool_ty = crate::check::ty::Ty(2);
            let r_dst = emitter.alloc_reg(bool_ty);
            match ty_kind {
                TyKind::Float => emitter.emit(Instruction::CmpLtF { r_dst, r_a: r_b, r_b: r_a }),
                _ => emitter.emit(Instruction::CmpLtI { r_dst, r_a: r_b, r_b: r_a }),
            }
            r_dst
        }
        BinaryOp::LtEq => {
            // a <= b  ≡  !(b < a)
            let bool_ty = crate::check::ty::Ty(2);
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
            let bool_ty = crate::check::ty::Ty(2);
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

// ─── If/else emission ────────────────────────────────────────────────────────

fn emit_if(
    emitter: &mut BodyEmitter<'_>,
    ty: crate::check::ty::Ty,
    condition: &TypedExpr,
    then_branch: &TypedExpr,
    else_branch: Option<&TypedExpr>,
) -> u16 {
    // Emit condition
    let r_cond = emit_expr(emitter, condition);

    // Allocate a shared result register that both branches MOV into (BUG-04 fix).
    // This ensures the RET instruction always references an initialized register
    // regardless of which branch was taken at runtime.
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

// ─── Concurrency emission ────────────────────────────────────────────────────

/// Emit a spawn expression.
///
/// `spawn expr` lowers to:
///   1. Emit the inner call expression's arguments
///   2. SPAWN_TASK { r_dst, method_idx, r_base, argc }
///
/// The inner expr must be a Call. method_idx is derived from the call's callee
/// (using the builder's def_token_map, or 0 as placeholder).
fn emit_spawn(
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
/// ```
///
/// The DeferPush.method_idx holds the instruction index of the handler body start
/// (index [3] in the example). On normal exit, DeferPop disarms the defer and
/// Br jumps past the handler. The runtime jumps to method_idx when the defer fires.
fn emit_defer(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    let void_ty = Ty(4); // Void
    let r_dst = emitter.alloc_reg(void_ty);

    // Emit DeferPush with placeholder method_idx; record index for patching
    let defer_push_idx = emitter.instructions.len();
    emitter.emit(Instruction::DeferPush { r_dst, method_idx: 0 }); // placeholder

    // DeferPop: disarm the defer on normal exit path
    emitter.emit(Instruction::DeferPop);

    // Branch past the handler on normal execution path (placeholder offset, patched below)
    let br_skip_idx = emitter.instructions.len();
    emitter.emit(Instruction::Br { offset: 0 }); // placeholder

    // Handler starts at the NEXT instruction index
    let handler_start_idx = emitter.instructions.len() as u32;

    // Emit the handler body (the deferred expression)
    let _ = emit_expr(emitter, expr);

    // DeferEnd: marks completion of handler execution
    emitter.emit(Instruction::DeferEnd);

    // Patch DeferPush with correct handler instruction index
    if let Instruction::DeferPush { method_idx, .. } = &mut emitter.instructions[defer_push_idx] {
        *method_idx = handler_start_idx;
    }

    // Patch the Br skip to jump past handler + DeferEnd
    let after_handler_idx = emitter.instructions.len() as i32;
    let br_target_offset = after_handler_idx - br_skip_idx as i32;
    if let Instruction::Br { offset } = &mut emitter.instructions[br_skip_idx] {
        *offset = br_target_offset;
    }

    r_dst
}

// --- Range construction emission (BF-02) -------------------------------------

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
fn emit_range(
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


// ─── Array literal emission ──────────────────────────────────────────────────

/// Emit an array literal. Non-empty arrays use ARRAY_INIT; empty arrays use NEW_ARRAY.
fn emit_array_lit(emitter: &mut BodyEmitter<'_>, ty: Ty, elements: &[TypedExpr]) -> u16 {
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

// ─── Built-in method shortcutting ────────────────────────────────────────────

/// Check if a Call expression is a built-in Option/Result/Array/constructor method.
/// If so, emit the dedicated instruction and return Some(reg). Otherwise None.
///
/// Built-ins detected:
/// - Option: .is_none(), .unwrap(), .is_some()
/// - Result: .is_err(), .is_ok(), .unwrap_ok(), .unwrap_err(), .extract_err()
/// - Array: .len(), .slice()
/// - Constructor patterns: Some(val), None, Ok(val), Err(val) via Path callee
fn try_emit_builtin_method(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> Option<u16> {
    let (ty, callee, args) = match expr {
        TypedExpr::Call { ty, callee, args, .. } => (*ty, callee.as_ref(), args),
        _ => return None,
    };

    match callee {
        // ── Method call on a receiver: Field { receiver, field, .. } ─────────
        TypedExpr::Field { receiver, field, .. } => {
            let recv_ty = receiver.ty();
            match emitter.interner.kind(recv_ty).clone() {
                TyKind::Option(_) => {
                    let r_opt = emit_expr(emitter, receiver);
                    match field.as_str() {
                        "is_none" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::IsNone { r_dst, r_opt });
                            return Some(r_dst);
                        }
                        "is_some" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::IsSome { r_dst, r_opt });
                            return Some(r_dst);
                        }
                        "unwrap" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::Unwrap { r_dst, r_opt });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::Result(_, _) => {
                    let r_result = emit_expr(emitter, receiver);
                    match field.as_str() {
                        "is_err" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::IsErr { r_dst, r_result });
                            return Some(r_dst);
                        }
                        "is_ok" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::IsOk { r_dst, r_result });
                            return Some(r_dst);
                        }
                        "unwrap_ok" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::UnwrapOk { r_dst, r_result });
                            return Some(r_dst);
                        }
                        "unwrap_err" | "extract_err" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::ExtractErr { r_dst, r_result });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::Array(_) => {
                    let r_arr = emit_expr(emitter, receiver);
                    match field.as_str() {
                        "len" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::ArrayLen { r_dst, r_arr });
                            return Some(r_dst);
                        }
                        "slice" if args.len() == 2 => {
                            let r_start = emit_expr(emitter, &args[0]);
                            let r_end = emit_expr(emitter, &args[1]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::ArraySlice { r_dst, r_arr, r_start, r_end });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::String => {
                    let r_str = emit_expr(emitter, receiver);
                    match field.as_str() {
                        "len" => {
                            // EMIT-20: string.len() -> STR_LEN
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrLen { r_dst, r_str });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::Int => {
                    let r_src = emit_expr(emitter, receiver);
                    match field.as_str() {
                        // EMIT-19: int.into<Float>() -> I2F
                        "into_float" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::I2f { r_dst, r_src });
                            return Some(r_dst);
                        }
                        // EMIT-19: int.into<String>() -> I2S
                        "into_string" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::I2s { r_dst, r_src });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::Float => {
                    let r_src = emit_expr(emitter, receiver);
                    match field.as_str() {
                        // EMIT-19: float.into<Int>() -> F2I
                        "into_int" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::F2i { r_dst, r_src });
                            return Some(r_dst);
                        }
                        // EMIT-19: float.into<String>() -> F2S
                        "into_string" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::F2s { r_dst, r_src });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::Bool => {
                    let r_src = emit_expr(emitter, receiver);
                    match field.as_str() {
                        // EMIT-19: bool.into<String>() -> B2S
                        "into_string" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::B2s { r_dst, r_src });
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // ── Constructor call patterns via Path: Some(val), None, Ok(val), Err(val) ─
        TypedExpr::Path { segments, .. } => {
            // Detect single-segment names that are constructor patterns
            let name = match segments.last() {
                Some(n) => n.as_str(),
                None => return None,
            };
            match name {
                "Some" if args.len() == 1 => {
                    let r_val = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapSome { r_dst, r_val });
                    return Some(r_dst);
                }
                "None" if args.is_empty() => {
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::LoadNull { r_dst });
                    return Some(r_dst);
                }
                "Ok" if args.len() == 1 => {
                    let r_val = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapOk { r_dst, r_val });
                    return Some(r_dst);
                }
                "Err" if args.len() == 1 => {
                    let r_err = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapErr { r_dst, r_err });
                    return Some(r_dst);
                }
                _ => {}
            }
        }

        // ── Var-based constructor patterns ────────────────────────────────────
        // TypedExpr::Var { name: "Some"/"None"/"Ok"/"Err", .. }
        TypedExpr::Var { name, .. } => {
            match name.as_str() {
                "Some" if args.len() == 1 => {
                    let r_val = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapSome { r_dst, r_val });
                    return Some(r_dst);
                }
                "None" if args.is_empty() => {
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::LoadNull { r_dst });
                    return Some(r_dst);
                }
                "Ok" if args.len() == 1 => {
                    let r_val = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapOk { r_dst, r_val });
                    return Some(r_dst);
                }
                "Err" if args.len() == 1 => {
                    let r_err = emit_expr(emitter, &args[0]);
                    let r_dst = emitter.alloc_reg(ty);
                    emitter.emit(Instruction::WrapErr { r_dst, r_err });
                    return Some(r_dst);
                }
                _ => {}
            }
        }

        _ => {}
    }

    None
}

// ─── Object construction (EMIT-10, EMIT-11) ──────────────────────────────────

/// Emit a struct or entity construction sequence.
///
/// Struct: NEW { type_idx } + SET_FIELD per explicit field.
/// Entity: SPAWN_ENTITY { type_idx } + SET_FIELD(explicit fields only) + INIT_ENTITY.
///
/// Entity default field values do NOT generate SET_FIELD (spec §2.16.7).
fn emit_new(
    emitter: &mut BodyEmitter<'_>,
    ty: Ty,
    target_def_id: crate::resolve::def_map::DefId,
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

// ─── Tail-call emission (EMIT-24) ────────────────────────────────────────────

/// Emit a TailCall instruction for a Return(Call(...)) pattern.
///
/// Dialogue transitions are lowered to `Return(Call(...))` at the AST level.
/// This function detects that pattern and emits TailCall instead of Call + Ret,
/// which is required for correct stack frame management in recursive state machines.
pub(crate) fn emit_tail_call(
    emitter: &mut BodyEmitter<'_>,
    callee: &TypedExpr,
    args: &[TypedExpr],
    callee_def_id: Option<crate::resolve::def_map::DefId>,
) -> u16 {
    // Emit arguments; pack into consecutive block (BUG-06 fix: skip MOV if already consecutive)
    let arg_regs: Vec<u16> = args.iter().map(|arg| emit_expr(emitter, arg)).collect();
    let argc = arg_regs.len() as u16;
    let r_base = pack_args_consecutive(emitter, &arg_regs);

    // MC-01 fix: use the callee_def_id propagated from TypedExpr::Call.
    // Previously extract_callee_def_id_opt always returned None causing method_idx=0.
    let _ = callee; // callee sub-expression no longer needed for DefId resolution
    let method_idx = callee_def_id
        .and_then(|id| emitter.builder.token_for_def(id))
        .map(|t| t.0)
        .unwrap_or(0);

    emitter.emit(Instruction::TailCall { method_idx, r_base, argc });

    // TailCall does not return to this frame; return a void register to satisfy
    // the invariant that every emit_expr call returns a register.
    emitter.alloc_void_reg()
}

// ─── StrBuild emission (EMIT-20 completeness) ────────────────────────────────

/// Attempt to collect a left-associative string Add chain of 3+ parts.
///
/// Format strings are lowered by fmt_string.rs to left-associative Binary(Add) trees
/// where every node has TyKind::String. A chain `a + b + c` becomes:
/// `Binary(Add, Binary(Add, a, b), c)`
///
/// Returns `Some(parts)` if the expression is a string Add chain of 3+ leaf nodes,
/// or `None` if it's a 2-part chain or not a string Add at all.
fn try_collect_str_build_parts<'a>(
    expr: &'a TypedExpr,
    interner: &crate::check::ty::TyInterner,
) -> Option<Vec<&'a TypedExpr>> {
    if let TypedExpr::Binary { op, ty, .. } = expr {
        if *op == BinaryOp::Add {
            if matches!(interner.kind(*ty), TyKind::String) {
                let mut parts = Vec::new();
                collect_string_chain(expr, interner, &mut parts);
                if parts.len() >= 3 {
                    return Some(parts);
                }
            }
        }
    }
    None
}

/// Recursively collect leaf nodes from a left-associative string Add chain.
fn collect_string_chain<'a>(
    expr: &'a TypedExpr,
    interner: &crate::check::ty::TyInterner,
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
fn emit_str_build(emitter: &mut BodyEmitter<'_>, ty: Ty, parts: &[&TypedExpr]) -> u16 {
    // Emit each part expression
    let part_regs: Vec<u16> = parts.iter().map(|p| emit_expr(emitter, p)).collect();
    let count = part_regs.len() as u16;

    // BUG-06 fix: pack into consecutive block, skipping MOV if already consecutive
    let r_base = pack_args_consecutive(emitter, &part_regs);

    let r_dst = emitter.alloc_reg(ty);
    emitter.emit(Instruction::StrBuild { r_dst, count, r_base });
    r_dst
}

// ─── Type/DefId extraction helpers ───────────────────────────────────────────

/// Extract the DefId from a TyKind::Struct, TyKind::Entity, or TyKind::Enum.
///
/// Returns None for primitive types and generic params.
pub(crate) fn extract_type_def_id(
    emitter: &BodyEmitter<'_>,
    ty: Ty,
) -> Option<crate::resolve::def_map::DefId> {
    match emitter.interner.kind(ty) {
        TyKind::Struct(def_id) | TyKind::Entity(def_id) | TyKind::Enum(def_id) => Some(*def_id),
        _ => None,
    }
}

