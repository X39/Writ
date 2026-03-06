//! Reference collection and binding/def lookup query functions for LSP handlers.
//!
//! Provides find-references, goto-definition fallback, and local binding hover
//! query functions used by LSP reference, definition, and hover handlers.

use chumsky::span::SimpleSpan;
use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::check::ty::Ty;
use writ_compiler::resolve::def_map::{DefId, DefMap};
use writ_diagnostics::FileId;

use super::walk::decl_file_id;

/// Collect all use-site spans of a definition across the entire TypedAst.
///
/// Returns `SimpleSpan` values for each reference. The definition site
/// itself is NOT included (the caller adds it separately if
/// `params.context.include_declaration` is true).
pub fn collect_references(
    ast: &TypedAst,
    target_def_id: DefId,
    def_map: &DefMap,
) -> Vec<SimpleSpan> {
    let mut refs = Vec::new();
    for decl in &ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                collect_refs_in_expr(body, target_def_id, def_map, &mut refs);
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    collect_refs_in_expr(body, target_def_id, def_map, &mut refs);
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                collect_refs_in_expr(value, target_def_id, def_map, &mut refs);
            }
            _ => {}
        }
    }
    refs
}

/// Recursively walk an expression tree collecting spans where `target_def_id` is referenced.
fn collect_refs_in_expr(
    expr: &TypedExpr,
    target_def_id: DefId,
    def_map: &DefMap,
    refs: &mut Vec<SimpleSpan>,
) {
    match expr {
        TypedExpr::Var { name, span, .. } => {
            // Check if this variable name resolves to the target definition.
            let resolved = def_map
                .by_fqn
                .values()
                .any(|&id| id == target_def_id && def_map.get_entry(id).name == *name)
                || def_map
                    .file_private
                    .values()
                    .any(|privs| privs.get(name.as_str()).copied() == Some(target_def_id));
            if resolved {
                refs.push(*span);
            }
        }
        TypedExpr::Call { callee_def_id: Some(id), span, callee, args, .. } => {
            if *id == target_def_id {
                refs.push(*span);
            }
            // Also recurse into callee and args
            collect_refs_in_expr(callee, target_def_id, def_map, refs);
            for arg in args {
                collect_refs_in_expr(arg, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Call { callee, args, .. } => {
            collect_refs_in_expr(callee, target_def_id, def_map, refs);
            for arg in args {
                collect_refs_in_expr(arg, target_def_id, def_map, refs);
            }
        }
        TypedExpr::New { target_def_id: id, span, fields, .. } => {
            if *id == target_def_id {
                refs.push(*span);
            }
            for (_, val) in fields {
                collect_refs_in_expr(val, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Path { segments, span, .. } => {
            let fqn = segments.join("::");
            if def_map.get(&fqn) == Some(target_def_id) {
                refs.push(*span);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            collect_refs_in_expr(receiver, target_def_id, def_map, refs);
        }
        TypedExpr::Index { receiver, index, .. } => {
            collect_refs_in_expr(receiver, target_def_id, def_map, refs);
            collect_refs_in_expr(index, target_def_id, def_map, refs);
        }
        TypedExpr::Binary { left, right, .. } => {
            collect_refs_in_expr(left, target_def_id, def_map, refs);
            collect_refs_in_expr(right, target_def_id, def_map, refs);
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            collect_refs_in_expr(inner, target_def_id, def_map, refs);
        }
        TypedExpr::Match { scrutinee, arms, .. } => {
            collect_refs_in_expr(scrutinee, target_def_id, def_map, refs);
            for arm in arms {
                collect_refs_in_expr(&arm.body, target_def_id, def_map, refs);
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            collect_refs_in_expr(condition, target_def_id, def_map, refs);
            collect_refs_in_expr(then_branch, target_def_id, def_map, refs);
            if let Some(eb) = else_branch {
                collect_refs_in_expr(eb, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Block { stmts, tail, .. } => {
            collect_refs_in_stmts(stmts, target_def_id, def_map, refs);
            if let Some(t) = tail {
                collect_refs_in_expr(t, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Lambda { body, .. } => {
            collect_refs_in_expr(body, target_def_id, def_map, refs);
        }
        TypedExpr::Assign { target, value, .. } => {
            collect_refs_in_expr(target, target_def_id, def_map, refs);
            collect_refs_in_expr(value, target_def_id, def_map, refs);
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for elem in elements {
                collect_refs_in_expr(elem, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                collect_refs_in_expr(s, target_def_id, def_map, refs);
            }
            if let Some(e) = end {
                collect_refs_in_expr(e, target_def_id, def_map, refs);
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            collect_refs_in_expr(inner, target_def_id, def_map, refs);
        }
        TypedExpr::Return { value: Some(v), .. } => {
            collect_refs_in_expr(v, target_def_id, def_map, refs);
        }
        // Leaf nodes: Literal, SelfRef, Error — no sub-expressions
        _ => {}
    }
}

/// Recursively walk statements collecting reference spans.
fn collect_refs_in_stmts(
    stmts: &[TypedStmt],
    target_def_id: DefId,
    def_map: &DefMap,
    refs: &mut Vec<SimpleSpan>,
) {
    for stmt in stmts {
        collect_refs_in_stmt(stmt, target_def_id, def_map, refs);
    }
}

/// Collect reference spans from a single statement.
fn collect_refs_in_stmt(
    stmt: &TypedStmt,
    target_def_id: DefId,
    def_map: &DefMap,
    refs: &mut Vec<SimpleSpan>,
) {
    match stmt {
        TypedStmt::Let { value, .. } => {
            collect_refs_in_expr(value, target_def_id, def_map, refs);
        }
        TypedStmt::Expr { expr, .. } => {
            collect_refs_in_expr(expr, target_def_id, def_map, refs);
        }
        TypedStmt::For { iterable, body, .. } => {
            collect_refs_in_expr(iterable, target_def_id, def_map, refs);
            collect_refs_in_stmts(body, target_def_id, def_map, refs);
        }
        TypedStmt::While { condition, body, .. } => {
            collect_refs_in_expr(condition, target_def_id, def_map, refs);
            collect_refs_in_stmts(body, target_def_id, def_map, refs);
        }
        TypedStmt::Atomic { body, .. } => {
            collect_refs_in_stmts(body, target_def_id, def_map, refs);
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_refs_in_expr(v, target_def_id, def_map, refs);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                collect_refs_in_expr(v, target_def_id, def_map, refs);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}

// =============================================================================
// Binding, Def, and TypeAnn query functions (LSP hover/goto-def fallbacks)
// =============================================================================

/// Information about a local binding (let variable, for-loop binding, fn param)
/// found at a cursor offset.
pub struct BindingInfo {
    pub name: String,
    pub ty: Ty,
    pub name_span: SimpleSpan,
}

/// Find a local binding (let name, for binding, fn param) whose name_span contains `offset`.
///
/// Used as a fallback in hover when `expr_at_offset` returns a void block or None.
/// The `type_env` is used to look up parameter types by index for fn params.
/// Only declarations from `file_id` are searched.
pub fn binding_at_offset(
    ast: &TypedAst,
    offset: usize,
    type_env: &writ_compiler::check::env::TypeEnv,
    file_id: FileId,
) -> Option<BindingInfo> {
    for decl in &ast.decls {
        if decl_file_id(decl, &ast.def_map) != file_id {
            continue;
        }
        match decl {
            TypedDecl::Fn { def_id, body, param_name_spans } => {
                // Check fn param name spans
                if let Some(sig) = type_env.fn_sigs.get(def_id) {
                    for (i, (param_name, param_ty)) in sig.params.iter().enumerate() {
                        if let Some(span) = param_name_spans.get(i)
                            && offset >= span.start && offset < span.end {
                                return Some(BindingInfo {
                                    name: param_name.to_string(),
                                    ty: *param_ty,
                                    name_span: *span,
                                });
                            }
                    }
                }
                // Check body bindings
                if let Some(b) = find_binding_in_expr(body, offset) {
                    return Some(b);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(b) = find_binding_in_expr(body, offset) {
                        return Some(b);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn find_binding_in_expr(expr: &TypedExpr, offset: usize) -> Option<BindingInfo> {
    match expr {
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                if let Some(b) = find_binding_in_stmt(stmt, offset) {
                    return Some(b);
                }
            }
            tail.as_ref().and_then(|t| find_binding_in_expr(t, offset))
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            find_binding_in_expr(condition, offset)
                .or_else(|| find_binding_in_expr(then_branch, offset))
                .or_else(|| else_branch.as_ref().and_then(|e| find_binding_in_expr(e, offset)))
        }
        TypedExpr::Lambda { body, .. } => find_binding_in_expr(body, offset),
        TypedExpr::Match { scrutinee, arms, .. } => {
            find_binding_in_expr(scrutinee, offset)
                .or_else(|| arms.iter().find_map(|arm| find_binding_in_expr(&arm.body, offset)))
        }
        _ => None,
    }
}

fn find_binding_in_stmt(stmt: &TypedStmt, offset: usize) -> Option<BindingInfo> {
    match stmt {
        TypedStmt::Let { name, name_span, ty, value, .. } => {
            if offset >= name_span.start && offset < name_span.end {
                return Some(BindingInfo {
                    name: name.clone(),
                    ty: *ty,
                    name_span: *name_span,
                });
            }
            // Also recurse into the value expression
            find_binding_in_expr(value, offset)
        }
        TypedStmt::For { binding, binding_span, binding_ty, iterable, body, .. } => {
            if offset >= binding_span.start && offset < binding_span.end {
                return Some(BindingInfo {
                    name: binding.clone(),
                    ty: *binding_ty,
                    name_span: *binding_span,
                });
            }
            find_binding_in_expr(iterable, offset)
                .or_else(|| body.iter().find_map(|s| find_binding_in_stmt(s, offset)))
        }
        TypedStmt::While { condition, body, .. } => {
            find_binding_in_expr(condition, offset)
                .or_else(|| body.iter().find_map(|s| find_binding_in_stmt(s, offset)))
        }
        TypedStmt::Atomic { body, .. } => {
            body.iter().find_map(|s| find_binding_in_stmt(s, offset))
        }
        TypedStmt::Expr { expr, .. } => find_binding_in_expr(expr, offset),
        TypedStmt::Return { value, .. } | TypedStmt::Break { value, .. } => {
            value.as_ref().and_then(|v| find_binding_in_expr(v, offset))
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => None,
    }
}

/// Find the DefId of a top-level definition whose name span contains `offset`.
///
/// Used as a fallback in goto-definition and find-references when `expr_at_offset`
/// returns nothing (cursor is on a declaration name, not a use site).
/// Only returns definitions from `file_id` (which also implicitly skips synthetic
/// builtins at `FileId(u32::MAX)` since the trigger file will never match that).
pub fn def_at_offset(def_map: &DefMap, offset: usize, file_id: FileId) -> Option<DefId> {
    for (id, entry) in &def_map.arena {
        // Only match declarations from the trigger file
        if entry.file_id != file_id {
            continue;
        }
        let span = entry.name_span;
        if offset >= span.start && offset < span.end {
            return Some(id);
        }
    }
    None
}

/// Find the DefId of a type annotation at `offset` by walking TypedStmt::Let nodes.
///
/// Used as a fallback in goto-definition when the cursor is on a type annotation
/// (e.g., `MyStruct` in `let x: MyStruct = ...`).
/// Only declarations from `file_id` are searched.
pub fn type_ann_def_id_at_offset(ast: &TypedAst, offset: usize, file_id: FileId) -> Option<DefId> {
    for decl in &ast.decls {
        if decl_file_id(decl, &ast.def_map) != file_id {
            continue;
        }
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(id) = find_type_ann_in_expr(body, offset) {
                    return Some(id);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(id) = find_type_ann_in_expr(body, offset) {
                        return Some(id);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn find_type_ann_in_expr(expr: &TypedExpr, offset: usize) -> Option<DefId> {
    match expr {
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                if let Some(id) = find_type_ann_in_stmt(stmt, offset) {
                    return Some(id);
                }
            }
            tail.as_ref().and_then(|t| find_type_ann_in_expr(t, offset))
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            find_type_ann_in_expr(condition, offset)
                .or_else(|| find_type_ann_in_expr(then_branch, offset))
                .or_else(|| else_branch.as_ref().and_then(|e| find_type_ann_in_expr(e, offset)))
        }
        TypedExpr::Lambda { body, .. } => find_type_ann_in_expr(body, offset),
        TypedExpr::Match { scrutinee, arms, .. } => {
            find_type_ann_in_expr(scrutinee, offset)
                .or_else(|| arms.iter().find_map(|arm| find_type_ann_in_expr(&arm.body, offset)))
        }
        _ => None,
    }
}

fn find_type_ann_in_stmt(stmt: &TypedStmt, offset: usize) -> Option<DefId> {
    match stmt {
        TypedStmt::Let { type_ann_span, type_ann_def_id, value, .. } => {
            if let (Some(ann_span), Some(def_id)) = (type_ann_span, type_ann_def_id)
                && offset >= ann_span.start && offset < ann_span.end {
                    return Some(*def_id);
                }
            find_type_ann_in_expr(value, offset)
        }
        TypedStmt::Expr { expr, .. } => find_type_ann_in_expr(expr, offset),
        TypedStmt::For { iterable, body, .. } => {
            find_type_ann_in_expr(iterable, offset)
                .or_else(|| body.iter().find_map(|s| find_type_ann_in_stmt(s, offset)))
        }
        TypedStmt::While { condition, body, .. } => {
            find_type_ann_in_expr(condition, offset)
                .or_else(|| body.iter().find_map(|s| find_type_ann_in_stmt(s, offset)))
        }
        TypedStmt::Atomic { body, .. } => {
            body.iter().find_map(|s| find_type_ann_in_stmt(s, offset))
        }
        TypedStmt::Return { value, .. } | TypedStmt::Break { value, .. } => {
            value.as_ref().and_then(|v| find_type_ann_in_expr(v, offset))
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_references, type_ann_def_id_at_offset};
    use writ_compiler::check::ir::TypedAst;
    use writ_compiler::check::ty::TyInterner;
    use writ_diagnostics::{FileId, Severity};

    fn build_typed_ast(src: &str) -> TypedAst {
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let file_id = FileId(0);

        let (cst_opt, parse_errs) = writ_parser::parse(src_static);
        assert!(parse_errs.is_empty(), "parse errors: {:?}", parse_errs);
        let cst = cst_opt.expect("parse returned no output");

        let (ast, lower_errs) = writ_compiler::lower(cst);
        assert!(lower_errs.is_empty(), "lower errors: {:?}", lower_errs);

        let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
            &[(file_id, &ast)],
            &[(file_id, "test.writ")],
        );
        let resolve_errors: Vec<_> = resolve_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(resolve_errors.is_empty(), "resolve errors: {:?}", resolve_errors);

        let (typed_ast, _interner, _type_env, type_diags) =
            writ_compiler::check::typecheck(resolved, &[(file_id, &ast)]);
        let type_errors: Vec<_> = type_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(type_errors.is_empty(), "type errors: {:?}", type_errors);

        typed_ast
    }

    fn build_typed_ast_full(
        src: &str,
    ) -> (TypedAst, TyInterner, writ_compiler::check::env::TypeEnv) {
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let file_id = FileId(0);

        let (cst_opt, parse_errs) = writ_parser::parse(src_static);
        assert!(parse_errs.is_empty(), "parse errors: {:?}", parse_errs);
        let cst = cst_opt.expect("parse returned no output");

        let (ast, lower_errs) = writ_compiler::lower(cst);
        assert!(lower_errs.is_empty(), "lower errors: {:?}", lower_errs);

        let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
            &[(file_id, &ast)],
            &[(file_id, "test.writ")],
        );
        let resolve_errors: Vec<_> = resolve_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(resolve_errors.is_empty(), "resolve errors: {:?}", resolve_errors);

        let (typed_ast, interner, type_env, type_diags) =
            writ_compiler::check::typecheck(resolved, &[(file_id, &ast)]);
        let type_errors: Vec<_> = type_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(type_errors.is_empty(), "type errors: {:?}", type_errors);

        (typed_ast, interner, type_env)
    }

    // ── collect_references tests ──────────────────────────────────────────────

    #[test]
    fn test_collect_references_finds_uses() {
        // Define a function and call it twice; collect_references should find both calls.
        let src = "fn helper() -> int { 42 } fn main() -> int { let a: int = helper(); helper() }";
        let (ast, _interner, _type_env) = build_typed_ast_full(src);

        // Find the DefId for 'helper' (may be in by_fqn or file_private)
        let helper_id = ast.def_map
            .by_fqn
            .values()
            .chain(ast.def_map.file_private.values().flat_map(|m| m.values()))
            .find(|&&id| ast.def_map.get_entry(id).name == "helper")
            .copied()
            .expect("should find 'helper' def");

        let refs = collect_references(&ast, helper_id, &ast.def_map);
        // Expect two references (the two calls inside main)
        assert!(refs.len() >= 2, "expected at least 2 references, got {:?}", refs);
    }

    // ── type_ann_def_id_at_offset tests ──────────────────────────────────────

    #[test]
    fn test_type_ann_def_id_at_struct_annotation() {
        let src = "struct MyStruct {} fn main() { let x: MyStruct = new MyStruct {}; }";
        let ast = build_typed_ast(src);

        // Find the offset of "MyStruct" in the type annotation
        let ann_offset = src.find(": MyStruct").map(|i| i + 2).unwrap();
        let def_id = type_ann_def_id_at_offset(&ast, ann_offset, FileId(0));
        assert!(def_id.is_some(), "expected DefId at type annotation 'MyStruct', offset {}", ann_offset);
        let entry = ast.def_map.get_entry(def_id.unwrap());
        assert_eq!(entry.name, "MyStruct", "expected entry name 'MyStruct', got '{}'", entry.name);
    }

    #[test]
    fn test_type_ann_def_id_none_without_annotation() {
        // `let x = 42` has no type annotation — should return None
        let src = "fn main() { let x = 42; }";
        let ast = build_typed_ast(src);

        let x_offset = src.find("let x").map(|i| i + "let ".len()).unwrap();
        let def_id = type_ann_def_id_at_offset(&ast, x_offset, FileId(0));
        assert!(
            def_id.is_none(),
            "expected None for unannotated let binding, got Some"
        );
    }
}
