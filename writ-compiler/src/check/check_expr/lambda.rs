//! Lambda / closure type checking.

use chumsky::span::SimpleSpan;
use std::collections::HashSet;

use crate::ast::expr::AstLambdaParam;
use crate::ast::types::AstType;
use super::CheckCtx;
use super::check_block_stmts;
use super::super::env::Mutability;
use super::super::ir::{Capture, CaptureMode, TypedExpr, TypedStmt};
use super::super::ty::TyKind;

pub(super) fn check_lambda(
    ctx: &mut CheckCtx,
    params: &[AstLambdaParam],
    return_type: Option<&AstType>,
    body: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    let generic_map = rustc_hash::FxHashMap::default();

    // Resolve parameter types
    let mut param_tys = Vec::new();
    let mut param_names = Vec::new();
    for p in params {
        let ty = if let Some(ref annotation) = p.ty {
            super::super::env::resolve_ast_type_with_file(annotation, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file)
        } else {
            // No annotation: create an inference variable
            let var = ctx.unify.new_var();
            ctx.interner.intern(TyKind::Infer(var))
        };
        param_tys.push(ty);
        param_names.push(p.name.clone());
    }

    // Resolve return type
    let ret_ty = if let Some(rt) = return_type {
        super::super::env::resolve_ast_type_with_file(rt, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file)
    } else {
        ctx.interner.void()
    };

    // Set up context for body checking
    let old_ret = ctx.current_fn_ret;
    ctx.current_fn_ret = Some(ret_ty);
    ctx.local_env.push_scope();

    // Define params in scope
    for (name, ty) in param_names.iter().zip(param_tys.iter()) {
        ctx.local_env.define(
            name.clone(),
            *ty,
            Mutability::Immutable,
            span,
        );
    }

    // Check body
    let typed_body = check_block_stmts(ctx, body, span);

    // Pop the lambda's param scope — params are now gone from local_env.
    // The enclosing function's scope is still visible.
    ctx.local_env.pop_scope();
    ctx.current_fn_ret = old_ret;

    // Build captures list by walking the typed body.
    // After pop_scope(), lambda params are gone from local_env, so any name found
    // in local_env that is NOT a lambda param must be from an enclosing scope.
    let param_set: HashSet<&str> = param_names.iter().map(|s| s.as_str()).collect();
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut captures: Vec<Capture> = Vec::new();
    collect_var_refs(&typed_body, &mut seen_names, &mut captures, &param_set, &ctx.local_env);

    // Build function type
    let func_ty = ctx.interner.func(param_tys.clone(), ret_ty);

    let typed_params: Vec<(String, super::super::ty::Ty)> = param_names
        .into_iter()
        .zip(param_tys)
        .collect();

    TypedExpr::Lambda {
        ty: func_ty,
        span,
        params: typed_params,
        ret_ty,
        captures,
        body: Box::new(typed_body),
    }
}

/// Walk a TypedExpr recursively and collect variable references that are captures
/// (i.e., present in the enclosing scope's local_env but not lambda parameters).
///
/// After the lambda's scope is popped, any name found in `local_env` is from an
/// enclosing function scope, making it a closure capture.
fn collect_var_refs(
    expr: &TypedExpr,
    seen: &mut HashSet<String>,
    captures: &mut Vec<Capture>,
    param_set: &HashSet<&str>,
    local_env: &super::super::env::LocalEnv,
) {
    match expr {
        TypedExpr::Var { name, ty, span } => {
            // Skip lambda params
            if param_set.contains(name.as_str()) {
                return;
            }
            // Skip already-seen names (dedup)
            if seen.contains(name) {
                return;
            }
            // If the name is found in the (now param-free) local_env, it's a capture
            if local_env.lookup(name).is_some() {
                seen.insert(name.clone());
                captures.push(Capture {
                    name: name.clone(),
                    ty: *ty,
                    mode: CaptureMode::ByValue,
                    binding_span: *span,
                });
            }
            // If not in local_env: it's a global/const/function — do not capture
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                collect_var_refs_stmt(stmt, seen, captures, param_set, local_env);
            }
            if let Some(t) = tail {
                collect_var_refs(t, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            collect_var_refs(condition, seen, captures, param_set, local_env);
            collect_var_refs(then_branch, seen, captures, param_set, local_env);
            if let Some(e) = else_branch {
                collect_var_refs(e, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::Binary { left, right, .. } => {
            collect_var_refs(left, seen, captures, param_set, local_env);
            collect_var_refs(right, seen, captures, param_set, local_env);
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            collect_var_refs(inner, seen, captures, param_set, local_env);
        }
        TypedExpr::Call { callee, args, .. } => {
            collect_var_refs(callee, seen, captures, param_set, local_env);
            for arg in args {
                collect_var_refs(arg, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            collect_var_refs(receiver, seen, captures, param_set, local_env);
        }
        TypedExpr::Index { receiver, index, .. } => {
            collect_var_refs(receiver, seen, captures, param_set, local_env);
            collect_var_refs(index, seen, captures, param_set, local_env);
        }
        TypedExpr::Assign { target, value, .. } => {
            collect_var_refs(target, seen, captures, param_set, local_env);
            collect_var_refs(value, seen, captures, param_set, local_env);
        }
        TypedExpr::New { fields, .. } => {
            for (_, v) in fields {
                collect_var_refs(v, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for e in elements {
                collect_var_refs(e, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                collect_var_refs(s, seen, captures, param_set, local_env);
            }
            if let Some(e) = end {
                collect_var_refs(e, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            collect_var_refs(inner, seen, captures, param_set, local_env);
        }
        TypedExpr::Match { scrutinee, arms, .. } => {
            collect_var_refs(scrutinee, seen, captures, param_set, local_env);
            for arm in arms {
                collect_var_refs(&arm.body, seen, captures, param_set, local_env);
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                collect_var_refs(v, seen, captures, param_set, local_env);
            }
        }
        // Nested lambdas: do NOT recurse into them — their own captures are handled
        // when check_lambda is called recursively for the inner lambda. Walking into
        // an inner lambda body here would incorrectly treat inner lambda's params as
        // captures of this outer lambda.
        TypedExpr::Lambda { .. } => {}
        // Leaf nodes
        TypedExpr::Literal { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Crash { .. }
        | TypedExpr::Error { .. }
        | TypedExpr::TypeOf { .. } => {}
    }
}

/// Walk a TypedStmt recursively, collecting var refs for capture analysis.
fn collect_var_refs_stmt(
    stmt: &TypedStmt,
    seen: &mut HashSet<String>,
    captures: &mut Vec<Capture>,
    param_set: &HashSet<&str>,
    local_env: &super::super::env::LocalEnv,
) {
    match stmt {
        TypedStmt::Let { value, .. } => {
            collect_var_refs(value, seen, captures, param_set, local_env);
        }
        TypedStmt::Expr { expr, .. } => {
            collect_var_refs(expr, seen, captures, param_set, local_env);
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_var_refs(v, seen, captures, param_set, local_env);
            }
        }
        TypedStmt::For { iterable, body, .. } => {
            collect_var_refs(iterable, seen, captures, param_set, local_env);
            for s in body {
                collect_var_refs_stmt(s, seen, captures, param_set, local_env);
            }
        }
        TypedStmt::While { condition, body, .. } => {
            collect_var_refs(condition, seen, captures, param_set, local_env);
            for s in body {
                collect_var_refs_stmt(s, seen, captures, param_set, local_env);
            }
        }
        TypedStmt::Atomic { body, .. } => {
            for s in body {
                collect_var_refs_stmt(s, seen, captures, param_set, local_env);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                collect_var_refs(v, seen, captures, param_set, local_env);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}
