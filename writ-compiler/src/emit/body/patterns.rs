//! Pattern matching emission for IL method bodies.
//!
//! Handles TypedExpr::Match lowering to IL instructions:
//! - Enum match: GET_TAG + SWITCH + per-variant arms
//! - Option/Result ?/try propagation: IS_NONE/IS_ERR + early return
//! - Literal/wildcard chains for non-enum matches

use writ_module::instruction::Instruction;

use crate::check::ir::{TypedExpr, TypedPattern};
use crate::check::ty::TyKind;

use super::BodyEmitter;
use super::expr::emit_expr;

/// Emit code for a TypedExpr::Match. Returns the destination register.
///
/// Dispatches based on scrutinee type:
/// - TyKind::Enum -> GET_TAG + SWITCH
/// - TyKind::Option -> IS_NONE + early return sequence (? propagation)
/// - TyKind::Result -> IS_ERR + early return sequence (try propagation)
/// - Other -> chain of comparisons
pub fn emit_match(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    let (ty, scrutinee, arms) = match expr {
        TypedExpr::Match { ty, scrutinee, arms, .. } => (*ty, scrutinee.as_ref(), arms),
        _ => panic!("emit_match called on non-Match expression"),
    };

    let r_scrutinee = emit_expr(emitter, scrutinee);
    let scrutinee_ty = scrutinee.ty();

    match emitter.interner.kind(scrutinee_ty).clone() {
        TyKind::Enum(_) => emit_enum_match(emitter, ty, r_scrutinee, arms),
        TyKind::Option(_) => emit_option_propagation(emitter, ty, r_scrutinee, arms),
        TyKind::Result(_, _) => emit_result_propagation(emitter, ty, r_scrutinee, arms),
        _ => emit_literal_match(emitter, ty, r_scrutinee, arms),
    }
}

// ─── Enum match ──────────────────────────────────────────────────────────────

fn emit_enum_match(
    emitter: &mut BodyEmitter<'_>,
    result_ty: crate::check::ty::Ty,
    r_scrutinee: u16,
    arms: &[crate::check::ir::TypedArm],
) -> u16 {
    // GET_TAG r_tag, r_enum
    let r_tag = emitter.alloc_reg(crate::check::ty::Ty(0)); // Int type for tag
    emitter.emit(Instruction::GetTag { r_dst: r_tag, r_enum: r_scrutinee });

    // Allocate result register
    let r_result = emitter.alloc_reg(result_ty);

    // Separate variant arms from wildcard
    let mut variant_arms: Vec<&crate::check::ir::TypedArm> = Vec::new();
    let mut wildcard_arm: Option<&crate::check::ir::TypedArm> = None;

    for arm in arms {
        match &arm.pattern {
            TypedPattern::EnumVariant { .. } => variant_arms.push(arm),
            TypedPattern::Wildcard { .. } | TypedPattern::Variable { .. } => {
                wildcard_arm = Some(arm);
            }
            _ => variant_arms.push(arm),
        }
    }

    // Create labels: one per variant arm + wildcard + end
    let n_variants = variant_arms.len();
    let arm_labels: Vec<_> = (0..n_variants).map(|_| emitter.new_label()).collect();
    let wildcard_label = emitter.new_label();
    let end_label = emitter.new_label();

    // Build offsets vector for SWITCH: for each tag index 0..n_variants, point to arm label.
    // If a variant has no explicit arm (shouldn't happen post-typecheck but handle it),
    // point to wildcard.
    let switch_offsets = arm_labels.iter().map(|_| 0i32).collect::<Vec<_>>();

    // Emit SWITCH with placeholder offsets — record fixup for each offset slot.
    // The switch instruction encodes as: opcode(2) + r_tag(2) + count(2) + offsets(4*n)
    // Each slot's fixup position = switch_start + 6 + slot_idx * 4
    let switch_idx = emitter.instructions.len();
    emitter.emit(Instruction::Switch {
        r_tag,
        offsets: switch_offsets,
    });

    // Emit each variant arm; arm labels are marked here (AFTER the SWITCH instruction).
    for (i, arm) in variant_arms.iter().enumerate() {
        emitter.mark_label_here(arm_labels[i]);

        // Handle bindings via EXTRACT_FIELD
        if let TypedPattern::EnumVariant { bindings, .. } = &arm.pattern {
            for (field_idx, binding) in bindings.iter().enumerate() {
                if let TypedPattern::Variable { name, ty, .. } = binding {
                    let r_field = emitter.alloc_reg(*ty);
                    emitter.emit(Instruction::ExtractField {
                        r_dst: r_field,
                        r_enum: r_scrutinee,
                        field_idx: field_idx as u16,
                    });
                    emitter.locals.insert(name.clone(), r_field);
                }
            }
        }

        // Emit arm body
        let r_arm = emit_expr(emitter, &arm.body);

        // MOV result
        emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_arm });

        // BR to end
        let br_idx = emitter.instructions.len();
        emitter.emit(Instruction::Br { offset: 0 });
        emitter.add_fixup(br_idx, end_label);
    }

    // Emit wildcard arm (if any)
    emitter.mark_label_here(wildcard_label);
    if let Some(arm) = wildcard_arm {
        // If wildcard has a variable binding, map it to scrutinee
        if let TypedPattern::Variable { name, .. } = &arm.pattern {
            emitter.locals.insert(name.clone(), r_scrutinee);
        }
        let r_arm = emit_expr(emitter, &arm.body);
        emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_arm });
    }

    // Patch SWITCH offsets now that all arm labels are marked.
    //
    // SWITCH has a variable-length `offsets: Vec<i32>` — one slot per variant.
    // The label allocator's add_fixup() handles single-offset branch instructions only,
    // so we directly patch the Switch instruction's offsets Vec instead.
    //
    // Each arm_labels[i] was marked with instruction index T_i via mark_label_here().
    // The Switch instruction is at switch_idx.
    // offset[i] = T_i - switch_idx  (relative instruction-index distance).
    {
        let mut patched_offsets: Vec<i32> = Vec::with_capacity(n_variants);
        for label in &arm_labels {
            let target_pos = emitter.labels.resolve(*label).unwrap_or(0);
            patched_offsets.push((target_pos as i64 - switch_idx as i64) as i32);
        }
        if let Instruction::Switch { offsets, .. } = &mut emitter.instructions[switch_idx] {
            *offsets = patched_offsets;
        }
    }

    // Mark end label
    emitter.mark_label_here(end_label);

    r_result
}

// ─── Option ?-propagation ─────────────────────────────────────────────────────

/// Detect if a match is an Option ?-propagation pattern:
/// exactly 2 arms where one arm's body is Return and one is Some(v) unwrap.
fn is_option_propagation(arms: &[crate::check::ir::TypedArm]) -> bool {
    if arms.len() != 2 {
        return false;
    }
    // Check if one arm has a Return body
    arms.iter().any(|arm| matches!(&arm.body, TypedExpr::Return { .. }))
        && arms.iter().any(|arm| {
            matches!(&arm.pattern, TypedPattern::EnumVariant { variant_name, .. }
                if variant_name == "Some" || variant_name == "Ok")
        })
}

/// Detect if a match is a Result try-propagation pattern.
fn is_result_propagation(arms: &[crate::check::ir::TypedArm]) -> bool {
    if arms.len() != 2 {
        return false;
    }
    arms.iter().any(|arm| matches!(&arm.body, TypedExpr::Return { .. }))
        && arms.iter().any(|arm| {
            matches!(&arm.pattern, TypedPattern::EnumVariant { variant_name, .. }
                if variant_name == "Err")
        })
}

fn emit_option_propagation(
    emitter: &mut BodyEmitter<'_>,
    result_ty: crate::check::ty::Ty,
    r_scrutinee: u16,
    arms: &[crate::check::ir::TypedArm],
) -> u16 {
    if is_option_propagation(arms) {
        // Optimized: IS_NONE + BR_FALSE + early_ret + UNWRAP
        let bool_ty = crate::check::ty::Ty(2);
        let r_is_none = emitter.alloc_reg(bool_ty);
        emitter.emit(Instruction::IsNone { r_dst: r_is_none, r_opt: r_scrutinee });

        let skip_label = emitter.new_label();
        let brf_idx = emitter.instructions.len();
        emitter.emit(Instruction::BrFalse { r_cond: r_is_none, offset: 0 });
        emitter.add_fixup(brf_idx, skip_label);

        // None branch: load null + ret
        let void_ty = crate::check::ty::Ty(4);
        let r_null = emitter.alloc_reg(void_ty);
        emitter.emit(Instruction::LoadNull { r_dst: r_null });
        emitter.emit(Instruction::Ret { r_src: r_null });

        emitter.mark_label_here(skip_label);

        // Unwrap the Some value
        let r_unwrapped = emitter.alloc_reg(result_ty);
        emitter.emit(Instruction::Unwrap { r_dst: r_unwrapped, r_opt: r_scrutinee });

        // Register the unwrapped value as the Some binding if present
        for arm in arms {
            if let TypedPattern::EnumVariant { variant_name, bindings, .. } = &arm.pattern {
                if variant_name == "Some" {
                    for binding in bindings {
                        if let TypedPattern::Variable { name, .. } = binding {
                            emitter.locals.insert(name.clone(), r_unwrapped);
                        }
                    }
                }
            }
        }

        r_unwrapped
    } else {
        // Fallback: treat as enum match
        emit_literal_match(emitter, result_ty, r_scrutinee, arms)
    }
}

fn emit_result_propagation(
    emitter: &mut BodyEmitter<'_>,
    result_ty: crate::check::ty::Ty,
    r_scrutinee: u16,
    arms: &[crate::check::ir::TypedArm],
) -> u16 {
    if is_result_propagation(arms) {
        // Optimized: IS_ERR + BR_FALSE + EXTRACT_ERR + WRAP_ERR + RET + UNWRAP_OK
        let bool_ty = crate::check::ty::Ty(2);
        let r_is_err = emitter.alloc_reg(bool_ty);
        emitter.emit(Instruction::IsErr { r_dst: r_is_err, r_result: r_scrutinee });

        let skip_label = emitter.new_label();
        let brf_idx = emitter.instructions.len();
        emitter.emit(Instruction::BrFalse { r_cond: r_is_err, offset: 0 });
        emitter.add_fixup(brf_idx, skip_label);

        // Err branch: extract err + wrap + ret
        // Find the Err arm's binding type for allocating the right register type
        let err_ty = result_ty; // fallback; ideally get from Err binding
        let r_err_val = emitter.alloc_reg(err_ty);
        emitter.emit(Instruction::ExtractErr { r_dst: r_err_val, r_result: r_scrutinee });

        let r_wrapped = emitter.alloc_reg(result_ty);
        emitter.emit(Instruction::WrapErr { r_dst: r_wrapped, r_err: r_err_val });
        emitter.emit(Instruction::Ret { r_src: r_wrapped });

        emitter.mark_label_here(skip_label);

        // Unwrap the Ok value
        let r_ok = emitter.alloc_reg(result_ty);
        emitter.emit(Instruction::UnwrapOk { r_dst: r_ok, r_result: r_scrutinee });

        // Register Ok binding if present
        for arm in arms {
            if let TypedPattern::EnumVariant { variant_name, bindings, .. } = &arm.pattern {
                if variant_name == "Ok" {
                    for binding in bindings {
                        if let TypedPattern::Variable { name, .. } = binding {
                            emitter.locals.insert(name.clone(), r_ok);
                        }
                    }
                }
            }
        }

        r_ok
    } else {
        emit_literal_match(emitter, result_ty, r_scrutinee, arms)
    }
}

// ─── Literal/wildcard chain ───────────────────────────────────────────────────

fn emit_literal_match(
    emitter: &mut BodyEmitter<'_>,
    result_ty: crate::check::ty::Ty,
    r_scrutinee: u16,
    arms: &[crate::check::ir::TypedArm],
) -> u16 {
    // For non-enum matches (int/string): emit CmpEq + BrFalse chain per arm.
    let r_result = emitter.alloc_reg(result_ty);
    let end_label = emitter.new_label();

    for arm in arms {
        match &arm.pattern {
            TypedPattern::Literal { value, .. } => {
                // Emit the literal to compare against
                let ty_int = crate::check::ty::Ty(0);
                let ty_bool = crate::check::ty::Ty(2);
                let r_lit = match value {
                    crate::check::ir::TypedLiteral::Int(v) => {
                        let r = emitter.alloc_reg(ty_int);
                        emitter.emit(Instruction::LoadInt { r_dst: r, value: *v });
                        r
                    }
                    crate::check::ir::TypedLiteral::Bool(true) => {
                        let r = emitter.alloc_reg(ty_bool);
                        emitter.emit(Instruction::LoadTrue { r_dst: r });
                        r
                    }
                    crate::check::ir::TypedLiteral::Bool(false) => {
                        let r = emitter.alloc_reg(ty_bool);
                        emitter.emit(Instruction::LoadFalse { r_dst: r });
                        r
                    }
                    _ => {
                        let r = emitter.alloc_reg(result_ty);
                        emitter.emit(Instruction::Nop);
                        r
                    }
                };

                let r_cmp = emitter.alloc_reg(ty_bool);
                emitter.emit(Instruction::CmpEqI { r_dst: r_cmp, r_a: r_scrutinee, r_b: r_lit });

                let next_label = emitter.new_label();
                let brf_idx = emitter.instructions.len();
                emitter.emit(Instruction::BrFalse { r_cond: r_cmp, offset: 0 });
                emitter.add_fixup(brf_idx, next_label);

                let r_arm = emit_expr(emitter, &arm.body);
                emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_arm });
                let br_idx = emitter.instructions.len();
                emitter.emit(Instruction::Br { offset: 0 });
                emitter.add_fixup(br_idx, end_label);

                emitter.mark_label_here(next_label);
            }
            TypedPattern::Wildcard { .. } | TypedPattern::Variable { .. } => {
                // Wildcard/variable — always matches; emit body
                if let TypedPattern::Variable { name, .. } = &arm.pattern {
                    emitter.locals.insert(name.clone(), r_scrutinee);
                }
                let r_arm = emit_expr(emitter, &arm.body);
                emitter.emit(Instruction::Mov { r_dst: r_result, r_src: r_arm });
                let br_idx = emitter.instructions.len();
                emitter.emit(Instruction::Br { offset: 0 });
                emitter.add_fixup(br_idx, end_label);
            }
            _ => {
                // Other patterns: emit Nop stub
                emitter.emit(Instruction::Nop);
            }
        }
    }

    emitter.mark_label_here(end_label);
    r_result
}
