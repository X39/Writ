//! Position-to-node walking utilities for LSP query handlers.
//!
//! Provides byte-offset conversion and typed AST node walking used by hover,
//! goto-def, find-refs, completions, and signature help.

use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::resolve::def_map::{DefId, DefMap};
use writ_diagnostics::FileId;

/// Convert an LSP `Position` (0-based line, UTF-16 character offset) to a byte
/// offset in `source`.
///
/// LSP positions use UTF-16 code units for character offsets. This function
/// walks `source` character-by-character, counting newlines to find the right
/// line and then counting UTF-16 code units within that line.
///
/// Returns `None` if the position is past the end of the source or on a line
/// that is shorter than `pos.character`.
pub fn position_to_byte_offset(source: &str, pos: lsp_types::Position) -> Option<usize> {
    let mut current_line: u32 = 0;
    let mut iter = source.char_indices().peekable();

    // Skip past pos.line newlines
    while current_line < pos.line {
        match iter.next() {
            Some((_, '\n')) => current_line += 1,
            None => return None,
            _ => {}
        }
    }

    // Now advance pos.character UTF-16 code units within the current line
    let mut utf16_col: u32 = 0;
    while utf16_col < pos.character {
        match iter.next() {
            Some((_, '\n')) | None => return None, // past end of line
            Some((_, ch)) => utf16_col += ch.len_utf16() as u32,
        }
    }

    // The next item in the iterator gives us our byte offset
    match iter.peek() {
        Some(&(idx, _)) => Some(idx),
        None => Some(source.len()), // end of file
    }
}

/// Extract the `FileId` from any `TypedDecl` variant by looking up its `def_id` in `def_map`.
///
/// Every `TypedDecl` variant carries a `def_id` field. The corresponding `DefEntry` in the
/// `DefMap` stores which file the declaration originated from.
pub(super) fn decl_file_id(decl: &TypedDecl, def_map: &DefMap) -> FileId {
    let def_id = match decl {
        TypedDecl::Fn { def_id, .. }
        | TypedDecl::Struct { def_id, .. }
        | TypedDecl::Class { def_id, .. }
        | TypedDecl::Entity { def_id, .. }
        | TypedDecl::Enum { def_id, .. }
        | TypedDecl::Contract { def_id, .. }
        | TypedDecl::Impl { def_id, .. }
        | TypedDecl::Const { def_id, .. }
        | TypedDecl::Global { def_id, .. }
        | TypedDecl::Component { def_id, .. }
        | TypedDecl::ExternFn { def_id, .. }
        | TypedDecl::ExternComponent { def_id, .. }
        | TypedDecl::AttributeDef { def_id, .. } => *def_id,
    };
    def_map.get_entry(def_id).file_id
}

/// Walk a `TypedAst` to find the innermost `TypedExpr` whose span contains
/// `offset` (byte offset), restricted to declarations from `file_id`.
///
/// Searches all body-containing declarations: free functions, impl methods,
/// const values, and global values. Only declarations originating from `file_id`
/// are searched, preventing false matches from other files in a multi-file project.
///
/// Returns the narrowest-span expression that contains `offset`, or `None` if
/// no expression spans that position.
pub fn expr_at_offset(ast: &TypedAst, offset: usize, file_id: FileId) -> Option<&TypedExpr> {
    let mut best: Option<&TypedExpr> = None;

    for decl in &ast.decls {
        if decl_file_id(decl, &ast.def_map) != file_id {
            continue;
        }
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(e) = find_in_expr(body, offset) {
                    update_best(&mut best, e);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_method_id, body) in methods {
                    if let Some(e) = find_in_expr(body, offset) {
                        update_best(&mut best, e);
                    }
                }
            }
            TypedDecl::Const { value, .. } => {
                if let Some(e) = find_in_expr(value, offset) {
                    update_best(&mut best, e);
                }
            }
            TypedDecl::Global { value, .. } => {
                if let Some(e) = find_in_expr(value, offset) {
                    update_best(&mut best, e);
                }
            }
            // Struct, Entity, Enum, Contract, Component, ExternFn, etc. have no body
            _ => {}
        }
    }

    best
}

/// Update `best` with `candidate` if candidate has a narrower (smaller) span.
fn update_best<'a>(best: &mut Option<&'a TypedExpr>, candidate: &'a TypedExpr) {
    match best {
        None => *best = Some(candidate),
        Some(current) => {
            let cspan = current.span();
            let nspan = candidate.span();
            if (nspan.end - nspan.start) < (cspan.end - cspan.start) {
                *best = Some(candidate);
            }
        }
    }
}

/// Recursively search `expr` and its children for the narrowest expression
/// that contains `offset`.
fn find_in_expr(expr: &TypedExpr, offset: usize) -> Option<&TypedExpr> {
    let span = expr.span();
    if offset < span.start || offset >= span.end {
        return None;
    }

    // Try to find a narrower match among children
    let child_result = find_in_expr_children(expr, offset);
    Some(child_result.unwrap_or(expr))
}

/// Search the children of `expr` for a narrower containing expression.
fn find_in_expr_children(expr: &TypedExpr, offset: usize) -> Option<&TypedExpr> {
    match expr {
        TypedExpr::Call { callee, args, .. } => {
            find_in_expr(callee, offset)
                .or_else(|| args.iter().find_map(|a| find_in_expr(a, offset)))
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            find_in_expr(receiver, offset)
        }
        TypedExpr::Index { receiver, index, .. } => {
            find_in_expr(receiver, offset).or_else(|| find_in_expr(index, offset))
        }
        TypedExpr::Binary { left, right, .. } => {
            find_in_expr(left, offset).or_else(|| find_in_expr(right, offset))
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => find_in_expr(inner, offset),
        TypedExpr::Match { scrutinee, arms, .. } => {
            find_in_expr(scrutinee, offset).or_else(|| {
                arms.iter().find_map(|arm| find_in_expr(&arm.body, offset))
            })
        }
        TypedExpr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => find_in_expr(condition, offset)
            .or_else(|| find_in_expr(then_branch, offset))
            .or_else(|| else_branch.as_ref().and_then(|e| find_in_expr(e, offset))),
        TypedExpr::Block { stmts, tail, .. } => {
            find_in_stmts(stmts, offset)
                .or_else(|| tail.as_ref().and_then(|t| find_in_expr(t, offset)))
        }
        TypedExpr::Lambda { body, .. } => find_in_expr(body, offset),
        TypedExpr::Assign { target, value, .. } => {
            find_in_expr(target, offset).or_else(|| find_in_expr(value, offset))
        }
        TypedExpr::New { fields, .. } => {
            fields.iter().find_map(|(_, v)| find_in_expr(v, offset))
        }
        TypedExpr::ArrayLit { elements, .. } => {
            elements.iter().find_map(|e| find_in_expr(e, offset))
        }
        TypedExpr::Range { start, end, .. } => {
            start
                .as_ref()
                .and_then(|s| find_in_expr(s, offset))
                .or_else(|| end.as_ref().and_then(|e| find_in_expr(e, offset)))
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => find_in_expr(inner, offset),
        TypedExpr::Return { value, .. } => {
            value.as_ref().and_then(|v| find_in_expr(v, offset))
        }
        // Leaf nodes: Literal, Var, SelfRef, Path, Error — no children
        _ => None,
    }
}

/// Search a slice of statements for a narrower containing expression.
fn find_in_stmts(stmts: &[TypedStmt], offset: usize) -> Option<&TypedExpr> {
    stmts.iter().find_map(|stmt| find_in_stmt(stmt, offset))
}

/// Search a single statement for a containing expression.
fn find_in_stmt(stmt: &TypedStmt, offset: usize) -> Option<&TypedExpr> {
    match stmt {
        TypedStmt::Let { value, .. } => find_in_expr(value, offset),
        TypedStmt::Expr { expr, .. } => find_in_expr(expr, offset),
        TypedStmt::For { iterable, body, .. } => {
            find_in_expr(iterable, offset).or_else(|| find_in_stmts(body, offset))
        }
        TypedStmt::While { condition, body, .. } => {
            find_in_expr(condition, offset).or_else(|| find_in_stmts(body, offset))
        }
        TypedStmt::Atomic { body, .. } => find_in_stmts(body, offset),
        TypedStmt::Return { value, .. } => value.as_ref().and_then(|v| find_in_expr(v, offset)),
        TypedStmt::Break { value, .. } => value.as_ref().and_then(|v| find_in_expr(v, offset)),
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => None,
    }
}

/// Given a `TypedExpr`, extract the `DefId` it directly references (if any).
///
/// Used by goto-definition and find-references handlers to map a cursor
/// position to a top-level definition.
pub fn find_def_id_at_offset(expr: &TypedExpr, def_map: &DefMap) -> Option<DefId> {
    match expr {
        TypedExpr::Call {
            callee_def_id: Some(id),
            ..
        } => Some(*id),
        TypedExpr::Var { name, .. } => {
            // Check public definitions by FQN match on simple name
            let by_fqn = def_map
                .by_fqn
                .values()
                .find(|&&id| def_map.get_entry(id).name == *name)
                .copied();

            by_fqn.or_else(|| {
                // Check file-private definitions
                for privs in def_map.file_private.values() {
                    if let Some(&id) = privs.get(name.as_str()) {
                        return Some(id);
                    }
                }
                None
            })
        }
        TypedExpr::New { target_def_id, .. } => Some(*target_def_id),
        TypedExpr::Path { segments, .. } => {
            let fqn = segments.join("::");
            def_map.get(&fqn)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{expr_at_offset, position_to_byte_offset};
    use lsp_types::Position;
    use writ_compiler::check::ir::{TypedAst, TypedExpr};
    use writ_diagnostics::FileId;

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
            &[],
        );
        let resolve_errors: Vec<_> = resolve_diags
            .iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .collect();
        assert!(resolve_errors.is_empty(), "resolve errors: {:?}", resolve_errors);

        let (typed_ast, _interner, _type_env, type_diags) =
            writ_compiler::check::typecheck(resolved, &[(file_id, &ast)], &[]);
        let type_errors: Vec<_> = type_diags
            .iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .collect();
        assert!(type_errors.is_empty(), "type errors: {:?}", type_errors);

        typed_ast
    }

    // ── position_to_byte_offset tests ─────────────────────────────────────────

    #[test]
    fn test_position_to_byte_offset_start() {
        let src = "hello world";
        let pos = Position { line: 0, character: 0 };
        assert_eq!(position_to_byte_offset(src, pos), Some(0));
    }

    #[test]
    fn test_position_to_byte_offset_second_line() {
        let src = "hello\nworld";
        let pos = Position { line: 1, character: 0 };
        // "hello\n" is 6 bytes; the 'w' of "world" is at byte 6
        assert_eq!(position_to_byte_offset(src, pos), Some(6));
    }

    #[test]
    fn test_position_to_byte_offset_utf16() {
        // U+1F600 (GRINNING FACE) encodes as 4 bytes in UTF-8 but 2 UTF-16 code units.
        let src = "\u{1F600}x";
        // character=2 should land on 'x', which starts at byte 4
        let pos = Position { line: 0, character: 2 };
        assert_eq!(position_to_byte_offset(src, pos), Some(4));
    }

    #[test]
    fn test_position_to_byte_offset_out_of_bounds() {
        let src = "hi";
        // Line 1 doesn't exist
        let pos = Position { line: 1, character: 0 };
        assert_eq!(position_to_byte_offset(src, pos), None);
    }

    // ── expr_at_offset / find_def_id_at_offset tests ─────────────────────────

    #[test]
    fn test_expr_at_offset_finds_var() {
        let src = "fn foo() -> int { 1 } fn main() -> int { let x: int = foo(); x }";
        let ast = build_typed_ast(src);

        // Find "x" in the tail expression (last 'x' in the source)
        let x_offset = src.rfind('x').unwrap();
        let expr = expr_at_offset(&ast, x_offset, FileId(0));
        assert!(expr.is_some(), "expected to find expression at 'x'");
    }

    #[test]
    fn test_expr_at_offset_finds_call() {
        let src = "fn foo() -> int { 1 } fn main() -> int { foo() }";
        let ast = build_typed_ast(src);

        // Find "foo" in the call expression (inside main)
        let foo_call_offset = src.rfind("foo").unwrap();
        let expr = expr_at_offset(&ast, foo_call_offset, FileId(0));
        assert!(expr.is_some(), "expected to find expression at 'foo' call");
        // Should be either the Call node or the Var inside the callee
        match expr.unwrap() {
            TypedExpr::Call { .. } | TypedExpr::Var { .. } => {}
            other => panic!("expected Call or Var, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_at_offset_in_impl_method() {
        // Struct with impl block containing a method
        let src = "struct Foo {} impl Foo { fn bar(self) -> int { 42 } }";
        let ast = build_typed_ast(src);

        // Find the literal '42' inside the impl method body
        let lit_offset = src.find("42").unwrap();
        let expr = expr_at_offset(&ast, lit_offset, FileId(0));
        assert!(expr.is_some(), "expected to find expression at '42' in impl method");
    }

    #[test]
    fn test_expr_at_offset_receiver_for_dot_completion() {
        // Simulate: user has "p." -- we strip the dot to get modified source
        // Then look up the receiver expression at the position just before where the dot was.
        let src = "pub struct Point { x: int, y: int }\nfn main() { let p: Point = new Point { x: 1, y: 2 }; p }";
        let ast = build_typed_ast(src);

        // The final "p" in the source -- its offset is just before " }" at end
        // Find the offset of the last standalone "p" in the source
        // The pattern "; p " gives us the semicolon before the tail "p"
        let p_offset = src.rfind("; p }").unwrap() + 2; // byte offset of 'p' (skip "; ")
        let expr = expr_at_offset(&ast, p_offset, FileId(0));
        assert!(expr.is_some(), "should find p expression at offset {}", p_offset);
        // Verify it's the Var("p") node
        match expr.unwrap() {
            TypedExpr::Var { name, .. } => assert_eq!(name.as_str(), "p"),
            other => panic!("expected Var(p), got {:?}", other),
        }
    }
}
