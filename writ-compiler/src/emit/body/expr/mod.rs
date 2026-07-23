//! Expression emission for IL method bodies.
//!
//! `emit_expr` dispatches on TypedExpr variants and returns the destination
//! register containing the result. Variants not handled in this plan (Match,
//! Lambda, etc.) emit a Nop placeholder and return a void register.

mod binary;
mod builtins;
mod construction;
mod control;
mod eq;
mod literal;
mod string;

use binary::emit_binary;
use builtins::try_emit_builtin_method;
use construction::{emit_array_lit, emit_new, emit_range};
use control::{emit_defer, emit_if, emit_spawn};
use literal::emit_literal;
use string::{emit_str_build, try_collect_str_build_parts};

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
        TypedExpr::Binary {
            left,
            op,
            right,
            ty,
            ..
        } => {
            let operand_ty = left.ty();
            let r_a = emit_expr(emitter, left);
            let r_b = emit_expr(emitter, right);
            emit_binary(emitter, *ty, op, r_a, r_b, operand_ty)
        }

        // ── Unary prefix ──────────────────────────────────────────────────────
        TypedExpr::UnaryPrefix {
            op,
            expr: inner,
            ty,
            ..
        } => {
            let r_src = emit_expr(emitter, inner);
            let r_dst = emitter.alloc_reg(*ty);
            match op {
                PrefixOp::Neg => match emitter.interner.kind(*ty) {
                    TyKind::Int => emitter.emit(Instruction::NegI { r_dst, r_src }),
                    TyKind::Float => emitter.emit(Instruction::NegF { r_dst, r_src }),
                    _ => emitter.emit(Instruction::NegI { r_dst, r_src }),
                },
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
        TypedExpr::If {
            condition,
            then_branch,
            else_branch,
            ty,
            ..
        } => emit_if(emitter, *ty, condition, then_branch, else_branch.as_deref()),

        // ── Block expression ──────────────────────────────────────────────────
        TypedExpr::Block {
            stmts, tail, ty, ..
        } => {
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
                if let crate::check::ir::TypedStmt::Expr {
                    expr: last_expr,
                    span,
                    ..
                } = last
                {
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
                    if *ty == Ty(4) {
                        0
                    } else {
                        emitter.alloc_void_reg()
                    }
                }
            } else {
                // Empty block — void.
                // BUG-16 fix: skip register allocation for void blocks — the
                // caller emits RetVoid without using the register, so allocating
                // a void register here produces a spurious .reg r0 void in the IL.
                if *ty == Ty(4) {
                    0
                } else {
                    emitter.alloc_void_reg()
                }
            }
        }

        // ── Assignment ────────────────────────────────────────────────────────
        TypedExpr::Assign {
            target, value, ty, ..
        } => {
            let r_val = emit_expr(emitter, value);
            match target.as_ref() {
                TypedExpr::Var { name, .. } => {
                    if let Some(&r_dst) = emitter.locals.get(name) {
                        emitter.emit(Instruction::Mov {
                            r_dst,
                            r_src: r_val,
                        });
                        r_dst
                    } else {
                        // New assignment target not in locals — treat as alloc
                        let r_dst = emitter.alloc_reg(*ty);
                        emitter.locals.insert(name.clone(), r_dst);
                        emitter.emit(Instruction::Mov {
                            r_dst,
                            r_src: r_val,
                        });
                        r_dst
                    }
                }
                TypedExpr::Field {
                    receiver, field, ..
                } => {
                    // Emit receiver, then SET_FIELD
                    let r_obj = emit_expr(emitter, receiver);
                    let receiver_def_id = extract_type_def_id(emitter, receiver.ty())
                        .expect("checked field assignment must have a nominal receiver type");
                    let field_idx = emitter
                        .builder
                        .field_token_by_name(receiver_def_id, field)
                        .unwrap_or_else(|| {
                            panic!("checked field assignment `{field}` has no metadata operand")
                        });
                    emitter.emit(Instruction::SetField {
                        r_obj,
                        field_idx,
                        r_val,
                    });
                    r_val
                }
                TypedExpr::Index {
                    receiver, index, ..
                } => {
                    // Array index write: ARRAY_STORE { r_arr, r_idx, r_val }
                    let r_arr = emit_expr(emitter, receiver);
                    let r_idx = emit_expr(emitter, index);
                    emitter.emit(Instruction::ArrayStore {
                        r_arr,
                        r_idx,
                        r_val,
                    });
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
            emitter.emit(Instruction::LoadString {
                r_dst: r_msg,
                string_idx: 0,
            }); // placeholder
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
        TypedExpr::Call {
            callee,
            ty,
            callee_def_id,
            callee_has_receiver,
            ..
        } => {
            let callee_ty = callee.ty();
            let concrete_target = resolve_concrete_call_target(
                emitter,
                callee,
                callee_ty,
                *callee_def_id,
                *callee_has_receiver,
            );

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
                if let TypedExpr::Field {
                    receiver, field, ..
                } = callee.as_ref()
                {
                    let receiver_ty = emitter.interner.resolve_infer(receiver.ty());
                    if let TyKind::Contract(contract_def_id) =
                        emitter.interner.kind(receiver_ty).clone()
                    {
                        let TypedExpr::Call { ty, args, .. } = expr else {
                            unreachable!()
                        };
                        let r_dst_call = emitter.alloc_reg(*ty);

                        // Emit self (receiver) first, then remaining args
                        let r_self = emit_expr(emitter, receiver);
                        let arg_regs: Vec<u16> = std::iter::once(r_self)
                            .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                            .collect();
                        let r_base = pack_args_consecutive(emitter, &arg_regs);

                        // Resolve contract token and slot by name (callee_def_id is None on this path)
                        let contract_token = emitter
                            .builder
                            .type_spec_token_for_encoded_ty(receiver_ty, emitter.interner)
                            .or_else(|| emitter.builder.token_for_def(contract_def_id))
                            .map(|t| t.0)
                            .unwrap_or(0);
                        let slot = emitter
                            .builder
                            .contract_method_slot_by_name(contract_def_id, field)
                            .unwrap_or(0);

                        // CALL_VIRT's argument block starts with the receiver.
                        let r_obj = r_base;
                        emitter.emit(Instruction::CallVirt {
                            r_dst: r_dst_call,
                            r_obj,
                            contract_idx: contract_token,
                            slot,
                            r_base,
                            argc: arg_regs.len() as u16,
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
                if let TypedExpr::Field { .. } = callee.as_ref() {
                    if let Some(target) = concrete_target {
                        // Found a MethodDef: emit direct CALL (not CALL_INDIRECT).
                        let r_dst_call = emitter.alloc_reg(*ty);
                        let TypedExpr::Call { args, .. } = expr else {
                            unreachable!()
                        };
                        let (r_base, argc) = pack_concrete_call_args(emitter, callee, args, target)
                            .expect("resolved instance call must have a field receiver");
                        emitter.emit(Instruction::Call {
                            r_dst: r_dst_call,
                            method_idx: target.token,
                            r_base,
                            argc,
                        });
                        return r_dst_call;
                    }
                }

                let r_delegate = emit_expr(emitter, callee);
                emit_call_indirect(emitter, expr, r_delegate)
            } else {
                // MC-01 fix: use the DefId stored directly in callee_def_id (populated by
                // check_call_with_sig and check_generic_call during type checking).
                let maybe_def_id = *callee_def_id;
                let declared_token = maybe_def_id.and_then(|id| emitter.builder.token_for_def(id));
                let is_extern = declared_token.is_some_and(|token| {
                    use crate::emit::metadata::TableId;
                    token.table() == TableId::ExternDef
                });

                let kind = if is_extern {
                    super::call::CallKind::Extern
                } else {
                    match callee.as_ref() {
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
                        _ => super::call::CallKind::Direct,
                    }
                };

                let r_dst_call = emitter.alloc_reg(*ty);

                let TypedExpr::Call { args, .. } = expr else {
                    unreachable!()
                };
                let packed_concrete_args = match (kind, concrete_target) {
                    (super::call::CallKind::Direct, Some(target)) => {
                        pack_concrete_call_args(emitter, callee, args, target)
                    }
                    _ => None,
                };
                let (r_base, argc) = if let Some(packed) = packed_concrete_args {
                    packed
                } else {
                    let arg_regs: Vec<u16> = match (kind, callee.as_ref()) {
                        (super::call::CallKind::Direct, TypedExpr::Field { receiver, .. })
                            if declared_token
                                .and_then(|token| emitter.builder.method_has_receiver(token))
                                .unwrap_or(true) =>
                        {
                            std::iter::once(emit_expr(emitter, receiver))
                                .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                                .collect()
                        }
                        (
                            super::call::CallKind::Virtual { .. },
                            TypedExpr::Field { receiver, .. },
                        ) => std::iter::once(emit_expr(emitter, receiver))
                            .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                            .collect(),
                        (super::call::CallKind::Virtual { .. }, _) => {
                            panic!("virtual call requires a field receiver");
                        }
                        _ => args.iter().map(|arg| emit_expr(emitter, arg)).collect(),
                    };
                    let argc = arg_regs.len() as u16;
                    (pack_args_consecutive(emitter, &arg_regs), argc)
                };

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
                let method_idx = if let TypedExpr::Field { receiver, .. } = callee.as_ref() {
                    match emitter.interner.kind(receiver.ty()) {
                        TyKind::Struct(_) | TyKind::Class(_) | TyKind::Entity(_) => {
                            concrete_target
                                .expect(
                                    "checked concrete method call has no non-null metadata target",
                                )
                                .token
                        }
                        _ => maybe_def_id
                            .and_then(|id| emitter.builder.token_for_def(id))
                            .filter(|token| !token.is_null())
                            .map(|t| t.0)
                            .expect("checked direct call has no non-null metadata target"),
                    }
                } else {
                    maybe_def_id
                        .and_then(|id| emitter.builder.token_for_def(id))
                        .filter(|token| !token.is_null())
                        .map(|t| t.0)
                        .expect("checked direct call has no non-null metadata target")
                };

                match kind {
                    super::call::CallKind::Direct => {
                        emitter.emit(Instruction::Call {
                            r_dst: r_dst_call,
                            method_idx,
                            r_base,
                            argc,
                        });
                    }
                    super::call::CallKind::Virtual { slot } => {
                        let r_obj = r_base;
                        let contract_idx: u32 = maybe_def_id
                            .and_then(|id| emitter.builder.contract_token_for_method_def_id(id))
                            .map(|t| t.0)
                            .unwrap_or(0);
                        emitter.emit(Instruction::CallVirt {
                            r_dst: r_dst_call,
                            r_obj,
                            contract_idx,
                            slot,
                            r_base,
                            argc,
                        });
                    }
                    super::call::CallKind::Extern => {
                        emitter.emit(Instruction::CallExtern {
                            r_dst: r_dst_call,
                            extern_idx: method_idx,
                            r_base,
                            argc,
                        });
                    }
                    super::call::CallKind::Indirect => {
                        let r_delegate = emitter.regs.next().saturating_sub(1);
                        emitter.emit(Instruction::CallIndirect {
                            r_dst: r_dst_call,
                            r_delegate,
                            r_base,
                            argc,
                        });
                    }
                }
                r_dst_call
            }
        }

        // ── Field access (GET_FIELD) ───────────────────────────────────────────
        TypedExpr::Field {
            receiver,
            field,
            ty,
            ..
        } => {
            let r_obj = emit_expr(emitter, receiver);
            let receiver_def_id = extract_type_def_id(emitter, receiver.ty())
                .expect("checked field access must have a nominal receiver type");
            let field_idx = emitter
                .builder
                .field_token_by_name(receiver_def_id, field)
                .unwrap_or_else(|| {
                    panic!("checked field access `{field}` has no metadata operand")
                });
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::GetField {
                r_dst,
                r_obj,
                field_idx,
            });
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
            emitter.emit(Instruction::GetComponent {
                r_dst,
                r_entity,
                comp_type_idx: comp_idx,
            });
            r_dst
        }

        // ── Index access — ARRAY_LOAD ─────────────────────────────────────────
        TypedExpr::Index {
            ty,
            receiver,
            index,
            ..
        } => {
            let r_arr = emit_expr(emitter, receiver);
            let r_idx = emit_expr(emitter, index);
            let r_dst = emitter.alloc_reg(*ty);
            emitter.emit(Instruction::ArrayLoad {
                r_dst,
                r_arr,
                r_idx,
            });
            r_dst
        }

        // ── Match — enum/option/result pattern lowering (EMIT-17, EMIT-23) ───
        TypedExpr::Match { .. } => super::patterns::emit_match(emitter, expr),

        // ── Lambda — closure/delegate lowering (EMIT-14) ─────────────────────
        TypedExpr::Lambda { ty, captures, .. } => {
            let closure_idx = emitter.lambda_ordinal(expr);
            super::closure::emit_lambda(emitter, captures, closure_idx, *ty)
        }

        // ── Object construction (EMIT-10, EMIT-11) ────────────────────────────
        TypedExpr::New {
            ty,
            target_def_id,
            fields,
            ..
        } => emit_new(emitter, *ty, *target_def_id, fields),
        TypedExpr::ArrayLit { ty, elements, .. } => emit_array_lit(emitter, *ty, elements),
        TypedExpr::Range {
            ty,
            start,
            end,
            inclusive,
            ..
        } => emit_range(emitter, *ty, start.as_deref(), end.as_deref(), *inclusive),
        // ── Spawn — SPAWN_TASK (EMIT-15) ──────────────────────────────────────
        TypedExpr::Spawn {
            ty, expr: inner, ..
        } => emit_spawn(emitter, *ty, inner),
        // ── Join — JOIN (EMIT-15) ──────────────────────────────────────────────
        TypedExpr::Join {
            ty, expr: inner, ..
        } => {
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
        TypedExpr::Defer { expr: inner, .. } => emit_defer(emitter, inner),
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
        | TyKind::Contract(def_id) => emitter
            .builder
            .token_for_def(*def_id)
            .map(|t| t.0)
            .unwrap_or(0),
        TyKind::Int => emitter.builder.type_ref_token_by_name("Int"),
        TyKind::Float => emitter.builder.type_ref_token_by_name("Float"),
        TyKind::Bool => emitter.builder.type_ref_token_by_name("Bool"),
        TyKind::String => emitter.builder.type_ref_token_by_name("String"),
        _ => 0, // Unsupported types get 0 (runtime handles gracefully)
    }
}

// ─── Tail-call emission (EMIT-24) ────────────────────────────────────────────

/// Emit a TailCall instruction for a terminal dialogue transition.
///
/// Transition intent is preserved as `TypedStmt::Transition`; ordinary return-call
/// expressions use the normal Call + Ret path so defer ordering remains unchanged.
pub(crate) fn emit_tail_call(emitter: &mut BodyEmitter<'_>, call: &TypedExpr) -> u16 {
    let TypedExpr::Call {
        callee,
        args,
        callee_def_id,
        callee_has_receiver,
        ..
    } = call
    else {
        unreachable!("typed dialogue transition must contain a call")
    };
    let target = resolve_concrete_call_target(
        emitter,
        callee,
        callee.ty(),
        *callee_def_id,
        *callee_has_receiver,
    )
    .expect("checked dialogue transition has no concrete non-null method target");
    let (r_base, argc) = pack_concrete_call_args(emitter, callee, args, target)
        .expect("resolved instance transition must have a field receiver");

    emitter.emit(Instruction::TailCall {
        method_idx: target.token,
        r_base,
        argc,
    });

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
        TyKind::Struct(def_id)
        | TyKind::Class(def_id)
        | TyKind::Entity(def_id)
        | TyKind::Enum(def_id) => Some(*def_id),
        _ => None,
    }
}

/// A statically resolvable CALL target and its ABI receiver requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConcreteCallTarget {
    pub token: u32,
    pub prepend_receiver: bool,
}

/// Emit and pack the argument block for a statically resolved concrete call.
/// Instance methods receive `self` first; static qualified calls intentionally
/// do not evaluate or pass the qualifier as an implicit argument.
pub(crate) fn pack_concrete_call_args(
    emitter: &mut BodyEmitter<'_>,
    callee: &TypedExpr,
    args: &[TypedExpr],
    target: ConcreteCallTarget,
) -> Option<(u16, u16)> {
    let arg_regs: Vec<u16> = if target.prepend_receiver {
        let TypedExpr::Field { receiver, .. } = callee else {
            return None;
        };
        std::iter::once(emit_expr(emitter, receiver))
            .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
            .collect()
    } else {
        args.iter().map(|arg| emit_expr(emitter, arg)).collect()
    };
    let argc = arg_regs.len() as u16;
    Some((pack_args_consecutive(emitter, &arg_regs), argc))
}

/// Resolve only calls that can use the concrete CALL ABI. Extern, virtual,
/// and delegate calls return `None`, allowing tail/spawn lowering to decline
/// the optimization rather than emit a null target.
pub(crate) fn resolve_concrete_call_target(
    emitter: &BodyEmitter<'_>,
    callee: &TypedExpr,
    checked_func_ty: Ty,
    callee_def_id: Option<crate::resolve::def_map::DefId>,
    callee_has_receiver: Option<bool>,
) -> Option<ConcreteCallTarget> {
    use crate::emit::metadata::{MetadataToken, TableId};

    let declared_token = callee_def_id.and_then(|id| emitter.builder.token_for_def(id));
    if declared_token.is_some_and(|token| token.table() == TableId::ExternDef) {
        return None;
    }

    match callee {
        TypedExpr::Field {
            receiver, field, ..
        } => {
            let has_receiver = callee_has_receiver?;
            if !matches!(
                emitter.interner.kind(receiver.ty()),
                TyKind::Struct(_) | TyKind::Class(_) | TyKind::Entity(_) | TyKind::Enum(_)
            ) {
                return None;
            }
            if let Some(token) = declared_token.filter(|token| {
                !token.is_null()
                    && matches!(token.table(), TableId::MethodDef | TableId::MethodRef)
                    && emitter.builder.method_has_receiver(*token) == Some(has_receiver)
            }) {
                return Some(ConcreteCallTarget {
                    token: token.0,
                    prepend_receiver: has_receiver,
                });
            }
            let signature = crate::emit::type_sig::encode_method_sig_for_fn_ty(
                checked_func_ty,
                emitter.interner,
                &|def_id| {
                    emitter
                        .builder
                        .token_for_def(def_id)
                        .unwrap_or(MetadataToken::NULL)
                },
            )?;
            let receiver_ty = emitter.interner.resolve_infer(receiver.ty());
            let base_parent = extract_type_def_id(emitter, receiver_ty)
                .and_then(|def_id| emitter.builder.token_for_def(def_id));
            let exact_parent = emitter
                .builder
                .type_spec_token_for_encoded_ty(receiver_ty, emitter.interner)
                .or(base_parent)?;
            let token = emitter.builder.method_token_by_parent_name_and_signature(
                exact_parent,
                base_parent,
                field,
                &signature,
                has_receiver,
            )?;
            let token_metadata = MetadataToken(token);
            (!token_metadata.is_null()).then_some(ConcreteCallTarget {
                token,
                prepend_receiver: has_receiver,
            })
        }
        _ => {
            let token = declared_token?;
            (!token.is_null() && matches!(token.table(), TableId::MethodDef | TableId::MethodRef))
                .then_some(ConcreteCallTarget {
                    token: token.0,
                    prepend_receiver: false,
                })
        }
    }
}
