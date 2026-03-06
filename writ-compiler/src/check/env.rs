//! Type environment: stores materialized type signatures for all definitions.
//!
//! `TypeEnv` is built at the start of type checking by walking the `NameResolvedAst`
//! and the original ASTs. It provides O(1) lookup of function signatures, struct fields,
//! enum variants, impl associations, and more.
//!
//! Builder helpers live in `env_build.rs` (declared as `pub(crate) mod env_build` in
//! `check/mod.rs`). Public callers only need `TypeEnv`, `FnSig`, `resolve_ast_type`, etc.

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;

use crate::ast::Ast;
use crate::resolve::def_map::{DefId, DefKind};
use crate::resolve::ir::NameResolvedAst;

use super::ty::{Ty, TyInterner};
use super::env_build;
use writ_diagnostics::{Diagnostic, FileId};

// Re-export the public resolve functions so callers who import from `check::env`
// continue to work without changes.
pub use super::env_build::{resolve_ast_type, resolve_ast_type_with_file};

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
            let def_id = env_build::decl_def_id(decl);
            let entry = resolved.def_map.get_entry(def_id);

            match &entry.kind {
                DefKind::Fn => {
                    if let Some(fn_decl) = env_build::find_fn_decl(asts, entry) {
                        let sig = env_build::build_fn_sig(fn_decl, entry, &resolved.def_map, interner);
                        env.fn_sigs.insert(def_id, sig);
                    }
                }
                DefKind::Struct => {
                    if let Some(struct_decl) = env_build::find_struct_decl(asts, entry) {
                        let fields = env_build::build_struct_fields(
                            &struct_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::Class => {
                    if let Some(class_decl) = env_build::find_class_decl(asts, entry) {
                        let fields = env_build::build_struct_fields(
                            &class_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::Entity => {
                    if let Some(entity_decl) = env_build::find_entity_decl(asts, entry) {
                        let fields = env_build::build_entity_fields(
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
                    if let Some(enum_decl) = env_build::find_enum_decl(asts, entry) {
                        let variants =
                            env_build::build_enum_variants(enum_decl, entry, &resolved.def_map, interner);
                        env.enum_variants.insert(def_id, variants);
                    }
                }
                DefKind::Contract => {
                    if let Some(contract_decl) = env_build::find_contract_decl(asts, entry) {
                        let methods = env_build::build_contract_methods(
                            contract_decl,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.contract_methods.insert(def_id, methods);
                    }
                }
                DefKind::Impl => {
                    if let Some(impl_decl) = env_build::find_impl_decl(asts, entry) {
                        env_build::build_impl_entry(
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
                    if let Some(comp_decl) = env_build::find_component_decl(asts, entry) {
                        let fields = env_build::build_component_fields(
                            &comp_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.component_fields.insert(def_id, fields);
                    }
                }
                DefKind::ExternFn => {
                    if let Some(fn_sig) = env_build::find_extern_fn_sig(asts, entry) {
                        let sig =
                            env_build::build_fn_sig_from_ast_sig(fn_sig, entry, &resolved.def_map, interner);
                        env.fn_sigs.insert(def_id, sig);
                    }
                }
                DefKind::ExternStruct => {
                    if let Some(struct_decl) = env_build::find_extern_struct_decl(asts, entry) {
                        let fields = env_build::build_struct_fields(
                            &struct_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::ExternClass => {
                    if let Some(class_decl) = env_build::find_extern_class_decl(asts, entry) {
                        let fields = env_build::build_struct_fields(
                            &class_decl.members,
                            entry,
                            &resolved.def_map,
                            interner,
                        );
                        env.struct_fields.insert(def_id, fields);
                    }
                }
                DefKind::Const => {
                    if let Some(const_decl) = env_build::find_const_decl(asts, entry) {
                        let generic_map = FxHashMap::default();
                        let ty = env_build::resolve_ast_type_with_file(&const_decl.ty, &resolved.def_map, interner, &generic_map, entry.file_id);
                        env.const_types.insert(def_id, ty);
                    }
                }
                DefKind::Global => {
                    if let Some(global_decl) = env_build::find_global_decl(asts, entry) {
                        let generic_map = FxHashMap::default();
                        let ty = env_build::resolve_ast_type_with_file(&global_decl.ty, &resolved.def_map, interner, &generic_map, entry.file_id);
                        env.global_types.insert(def_id, (ty, true));
                    }
                }
            }
        }

        // Inject synthetic FnSig entries for log-level builtins (log::trace .. log::error).
        // These are injected by inject_log_namespace in the resolver — no AST entry exists,
        // so we construct the FnSig directly: (msg: string) -> void.
        let string_ty = interner.string_ty();
        let void_ty = interner.void();
        for &level in crate::resolve::prelude::LOG_NAMESPACE_LEVELS {
            let fqn = format!("log::{}", level);
            if let Some(def_id) = resolved.def_map.get(&fqn) {
                let sig = FnSig {
                    name: fqn,
                    params: vec![("msg".to_string(), string_ty)],
                    ret: void_ty,
                    generics: vec![],
                    self_param: None,
                    bounds: vec![],
                };
                env.fn_sigs.insert(def_id, sig);
            }
        }

        // Inject synthetic FnSig entries for dialogue builtins (say, say_localized, choice, ChoiceOption).
        // These are injected by inject_dialogue_namespace in the resolver — no AST entry exists.
        let int_ty = interner.int();
        let fn_void_void = interner.func(vec![], void_ty);
        let array_int_ty = interner.array(int_ty);

        #[allow(clippy::type_complexity)] // dialogue signature table is a static data literal
        let dialogue_sigs: &[(&str, Vec<(&str, Ty)>, Ty)] = &[
            ("say", vec![("text", string_ty)], void_ty),
            ("say_localized", vec![("key", string_ty), ("locale", string_ty)], void_ty),
            ("choice", vec![("options", array_int_ty)], int_ty),
            ("ChoiceOption", vec![("label", string_ty), ("key", string_ty), ("body", fn_void_void)], int_ty),
        ];

        for (name, params, ret) in dialogue_sigs {
            if let Some(def_id) = resolved.def_map.get(name) {
                // Only inject for synthetic entries (FileId::MAX sentinel).
                // User-declared `extern fn say(...)` gets its sig from the AST.
                let entry = resolved.def_map.get_entry(def_id);
                if entry.file_id != FileId(u32::MAX) {
                    continue;
                }
                let sig = FnSig {
                    name: name.to_string(),
                    params: params.iter().map(|(n, t)| (n.to_string(), *t)).collect(),
                    ret: *ret,
                    generics: vec![],
                    self_param: None,
                    bounds: vec![],
                };
                env.fn_sigs.insert(def_id, sig);
            }
        }

        (env, diags)
    }
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

impl Default for LocalEnv {
    fn default() -> Self {
        Self::new()
    }
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
