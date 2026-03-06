//! Hover text query functions for LSP hover handler.
//!
//! Provides formatted hover text for typed expressions and definitions,
//! doc comment extraction, and pattern hover info for match arms.

use chumsky::span::SimpleSpan;
use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::check::ty::{TyInterner};
use writ_compiler::resolve::def_map::DefMap;
use writ_diagnostics::FileId;

use super::walk::decl_file_id;

/// Build hover text for a TypedExpr.
///
/// Returns markdown-formatted text showing the type or signature.
/// `source` is the full source text used for doc comment and const value extraction.
/// `ast` is the typed AST used for const value lookup.
pub fn hover_text_for_expr(
    expr: &TypedExpr,
    def_map: &DefMap,
    interner: &TyInterner,
    type_env: &writ_compiler::check::env::TypeEnv,
    source: &str,
    ast: &TypedAst,
) -> String {
    use writ_compiler::check::ir::TypedDecl;
    match expr {
        TypedExpr::Var { name, ty, .. } => {
            use writ_compiler::resolve::def_map::DefKind;

            let ty_str = interner.display_named(*ty, def_map);

            // Look up a def with this name in public + file-private tables.
            let matching_def = def_map
                .by_fqn
                .values()
                .chain(
                    def_map
                        .file_private
                        .values()
                        .flat_map(|m| m.values()),
                )
                .find(|&&id| def_map.get_entry(id).name == *name)
                .copied();

            if let Some(def_id) = matching_def {
                let entry = def_map.get_entry(def_id);
                match entry.kind {
                    DefKind::Fn | DefKind::ExternFn => {
                        // Show fn signature with optional doc comment
                        if let Some(sig) = type_env.fn_sigs.get(&def_id) {
                            let sig_text = format_fn_sig_hover(sig, def_map, interner);
                            if entry.file_id != writ_diagnostics::FileId(u32::MAX)
                                && let Some(doc) = extract_doc_comment(source, entry.span.start) {
                                    return format!("{}\n\n{}", sig_text, doc);
                                }
                            return sig_text;
                        }
                    }
                    DefKind::Const => {
                        // Show const with value
                        let value_text = ast.decls.iter().find_map(|decl| {
                            if let TypedDecl::Const { def_id: did, value, .. } = decl {
                                if *did == def_id {
                                    let span = value.span();
                                    source.get(span.start..span.end).map(|s| s.trim().to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        });
                        if let Some(val) = value_text {
                            return format!("```writ\n{}: {} = {}\n```", name, ty_str, val);
                        }
                    }
                    _ => {}
                }
            }

            format!("```writ\n{}: {}\n```", name, ty_str)
        }
        TypedExpr::Call { callee_def_id: Some(def_id), ty, .. } => {
            // Show function signature if available, with optional doc comment
            if let Some(sig) = type_env.fn_sigs.get(def_id) {
                let sig_text = format_fn_sig_hover(sig, def_map, interner);
                let entry = def_map.get_entry(*def_id);
                // Only attempt doc extraction for real file entries (not builtins)
                if entry.file_id != writ_diagnostics::FileId(u32::MAX)
                    && let Some(doc) = extract_doc_comment(source, entry.span.start) {
                        return format!("{}\n\n{}", sig_text, doc);
                    }
                sig_text
            } else {
                let ty_str = interner.display_named(*ty, def_map);
                format!("```writ\n{}\n```", ty_str)
            }
        }
        TypedExpr::Field { field, ty, .. } => {
            let ty_str = interner.display_named(*ty, def_map);
            format!("```writ\n{}: {}\n```", field, ty_str)
        }
        TypedExpr::ComponentAccess { component, ty, .. } => {
            let ty_str = interner.display_named(*ty, def_map);
            format!("```writ\ncomponent {}: {}\n```", component, ty_str)
        }
        TypedExpr::New { target_def_id, .. } => {
            let entry = def_map.get_entry(*target_def_id);
            format!("```writ\nnew {}\n```", entry.name)
        }
        TypedExpr::SelfRef { ty, .. } => {
            let ty_str = interner.display_named(*ty, def_map);
            format!("```writ\nself: {}\n```", ty_str)
        }
        TypedExpr::Path { segments, ty, .. } => {
            let path = segments.join("::");
            let ty_str = interner.display_named(*ty, def_map);
            format!("```writ\n{}: {}\n```", path, ty_str)
        }
        TypedExpr::Literal { ty, .. } => {
            // Enum variant literals (from check_path) — show their type name.
            let ty_str = interner.display_named(*ty, def_map);
            if ty_str == "void" {
                String::new()
            } else {
                format!("```writ\n{}\n```", ty_str)
            }
        }
        TypedExpr::Error { .. } => String::new(), // No hover for error nodes
        _ => {
            // For all other expressions, show the inferred type.
            // Suppress hover for void-typed expressions (e.g., blocks, atomic).
            let ty_str = interner.display_named(expr.ty(), def_map);
            if ty_str == "void" {
                String::new()
            } else {
                format!("```writ\n{}\n```", ty_str)
            }
        }
    }
}

fn format_fn_sig_hover(
    sig: &writ_compiler::check::env::FnSig,
    def_map: &DefMap,
    interner: &TyInterner,
) -> String {
    let mut parts = Vec::new();
    if let Some(mutable) = sig.self_param {
        parts.push(if mutable { "mut self".to_string() } else { "self".to_string() });
    }
    for (name, ty) in &sig.params {
        parts.push(format!("{}: {}", name, interner.display_named(*ty, def_map)));
    }
    let ret_str = interner.display_named(sig.ret, def_map);
    let generics = if sig.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", sig.generics.join(", "))
    };
    format!("```writ\nfn {}{}({}) -> {}\n```", sig.name, generics, parts.join(", "), ret_str)
}

/// Extract doc comments that immediately precede a declaration.
///
/// Looks backward from `decl_span_start` (a byte offset), collecting
/// consecutive `//` comment lines. Blank lines between the comments and
/// the declaration are allowed; blank lines inside the comment block
/// terminate collection.
pub fn extract_doc_comment(source: &str, decl_span_start: usize) -> Option<String> {
    let before = &source[..decl_span_start];
    let mut lines: Vec<&str> = Vec::new();
    let mut saw_comment = false;

    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            // Strip the comment marker and any leading whitespace from the content
            let content = trimmed.trim_start_matches('/').trim();
            lines.push(content);
            saw_comment = true;
        } else if trimmed.is_empty() {
            // Allow blank lines only before the first comment
            if saw_comment {
                break;
            }
            // Blank line before any comment: keep scanning
        } else {
            // Non-comment, non-blank line: stop
            break;
        }
    }

    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

/// Hover text for a definition name (fn, enum, struct, const, global).
///
/// Called when the cursor is on the name span of a top-level declaration.
pub fn hover_text_for_def(
    def_id: writ_compiler::resolve::def_map::DefId,
    def_map: &DefMap,
    interner: &TyInterner,
    type_env: &writ_compiler::check::env::TypeEnv,
    source: &str,
    ast: &TypedAst,
) -> String {
    use writ_compiler::check::ir::TypedDecl;
    use writ_compiler::resolve::def_map::DefKind;

    let entry = def_map.get_entry(def_id);
    match entry.kind {
        DefKind::Fn | DefKind::ExternFn => {
            if let Some(sig) = type_env.fn_sigs.get(&def_id) {
                let sig_text = format_fn_sig_hover(sig, def_map, interner);
                if entry.file_id != writ_diagnostics::FileId(u32::MAX)
                    && let Some(doc) = extract_doc_comment(source, entry.span.start) {
                        return format!("{}\n\n{}", sig_text, doc);
                    }
                sig_text
            } else {
                format!("```writ\nfn {}\n```", entry.name)
            }
        }
        DefKind::Enum => {
            if let Some(variants) = type_env.enum_variants.get(&def_id) {
                let names: Vec<&str> = variants.iter().map(|v| v.name.as_str()).collect();
                format!("```writ\nenum {} {{ {} }}\n```", entry.name, names.join(", "))
            } else {
                format!("```writ\nenum {}\n```", entry.name)
            }
        }
        DefKind::Const => {
            let ty = type_env
                .const_types
                .get(&def_id)
                .map(|t| interner.display_named(*t, def_map))
                .unwrap_or_else(|| "?".to_string());
            let value_text = ast.decls.iter().find_map(|decl| {
                if let TypedDecl::Const { def_id: did, value, .. } = decl {
                    if *did == def_id {
                        let span = value.span();
                        source.get(span.start..span.end).map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            if let Some(val) = value_text {
                format!("```writ\n{}: {} = {}\n```", entry.name, ty, val)
            } else {
                format!("```writ\n{}: {}\n```", entry.name, ty)
            }
        }
        DefKind::Struct | DefKind::ExternStruct => {
            format!("```writ\nstruct {}\n```", entry.name)
        }
        DefKind::Class | DefKind::ExternClass => {
            format!("```writ\nclass {}\n```", entry.name)
        }
        DefKind::Entity => {
            format!("```writ\nentity {}\n```", entry.name)
        }
        DefKind::Global => {
            let ty = type_env
                .global_types
                .get(&def_id)
                .map(|(t, _)| interner.display_named(*t, def_map))
                .unwrap_or_else(|| "?".to_string());
            format!("```writ\nglobal {}: {}\n```", entry.name, ty)
        }
        _ => String::new(),
    }
}

/// Information returned when hovering over a match arm pattern.
pub struct PatternHoverInfo {
    pub text: String,
    pub span: SimpleSpan,
}

/// Search all match arm patterns for one whose span contains `offset`.
///
/// When found, returns hover text showing the enum type of the pattern.
/// This handles the case where the cursor is on an enum variant name inside
/// a match pattern (e.g., `QuestStatus::Completed` in `match x { QuestStatus::Completed => ... }`).
pub fn pattern_at_offset(ast: &TypedAst, offset: usize, file_id: FileId) -> Option<PatternHoverInfo> {

    for decl in &ast.decls {
        if decl_file_id(decl, &ast.def_map) != file_id {
            continue;
        }
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(info) = find_pattern_in_expr(body, offset, &ast.def_map) {
                    return Some(info);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(info) = find_pattern_in_expr(body, offset, &ast.def_map) {
                        return Some(info);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn find_pattern_in_expr(expr: &TypedExpr, offset: usize, def_map: &DefMap) -> Option<PatternHoverInfo> {
    match expr {
        TypedExpr::Match { scrutinee, arms, .. } => {
            // Check arm patterns first
            for arm in arms {
                if let Some(info) = find_in_pattern(&arm.pattern, offset, def_map) {
                    return Some(info);
                }
                // Also recurse into arm body
                if let Some(info) = find_pattern_in_expr(&arm.body, offset, def_map) {
                    return Some(info);
                }
            }
            find_pattern_in_expr(scrutinee, offset, def_map)
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                if let Some(info) = find_pattern_in_stmt(stmt, offset, def_map) {
                    return Some(info);
                }
            }
            tail.as_ref().and_then(|t| find_pattern_in_expr(t, offset, def_map))
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            find_pattern_in_expr(condition, offset, def_map)
                .or_else(|| find_pattern_in_expr(then_branch, offset, def_map))
                .or_else(|| else_branch.as_ref().and_then(|e| find_pattern_in_expr(e, offset, def_map)))
        }
        TypedExpr::Lambda { body, .. } => find_pattern_in_expr(body, offset, def_map),
        _ => None,
    }
}

fn find_pattern_in_stmt(stmt: &TypedStmt, offset: usize, def_map: &DefMap) -> Option<PatternHoverInfo> {
    match stmt {
        TypedStmt::Let { value, .. } => find_pattern_in_expr(value, offset, def_map),
        TypedStmt::Expr { expr, .. } => find_pattern_in_expr(expr, offset, def_map),
        TypedStmt::For { iterable, body, .. } => {
            find_pattern_in_expr(iterable, offset, def_map)
                .or_else(|| body.iter().find_map(|s| find_pattern_in_stmt(s, offset, def_map)))
        }
        TypedStmt::While { condition, body, .. } => {
            find_pattern_in_expr(condition, offset, def_map)
                .or_else(|| body.iter().find_map(|s| find_pattern_in_stmt(s, offset, def_map)))
        }
        TypedStmt::Atomic { body, .. } => {
            body.iter().find_map(|s| find_pattern_in_stmt(s, offset, def_map))
        }
        TypedStmt::Return { value, .. } | TypedStmt::Break { value, .. } => {
            value.as_ref().and_then(|v| find_pattern_in_expr(v, offset, def_map))
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => None,
    }
}

fn find_in_pattern(
    pattern: &writ_compiler::check::ir::TypedPattern,
    offset: usize,
    def_map: &DefMap,
) -> Option<PatternHoverInfo> {
    use writ_compiler::check::ir::TypedPattern as TP;

    match pattern {
        TP::EnumVariant { enum_def_id, variant_name, span, bindings } => {
            if offset >= span.start && offset < span.end {
                let entry = def_map.get_entry(*enum_def_id);
                let text = format!("```writ\n{}::{}\n```", entry.name, variant_name);
                return Some(PatternHoverInfo { text, span: *span });
            }
            // Recurse into bindings
            for binding in bindings {
                if let Some(info) = find_in_pattern(binding, offset, def_map) {
                    return Some(info);
                }
            }
            None
        }
        TP::Or { patterns, .. } => {
            patterns.iter().find_map(|p| find_in_pattern(p, offset, def_map))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{hover_text_for_expr};
    use writ_compiler::check::ir::TypedAst;
    use writ_compiler::check::ty::TyInterner;
    use writ_diagnostics::{FileId, Severity};

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

    // ── hover_text_for_expr tests ─────────────────────────────────────────────

    #[test]
    fn test_hover_text_var() {
        // Variable hover should show "varname: TypeName"
        let src = "fn main() { let x: int = 1; x }";
        let (ast, interner, type_env) = build_typed_ast_full(src);

        // Find 'x' in tail position
        let x_offset = src.find(" x }").map(|i| i + 1).unwrap();
        let expr = super::super::walk::expr_at_offset(&ast, x_offset, FileId(0)).expect("should find expression");

        let hover = hover_text_for_expr(expr, &ast.def_map, &interner, &type_env, src, &ast);
        assert!(!hover.is_empty(), "hover text should not be empty");
        assert!(hover.contains("x"), "hover should contain variable name");
        assert!(hover.contains("int"), "hover should contain type name 'int'");
    }

    #[test]
    fn test_hover_text_fn_call() {
        // Hovering on a function call should show the function signature
        let src = "fn foo(x: int) -> int { x } fn main() -> int { foo(1) }";
        let (ast, interner, type_env) = build_typed_ast_full(src);

        // Find the call to 'foo' in main
        let foo_call_offset = src.rfind("foo").unwrap();
        let expr = super::super::walk::expr_at_offset(&ast, foo_call_offset, FileId(0)).expect("should find expression");

        let hover = hover_text_for_expr(expr, &ast.def_map, &interner, &type_env, src, &ast);
        assert!(hover.contains("foo"), "hover should contain function name");
    }
}
