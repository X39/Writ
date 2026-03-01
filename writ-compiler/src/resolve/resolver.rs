//! Pass 2: Body resolver.
//!
//! Walks every declaration body in the AST, resolving all identifier and type
//! references against the DefMap populated by Pass 1.

use std::cell::Cell;

use chumsky::span::SimpleSpan;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::*;
use crate::ast::types::AstType;
use crate::ast::Ast;
use crate::resolve::def_map::{DefKind, DefMap};
use crate::resolve::error::ResolutionError;
use crate::resolve::ir::*;
use crate::resolve::scope::{LookupResult, ScopeChain, UsingEntry};
use crate::resolve::suggest;

/// Resolve all bodies across all files, producing resolved declarations.
pub fn resolve_bodies(
    asts: &[(FileId, &Ast)],
    file_paths: &[(FileId, &str)],
    def_map: &DefMap,
) -> (Vec<ResolvedDecl>, Vec<Diagnostic>) {
    let mut all_decls = Vec::new();
    let mut all_diags = Vec::new();

    // Build file path lookup for namespace detection
    let _path_map: std::collections::HashMap<FileId, &str> =
        file_paths.iter().copied().collect();

    for &(file_id, ast) in asts {
        // Determine current namespace from declarative namespace in this file
        let current_ns = detect_file_namespace(ast);

        let mut scope = ScopeChain::new(def_map, file_id, current_ns);

        // Process using declarations first
        process_usings(&ast.items, &mut scope, &mut all_diags);

        // Resolve all declarations
        resolve_decl_list(&ast.items, &mut scope, &mut all_decls, &mut all_diags);

        // Emit unused import warnings
        scope.emit_unused_import_warnings(&mut all_diags);
    }

    (all_decls, all_diags)
}

/// Detect the file-level namespace from declarative namespace declarations.
fn detect_file_namespace(ast: &Ast) -> String {
    for item in &ast.items {
        if let AstDecl::Namespace(AstNamespaceDecl::Declarative { path, .. }) = item {
            return path.join("::");
        }
    }
    String::new()
}

/// Process using declarations, adding them to the scope.
fn process_usings(items: &[AstDecl], scope: &mut ScopeChain<'_>, diags: &mut Vec<Diagnostic>) {
    for item in items {
        if let AstDecl::Using(using) = item {
            let path = &using.path;

            if path.len() == 1 {
                // Namespace import: `using survival;`
                let ns_name = &path[0];
                // Verify namespace exists
                if scope.def_map.namespace_members.contains_key(ns_name.as_str()) {
                    scope.active_usings.push(UsingEntry {
                        alias: ns_name.clone(),
                        target_ns: Some(ns_name.clone()),
                        target_fqn: None,
                        span: using.span,
                        used: Cell::new(false),
                    });
                } else {
                    diags.push(
                        ResolutionError::UnresolvedNamespace {
                            name: ns_name.clone(),
                            file: scope.current_file,
                            span: using.span,
                        }
                        .into(),
                    );
                }
            } else if path.len() >= 2 {
                // Specific import: `using survival::HealthPotion;`
                let fqn = path.join("::");
                let terminal = path.last().unwrap().clone();
                let alias = using.alias.clone().unwrap_or(terminal);

                // Check if the FQN resolves
                if scope.def_map.get(&fqn).is_some() {
                    scope.active_usings.push(UsingEntry {
                        alias,
                        target_ns: None,
                        target_fqn: Some(fqn),
                        span: using.span,
                        used: Cell::new(false),
                    });
                } else {
                    // Try as namespace import (all-but-last is a namespace)
                    let ns = path[..path.len() - 1].join("::");
                    if scope.def_map.namespace_members.contains_key(ns.as_str()) {
                        // Namespace import with terminal
                        scope.active_usings.push(UsingEntry {
                            alias,
                            target_ns: Some(ns),
                            target_fqn: None,
                            span: using.span,
                            used: Cell::new(false),
                        });
                    } else {
                        let suggestion = get_suggestion(&fqn, scope);
                        diags.push(
                            ResolutionError::UnresolvedName {
                                name: fqn,
                                file: scope.current_file,
                                span: using.span,
                                suggestion,
                            }
                            .into(),
                        );
                    }
                }
            }
        }
    }
}

/// Resolve a list of declarations.
fn resolve_decl_list(
    items: &[AstDecl],
    scope: &mut ScopeChain<'_>,
    decls: &mut Vec<ResolvedDecl>,
    diags: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            AstDecl::Namespace(AstNamespaceDecl::Block { path, items, .. }) => {
                let saved_ns = scope.current_ns.clone();
                let block_ns = path.join("::");
                scope.current_ns = if saved_ns.is_empty() {
                    block_ns
                } else {
                    format!("{saved_ns}::{block_ns}")
                };
                resolve_decl_list(items, scope, decls, diags);
                scope.current_ns = saved_ns;
            }
            AstDecl::Namespace(AstNamespaceDecl::Declarative { .. }) => {
                // Already processed in detect_file_namespace
            }
            AstDecl::Using(_) => {
                // Already processed in process_usings
            }
            AstDecl::Stmt(_) => {
                // Bare statements: no name resolution needed at top level
            }

            AstDecl::Fn(f) => {
                let fqn = make_fqn(&scope.current_ns, &f.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&f.name).copied())
                }) {
                    // Push generics
                    let generic_names: Vec<(String, SimpleSpan)> =
                        f.generics.iter().map(|g| (g.name.clone(), g.name_span)).collect();
                    if !generic_names.is_empty() {
                        check_generic_shadows(&generic_names, scope, diags);
                        scope.push_generics(generic_names);
                    }

                    // Resolve param types
                    let params: Vec<ResolvedType> = f.params.iter().filter_map(|p| {
                        match p {
                            AstFnParam::Regular(param) => Some(resolve_ast_type(&param.ty, scope, diags)),
                            AstFnParam::SelfParam { .. } => None,
                        }
                    }).collect();

                    let return_type = f.return_type.as_ref()
                        .map(|t| resolve_ast_type(t, scope, diags))
                        .unwrap_or(ResolvedType::Void);

                    decls.push(ResolvedDecl::Fn { def_id });

                    if !f.generics.is_empty() {
                        scope.pop_layer();
                    }

                    let _ = (params, return_type); // Used for resolution side-effects
                }
            }

            AstDecl::Struct(s) => {
                let fqn = make_fqn(&scope.current_ns, &s.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&s.name).copied())
                }) {
                    let generic_names: Vec<(String, SimpleSpan)> =
                        s.generics.iter().map(|g| (g.name.clone(), g.name_span)).collect();
                    if !generic_names.is_empty() {
                        check_generic_shadows(&generic_names, scope, diags);
                        scope.push_generics(generic_names);
                    }

                    // Resolve field types
                    for member in &s.members {
                        if let AstStructMember::Field(field) = member {
                            resolve_ast_type(&field.ty, scope, diags);
                        }
                    }

                    decls.push(ResolvedDecl::Struct { def_id });

                    if !s.generics.is_empty() {
                        scope.pop_layer();
                    }
                }
            }

            AstDecl::Entity(e) => {
                let fqn = make_fqn(&scope.current_ns, &e.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&e.name).copied())
                }) {
                    // Resolve property types
                    for prop in &e.properties {
                        resolve_ast_type(&prop.ty, scope, diags);
                    }

                    // Resolve component slot names
                    for slot in &e.component_slots {
                        let slot_result = scope.resolve_type(&slot.component);
                        match slot_result {
                            LookupResult::Def(comp_id) => {
                                let entry = scope.def_map.get_entry(comp_id);
                                if !matches!(entry.kind, DefKind::Component | DefKind::ExternComponent) {
                                    diags.push(
                                        ResolutionError::NotAComponent {
                                            name: slot.component.clone(),
                                            file: scope.current_file,
                                            span: slot.component_span,
                                        }
                                        .into(),
                                    );
                                }
                            }
                            _ => {
                                let suggestion = get_suggestion(&slot.component, scope);
                                diags.push(
                                    ResolutionError::UnresolvedName {
                                        name: slot.component.clone(),
                                        file: scope.current_file,
                                        span: slot.component_span,
                                        suggestion,
                                    }
                                    .into(),
                                );
                            }
                        }
                    }

                    // Set self type for hooks/methods
                    scope.self_type = Some(def_id);

                    decls.push(ResolvedDecl::Entity { def_id });

                    scope.self_type = None;
                }
            }

            AstDecl::Enum(e) => {
                let fqn = make_fqn(&scope.current_ns, &e.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&e.name).copied())
                }) {
                    let generic_names: Vec<(String, SimpleSpan)> =
                        e.generics.iter().map(|g| (g.name.clone(), g.name_span)).collect();
                    if !generic_names.is_empty() {
                        check_generic_shadows(&generic_names, scope, diags);
                        scope.push_generics(generic_names);
                    }

                    // Resolve variant field types
                    for variant in &e.variants {
                        if let Some(fields) = &variant.fields {
                            for field in fields {
                                resolve_ast_type(&field.ty, scope, diags);
                            }
                        }
                    }

                    decls.push(ResolvedDecl::Enum { def_id });

                    if !e.generics.is_empty() {
                        scope.pop_layer();
                    }
                }
            }

            AstDecl::Contract(c) => {
                let fqn = make_fqn(&scope.current_ns, &c.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&c.name).copied())
                }) {
                    let generic_names: Vec<(String, SimpleSpan)> =
                        c.generics.iter().map(|g| (g.name.clone(), g.name_span)).collect();
                    if !generic_names.is_empty() {
                        check_generic_shadows(&generic_names, scope, diags);
                        scope.push_generics(generic_names);
                    }

                    // Resolve member signatures
                    for member in &c.members {
                        match member {
                            AstContractMember::FnSig(sig) => {
                                for param in &sig.params {
                                    match param {
                                        AstFnParam::Regular(p) => { resolve_ast_type(&p.ty, scope, diags); }
                                        AstFnParam::SelfParam { .. } => {}
                                    }
                                }
                                if let Some(ret) = &sig.return_type {
                                    resolve_ast_type(ret, scope, diags);
                                }
                            }
                            AstContractMember::OpSig(sig) => {
                                for p in &sig.params {
                                    resolve_ast_type(&p.ty, scope, diags);
                                }
                                if let Some(ret) = &sig.return_type {
                                    resolve_ast_type(ret, scope, diags);
                                }
                            }
                        }
                    }

                    decls.push(ResolvedDecl::Contract { def_id });

                    if !c.generics.is_empty() {
                        scope.pop_layer();
                    }
                }
            }

            AstDecl::Impl(imp) => {
                // Resolve target type
                let target_result = resolve_ast_type(&imp.target, scope, diags);

                // Resolve contract type if present
                let contract_type = imp.contract.as_ref().map(|c| resolve_ast_type(c, scope, diags));

                // Find the impl DefId from impl_blocks
                // (Impls are tracked by index; find matching span)
                let impl_def_id = scope.def_map.impl_blocks.iter().find(|&&id| {
                    scope.def_map.get_entry(id).span == imp.span
                }).copied();

                if let Some(impl_id) = impl_def_id {
                    let generic_names: Vec<(String, SimpleSpan)> =
                        imp.generics.iter().map(|g| (g.name.clone(), g.name_span)).collect();
                    if !generic_names.is_empty() {
                        check_generic_shadows(&generic_names, scope, diags);
                        scope.push_generics(generic_names);
                    }

                    // Set self type from target
                    if let ResolvedType::Named { def_id, .. } = &target_result {
                        scope.self_type = Some(*def_id);
                    }

                    // Resolve member bodies
                    for member in &imp.members {
                        match member {
                            AstImplMember::Fn(f) => {
                                for param in &f.params {
                                    match param {
                                        AstFnParam::Regular(p) => { resolve_ast_type(&p.ty, scope, diags); }
                                        AstFnParam::SelfParam { .. } => {}
                                    }
                                }
                                if let Some(ret) = &f.return_type {
                                    resolve_ast_type(ret, scope, diags);
                                }
                            }
                            AstImplMember::Op(op) => {
                                for p in &op.params {
                                    resolve_ast_type(&p.ty, scope, diags);
                                }
                                if let Some(ret) = &op.return_type {
                                    resolve_ast_type(ret, scope, diags);
                                }
                            }
                        }
                    }

                    scope.self_type = None;
                    decls.push(ResolvedDecl::Impl { def_id: impl_id });

                    if !imp.generics.is_empty() {
                        scope.pop_layer();
                    }
                }

                let _ = contract_type; // Used for resolution side-effects
            }

            AstDecl::Component(c) => {
                let fqn = make_fqn(&scope.current_ns, &c.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&c.name).copied())
                }) {
                    for member in &c.members {
                        if let AstComponentMember::Field(field) = member {
                            resolve_ast_type(&field.ty, scope, diags);
                        }
                    }
                    decls.push(ResolvedDecl::Component { def_id });
                }
            }

            AstDecl::Extern(ext) => match ext {
                AstExternDecl::Fn(_, sig) => {
                    let fqn = make_fqn(&scope.current_ns, &sig.name);
                    if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                        scope.def_map.file_private
                            .get(&scope.current_file)
                            .and_then(|m| m.get(&sig.name).copied())
                    }) {
                        for param in &sig.params {
                            match param {
                                AstFnParam::Regular(p) => { resolve_ast_type(&p.ty, scope, diags); }
                                AstFnParam::SelfParam { .. } => {}
                            }
                        }
                        if let Some(ret) = &sig.return_type {
                            resolve_ast_type(ret, scope, diags);
                        }
                        decls.push(ResolvedDecl::ExternFn { def_id });
                    }
                }
                AstExternDecl::Struct(_, s) => {
                    let fqn = make_fqn(&scope.current_ns, &s.name);
                    if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                        scope.def_map.file_private
                            .get(&scope.current_file)
                            .and_then(|m| m.get(&s.name).copied())
                    }) {
                        decls.push(ResolvedDecl::ExternStruct { def_id });
                    }
                }
                AstExternDecl::Component(_, c) => {
                    let fqn = make_fqn(&scope.current_ns, &c.name);
                    if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                        scope.def_map.file_private
                            .get(&scope.current_file)
                            .and_then(|m| m.get(&c.name).copied())
                    }) {
                        decls.push(ResolvedDecl::ExternComponent { def_id });
                    }
                }
            },

            AstDecl::Const(c) => {
                let fqn = make_fqn(&scope.current_ns, &c.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&c.name).copied())
                }) {
                    resolve_ast_type(&c.ty, scope, diags);
                    decls.push(ResolvedDecl::Const { def_id });
                }
            }

            AstDecl::Global(g) => {
                let fqn = make_fqn(&scope.current_ns, &g.name);
                if let Some(def_id) = scope.def_map.get(&fqn).or_else(|| {
                    scope.def_map.file_private
                        .get(&scope.current_file)
                        .and_then(|m| m.get(&g.name).copied())
                }) {
                    resolve_ast_type(&g.ty, scope, diags);
                    decls.push(ResolvedDecl::Global { def_id });
                }
            }
        }
    }
}

/// Resolve an AstType to a ResolvedType.
pub fn resolve_ast_type(
    ty: &AstType,
    scope: &ScopeChain<'_>,
    diags: &mut Vec<Diagnostic>,
) -> ResolvedType {
    match ty {
        AstType::Named { name, span } => {
            match scope.resolve_type(name) {
                LookupResult::Def(def_id) => ResolvedType::Named {
                    def_id,
                    type_args: Vec::new(),
                },
                LookupResult::Primitive(tag) => ResolvedType::Primitive(tag),
                LookupResult::GenericParam(name) => ResolvedType::GenericParam(name),
                LookupResult::PreludeType(name) => ResolvedType::PreludeType(name),
                LookupResult::PreludeContract(name) => ResolvedType::PreludeContract(name),
                LookupResult::Ambiguous(candidates) => {
                    let cand_info: Vec<(FileId, SimpleSpan, String)> = candidates
                        .iter()
                        .map(|(id, fqn)| {
                            let entry = scope.def_map.get_entry(*id);
                            (entry.file_id, entry.name_span, fqn.clone())
                        })
                        .collect();
                    diags.push(
                        ResolutionError::AmbiguousName {
                            name: name.clone(),
                            file: scope.current_file,
                            span: *span,
                            candidates: cand_info,
                        }
                        .into(),
                    );
                    ResolvedType::Error
                }
                LookupResult::VisibilityError(def_id) => {
                    let entry = scope.def_map.get_entry(def_id);
                    diags.push(
                        ResolutionError::VisibilityViolation {
                            name: name.clone(),
                            file: scope.current_file,
                            span: *span,
                            defined_in: entry.file_id,
                            defined_span: entry.name_span,
                        }
                        .into(),
                    );
                    ResolvedType::Error
                }
                LookupResult::NotFound => {
                    let suggestion = get_suggestion(name, scope);
                    diags.push(
                        ResolutionError::UnresolvedName {
                            name: name.clone(),
                            file: scope.current_file,
                            span: *span,
                            suggestion,
                        }
                        .into(),
                    );
                    ResolvedType::Error
                }
            }
        }
        AstType::Generic { name, args, span } => {
            let outer = match scope.resolve_type(name) {
                LookupResult::Def(def_id) => Some(ResolvedType::Named {
                    def_id,
                    type_args: Vec::new(),
                }),
                LookupResult::PreludeType(_) => None, // Will be handled below
                LookupResult::PreludeContract(_) => None,
                LookupResult::NotFound => {
                    let suggestion = get_suggestion(name, scope);
                    diags.push(
                        ResolutionError::UnresolvedName {
                            name: name.clone(),
                            file: scope.current_file,
                            span: *span,
                            suggestion,
                        }
                        .into(),
                    );
                    return ResolvedType::Error;
                }
                _ => None,
            };

            let resolved_args: Vec<ResolvedType> =
                args.iter().map(|a| resolve_ast_type(a, scope, diags)).collect();

            if let Some(ResolvedType::Named { def_id, .. }) = outer {
                ResolvedType::Named {
                    def_id,
                    type_args: resolved_args,
                }
            } else {
                // Prelude generic type (Option<T>, Result<T, E>, etc.)
                ResolvedType::PreludeType(name.clone())
            }
        }
        AstType::Array { elem, .. } => {
            let resolved_elem = resolve_ast_type(elem, scope, diags);
            ResolvedType::Array(Box::new(resolved_elem))
        }
        AstType::Func { params, ret, .. } => {
            let resolved_params: Vec<ResolvedType> =
                params.iter().map(|p| resolve_ast_type(p, scope, diags)).collect();
            let resolved_ret = ret
                .as_ref()
                .map(|r| resolve_ast_type(r, scope, diags))
                .unwrap_or(ResolvedType::Void);
            ResolvedType::Func {
                params: resolved_params,
                ret: Box::new(resolved_ret),
            }
        }
        AstType::Void { .. } => ResolvedType::Void,
    }
}

/// Build a fully-qualified name.
fn make_fqn(ns: &str, name: &str) -> String {
    if ns.is_empty() {
        name.to_string()
    } else {
        format!("{ns}::{name}")
    }
}

/// Check if any generic parameter shadows an existing type and emit W0003.
/// Get a suggestion string for an unresolved name.
fn get_suggestion(name: &str, scope: &ScopeChain<'_>) -> Option<String> {
    let visible = scope.visible_names();
    let suggestions = suggest::suggest_similar(name, &visible, scope.def_map);
    suggest::format_suggestions(&suggestions)
}

fn check_generic_shadows(
    generic_names: &[(String, SimpleSpan)],
    scope: &ScopeChain<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    for (name, span) in generic_names {
        // Check if the name resolves to something already
        match scope.resolve_type(name) {
            LookupResult::NotFound => {} // Good -- no shadow
            _ => {
                diags.push(
                    ResolutionError::GenericShadow {
                        name: name.clone(),
                        file: scope.current_file,
                        span: *span,
                    }
                    .into(),
                );
            }
        }
    }
}
