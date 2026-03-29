//! Expression emission for IL method bodies.
//!
//! `emit_expr` dispatches on TypedExpr variants and returns the destination
//! register containing the result. Variants not handled in this plan (Match,
//! Lambda, etc.) emit a Nop placeholder and return a void register.

mod literal;
mod binary;
mod control;
mod construction;
mod builtins;
mod string;
mod eq;

use literal::emit_literal;
use binary::emit_binary;
use control::{emit_if, emit_spawn, emit_defer};
use construction::{emit_range, emit_array_lit, emit_new};
use builtins::try_emit_builtin_method;
use string::{try_collect_str_build_parts, emit_str_build};

use writ_module::instruction::Instruction;

use crate::ast::expr::PrefixOp;
use crate::check::ir::TypedExpr;
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
            } else if name == "None" {
                // Standalone None (not a call) — emit LoadNull so the statement gets
                // its own instruction and source span entry.
                // Handles `let x: int? = None;` and `let x: int? = null;`
                // (null is lowered to Option::None path, then resolved to Var "None").
                let r_dst = emitter.alloc_reg(*ty);
                emitter.emit(Instruction::LoadNull { r_dst });
                r_dst
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
            let operand_ty = left.ty();
            let r_a = emit_expr(emitter, left);
            let r_b = emit_expr(emitter, right);
            emit_binary(emitter, *ty, op, r_a, r_b, operand_ty)
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
                if let crate::check::ir::TypedStmt::Expr { expr: last_expr, span, .. } = last {
                    for stmt in rest {
                        emit_stmt(emitter, stmt);
                    }
                    // Push source span for the tail expression (mirrors emit_stmt line 35).
                    // Without this, functions where the last/only statement is the tail
                    // have no source span, breaking breakpoints and stepping. The span
                    // comes from the TypedStmt::Expr wrapper, which records the statement's
                    // position in the source file.
                    let instr_idx = emitter.instructions.len() as u32;
                    emitter.source_spans.push((instr_idx, *span));
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
            } else if name == "None" {
                // Standalone Option::None path — emit LoadNull so the statement gets
                // its own instruction and source span entry.
                // Handles `let x = Option::None;` patterns.
                let r_dst = emitter.alloc_reg(*ty);
                emitter.emit(Instruction::LoadNull { r_dst });
                r_dst
            } else {
                emitter.alloc_reg(*ty)
            }
        }

        // ── Crash (intentional runtime panic from force-unwrap) ───────────────
        TypedExpr::Crash { ty, message, .. } => {
            // Load crash message as a string constant, then emit Crash instruction.
            let r_msg = emitter.alloc_reg(Ty(3)); // String type is Ty(3)
            let instr_idx = emitter.instructions.len();
            emitter.emit(Instruction::LoadString { r_dst: r_msg, string_idx: 0 }); // placeholder
            emitter.pending_strings.push((instr_idx, message.clone()));
            emitter.emit(Instruction::Crash { r_msg });
            // Allocate result register for type continuity (unreachable at runtime)
            emitter.alloc_reg(*ty)
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

            // EMIT-01/EMIT-02: Contract-typed receiver dispatch.
            // Must come BEFORE the !is_static_call + Func-typed Branch A check because:
            // - callee_def_id is None for contract method calls (Branch A intercepts)
            // - callee type IS TyKind::Func (Branch A matches! check passes)
            // - extract_type_def_id returns None for TyKind::Contract (falls to CALL_INDIRECT)
            if !is_static_call {
                if let TypedExpr::Field { receiver, field, .. } = callee.as_ref() {
                    if let TyKind::Contract(contract_def_id) = emitter.interner.kind(receiver.ty()).clone() {
                        let TypedExpr::Call { ty, args, .. } = expr else { unreachable!() };
                        let r_dst_call = emitter.alloc_reg(*ty);

                        // Emit self (receiver) first, then remaining args
                        let r_self = emit_expr(emitter, receiver);
                        let arg_regs: Vec<u16> = std::iter::once(r_self)
                            .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                            .collect();
                        let r_base = pack_args_consecutive(emitter, &arg_regs);

                        // Resolve contract token and slot by name (callee_def_id is None on this path)
                        let contract_token = emitter.builder.token_for_def(contract_def_id)
                            .map(|t| t.0)
                            .unwrap_or(0);
                        let slot = emitter.builder.contract_method_slot_by_name(contract_def_id, field)
                            .unwrap_or(0);

                        // CALL_VIRT layout: r_obj = receiver, r_base = first extra arg, argc = n-1
                        let r_obj = r_base;
                        let r_args_base = if arg_regs.len() > 1 { r_base + 1 } else { r_base };
                        let n_args = (arg_regs.len() as u16).saturating_sub(1);
                        emitter.emit(Instruction::CallVirt {
                            r_dst: r_dst_call,
                            r_obj,
                            contract_idx: contract_token,
                            slot,
                            r_base: r_args_base,
                            argc: n_args,
                        });
                        return r_dst_call;
                    }
                }
            }

            // IMPL-METHOD fix: when callee_def_id is None but the callee is a
            // Field access on a concrete Struct/Class receiver (e.g. `f.compute()`
            // from `impl Contract for Foo`), look up the MethodDef by type+name
            // and emit a direct CALL rather than CALL_INDIRECT.
            if !is_static_call && matches!(emitter.interner.kind(callee_ty), TyKind::Func { .. }) {
                if let TypedExpr::Field { receiver, field, .. } = callee.as_ref() {
                    let receiver_def_id = extract_type_def_id(emitter, receiver.ty());
                    if let Some(rdid) = receiver_def_id {
                        if let Some(method_token) = emitter.builder.methoddef_token_by_type_and_name(rdid, field) {
                            // Found a MethodDef: emit direct CALL (not CALL_INDIRECT).
                            let r_dst_call = emitter.alloc_reg(*ty);
                            let TypedExpr::Call { args, .. } = expr else { unreachable!() };
                            // First arg is self (the receiver object).
                            let r_self = emit_expr(emitter, receiver);
                            let arg_regs: Vec<u16> = std::iter::once(r_self)
                                .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                                .collect();
                            let argc = arg_regs.len() as u16;
                            let r_base = pack_args_consecutive(emitter, &arg_regs);
                            emitter.emit(Instruction::Call { r_dst: r_dst_call, method_idx: method_token, r_base, argc });
                            return r_dst_call;
                        }
                    }
                }

                let r_delegate = emit_expr(emitter, callee);
                emit_call_indirect(emitter, expr, r_delegate)
            } else {
                // MC-01 fix: use the DefId stored directly in callee_def_id (populated by
                // check_call_with_sig and check_generic_call during type checking).
                let maybe_def_id = *callee_def_id;

                let kind = match callee.as_ref() {
                    TypedExpr::Field { receiver, .. } => {
                        // Dispatch based on receiver's concrete/generic type
                        match emitter.interner.kind(receiver.ty()) {
                            TyKind::Struct(_) | TyKind::Class(_) | TyKind::Entity(_) => {
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

                let r_dst_call = emitter.alloc_reg(*ty);

                let TypedExpr::Call { args, .. } = expr else { unreachable!() };
                let arg_regs: Vec<u16> = args.iter().map(|arg| emit_expr(emitter, arg)).collect();
                let argc = arg_regs.len() as u16;
                let r_base = pack_args_consecutive(emitter, &arg_regs);

                // IMPL-METHOD-TOKEN fix: impl methods share the impl_def_id as their callee_def_id
                // (all methods in an impl block have the same DefId — the impl block's DefId).
                // token_for_def(impl_def_id) always returns the LAST method's token because
                // collect_impl overwrites the same key on each iteration. This causes all intra-impl
                // calls to target the wrong method (the last one registered).
                //
                // Fix: for Field-on-Class/Struct/Entity callee, resolve the method token by
                // (receiver_type_def_id, method_name) which is always unique and correct.
                // Fall back to token_for_def only for free-function calls where the def_id
                // uniquely identifies a single method.
                let method_idx = if let TypedExpr::Field { receiver, field, .. } = callee.as_ref() {
                    match emitter.interner.kind(receiver.ty()) {
                        TyKind::Struct(rdid) | TyKind::Class(rdid) | TyKind::Entity(rdid) => {
                            let rdid = *rdid;
                            emitter.builder.methoddef_token_by_type_and_name(rdid, field)
                                .unwrap_or_else(|| {
                                    // Fallback: token_for_def (works for non-impl methods)
                                    maybe_def_id
                                        .and_then(|id| emitter.builder.token_for_def(id))
                                        .map(|t| t.0)
                                        .unwrap_or(0)
                                })
                        }
                        _ => {
                            maybe_def_id
                                .and_then(|id| emitter.builder.token_for_def(id))
                                .map(|t| t.0)
                                .unwrap_or(0)
                        }
                    }
                } else {
                    maybe_def_id
                        .and_then(|id| emitter.builder.token_for_def(id))
                        .map(|t| t.0)
                        .unwrap_or(0)
                };

                match kind {
                    super::call::CallKind::Direct => {
                        emitter.emit(Instruction::Call { r_dst: r_dst_call, method_idx, r_base, argc });
                    }
                    super::call::CallKind::Virtual { slot } => {
                        let r_obj = r_base;
                        let r_args_base = if argc > 0 { r_base + 1 } else { r_base };
                        let n_args = argc.saturating_sub(1);
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
        // ── TypeOf — emit TypeOf instruction with baked-in type_idx ─────────
        TypedExpr::TypeOf { ty, static_ty, .. } => {
            let r_dst = emitter.alloc_reg(*ty);
            let type_idx = resolve_typeof_type_idx(emitter, *static_ty);
            emitter.emit(Instruction::TypeOf { r_dst, type_idx });
            r_dst
        }
    }
}

// ─── typeof type_idx resolution ──────────────────────────────────────────────

/// Resolve the type_idx token for a TypeOf instruction.
///
/// For user-defined types (struct, class, entity, enum, contract): uses `token_for_def`
/// to look up the TypeDef MetadataToken for the type's DefId.
///
/// For primitive types (int, float, bool, string): uses `type_ref_token_by_name` to look
/// up the writ-runtime pseudo-TypeDef TypeRef token registered in collect_defs.
///
/// Returns 0 for unsupported types (e.g. generic params, infer) — the runtime handles
/// a 0 type_idx gracefully.
fn resolve_typeof_type_idx(emitter: &BodyEmitter<'_>, static_ty: Ty) -> u32 {
    match emitter.interner.kind(static_ty) {
        TyKind::Struct(def_id)
        | TyKind::Class(def_id)
        | TyKind::Entity(def_id)
        | TyKind::Enum(def_id)
        | TyKind::Contract(def_id) => {
            emitter.builder.token_for_def(*def_id)
                .map(|t| t.0)
                .unwrap_or(0)
        }
        TyKind::Int => emitter.builder.type_ref_token_by_name("Int"),
        TyKind::Float => emitter.builder.type_ref_token_by_name("Float"),
        TyKind::Bool => emitter.builder.type_ref_token_by_name("Bool"),
        TyKind::String => emitter.builder.type_ref_token_by_name("String"),
        _ => 0, // Unsupported types get 0 (runtime handles gracefully)
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

// ─── Type/DefId extraction helpers ───────────────────────────────────────────

/// Extract the DefId from a TyKind::Struct, TyKind::Class, TyKind::Entity, or TyKind::Enum.
///
/// Returns None for primitive types and generic params.
pub(crate) fn extract_type_def_id(
    emitter: &BodyEmitter<'_>,
    ty: Ty,
) -> Option<crate::resolve::def_map::DefId> {
    match emitter.interner.kind(ty) {
        TyKind::Struct(def_id) | TyKind::Class(def_id) | TyKind::Entity(def_id) | TyKind::Enum(def_id) => Some(*def_id),
        _ => None,
    }
}
