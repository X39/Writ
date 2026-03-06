//! AST walker for collecting called DefIds (dead-import elimination).

use rustc_hash::FxHashSet;

use crate::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use crate::resolve::def_map::DefId;

// =============================================================================
// Called-DefId collection (for dead-import elimination)
// =============================================================================

/// Walk the entire TypedAst and collect all DefIds referenced from Call expressions.
///
/// Used by `inject_log_extern_defs` and `inject_dialogue_extern_defs` to avoid
/// emitting ExternDef rows for builtin functions that are never called.
pub(super) fn collect_called_def_ids(typed_ast: &TypedAst) -> FxHashSet<DefId> {
    let mut ids = FxHashSet::default();
    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => walk_expr(body, &mut ids),
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
        TypedExpr::Call { callee, args, callee_def_id, .. } => {
            if let Some(id) = callee_def_id {
                ids.insert(*id);
            }
            walk_expr(callee, ids);
            for arg in args {
                walk_expr(arg, ids);
            }
        }
        TypedExpr::Field { receiver, .. }
        | TypedExpr::ComponentAccess { receiver, .. } => walk_expr(receiver, ids),
        TypedExpr::Index { receiver, index, .. } => {
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
        TypedExpr::Match { scrutinee, arms, .. } => {
            walk_expr(scrutinee, ids);
            for arm in arms {
                walk_expr(&arm.body, ids);
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
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
            if let Some(s) = start { walk_expr(s, ids); }
            if let Some(e) = end { walk_expr(e, ids); }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value { walk_expr(v, ids); }
        }
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Error { .. }
        | TypedExpr::Crash { .. } => {}
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
            for s in body { walk_stmt(s, ids); }
        }
        TypedStmt::While { condition, body, .. } => {
            walk_expr(condition, ids);
            for s in body { walk_stmt(s, ids); }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value { walk_expr(v, ids); }
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value { walk_expr(v, ids); }
        }
        TypedStmt::Atomic { body, .. } => {
            for s in body { walk_stmt(s, ids); }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}
