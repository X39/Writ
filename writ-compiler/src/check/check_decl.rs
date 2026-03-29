//! Declaration type checking.

use crate::ast::decl::{AstDecl, AstFnDecl, AstFnParam, AstImplDecl, AstImplMember, AstConstDecl, AstGlobalDecl};
use crate::ast::Ast;
use crate::resolve::def_map::{DefId, DefKind};
use crate::resolve::ir::ResolvedDecl;

use super::check_expr::{check_block_stmts, check_expr, CheckCtx};
use super::env::Mutability;
use super::ir::{TypedDecl, TypedExpr};
use super::ty::TyKind;
use writ_diagnostics::{Diagnostic, FileId};
use writ_diagnostics::code;

/// Type-check a resolved declaration, returning a TypedDecl.
pub fn check_decl(
    ctx: &mut CheckCtx,
    decl: &ResolvedDecl,
    asts: &[(FileId, &Ast)],
) -> TypedDecl {
    match decl {
        ResolvedDecl::Fn { def_id } => check_fn_decl(ctx, *def_id, asts),
        ResolvedDecl::Struct { def_id } => TypedDecl::Struct { def_id: *def_id },
        ResolvedDecl::Class { def_id } => TypedDecl::Class { def_id: *def_id },
        ResolvedDecl::Entity { def_id } => TypedDecl::Entity { def_id: *def_id },
        ResolvedDecl::Enum { def_id } => TypedDecl::Enum { def_id: *def_id },
        ResolvedDecl::Contract { def_id } => TypedDecl::Contract { def_id: *def_id },
        ResolvedDecl::Impl { def_id } => check_impl_decl(ctx, *def_id, asts),
        ResolvedDecl::Component { def_id } => TypedDecl::Component { def_id: *def_id },
        ResolvedDecl::ExternFn { def_id } => TypedDecl::ExternFn { def_id: *def_id },
        ResolvedDecl::ExternComponent { def_id } => TypedDecl::ExternComponent { def_id: *def_id },
        ResolvedDecl::Const { def_id } => check_const_decl(ctx, *def_id, asts),
        ResolvedDecl::Global { def_id } => check_global_decl(ctx, *def_id, asts),
        ResolvedDecl::AttributeDef { def_id } => check_attribute_decl(ctx, *def_id, asts),
    }
}

fn check_fn_decl(ctx: &mut CheckCtx, def_id: DefId, asts: &[(FileId, &Ast)]) -> TypedDecl {
    let entry = ctx.def_map.get_entry(def_id);
    let _name = entry.name.clone();
    let file_id = entry.file_id;
    let span = entry.span;

    // Find the AST function declaration
    let fn_decl = find_fn_ast(asts, entry);
    if fn_decl.is_none() {
        return TypedDecl::Fn {
            def_id,
            body: TypedExpr::Error {
                ty: ctx.interner.error(),
                span,
            },
            param_name_spans: vec![],
        };
    }
    let fn_decl = fn_decl.unwrap();

    // Get fn signature from TypeEnv
    let sig = ctx.type_env.fn_sigs.get(&def_id).cloned();
    let ret_ty = sig.as_ref().map(|s| s.ret).unwrap_or_else(|| ctx.interner.void());

    // Set up context for checking the body
    let old_ret = ctx.current_fn_ret;
    let old_file = ctx.current_file;
    let old_namespace = ctx.current_namespace.clone();
    ctx.current_fn_ret = Some(ret_ty);
    ctx.current_file = file_id;
    ctx.current_namespace = entry.namespace.clone();

    ctx.local_env.push_scope();

    // Define parameters in local env
    if let Some(ref sig) = sig {
        for (param_name, param_ty) in &sig.params {
            ctx.local_env.define(
                param_name.clone(),
                *param_ty,
                Mutability::Immutable,
                span,
            );
        }
    }

    // Check body
    let body = check_block_stmts(ctx, &fn_decl.body, span);

    ctx.local_env.pop_scope();

    // Restore context
    ctx.current_fn_ret = old_ret;
    ctx.current_file = old_file;
    ctx.current_namespace = old_namespace;

    // Collect param name spans for LSP hover support
    let param_name_spans: Vec<chumsky::span::SimpleSpan> = fn_decl.params.iter().map(|p| {
        match p {
            AstFnParam::Regular(param) => param.name_span,
            AstFnParam::SelfParam { span: self_span, .. } => *self_span,
        }
    }).collect();

    TypedDecl::Fn { def_id, body, param_name_spans }
}

fn check_impl_decl(ctx: &mut CheckCtx, def_id: DefId, asts: &[(FileId, &Ast)]) -> TypedDecl {
    let entry = ctx.def_map.get_entry(def_id);
    let file_id = entry.file_id;
    let span = entry.span;

    let impl_decl = find_impl_ast(asts, entry);
    if impl_decl.is_none() {
        return TypedDecl::Impl {
            def_id,
            methods: Vec::new(),
        };
    }
    let impl_decl = impl_decl.unwrap();

    // Resolve self type for methods.
    // Handle both `impl Foo` (Named) and `impl<T> Foo<T>` (Generic).
    let target_type_name = match &impl_decl.target {
        crate::ast::types::AstType::Named { name, .. } => Some(name.as_str()),
        crate::ast::types::AstType::Generic { name, .. } => Some(name.as_str()),
        _ => None,
    };
    let self_type = target_type_name.and_then(|name| {
        let target_def_id = ctx.def_map.get(name)?;
        let target_entry = ctx.def_map.get_entry(target_def_id);
        match target_entry.kind {
            DefKind::Struct => Some(ctx.interner.intern(TyKind::Struct(target_def_id))),
            DefKind::Class => Some(ctx.interner.intern(TyKind::Class(target_def_id))),
            DefKind::Entity => Some(ctx.interner.intern(TyKind::Entity(target_def_id))),
            DefKind::Enum => Some(ctx.interner.intern(TyKind::Enum(target_def_id))),
            DefKind::Contract => Some(ctx.interner.intern(TyKind::Contract(target_def_id))),
            _ => None,
        }
    });

    let mut methods = Vec::new();

    for member in &impl_decl.members {
        if let AstImplMember::Fn(fn_decl) = member {
            // Look up the method's DefId from the impl_index
            // For simplicity, we check the method body inline
            let old_ret = ctx.current_fn_ret;
            let old_file = ctx.current_file;
            let old_self = ctx.self_type;
            let old_namespace = ctx.current_namespace.clone();

            ctx.current_file = file_id;
            ctx.self_type = self_type;
            ctx.current_namespace = entry.namespace.clone();

            // Find the method signature from impl_index
            let method_ret = find_impl_method_ret(ctx, def_id, &fn_decl.name);
            ctx.current_fn_ret = Some(method_ret.unwrap_or_else(|| ctx.interner.void()));

            ctx.local_env.push_scope();

            // Define self if present, using the actual mutability from the SelfParam.
            if let Some(self_ty) = self_type {
                for param in &fn_decl.params {
                    if let AstFnParam::SelfParam { mutable, .. } = param {
                        let self_mut = if *mutable { Mutability::Mutable } else { Mutability::Immutable };
                        ctx.local_env.define(
                            "self".to_string(),
                            self_ty,
                            self_mut,
                            span,
                        );
                    }
                }
            }

            // Define regular params - clone to avoid borrow conflict
            let params: Vec<(String, super::ty::Ty)> = find_impl_method_sig(ctx, def_id, &fn_decl.name)
                .map(|sig| sig.params.clone())
                .unwrap_or_default();
            for (param_name, param_ty) in &params {
                ctx.local_env.define(
                    param_name.clone(),
                    *param_ty,
                    Mutability::Immutable,
                    span,
                );
            }

            let body = check_block_stmts(ctx, &fn_decl.body, span);

            ctx.local_env.pop_scope();
            ctx.current_fn_ret = old_ret;
            ctx.current_file = old_file;
            ctx.self_type = old_self;
            ctx.current_namespace = old_namespace;

            // Use the impl_def_id as a placeholder for the method DefId
            methods.push((def_id, body));
        }
    }

    TypedDecl::Impl { def_id, methods }
}

fn check_const_decl(ctx: &mut CheckCtx, def_id: DefId, asts: &[(FileId, &Ast)]) -> TypedDecl {
    let entry = ctx.def_map.get_entry(def_id);
    let file_id = entry.file_id;
    let span = entry.span;

    let const_decl = find_const_ast(asts, entry);
    if let Some(decl) = const_decl {
        let old_file = ctx.current_file;
        ctx.current_file = file_id;
        let typed_value = check_expr(ctx, &decl.value);
        ctx.current_file = old_file;

        TypedDecl::Const {
            def_id,
            value: typed_value,
        }
    } else {
        TypedDecl::Const {
            def_id,
            value: TypedExpr::Error {
                ty: ctx.interner.error(),
                span,
            },
        }
    }
}

fn check_global_decl(ctx: &mut CheckCtx, def_id: DefId, asts: &[(FileId, &Ast)]) -> TypedDecl {
    let entry = ctx.def_map.get_entry(def_id);
    let file_id = entry.file_id;
    let span = entry.span;

    let global_decl = find_global_ast(asts, entry);
    if let Some(decl) = global_decl {
        let old_file = ctx.current_file;
        ctx.current_file = file_id;
        let typed_value = check_expr(ctx, &decl.value);
        ctx.current_file = old_file;

        TypedDecl::Global {
            def_id,
            value: typed_value,
        }
    } else {
        TypedDecl::Global {
            def_id,
            value: TypedExpr::Error {
                ty: ctx.interner.error(),
                span,
            },
        }
    }
}

fn check_attribute_decl(ctx: &mut CheckCtx, def_id: DefId, asts: &[(FileId, &Ast)]) -> TypedDecl {
    // UATTR-01: Validate attribute parameter types are string, int, or bool
    let entry = ctx.def_map.get_entry(def_id);
    let file_id = entry.file_id;
    let name = entry.name.clone();
    let name_span = entry.name_span;

    // Find the AstAttributeDecl in the ASTs
    for (fid, ast) in asts {
        if *fid != file_id {
            continue;
        }
        for decl in &ast.items {
            if let AstDecl::Attribute(a) = decl {
                if a.name == name && a.name_span == name_span {
                    for param in &a.params {
                        let valid = match &param.ty {
                            crate::ast::types::AstType::Named { name: ty_name, .. } => {
                                matches!(ty_name.as_str(), "string" | "int" | "bool")
                            }
                            _ => false,
                        };
                        if !valid {
                            ctx.diags.push(
                                Diagnostic::error(
                                    code::E0006,
                                    format!(
                                        "unsupported attribute parameter type for `{}`",
                                        param.name
                                    ),
                                )
                                .with_primary(
                                    file_id,
                                    param.span,
                                    "attribute parameters must be `string`, `int`, or `bool`",
                                )
                                .build(),
                            );
                        }
                    }
                }
            }
        }
    }

    TypedDecl::AttributeDef { def_id }
}

// AST lookup helpers
fn find_fn_ast<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &crate::resolve::def_map::DefEntry,
) -> Option<&'a AstFnDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let AstDecl::Fn(fn_decl) = decl
                && fn_decl.name == entry.name && fn_decl.name_span == entry.name_span {
                    return Some(fn_decl);
                }
        }
    }
    None
}

fn find_impl_ast<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &crate::resolve::def_map::DefEntry,
) -> Option<&'a AstImplDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let AstDecl::Impl(impl_decl) = decl
                && impl_decl.span == entry.span {
                    return Some(impl_decl);
                }
        }
    }
    None
}

fn find_const_ast<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &crate::resolve::def_map::DefEntry,
) -> Option<&'a AstConstDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let AstDecl::Const(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

fn find_global_ast<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &crate::resolve::def_map::DefEntry,
) -> Option<&'a AstGlobalDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let AstDecl::Global(g) = decl
                && g.name == entry.name && g.name_span == entry.name_span {
                    return Some(g);
                }
        }
    }
    None
}

fn find_impl_method_ret(ctx: &CheckCtx, impl_def_id: DefId, method_name: &str) -> Option<super::ty::Ty> {
    // Look through the impl_index for the impl_def_id
    for impls in ctx.type_env.impl_index.values() {
        for impl_entry in impls {
            if impl_entry.impl_def_id == impl_def_id {
                for (name, sig) in &impl_entry.methods {
                    if name == method_name {
                        return Some(sig.ret);
                    }
                }
            }
        }
    }
    None
}

fn find_impl_method_sig<'a>(
    ctx: &'a CheckCtx,
    impl_def_id: DefId,
    method_name: &str,
) -> Option<&'a super::env::FnSig> {
    for impls in ctx.type_env.impl_index.values() {
        for impl_entry in impls {
            if impl_entry.impl_def_id == impl_def_id {
                for (name, sig) in &impl_entry.methods {
                    if name == method_name {
                        return Some(sig);
                    }
                }
            }
        }
    }
    None
}
