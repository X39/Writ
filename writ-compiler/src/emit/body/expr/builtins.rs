//! Built-in method shortcut emission for IL method bodies.
//!
//! Handles Option/Result/Array/String/numeric built-in method calls,
//! Entity namespace static methods (getOrCreate, findAll, destroy, isAlive),
//! and constructor patterns (Some, None, Ok, Err).

use writ_module::instruction::Instruction;

use crate::check::ir::TypedExpr;
use crate::check::ty::TyKind;

use super::super::BodyEmitter;
use super::{emit_expr, extract_type_def_id};

/// Check if a Call expression is a built-in Option/Result/Array/constructor method.
/// If so, emit the dedicated instruction and return Some(reg). Otherwise None.
///
/// Built-ins detected:
/// - Option: .is_none(), .unwrap(), .is_some()
/// - Result: .is_err(), .is_ok(), .unwrap_ok(), .unwrap_err(), .extract_err()
/// - Array: .len(), .slice(), .resize(), .copy_from()
/// - Constructor patterns: Some(val), None, Ok(val), Err(val) via Path callee
pub(super) fn try_emit_builtin_method(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> Option<u16> {
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
                        "unwrap" | "unwrap_ok" => {
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
                        "resize" if args.len() == 1 => {
                            let r_new_len = emit_expr(emitter, &args[0]);
                            emitter.emit(Instruction::ArrayResize { r_arr, r_new_len });
                            let r_dst = emitter.alloc_reg(ty);
                            return Some(r_dst);
                        }
                        "copy_from" if args.len() == 4 => {
                            // copy_from(src, src_idx, dst_idx, len)
                            // Receiver is DESTINATION. Per D-07: directionally unambiguous.
                            // IL operand order per D-03: r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len
                            let r_src_arr = emit_expr(emitter, &args[0]);
                            let r_src_idx = emit_expr(emitter, &args[1]);
                            let r_dst_idx = emit_expr(emitter, &args[2]);
                            let r_len_val = emit_expr(emitter, &args[3]);
                            emitter.emit(Instruction::ArrayCopy {
                                r_dst_arr: r_arr,
                                r_dst_idx,
                                r_src_arr,
                                r_src_idx,
                                r_len: r_len_val,
                            });
                            let r_dst = emitter.alloc_reg(ty);
                            return Some(r_dst);
                        }
                        _ => {}
                    }
                }
                TyKind::String => {
                    let r_src = emit_expr(emitter, receiver);
                    match field.as_str() {
                        "len" => {
                            // EMIT-20: string.len() -> STR_LEN
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrLen { r_dst, r_str: r_src });
                            return Some(r_dst);
                        }
                        "into_int" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::S2i { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "into_float" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::S2f { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "into_bool" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::S2b { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "into_string" => {
                            // string -> string is a no-op
                            return Some(r_src);
                        }
                        "trim" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrTrim { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "to_upper" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrToUpper { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "to_lower" => {
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrToLower { r_dst, r_src });
                            return Some(r_dst);
                        }
                        "starts_with" if args.len() == 1 => {
                            let r_prefix = emit_expr(emitter, &args[0]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrStartsWith { r_dst, r_str: r_src, r_prefix });
                            return Some(r_dst);
                        }
                        "ends_with" if args.len() == 1 => {
                            let r_suffix = emit_expr(emitter, &args[0]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrEndsWith { r_dst, r_str: r_src, r_suffix });
                            return Some(r_dst);
                        }
                        "contains" if args.len() == 1 => {
                            let r_sub = emit_expr(emitter, &args[0]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrContains { r_dst, r_str: r_src, r_sub });
                            return Some(r_dst);
                        }
                        "split" if args.len() == 1 => {
                            let r_sep = emit_expr(emitter, &args[0]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrSplit { r_dst, r_str: r_src, r_sep });
                            return Some(r_dst);
                        }
                        "replace" if args.len() == 2 => {
                            let r_from = emit_expr(emitter, &args[0]);
                            let r_to = emit_expr(emitter, &args[1]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::StrReplace { r_dst, r_str: r_src, r_from, r_to });
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
                    // EMIT-19: bool.into<String>() -> B2S
                    if field.as_str() == "into_string" {
                        let r_dst = emitter.alloc_reg(ty);
                        emitter.emit(Instruction::B2s { r_dst, r_src });
                        return Some(r_dst);
                    }
                }
                TyKind::AnyEntity => {
                    // Entity namespace static methods — no receiver to emit
                    // (Entity is a synthetic namespace, not a runtime value).
                    match field.as_str() {
                        "getOrCreate" => {
                            // Entity.getOrCreate<T>() → GET_OR_CREATE { r_dst, type_idx }
                            // The call's return type (ty) is the specific entity type.
                            let type_idx = extract_type_def_id(emitter, ty)
                                .and_then(|def_id| emitter.builder.token_for_def(def_id))
                                .map(|t| t.0)
                                .unwrap_or(0);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::GetOrCreate { r_dst, type_idx });
                            return Some(r_dst);
                        }
                        "findAll" => {
                            // Entity.findAll<T>() → FIND_ALL { r_dst, type_idx }
                            let type_idx = extract_type_def_id(emitter, ty)
                                .and_then(|def_id| emitter.builder.token_for_def(def_id))
                                .map(|t| t.0)
                                .unwrap_or(0);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::FindAll { r_dst, type_idx });
                            return Some(r_dst);
                        }
                        "destroy" if args.len() == 1 => {
                            // Entity.destroy(entity) → DESTROY_ENTITY { r_entity }
                            let r_entity = emit_expr(emitter, &args[0]);
                            emitter.emit(Instruction::DestroyEntity { r_entity });
                            let r_dst = emitter.alloc_reg(ty);
                            return Some(r_dst);
                        }
                        "isAlive" if args.len() == 1 => {
                            // Entity.isAlive(entity) → ENTITY_IS_ALIVE { r_dst, r_entity }
                            let r_entity = emit_expr(emitter, &args[0]);
                            let r_dst = emitter.alloc_reg(ty);
                            emitter.emit(Instruction::EntityIsAlive { r_dst, r_entity });
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
