//! AST walker for collecting called DefIds (dead-import elimination).

use std::collections::HashSet;

use rustc_hash::FxHashSet;

use crate::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use crate::check::ty::{Ty, TyInterner, TyKind};
use crate::resolve::def_map::DefId;

// =============================================================================
// Called-DefId collection (for dead-import elimination)
// =============================================================================

/// Walk the entire TypedAst and collect all DefIds referenced from Call expressions.
///
/// Used by `inject_log_extern_defs` and `inject_dialogue_extern_defs` to avoid
/// emitting ExternDef rows for builtin functions that are never called.
pub(super) fn collect_called_def_ids(
    typed_ast: &TypedAst,
    skipped_def_ids: &HashSet<DefId>,
) -> FxHashSet<DefId> {
    let mut ids = FxHashSet::default();
    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { def_id, body, .. } if !skipped_def_ids.contains(def_id) => {
                walk_expr(body, &mut ids)
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    walk_expr(body, &mut ids);
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                walk_expr(value, &mut ids);
            }
            _ => {}
        }
    }
    ids
}

/// Recursively walk a TypedExpr, collecting callee DefIds from Call nodes.
fn walk_expr(expr: &TypedExpr, ids: &mut FxHashSet<DefId>) {
    match expr {
        TypedExpr::Call {
            callee,
            args,
            callee_def_id,
            ..
        } => {
            if let Some(id) = callee_def_id {
                ids.insert(*id);
            }
            walk_expr(callee, ids);
            for arg in args {
                walk_expr(arg, ids);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            walk_expr(receiver, ids)
        }
        TypedExpr::Index {
            receiver, index, ..
        } => {
            walk_expr(receiver, ids);
            walk_expr(index, ids);
        }
        TypedExpr::Binary { left, right, .. } => {
            walk_expr(left, ids);
            walk_expr(right, ids);
        }
        TypedExpr::UnaryPrefix { expr, .. }
        | TypedExpr::Spawn { expr, .. }
        | TypedExpr::SpawnDetached { expr, .. }
        | TypedExpr::Join { expr, .. }
        | TypedExpr::Cancel { expr, .. }
        | TypedExpr::Defer { expr, .. } => walk_expr(expr, ids),
        TypedExpr::Match {
            scrutinee, arms, ..
        } => {
            walk_expr(scrutinee, ids);
            for arm in arms {
                walk_expr(&arm.body, ids);
            }
        }
        TypedExpr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr(condition, ids);
            walk_expr(then_branch, ids);
            if let Some(e) = else_branch {
                walk_expr(e, ids);
            }
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                walk_stmt(stmt, ids);
            }
            if let Some(t) = tail {
                walk_expr(t, ids);
            }
        }
        TypedExpr::Lambda { body, .. } => walk_expr(body, ids),
        TypedExpr::Assign { target, value, .. } => {
            walk_expr(target, ids);
            walk_expr(value, ids);
        }
        TypedExpr::New { fields, .. } => {
            for (_, val) in fields {
                walk_expr(val, ids);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for elem in elements {
                walk_expr(elem, ids);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                walk_expr(s, ids);
            }
            if let Some(e) = end {
                walk_expr(e, ids);
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, ids);
            }
        }
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Error { .. }
        | TypedExpr::Crash { .. }
        | TypedExpr::TypeOf { .. } => {}
    }
}

/// Recursively walk a TypedStmt, collecting callee DefIds from Call nodes.
fn walk_stmt(stmt: &TypedStmt, ids: &mut FxHashSet<DefId>) {
    match stmt {
        TypedStmt::Let { value, .. } | TypedStmt::Expr { expr: value, .. } => {
            walk_expr(value, ids);
        }
        TypedStmt::For { iterable, body, .. } => {
            walk_expr(iterable, ids);
            for s in body {
                walk_stmt(s, ids);
            }
        }
        TypedStmt::While {
            condition, body, ..
        } => {
            walk_expr(condition, ids);
            for s in body {
                walk_stmt(s, ids);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, ids);
            }
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, ids);
            }
        }
        TypedStmt::Transition { call, .. } => walk_expr(call, ids),
        TypedStmt::Atomic { body, .. } => {
            for s in body {
                walk_stmt(s, ids);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}

// =============================================================================
// Addressable generic type collection
// =============================================================================

/// Collect structurally generic types referenced by executable typed IR.
///
/// Body emission happens after metadata finalization and therefore cannot add a
/// TypeSpec on demand. This pre-scan gives every generic receiver type a stable
/// table-4 token before instructions are emitted.
pub(super) fn collect_addressable_generic_types(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    skipped_def_ids: &HashSet<DefId>,
) -> Vec<Ty> {
    let mut seen = FxHashSet::default();
    let mut types = Vec::new();
    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { def_id, body, .. } if !skipped_def_ids.contains(def_id) => {
                walk_expr_types(body, interner, &mut seen, &mut types)
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    walk_expr_types(body, interner, &mut seen, &mut types);
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                walk_expr_types(value, interner, &mut seen, &mut types);
            }
            _ => {}
        }
    }
    types
}

fn record_type(ty: Ty, interner: &TyInterner, seen: &mut FxHashSet<Ty>, types: &mut Vec<Ty>) {
    let ty = interner.resolve_infer(ty);
    if !seen.insert(ty) {
        return;
    }

    match interner.full_kind(ty) {
        TyKind::GenericInstance { args, .. } => {
            types.push(ty);
            for arg in args {
                record_type(*arg, interner, seen, types);
            }
        }
        TyKind::Array(element)
        | TyKind::Option(element)
        | TyKind::TaskHandle(element)
        | TyKind::ReflectionType(element) => record_type(*element, interner, seen, types),
        TyKind::Result(ok, err) => {
            record_type(*ok, interner, seen, types);
            record_type(*err, interner, seen, types);
        }
        TyKind::Func { params, ret } => {
            for param in params {
                record_type(*param, interner, seen, types);
            }
            record_type(*ret, interner, seen, types);
        }
        _ => {}
    }
}

fn walk_expr_types(
    expr: &TypedExpr,
    interner: &TyInterner,
    seen: &mut FxHashSet<Ty>,
    types: &mut Vec<Ty>,
) {
    record_type(expr.ty(), interner, seen, types);
    match expr {
        TypedExpr::Call { callee, args, .. } => {
            walk_expr_types(callee, interner, seen, types);
            for arg in args {
                walk_expr_types(arg, interner, seen, types);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            walk_expr_types(receiver, interner, seen, types)
        }
        TypedExpr::Index {
            receiver, index, ..
        }
        | TypedExpr::Binary {
            left: receiver,
            right: index,
            ..
        } => {
            walk_expr_types(receiver, interner, seen, types);
            walk_expr_types(index, interner, seen, types);
        }
        TypedExpr::UnaryPrefix { expr, .. }
        | TypedExpr::Spawn { expr, .. }
        | TypedExpr::SpawnDetached { expr, .. }
        | TypedExpr::Join { expr, .. }
        | TypedExpr::Cancel { expr, .. }
        | TypedExpr::Defer { expr, .. } => walk_expr_types(expr, interner, seen, types),
        TypedExpr::Match {
            scrutinee, arms, ..
        } => {
            walk_expr_types(scrutinee, interner, seen, types);
            for arm in arms {
                walk_expr_types(&arm.body, interner, seen, types);
            }
        }
        TypedExpr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr_types(condition, interner, seen, types);
            walk_expr_types(then_branch, interner, seen, types);
            if let Some(branch) = else_branch {
                walk_expr_types(branch, interner, seen, types);
            }
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                walk_stmt_types(stmt, interner, seen, types);
            }
            if let Some(tail) = tail {
                walk_expr_types(tail, interner, seen, types);
            }
        }
        TypedExpr::Lambda {
            params,
            ret_ty,
            captures,
            body,
            ..
        } => {
            for (_, ty) in params {
                record_type(*ty, interner, seen, types);
            }
            record_type(*ret_ty, interner, seen, types);
            for capture in captures {
                record_type(capture.ty, interner, seen, types);
            }
            walk_expr_types(body, interner, seen, types);
        }
        TypedExpr::Assign { target, value, .. } => {
            walk_expr_types(target, interner, seen, types);
            walk_expr_types(value, interner, seen, types);
        }
        TypedExpr::New { fields, .. } => {
            for (_, value) in fields {
                walk_expr_types(value, interner, seen, types);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for element in elements {
                walk_expr_types(element, interner, seen, types);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                walk_expr_types(start, interner, seen, types);
            }
            if let Some(end) = end {
                walk_expr_types(end, interner, seen, types);
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(value) = value {
                walk_expr_types(value, interner, seen, types);
            }
        }
        TypedExpr::TypeOf { static_ty, .. } => record_type(*static_ty, interner, seen, types),
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Error { .. }
        | TypedExpr::Crash { .. } => {}
    }
}

fn walk_stmt_types(
    stmt: &TypedStmt,
    interner: &TyInterner,
    seen: &mut FxHashSet<Ty>,
    types: &mut Vec<Ty>,
) {
    match stmt {
        TypedStmt::Let { ty, value, .. } => {
            record_type(*ty, interner, seen, types);
            walk_expr_types(value, interner, seen, types);
        }
        TypedStmt::Expr { expr, .. } => walk_expr_types(expr, interner, seen, types),
        TypedStmt::For {
            binding_ty,
            iterable,
            body,
            ..
        } => {
            record_type(*binding_ty, interner, seen, types);
            walk_expr_types(iterable, interner, seen, types);
            for stmt in body {
                walk_stmt_types(stmt, interner, seen, types);
            }
        }
        TypedStmt::While {
            condition, body, ..
        } => {
            walk_expr_types(condition, interner, seen, types);
            for stmt in body {
                walk_stmt_types(stmt, interner, seen, types);
            }
        }
        TypedStmt::Break { value, .. } | TypedStmt::Return { value, .. } => {
            if let Some(value) = value {
                walk_expr_types(value, interner, seen, types);
            }
        }
        TypedStmt::Transition { call, .. } => {
            walk_expr_types(call, interner, seen, types);
        }
        TypedStmt::Atomic { body, .. } => {
            for stmt in body {
                walk_stmt_types(stmt, interner, seen, types);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}
