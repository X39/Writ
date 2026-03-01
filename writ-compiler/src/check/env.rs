//! Type environment: stores materialized type signatures for all definitions.
//!
//! `TypeEnv` is built at the start of type checking by walking the `NameResolvedAst`
//! and the original ASTs. It provides O(1) lookup of function signatures, struct fields,
//! enum variants, impl associations, and more.

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;

use crate::ast::decl::*;
use crate::ast::types::AstType;
use crate::ast::Ast;
use crate::resolve::def_map::{DefEntry, DefId, DefKind, DefMap};
use crate::resolve::ir::NameResolvedAst;

use super::ty::{Ty, TyInterner, TyKind};
use writ_diagnostics::{Diagnostic, FileId};

/// Function signature.
#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub generics: Vec<String>,
    /// If this is a method: Some(mutable) where mutable indicates `mut self`.
    pub self_param: Option<bool>,
    /// Contract bounds per generic param: bounds[i] = DefIds of required contracts for generics[i].
    pub bounds: Vec<Vec<DefId>>,
}

/// An enum variant signature.
#[derive(Debug, Clone)]
pub struct EnumVariantSig {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

/// An impl block entry.
#[derive(Debug, Clone)]
pub struct ImplEntry {
    pub impl_def_id: DefId,
    pub contract_def_id: Option<DefId>,
    pub methods: Vec<(String, FnSig)>,
}

/// The materialized type environment.
#[derive(Debug)]
pub struct TypeEnv {
    pub fn_sigs: FxHashMap<DefId, FnSig>,
    pub struct_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
    pub entity_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
    pub entity_components: FxHashMap<DefId, Vec<String>>,
    pub enum_variants: FxHashMap<DefId, Vec<EnumVariantSig>>,
    pub contract_methods: FxHashMap<DefId, Vec<FnSig>>,
    pub impl_index: FxHashMap<DefId, Vec<ImplEntry>>,
    pub const_types: FxHashMap<DefId, Ty>,
    pub global_types: FxHashMap<DefId, (Ty, bool)>,
    pub component_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
}

impl TypeEnv {
    /// Build the type environment from the resolved AST and original ASTs.
    pub fn build(
        resolved: &NameResolvedAst,
        asts: &[(FileId, &Ast)],
        interner: &mut TyInterner,
    ) -> (TypeEnv, Vec<Diagnostic>) {
        let diags = Vec::new();
        let mut env = TypeEnv {
            fn_sigs: FxHashMap::default(),
            struct_fields: FxHashMap::default(),
            entity_fields: FxHashMap::default(),
            entity_components: FxHashMap::default(),
            enum_variants: FxHashMap::default(),
            contract_methods: FxHashMap::default(),
            impl_index: FxHashMap::default(),
            const_types: FxHashMap::default(),
            global_types: FxHashMap::default(),
            component_fields: FxHashMap::default(),
        };

        // Walk each resolved decl and find matching AST decls
        for decl in &resolved.decls {
            let def_id = decl_def_id(decl);
            let entry = resolved.def_map.get_entry(def_id);

            match &entry.kind {
                DefKind::Fn => {
                    if let Some(fn_decl) = find_fn_decl(asts, entry) {
                        let sig = build_fn_sig(fn_decl, entry, &resolved.def_map, interner);
                        env.fn_sigs.insert(def_id, sig);
                    }
                }
                DefKind::Struct => {
                    if let Some(struct_decl) = find_struct_decl(asts, entry) {
                        let fields = build_struct_fields(
                            &struct_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::Entity => {
                    if let Some(entity_decl) = find_entity_decl(asts, entry) {
                        let fields = build_entity_fields(
                            &entity_decl.properties,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.entity_fields.insert(def_id, fields);

                        let components: Vec<String> = entity_decl
                            .component_slots
                            .iter()
                            .map(|s| s.component.clone())
                            .collect();
                        env.entity_components.insert(def_id, components);
                    }
                }
                DefKind::Enum => {
                    if let Some(enum_decl) = find_enum_decl(asts, entry) {
                        let variants =
                            build_enum_variants(enum_decl, entry, &resolved.def_map, interner);
                        env.enum_variants.insert(def_id, variants);
                    }
                }
                DefKind::Contract => {
                    if let Some(contract_decl) = find_contract_decl(asts, entry) {
                        let methods = build_contract_methods(
                            contract_decl,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.contract_methods.insert(def_id, methods);
                    }
                }
                DefKind::Impl => {
                    if let Some(impl_decl) = find_impl_decl(asts, entry) {
                        build_impl_entry(
                            def_id,
                            impl_decl,
                            entry,
                            &resolved.def_map,
                            interner,
                            &mut env,
                        );
                    }
                }
                DefKind::Component | DefKind::ExternComponent => {
                    if let Some(comp_decl) = find_component_decl(asts, entry) {
                        let fields = build_component_fields(
                            &comp_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.component_fields.insert(def_id, fields);
                    }
                }
                DefKind::ExternFn => {
                    if let Some(fn_sig) = find_extern_fn_sig(asts, entry) {
                        let sig =
                            build_fn_sig_from_ast_sig(fn_sig, entry, &resolved.def_map, interner);
                        env.fn_sigs.insert(def_id, sig);
                    }
                }
                DefKind::ExternStruct => {
                    if let Some(struct_decl) = find_extern_struct_decl(asts, entry) {
                        let fields = build_struct_fields(
                            &struct_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::Const => {
                    if let Some(const_decl) = find_const_decl(asts, entry) {
                        let generic_map = FxHashMap::default();
                        let ty = resolve_ast_type(&const_decl.ty, &resolved.def_map, interner, &generic_map);
                        env.const_types.insert(def_id, ty);
                    }
                }
                DefKind::Global => {
                    if let Some(global_decl) = find_global_decl(asts, entry) {
                        let generic_map = FxHashMap::default();
                        let ty = resolve_ast_type(&global_decl.ty, &resolved.def_map, interner, &generic_map);
                        env.global_types.insert(def_id, (ty, true));
                    }
                }
            }
        }

        (env, diags)
    }
}

/// Extract the DefId from a ResolvedDecl.
fn decl_def_id(decl: &crate::resolve::ir::ResolvedDecl) -> DefId {
    use crate::resolve::ir::ResolvedDecl;
    match decl {
        ResolvedDecl::Fn { def_id }
        | ResolvedDecl::Struct { def_id }
        | ResolvedDecl::Entity { def_id }
        | ResolvedDecl::Enum { def_id }
        | ResolvedDecl::Contract { def_id }
        | ResolvedDecl::Impl { def_id }
        | ResolvedDecl::Component { def_id }
        | ResolvedDecl::ExternFn { def_id }
        | ResolvedDecl::ExternStruct { def_id }
        | ResolvedDecl::ExternComponent { def_id }
        | ResolvedDecl::Const { def_id }
        | ResolvedDecl::Global { def_id } => *def_id,
    }
}

// =============================================================================
// AST lookup helpers: find AST declarations by matching DefEntry name/file
// =============================================================================

fn find_fn_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Fn(fn_decl) = decl {
                if fn_decl.name == entry.name && fn_decl.name_span == entry.name_span {
                    return Some(fn_decl);
                }
            }
        }
    }
    None
}

fn find_struct_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Struct(s) = decl {
                if s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn find_entity_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstEntityDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Entity(e) = decl {
                if e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
            }
        }
    }
    None
}

fn find_enum_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEnumDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Enum(e) = decl {
                if e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
            }
        }
    }
    None
}

fn find_contract_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstContractDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Contract(c) = decl {
                if c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn find_impl_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstImplDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Impl(i) = decl {
                if i.span == entry.span {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn find_component_decl<'a>(
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

fn find_extern_fn_sig<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnSig> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Extern(AstExternDecl::Fn(_, sig)) = decl {
                if sig.name == entry.name && sig.name_span == entry.name_span {
                    return Some(sig);
                }
            }
        }
    }
    None
}

fn find_extern_struct_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Extern(AstExternDecl::Struct(_, s)) = decl {
                if s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn find_const_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstConstDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Const(c) = decl {
                if c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn find_global_decl<'a>(
    asts: &'a [(FileId, &Ast)],
    entry: &DefEntry,
) -> Option<&'a AstGlobalDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            if let crate::ast::decl::AstDecl::Global(g) = decl {
                if g.name == entry.name && g.name_span == entry.name_span {
                    return Some(g);
                }
            }
        }
    }
    None
}

// =============================================================================
// Build helpers: convert AST declarations to type signatures
// =============================================================================

fn build_generic_map(generics: &[String]) -> FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect()
}

/// Resolve an AstType to a Ty using the DefMap for named type lookup.
pub fn resolve_ast_type(
    ast_type: &AstType,
    def_map: &DefMap,
    interner: &mut TyInterner,
    generic_map: &FxHashMap<String, u32>,
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
                _ => {
                    // Try to resolve from DefMap
                    if let Some(def_id) = def_map.get(name) {
                        let entry = def_map.get_entry(def_id);
                        match entry.kind {
                            DefKind::Struct | DefKind::ExternStruct => {
                                interner.intern(TyKind::Struct(def_id))
                            }
                            DefKind::Entity => interner.intern(TyKind::Entity(def_id)),
                            DefKind::Enum => interner.intern(TyKind::Enum(def_id)),
                            _ => interner.error(),
                        }
                    } else {
                        // Try file-private lookup - not available without file_id context.
                        // For now just look up by fqn  with various namespace prefixes
                        // This is a best-effort lookup; in practice the resolver has already
                        // resolved all names.
                        interner.error()
                    }
                }
            }
        }
        AstType::Generic { name, args, .. } => {
            let resolved_args: Vec<Ty> = args
                .iter()
                .map(|a| resolve_ast_type(a, def_map, interner, generic_map))
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
                    // Named generic type - try DefMap
                    if let Some(def_id) = def_map.get(name) {
                        let entry = def_map.get_entry(def_id);
                        match entry.kind {
                            DefKind::Struct | DefKind::ExternStruct => {
                                interner.intern(TyKind::Struct(def_id))
                            }
                            DefKind::Entity => interner.intern(TyKind::Entity(def_id)),
                            DefKind::Enum => interner.intern(TyKind::Enum(def_id)),
                            _ => interner.error(),
                        }
                    } else {
                        interner.error()
                    }
                }
            }
        }
        AstType::Array { elem, .. } => {
            let inner = resolve_ast_type(elem, def_map, interner, generic_map);
            interner.array(inner)
        }
        AstType::Func { params, ret, .. } => {
            let param_tys: Vec<Ty> = params
                .iter()
                .map(|p| resolve_ast_type(p, def_map, interner, generic_map))
                .collect();
            let ret_ty = match ret {
                Some(r) => resolve_ast_type(r, def_map, interner, generic_map),
                None => interner.void(),
            };
            interner.func(param_tys, ret_ty)
        }
        AstType::Void { .. } => interner.void(),
    }
}

fn build_fn_sig(
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
                let ty = resolve_ast_type(&p.ty, def_map, interner, &generic_map);
                params.push((p.name.clone(), ty));
            }
            AstFnParam::SelfParam { mutable, .. } => {
                self_param = Some(*mutable);
            }
        }
    }

    let ret = match &fn_decl.return_type {
        Some(rt) => resolve_ast_type(rt, def_map, interner, &generic_map),
        None => interner.void(),
    };

    let bounds = build_generic_bounds(&fn_decl.generics, def_map);

    FnSig {
        name: entry.name.clone(),
        params,
        ret,
        generics: entry.generics.clone(),
        self_param,
        bounds,
    }
}

fn build_fn_sig_from_ast_sig(
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
                let ty = resolve_ast_type(&p.ty, def_map, interner, &generic_map);
                params.push((p.name.clone(), ty));
            }
            AstFnParam::SelfParam { mutable, .. } => {
                self_param = Some(*mutable);
            }
        }
    }

    let ret = match &sig.return_type {
        Some(rt) => resolve_ast_type(rt, def_map, interner, &generic_map),
        None => interner.void(),
    };

    let bounds = build_generic_bounds(&sig.generics, def_map);

    FnSig {
        name: entry.name.clone(),
        params,
        ret,
        generics: entry.generics.clone(),
        self_param,
        bounds,
    }
}

fn build_generic_bounds(generics: &[AstGenericParam], def_map: &DefMap) -> Vec<Vec<DefId>> {
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

fn build_struct_fields(
    members: &[AstStructMember],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    let mut fields = Vec::new();
    for member in members {
        if let AstStructMember::Field(f) = member {
            let ty = resolve_ast_type(&f.ty, def_map, interner, &generic_map);
            fields.push((f.name.clone(), ty, f.name_span));
        }
    }
    fields
}

fn build_entity_fields(
    properties: &[AstStructField],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    properties
        .iter()
        .map(|f| {
            let ty = resolve_ast_type(&f.ty, def_map, interner, &generic_map);
            (f.name.clone(), ty, f.name_span)
        })
        .collect()
}

fn build_enum_variants(
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
                        let ty = resolve_ast_type(&p.ty, def_map, interner, &generic_map);
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

fn build_contract_methods(
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
                        let ty = resolve_ast_type(&p.ty, def_map, interner, &generic_map);
                        params.push((p.name.clone(), ty));
                    }
                    AstFnParam::SelfParam { mutable, .. } => {
                        self_param = Some(*mutable);
                    }
                }
            }
            let ret = match &sig.return_type {
                Some(rt) => resolve_ast_type(rt, def_map, interner, &generic_map),
                None => interner.void(),
            };
            methods.push(FnSig {
                name: sig.name.clone(),
                params,
                ret,
                generics: sig.generics.iter().map(|g| g.name.clone()).collect(),
                self_param,
                bounds: Vec::new(),
            });
        }
    }
    methods
}

fn build_impl_entry(
    impl_def_id: DefId,
    impl_decl: &AstImplDecl,
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
    env: &mut TypeEnv,
) {
    let generic_map = build_generic_map(&entry.generics);

    // Resolve target type to get target DefId
    let target_def_id = match &impl_decl.target {
        AstType::Named { name, .. } => def_map.get(name),
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
                        let ty = resolve_ast_type(&p.ty, def_map, interner, &generic_map);
                        params.push((p.name.clone(), ty));
                    }
                    AstFnParam::SelfParam { mutable, .. } => {
                        self_param = Some(*mutable);
                    }
                }
            }
            let ret = match &fn_decl.return_type {
                Some(rt) => resolve_ast_type(rt, def_map, interner, &generic_map),
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

fn build_component_fields(
    members: &[AstComponentMember],
    entry: &DefEntry,
    def_map: &DefMap,
    interner: &mut TyInterner,
) -> Vec<(String, Ty, SimpleSpan)> {
    let generic_map = build_generic_map(&entry.generics);
    let mut fields = Vec::new();
    for member in members {
        if let AstComponentMember::Field(f) = member {
            let ty = resolve_ast_type(&f.ty, def_map, interner, &generic_map);
            fields.push((f.name.clone(), ty, f.name_span));
        }
    }
    fields
}

/// Local variable environment with scoped lookup.
#[derive(Debug, Clone)]
pub struct LocalEnv {
    scopes: Vec<Vec<(String, Ty, Mutability, SimpleSpan)>>,
}

/// Mutability of a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

impl LocalEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![Vec::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: String, ty: Ty, mutability: Mutability, span: SimpleSpan) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push((name, ty, mutability, span));
        }
    }

    pub fn lookup(&self, name: &str) -> Option<(Ty, Mutability, SimpleSpan)> {
        for scope in self.scopes.iter().rev() {
            for (n, ty, m, sp) in scope.iter().rev() {
                if n == name {
                    return Some((*ty, *m, *sp));
                }
            }
        }
        None
    }
}
