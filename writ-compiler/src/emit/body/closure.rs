//! Closure/delegate emission for IL method bodies.
//!
//! Handles:
//! - `pre_scan_lambdas()`: walks all method bodies for Lambda nodes and registers
//!   synthetic TypeDefs (capture structs) + MethodDefs (closure bodies) BEFORE finalize().
//! - `emit_lambda()`: emits NEW(capture_struct) + SET_FIELD per capture + NEW_DELEGATE at lambda site.
//!
//! **Critical ordering (Pitfall 2 from RESEARCH.md):**
//! `pre_scan_lambdas()` MUST be called BEFORE `builder.finalize()`.

use std::collections::HashSet;

use crate::check::ir::{Capture, TypedAst, TypedDecl, TypedExpr};
use crate::check::ty::{Ty, TyInterner};
use crate::emit::metadata::{HookKind, MetadataToken, TypeDefKind, method_flags};
use crate::emit::module_builder::ModuleBuilder;
use crate::resolve::def_map::DefId;

use super::BodyEmitter;
use writ_module::instruction::Instruction;

/// Information about a lambda discovered during pre-scan.
pub struct LambdaInfo {
    /// Unique index for this lambda within the module.
    pub closure_idx: usize,
    /// Captures: (field_name, capture_ty) pairs registered in the capture struct.
    pub captures_info: Vec<(String, Ty)>,
}

/// Pre-scan all method bodies in the TypedAst for Lambda nodes.
///
/// For each Lambda found:
/// - Register a synthetic TypeDef (the capture struct) with one FieldDef per capture.
/// - Register a synthetic MethodDef (the closure body method).
///
/// **MUST be called BEFORE builder.finalize().**
///
/// Returns a Vec<LambdaInfo> recording the metadata for each lambda in discovery order.
/// The order determines which builder token corresponds to which lambda.
pub fn pre_scan_lambdas(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
) -> Vec<LambdaInfo> {
    pre_scan_lambdas_excluding(typed_ast, interner, builder, &HashSet::new())
}

/// Pre-scan executable bodies while omitting conditionally suppressed functions.
pub(in crate::emit) fn pre_scan_lambdas_excluding(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    skipped_def_ids: &HashSet<DefId>,
) -> Vec<LambdaInfo> {
    let mut infos = Vec::new();
    let mut counter = 0usize;

    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { def_id, body, .. } if !skipped_def_ids.contains(def_id) => {
                scan_expr_for_lambdas(body, interner, builder, &mut counter, &mut infos);
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    scan_expr_for_lambdas(body, interner, builder, &mut counter, &mut infos);
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                scan_expr_for_lambdas(value, interner, builder, &mut counter, &mut infos);
            }
            _ => {}
        }
    }

    infos
}

/// Recursively scan a TypedExpr for Lambda nodes and register synthetic metadata.
fn scan_expr_for_lambdas(
    expr: &TypedExpr,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    counter: &mut usize,
    infos: &mut Vec<LambdaInfo>,
) {
    match expr {
        TypedExpr::Lambda {
            params,
            ret_ty,
            captures,
            body,
            ..
        } => {
            let idx = *counter;
            *counter += 1;

            // Register capture struct TypeDef as Class (kind=4).
            // Closure capture environments are heap-allocated reference objects,
            // so TypeDefKind::Class is semantically correct (not Struct/kind=0).
            let type_handle = builder.add_typedef(
                &format!("__closure_{}", idx),
                "", // no namespace
                TypeDefKind::Class,
                0,
                None, // synthetic — no source DefId
            );

            let def_tokens = builder.def_token_map.clone();
            let token_for_def = |def_id: DefId| {
                def_tokens
                    .get(&def_id)
                    .copied()
                    .unwrap_or(MetadataToken::NULL)
            };

            // Register each capture as a FieldDef
            let captures_info: Vec<(String, Ty)> = captures
                .iter()
                .map(|cap| {
                    let type_bytes =
                        crate::emit::type_sig::encode_type_bytes(cap.ty, interner, &token_for_def);
                    let type_sig = builder.blob_heap.intern(&type_bytes);
                    builder.add_fielddef(type_handle, &cap.name, type_sig, 0);
                    (cap.name.clone(), cap.ty)
                })
                .collect();

            // A capturing closure body is an instance method: the delegate target
            // becomes r0. A zero-capture body is static, so its first explicit
            // parameter is r0. Signatures and ParamDefs exclude the receiver.
            let param_types: Vec<Ty> = params.iter().map(|(_, ty)| *ty).collect();
            let signature = crate::emit::type_sig::encode_method_sig(
                &param_types,
                *ret_ty,
                interner,
                &token_for_def,
                &mut builder.blob_heap,
            );
            let is_static = captures.is_empty();
            let regular_param_count = u16::try_from(params.len())
                .expect("lambda parameter count exceeds module format limits");
            let param_count = regular_param_count
                .checked_add(if is_static { 0 } else { 1 })
                .expect("lambda entry parameter count exceeds module format limits");
            let method_handle = builder.add_methoddef(
                Some(type_handle),
                &format!("__invoke_{}", idx),
                signature,
                method_flags(false, is_static, false, HookKind::None),
                None,
                param_count,
            );
            for (sequence, (name, ty)) in params.iter().enumerate() {
                let type_bytes =
                    crate::emit::type_sig::encode_type_bytes(*ty, interner, &token_for_def);
                let type_sig = builder.blob_heap.intern(&type_bytes);
                builder.add_paramdef(
                    method_handle,
                    name,
                    type_sig,
                    u16::try_from(sequence)
                        .expect("lambda parameter sequence exceeds module format limits"),
                );
            }

            infos.push(LambdaInfo {
                closure_idx: idx,
                captures_info,
            });

            // Recurse into lambda body
            scan_expr_for_lambdas(body, interner, builder, counter, infos);
        }

        // Recurse into all child expressions
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                scan_stmt_for_lambdas(stmt, interner, builder, counter, infos);
            }
            if let Some(t) = tail {
                scan_expr_for_lambdas(t, interner, builder, counter, infos);
            }
        }
        TypedExpr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            scan_expr_for_lambdas(condition, interner, builder, counter, infos);
            scan_expr_for_lambdas(then_branch, interner, builder, counter, infos);
            if let Some(e) = else_branch {
                scan_expr_for_lambdas(e, interner, builder, counter, infos);
            }
        }
        TypedExpr::Binary { left, right, .. } => {
            scan_expr_for_lambdas(left, interner, builder, counter, infos);
            scan_expr_for_lambdas(right, interner, builder, counter, infos);
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            scan_expr_for_lambdas(inner, interner, builder, counter, infos);
        }
        TypedExpr::Call { callee, args, .. } => {
            scan_expr_for_lambdas(callee, interner, builder, counter, infos);
            for arg in args {
                scan_expr_for_lambdas(arg, interner, builder, counter, infos);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            scan_expr_for_lambdas(receiver, interner, builder, counter, infos);
        }
        TypedExpr::Index {
            receiver, index, ..
        } => {
            scan_expr_for_lambdas(receiver, interner, builder, counter, infos);
            scan_expr_for_lambdas(index, interner, builder, counter, infos);
        }
        TypedExpr::Assign { target, value, .. } => {
            scan_expr_for_lambdas(target, interner, builder, counter, infos);
            scan_expr_for_lambdas(value, interner, builder, counter, infos);
        }
        TypedExpr::New { fields, .. } => {
            for (_, v) in fields {
                scan_expr_for_lambdas(v, interner, builder, counter, infos);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for e in elements {
                scan_expr_for_lambdas(e, interner, builder, counter, infos);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                scan_expr_for_lambdas(s, interner, builder, counter, infos);
            }
            if let Some(e) = end {
                scan_expr_for_lambdas(e, interner, builder, counter, infos);
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            scan_expr_for_lambdas(inner, interner, builder, counter, infos);
        }
        TypedExpr::Match {
            scrutinee, arms, ..
        } => {
            scan_expr_for_lambdas(scrutinee, interner, builder, counter, infos);
            for arm in arms {
                scan_expr_for_lambdas(&arm.body, interner, builder, counter, infos);
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr_for_lambdas(v, interner, builder, counter, infos);
            }
        }
        // Leaf nodes
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Error { .. }
        | TypedExpr::Crash { .. }
        | TypedExpr::TypeOf { .. } => {}
    }
}

/// Recursively scan a TypedStmt for Lambda nodes.
fn scan_stmt_for_lambdas(
    stmt: &crate::check::ir::TypedStmt,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    counter: &mut usize,
    infos: &mut Vec<LambdaInfo>,
) {
    use crate::check::ir::TypedStmt;
    match stmt {
        TypedStmt::Let { value, .. } => {
            scan_expr_for_lambdas(value, interner, builder, counter, infos)
        }
        TypedStmt::Expr { expr, .. } => {
            scan_expr_for_lambdas(expr, interner, builder, counter, infos)
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr_for_lambdas(v, interner, builder, counter, infos);
            }
        }
        TypedStmt::Transition { call, .. } => {
            scan_expr_for_lambdas(call, interner, builder, counter, infos);
        }
        TypedStmt::For { iterable, body, .. } => {
            scan_expr_for_lambdas(iterable, interner, builder, counter, infos);
            for s in body {
                scan_stmt_for_lambdas(s, interner, builder, counter, infos);
            }
        }
        TypedStmt::While {
            condition, body, ..
        } => {
            scan_expr_for_lambdas(condition, interner, builder, counter, infos);
            for s in body {
                scan_stmt_for_lambdas(s, interner, builder, counter, infos);
            }
        }
        TypedStmt::Atomic { body, .. } => {
            for s in body {
                scan_stmt_for_lambdas(s, interner, builder, counter, infos);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                scan_expr_for_lambdas(v, interner, builder, counter, infos);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}

/// Emit a lambda expression at the call site.
///
/// For zero-capture lambda:
///   LOAD_NULL r_null
///   NEW_DELEGATE r_delegate, closure_method_token, r_null
///
/// For capturing lambda:
///   NEW r_env, capture_struct_type_token
///   // For each capture:
///   MOV/GET r_captured, <local reg>
///   SET_FIELD r_env, capture_field_token, r_captured
///   NEW_DELEGATE r_delegate, closure_method_token, r_env
///
/// The lambda body is registered as a separate EmittedBody (via closure.rs pre_scan).
///
/// `closure_idx` is the module-wide discovery ordinal from `pre_scan_lambdas`.
pub fn emit_lambda(
    emitter: &mut BodyEmitter<'_>,
    captures: &[Capture],
    closure_idx: usize,
    ty: Ty,
) -> u16 {
    let closure_name = format!("__closure_{}", closure_idx);
    let invoke_name = format!("__invoke_{}", closure_idx);

    // Look up the capture struct type token
    let capture_type_token = emitter
        .builder
        .typedef_token_by_name(&closure_name)
        .unwrap_or(0);
    let invoke_method_token = emitter
        .builder
        .methoddef_token_by_name(&invoke_name)
        .unwrap_or(0);

    let r_dst = emitter.alloc_reg(ty);

    if captures.is_empty() {
        // Zero-capture: LOAD_NULL + NEW_DELEGATE
        // Void = Ty(4)
        let void_ty = crate::check::ty::Ty(4);
        let r_null = emitter.alloc_reg(void_ty);
        emitter.emit(Instruction::LoadNull { r_dst: r_null });
        emitter.emit(Instruction::NewDelegate {
            r_dst,
            method_idx: invoke_method_token,
            r_target: r_null,
        });
    } else {
        // Capturing: NEW(capture_struct) + SET_FIELD per capture + NEW_DELEGATE
        let r_env = emitter.alloc_reg(ty); // capture struct register (typed as closure ty)
        emitter.emit(Instruction::New {
            r_dst: r_env,
            type_idx: capture_type_token,
        });

        for cap in captures {
            // Load capture from local or use 0 if not found
            let r_cap = emitter.locals.get(&cap.name).copied().unwrap_or(0);
            // Look up field token
            let field_idx = emitter
                .builder
                .field_token_by_name_on_closure(&closure_name, &cap.name)
                .unwrap_or(0);
            emitter.emit(Instruction::SetField {
                r_obj: r_env,
                field_idx,
                r_val: r_cap,
            });
        }

        emitter.emit(Instruction::NewDelegate {
            r_dst,
            method_idx: invoke_method_token,
            r_target: r_env,
        });
    }

    r_dst
}
