//! Pass 1: Declaration collector.
//!
//! Walks all AST files and collects every top-level declaration into the DefMap.
//! Handles namespace context (both declarative and block forms), visibility,
//! prelude shadow checks, and namespace/path mismatch warnings.

use crate::ast::decl::{AstDecl, AstExternDecl, AstNamespaceDecl, AstVisibility};
use crate::ast::Ast;
use crate::resolve::def_map::{DefEntry, DefKind, DefMap, DefVis};
use crate::resolve::error::ResolutionError;
use crate::resolve::prelude::is_prelude_name;
use chumsky::span::SimpleSpan;
use writ_diagnostics::{Diagnostic, FileId};

/// Collect all top-level declarations from multiple files into a DefMap.
///
/// `file_paths` maps each FileId to its file path (for W0004 namespace/path mismatch).
pub fn collect_declarations(
    asts: &[(FileId, &Ast)],
    file_paths: &[(FileId, &str)],
) -> (DefMap, Vec<Diagnostic>) {
    let mut def_map = DefMap::new();
    let mut diags = Vec::new();
    let mut impl_counter: usize = 0;

    // Build file path lookup
    let path_map: std::collections::HashMap<FileId, &str> =
        file_paths.iter().copied().collect();

    for &(file_id, ast) in asts {
        let file_path = path_map.get(&file_id).copied().unwrap_or("");
        let mut ctx = CollectorContext {
            file_id,
            file_path,
            namespace: String::new(),
            impl_counter: &mut impl_counter,
        };
        collect_items(&ast.items, &mut ctx, &mut def_map, &mut diags);
    }

    (def_map, diags)
}

struct CollectorContext<'a> {
    file_id: FileId,
    file_path: &'a str,
    namespace: String,
    impl_counter: &'a mut usize,
}

fn collect_items(
    items: &[AstDecl],
    ctx: &mut CollectorContext<'_>,
    def_map: &mut DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            AstDecl::Namespace(ns) => match ns {
                AstNamespaceDecl::Declarative { path, span } => {
                    let ns = path.join("::");
                    ctx.namespace = ns.clone();

                    // W0004: Check namespace/path mismatch
                    check_namespace_path_mismatch(
                        &ns,
                        ctx.file_path,
                        ctx.file_id,
                        *span,
                        diags,
                    );
                }
                AstNamespaceDecl::Block { path, items, .. } => {
                    let saved_ns = ctx.namespace.clone();
                    let block_ns = path.join("::");
                    ctx.namespace = if saved_ns.is_empty() {
                        block_ns
                    } else {
                        format!("{saved_ns}::{block_ns}")
                    };
                    collect_items(items, ctx, def_map, diags);
                    ctx.namespace = saved_ns;
                }
            },

            AstDecl::Using(_) => {
                // Using declarations are resolved in Pass 2
            }

            AstDecl::Stmt(_) => {
                // Bare statements don't introduce names
            }

            AstDecl::Fn(f) => {
                let vis = ast_vis_to_def_vis(f.vis.as_ref());
                let generics = f.generics.iter().map(|g| g.name.clone()).collect();
                try_insert(
                    &f.name,
                    f.name_span,
                    f.span,
                    DefKind::Fn,
                    vis,
                    generics,
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Struct(s) => {
                let vis = ast_vis_to_def_vis(s.vis.as_ref());
                let generics = s.generics.iter().map(|g| g.name.clone()).collect();
                try_insert(
                    &s.name,
                    s.name_span,
                    s.span,
                    DefKind::Struct,
                    vis,
                    generics,
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Entity(e) => {
                let vis = ast_vis_to_def_vis(e.vis.as_ref());
                try_insert(
                    &e.name,
                    e.name_span,
                    e.span,
                    DefKind::Entity,
                    vis,
                    Vec::new(),
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Enum(e) => {
                let vis = ast_vis_to_def_vis(e.vis.as_ref());
                let generics = e.generics.iter().map(|g| g.name.clone()).collect();
                try_insert(
                    &e.name,
                    e.name_span,
                    e.span,
                    DefKind::Enum,
                    vis,
                    generics,
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Contract(c) => {
                let vis = ast_vis_to_def_vis(c.vis.as_ref());
                let generics = c.generics.iter().map(|g| g.name.clone()).collect();
                try_insert(
                    &c.name,
                    c.name_span,
                    c.span,
                    DefKind::Contract,
                    vis,
                    generics,
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Impl(imp) => {
                let counter = *ctx.impl_counter;
                *ctx.impl_counter += 1;
                let name = format!("impl#{counter}");
                let name_span = imp.span;
                let generics = imp.generics.iter().map(|g| g.name.clone()).collect();

                let fqn = if ctx.namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{name}", ctx.namespace)
                };

                let entry = DefEntry {
                    id: None,
                    kind: DefKind::Impl,
                    vis: DefVis::Pub,
                    file_id: ctx.file_id,
                    namespace: ctx.namespace.clone(),
                    name,
                    name_span,
                    generics,
                    span: imp.span,
                };

                def_map.insert(fqn, entry, diags);
            }

            AstDecl::Component(c) => {
                let vis = ast_vis_to_def_vis(c.vis.as_ref());
                try_insert(
                    &c.name,
                    c.name_span,
                    c.span,
                    DefKind::Component,
                    vis,
                    Vec::new(),
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Extern(ext) => match ext {
                AstExternDecl::Fn(vis, sig) => {
                    let vis = ast_vis_to_def_vis(vis.as_ref());
                    let generics = sig.generics.iter().map(|g| g.name.clone()).collect();
                    try_insert(
                        &sig.name,
                        sig.name_span,
                        sig.span,
                        DefKind::ExternFn,
                        vis,
                        generics,
                        ctx,
                        def_map,
                        diags,
                    );
                }
                AstExternDecl::Struct(vis, s) => {
                    let vis = ast_vis_to_def_vis(vis.as_ref());
                    let generics = s.generics.iter().map(|g| g.name.clone()).collect();
                    try_insert(
                        &s.name,
                        s.name_span,
                        s.span,
                        DefKind::ExternStruct,
                        vis,
                        generics,
                        ctx,
                        def_map,
                        diags,
                    );
                }
                AstExternDecl::Component(vis, c) => {
                    let vis = ast_vis_to_def_vis(vis.as_ref());
                    try_insert(
                        &c.name,
                        c.name_span,
                        c.span,
                        DefKind::ExternComponent,
                        vis,
                        Vec::new(),
                        ctx,
                        def_map,
                        diags,
                    );
                }
            },

            AstDecl::Const(c) => {
                let vis = ast_vis_to_def_vis(c.vis.as_ref());
                try_insert(
                    &c.name,
                    c.name_span,
                    c.span,
                    DefKind::Const,
                    vis,
                    Vec::new(),
                    ctx,
                    def_map,
                    diags,
                );
            }

            AstDecl::Global(g) => {
                let vis = ast_vis_to_def_vis(g.vis.as_ref());
                try_insert(
                    &g.name,
                    g.name_span,
                    g.span,
                    DefKind::Global,
                    vis,
                    Vec::new(),
                    ctx,
                    def_map,
                    diags,
                );
            }
        }
    }
}

/// Attempt to insert a named declaration into the DefMap.
///
/// Checks for prelude shadow violations before insertion.
#[allow(clippy::too_many_arguments)]
fn try_insert(
    name: &str,
    name_span: SimpleSpan,
    decl_span: SimpleSpan,
    kind: DefKind,
    vis: DefVis,
    generics: Vec<String>,
    ctx: &CollectorContext<'_>,
    def_map: &mut DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    // Check prelude shadow
    if is_prelude_name(name) {
        diags.push(
            ResolutionError::PreludeShadow {
                name: name.to_string(),
                file: ctx.file_id,
                span: name_span,
            }
            .into(),
        );
        return;
    }

    let fqn = if ctx.namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", ctx.namespace)
    };

    let entry = DefEntry {
        id: None,
        kind,
        vis,
        file_id: ctx.file_id,
        namespace: ctx.namespace.clone(),
        name: name.to_string(),
        name_span,
        generics,
        span: decl_span,
    };

    def_map.insert(fqn, entry, diags);
}

/// Convert AST visibility to DefVis.
///
/// Default (no visibility annotation) is treated as private.
fn ast_vis_to_def_vis(vis: Option<&AstVisibility>) -> DefVis {
    match vis {
        Some(AstVisibility::Pub) => DefVis::Pub,
        _ => DefVis::Private,
    }
}

/// Check if the file path mirrors the declared namespace.
///
/// Emits W0004 if the namespace path segments don't appear as a suffix
/// of the file's directory path.
fn check_namespace_path_mismatch(
    declared_ns: &str,
    file_path: &str,
    file_id: FileId,
    span: SimpleSpan,
    diags: &mut Vec<Diagnostic>,
) {
    if declared_ns.is_empty() || file_path.is_empty() {
        return;
    }

    // Convert namespace to path segments: "survival::combat" -> "survival/combat"
    let ns_as_path = declared_ns.replace("::", "/");

    // Normalize file path separators to forward slashes
    let normalized_path = file_path.replace('\\', "/");

    // Get the directory portion of the file path (strip the filename)
    let dir_path = if let Some(pos) = normalized_path.rfind('/') {
        &normalized_path[..pos]
    } else {
        ""
    };

    // Check if the directory path ends with the namespace path
    if !dir_path.ends_with(&ns_as_path) {
        diags.push(
            ResolutionError::NamespacePathMismatch {
                declared_ns: declared_ns.to_string(),
                file_path: file_path.to_string(),
                file: file_id,
                span,
            }
            .into(),
        );
    }
}
