//! Type environment builder helpers.
//!
//! Contains AST-lookup functions and type signature builder functions used
//! exclusively by `TypeEnv::build`. These are private implementation details
//! of the type environment; callers outside the `check` module use `TypeEnv`
//! and `resolve_ast_type` from `env.rs`.

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;

use crate::ast::decl::{
    AstFnDecl, AstFnSig, AstFnParam, AstGenericParam,
    AstStructDecl, AstStructMember, AstStructField,
    AstClassDecl, AstEntityDecl, AstEnumDecl,
    AstContractDecl, AstContractMember,
    AstImplDecl, AstImplMember,
    AstComponentDecl, AstComponentMember,
    AstExternDecl, AstConstDecl, AstGlobalDecl,
    AstAttribute, AstAttributeArg,
};
use crate::ast::types::AstType;
use crate::ast::Ast;
use crate::resolve::def_map::{DefEntry, DefId, DefKind, DefMap};
use writ_diagnostics::FileId;

use super::ty::{Ty, TyInterner, TyKind};
use super::env::{EnumVariantSig, FnSig, ImplEntry, TypeEnv};

// =============================================================================
// DefId extraction
// =============================================================================

/// Extract the DefId from a ResolvedDecl.
pub(super) fn decl_def_id(decl: &crate::resolve::ir::ResolvedDecl) -> DefId {
    use crate::resolve::ir::ResolvedDecl;
    match decl {
        ResolvedDecl::Fn { def_id }
        | ResolvedDecl::Struct { def_id }
        | ResolvedDecl::Class { def_id }
        | ResolvedDecl::Entity { def_id }
        | ResolvedDecl::Enum { def_id }
        | ResolvedDecl::Contract { def_id }
        | ResolvedDecl::Impl { def_id }
        | ResolvedDecl::Component { def_id }
        | ResolvedDecl::ExternFn { def_id }
        | ResolvedDecl::ExternComponent { def_id }
        | ResolvedDecl::Const { def_id }
        | ResolvedDecl::Global { def_id }
        | ResolvedDecl::AttributeDef { def_id } => *def_id,
    }
}

// =============================================================================
// AST lookup helpers: find AST declarations by matching DefEntry name/file
// =============================================================================

pub(super) fn find_fn_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Fn(fn_decl) = decl
                && fn_decl.name == entry.name && fn_decl.name_span == entry.name_span {
                    return Some(fn_decl);
                }
        }
    }
    None
}

pub(super) fn find_struct_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Struct(s) = decl
                && s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
        }
    }
    None
}

pub(super) fn find_entity_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstEntityDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Entity(e) = decl
                && e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
        }
    }
    None
}

pub(super) fn find_enum_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEnumDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Enum(e) = decl
                && e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
        }
    }
    None
}

pub(super) fn find_contract_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstContractDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Contract(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_impl_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstImplDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Impl(i) = decl
                && i.span == entry.span {
                    return Some(i);
                }
        }
    }
    None
}

pub(super) fn find_component_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstComponentDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            match decl {
                crate::ast::decl::AstDecl::Component(c) => {
                    if c.name == entry.name && c.name_span == entry.name_span {
                        return Some(c);
                    }
                }
                crate::ast::decl::AstDecl::Extern(AstExternDecl::Component(_, c)) => {
                    if c.name == entry.name && c.name_span == entry.name_span {
                        return Some(c);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(super) fn find_extern_fn_sig<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnSig> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Extern(AstExternDecl::Fn(_, sig)) = decl
                && sig.name == entry.name && sig.name_span == entry.name_span {
                    return Some(sig);
                }
        }
    }
    None
}

pub(super) fn find_class_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstClassDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Class(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_const_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstConstDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Const(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_global_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstGlobalDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Global(g) = decl
                && g.name == entry.name && g.name_span == entry.name_span {
                    return Some(g);
                }
        }
    }
    None
}

// =============================================================================
// Build helpers: convert AST declarations to type signatures
// =============================================================================

pub(super) fn build_generic_map(generics: &[String]) -> FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect()
}

/// Resolve an AstType to a Ty using the DefMap for named type lookup.
///
/// When `file_id` is `Some`, private types declared in that file (stored in
/// `def_map.file_private`) are also searched. This fixes the `<error>[]` bug
/// for type annotations that refer to file-private enums/structs.
pub fn resolve_ast_type(
    ast_type: &AstType,
    def_map: &DefMap,
    interner: &mut TyInterner,
    generic_map: &FxHashMap<String, u32>,
) -> Ty {
    resolve_ast_type_inner(ast_type, def_map, interner, generic_map, None)
}

/// Variant of `resolve_ast_type` that also searches `def_map.file_private` for
/// the given `file_id`. Use this in the type checker where `ctx.current_file` is
/// available so that private type annotations resolve correctly.
pub fn resolve_ast_type_with_file(
    ast_type: &AstType,
    def_map: &DefMap,
    interner: &mut TyInterner,
    generic_map: &FxHashMap<String, u32>,
    file_id: FileId,
) -> Ty {
    resolve_ast_type_inner(ast_type, def_map, interner, generic_map, Some(file_id))
}

fn resolve_named_def_id(
    name: &str,
    def_map: &DefMap,
    file_id: Option<FileId>,
) -> Option<DefId> {
    // 1. Public FQN table
    if let Some(def_id) = def_map.get(name) {
        return Some(def_id);
    }
    // 2. File-private table (requires file_id context)
    if let Some(fid) = file_id
        && let Some(privs) = def_map.file_private.get(&fid)
            && let Some(&def_id) = privs.get(name) {
                return Some(def_id);
            }
    None
}

fn def_id_to_ty(def_id: DefId, def_map: &DefMap, interner: &mut TyInterner) -> Ty {
    let entry = def_map.get_entry(def_id);
    match entry.kind {
        DefKind::Struct => interner.intern(TyKind::Struct(def_id)),
        DefKind::Class => interner.intern(TyKind::Class(def_id)),
        DefKind::Entity => interner.intern(TyKind::Entity(def_id)),
        DefKind::Enum => interner.intern(TyKind::Enum(def_id)),
        DefKind::Contract => interner.intern(TyKind::Contract(def_id)),
        _ => interner.error(),
    }
}

fn resolve_ast_type_inner(
    ast_type: &AstType,
    def_map: &DefMap,
    interner: &mut TyInterner,
    generic_map: &FxHashMap<String, u32>,
    file_id: Option<FileId>,
) -> Ty {
    match ast_type {
        AstType::Named { name, .. } => {
            // Check if it's a generic param
            if let Some(&idx) = generic_map.get(name.as_str()) {
                return interner.intern(TyKind::GenericParam(idx));
            }
            // Check primitive names
            match name.as_str() {
                "int" => interner.int(),
                "float" => interner.float(),
                "bool" => interner.bool_ty(),
                "string" => interner.string_ty(),
                "void" => interner.void(),
                "Entity" => interner.any_entity(),
                _ => {
                    if let Some(def_id) = resolve_named_def_id(name, def_map, file_id) {
                        def_id_to_ty(def_id, def_map, interner)
                    } else {
                        interner.error()
                    }
                }
            }
        }
        AstType::Generic { name, args, .. } => {
            let resolved_args: Vec<Ty> = args
                .iter()
                .map(|a| resolve_ast_type_inner(a, def_map, interner, generic_map, file_id))
                .collect();

            match name.as_str() {
                "Option" => {
                    if let Some(&inner) = resolved_args.first() {
                        interner.option(inner)
                    } else {
                        interner.error()
                    }
                }
                "Result" => {
                    if resolved_args.len() >= 2 {
                        interner.result(resolved_args[0], resolved_args[1])
                    } else {
                        interner.error()
                    }
                }
                "Array" => {
                    if let Some(&inner) = resolved_args.first() {
                        interner.array(inner)
                    } else {
                        interner.error()
                    }
                }
                "TaskHandle" => {
                    if let Some(&inner) = resolved_args.first() {
                        interner.task_handle(inner)
                    } else {
                        interner.error()
                    }
                }
                _ => {
                    // Named generic type - try DefMap (public + file-private)
                    if let Some(def_id) = resolve_named_def_id(name, def_map, file_id) {
                        def_id_to_ty(def_id, def_map, interner)
                    } else {
                        interner.error()
                    }
                }
            }
        }
        AstType::Array { elem, .. } => {
            let inner = resolve_ast_type_inner(elem, def_map, interner, generic_map, file_id);
            interner.array(inner)
        }
        AstType::Func { params, ret, .. } => {
            let param_tys: Vec<Ty> = params
                .iter()
                .map(|p| resolve_ast_type_inner(p, def_map, interner, generic_map, file_id))
                .collect();
            let ret_ty = match ret {
                Some(r) => resolve_ast_type_inner(r, def_map, interner, generic_map, file_id),
                None => interner.void(),
            };
            interner.func(param_tys, ret_ty)
        }
        AstType::Void { .. } => interner.void(),
    }
}

pub(super) fn build_fn_sig(
    fn_decl: &AstFnDecl,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> FnSig {
    let generic_map = build_generic_map(&entry.generics);
    let mut params = Vec::new();
    let mut self_param = None;

    for param in &fn_decl.params {
        match param {
            AstFnParam::Regular(p) => {
                let ty = resolve_ast_type_with_file(&p.ty, def_map, interner, &generic_map, entry.file_id);
                params.push((p.name.clone(), ty));
            }
            AstFnParam::SelfParam { mutable, .. } => {
                self_param = Some(*mutable);
            }
        }
    }

    let ret = match &fn_decl.return_type {
        Some(rt) => resolve_ast_type_with_file(rt, def_map, interner, &generic_map, entry.file_id),
        None => interner.void(),
    };

    let bounds = build_generic_bounds(&fn_decl.generics, def_map);
    let bound_decl_spans: Vec<SimpleSpan> = fn_decl.generics.iter().map(|gp| gp.span).collect();

    FnSig {
        name: entry.name.clone(),
        params,
        ret,
        generics: entry.generics.clone(),
        self_param,
        bounds,
        bound_decl_spans,
        fn_file: entry.file_id,
    }
}

pub(super) fn build_fn_sig_from_ast_sig(
    sig: &AstFnSig,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> FnSig {
    let generic_map = build_generic_map(&entry.generics);
    let mut params = Vec::new();
    let mut self_param = None;

    for param in &sig.params {
        match param {
            AstFnParam::Regular(p) => {
                let ty = resolve_ast_type_with_file(&p.ty, def_map, interner, &generic_map, entry.file_id);
                params.push((p.name.clone(), ty));
            }
            AstFnParam::SelfParam { mutable, .. } => {
                self_param = Some(*mutable);
            }
        }
    }

    let ret = match &sig.return_type {
        Some(rt) => resolve_ast_type_with_file(rt, def_map, interner, &generic_map, entry.file_id),
        None => interner.void(),
    };

    let bounds = build_generic_bounds(&sig.generics, def_map);
    let bound_decl_spans: Vec<SimpleSpan> = sig.generics.iter().map(|gp| gp.span).collect();

    FnSig {
        name: entry.name.clone(),
        params,
        ret,
        generics: entry.generics.clone(),
        self_param,
        bounds,
        bound_decl_spans,
        fn_file: entry.file_id,
    }
}

pub(super) fn build_generic_bounds(generics: &[AstGenericParam], def_map: &DefMap) -> Vec<Vec<DefId>> {
    generics
        .iter()
        .map(|gp| {
            gp.bounds
                .iter()
                .filter_map(|bound| {
                    if let AstType::Named { name, .. } = bound {
                        def_map.get(name)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect()
}

pub(super) fn build_struct_fields(
    members: &[AstStructMember],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    let mut fields = Vec::new();
    for member in members {
        if let AstStructMember::Field(f) = member {
            let ty = resolve_ast_type_with_file(&f.ty, def_map, interner, &generic_map, entry.file_id);
            fields.push((f.name.clone(), ty, f.name_span));
        }
    }
    fields
}

pub(super) fn build_entity_fields(
    properties: &[AstStructField],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    properties
        .iter()
        .map(|f| {
            let ty = resolve_ast_type_with_file(&f.ty, def_map, interner, &generic_map, entry.file_id);
            (f.name.clone(), ty, f.name_span)
        })
        .collect()
}

pub(super) fn build_enum_variants(
    enum_decl: &AstEnumDecl,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<EnumVariantSig> {
    let generic_map = build_generic_map(&entry.generics);
    enum_decl
        .variants
        .iter()
        .map(|v| {
            let fields = match &v.fields {
                Some(params) => params
                    .iter()
                    .map(|p| {
                        let ty = resolve_ast_type_with_file(&p.ty, def_map, interner, &generic_map, entry.file_id);
                        (p.name.clone(), ty)
                    })
                    .collect(),
                None => Vec::new(),
            };
            EnumVariantSig {
                name: v.name.clone(),
                fields,
            }
        })
        .collect()
}

pub(super) fn build_contract_methods(
    contract_decl: &AstContractDecl,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<FnSig> {
    let generic_map = build_generic_map(&entry.generics);
    let mut methods = Vec::new();
    for member in &contract_decl.members {
        if let AstContractMember::FnSig(sig) = member {
            let mut params = Vec::new();
            let mut self_param = None;
            for param in &sig.params {
                match param {
                    AstFnParam::Regular(p) => {
                        let ty = resolve_ast_type_with_file(&p.ty, def_map, interner, &generic_map, entry.file_id);
                        params.push((p.name.clone(), ty));
                    }
                    AstFnParam::SelfParam { mutable, .. } => {
                        self_param = Some(*mutable);
                    }
                }
            }
            let ret = match &sig.return_type {
                Some(rt) => resolve_ast_type_with_file(rt, def_map, interner, &generic_map, entry.file_id),
                None => interner.void(),
            };
            methods.push(FnSig {
                name: sig.name.clone(),
                params,
                ret,
                generics: sig.generics.iter().map(|g| g.name.clone()).collect(),
                self_param,
                bounds: Vec::new(),
                bound_decl_spans: Vec::new(),
                fn_file: entry.file_id,
            });
        }
    }
    methods
}

pub(super) fn build_impl_entry(
    impl_def_id: DefId,
    impl_decl: &AstImplDecl,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
    env: &mut TypeEnv,
) {
    let generic_map = build_generic_map(&entry.generics);

    // Resolve target type to get target DefId.
    // Handle both `impl Foo` (Named) and `impl<T> Foo<T>` (Generic).
    let target_def_id = match &impl_decl.target {
        AstType::Named { name, .. } => def_map.get(name),
        AstType::Generic { name, .. } => def_map.get(name),
        _ => None,
    };

    // Resolve contract DefId if present
    let contract_def_id = impl_decl.contract.as_ref().and_then(|c| {
        if let AstType::Named { name, .. } = c {
            def_map.get(name)
        } else {
            None
        }
    });

    let mut methods = Vec::new();
    for member in &impl_decl.members {
        if let AstImplMember::Fn(fn_decl) = member {
            let mut params = Vec::new();
            let mut self_param = None;
            for param in &fn_decl.params {
                match param {
                    AstFnParam::Regular(p) => {
                        let ty = resolve_ast_type_with_file(&p.ty, def_map, interner, &generic_map, entry.file_id);
                        params.push((p.name.clone(), ty));
                    }
                    AstFnParam::SelfParam { mutable, .. } => {
                        self_param = Some(*mutable);
                    }
                }
            }
            let ret = match &fn_decl.return_type {
                Some(rt) => resolve_ast_type_with_file(rt, def_map, interner, &generic_map, entry.file_id),
                None => interner.void(),
            };
            let bounds = build_generic_bounds(&fn_decl.generics, def_map);
            methods.push((
                fn_decl.name.clone(),
                FnSig {
                    name: fn_decl.name.clone(),
                    params,
                    ret,
                    generics: fn_decl.generics.iter().map(|g| g.name.clone()).collect(),
                    self_param,
                    bounds,
                    bound_decl_spans: fn_decl.generics.iter().map(|gp| gp.span).collect(),
                    fn_file: entry.file_id,
                },
            ));
        }
    }

    if let Some(target_id) = target_def_id {
        let impl_entry = ImplEntry {
            impl_def_id,
            contract_def_id,
            methods,
        };
        env.impl_index
            .entry(target_id)
            .or_default()
            .push(impl_entry);
    }
}

pub(super) fn build_component_fields(
    members: &[AstComponentMember],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    let mut fields = Vec::new();
    for member in members {
        if let AstComponentMember::Field(f) = member {
            let ty = resolve_ast_type_with_file(&f.ty, def_map, interner, &generic_map, entry.file_id);
            fields.push((f.name.clone(), ty, f.name_span));
        }
    }
    fields
}

// =============================================================================
// Conditional attribute helpers
// =============================================================================

/// Extract the condition name from a `[Conditional("name")]` attribute.
///
/// Returns:
/// - `Some(name)` if `[Conditional("name")]` is present (user-supplied condition name).
/// - `None` if no `[Conditional]` attribute exists in `attrs`, or if the arg is not a string.
pub(super) fn extract_conditional_name(attrs: &[AstAttribute]) -> Option<String> {
    for attr in attrs {
        if attr.name == "Conditional" {
            for arg in &attr.args {
                if let AstAttributeArg::Positional(crate::ast::expr::AstExpr::StringLit { value, .. }) = arg {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}

// =============================================================================
// Deprecated attribute helpers
// =============================================================================

/// Extract the message from a `[Deprecated]` or `[Deprecated("msg")]` attribute.
///
/// Returns:
/// - `Some(msg)` if `[Deprecated("msg")]` is present (user-supplied message).
/// - `Some("")` if `[Deprecated]` (bare, no args) is present.
/// - `None` if no `[Deprecated]` attribute exists in `attrs`.
pub(super) fn extract_deprecated_msg(attrs: &[AstAttribute]) -> Option<String> {
    for attr in attrs {
        if attr.name == "Deprecated" {
            // Bare [Deprecated] with no args
            if attr.args.is_empty() {
                return Some(String::new());
            }
            // [Deprecated("msg")] — look for first positional string arg
            for arg in &attr.args {
                if let AstAttributeArg::Positional(crate::ast::expr::AstExpr::StringLit { value, .. }) = arg {
                    return Some(value.clone());
                }
            }
            // [Deprecated] with non-string or named args — treat as bare
            return Some(String::new());
        }
    }
    None
}

/// Collect attributes from an AST declaration matched by the given DefEntry.
///
/// This is the env_build analogue of `emit::collect::lookup::find_attrs_for_entry`.
/// Duplicated here because that function is `pub(super)` to the emit module.
pub(super) fn find_attrs_for_entry(asts: &[(FileId, &crate::ast::Ast)], entry: &DefEntry) -> Vec<AstAttribute> {
    use crate::ast::decl::AstDecl;
    use crate::ast::decl::AstExternDecl;

    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            match decl {
                AstDecl::Fn(f) if f.name == entry.name && f.name_span == entry.name_span => {
                    return f.attrs.clone();
                }
                AstDecl::Struct(s) if s.name == entry.name && s.name_span == entry.name_span => {
                    return s.attrs.clone();
                }
                AstDecl::Class(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Entity(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Enum(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Contract(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Component(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Extern(AstExternDecl::Fn(_, sig)) if sig.name == entry.name && sig.name_span == entry.name_span => {
                    return sig.attrs.clone();
                }
                AstDecl::Const(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Global(g) if g.name == entry.name && g.name_span == entry.name_span => {
                    return g.attrs.clone();
                }
                _ => {}
            }
        }
    }
    Vec::new()
}
