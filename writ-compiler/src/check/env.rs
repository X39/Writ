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
use crate::resolve::def_map::{DefId, DefKind, DefMap};
use crate::resolve::ir::NameResolvedAst;

use super::ty::{Ty, TyInterner};
use super::env_build;
use writ_diagnostics::{Diagnostic, FileId, code};

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
    /// Spans of the generic param declarations, parallel to `bounds`.
    /// `bound_decl_spans[i]` is the span of `AstGenericParam[i]` — used
    /// for secondary labels in UnsatisfiedBound diagnostics.
    pub bound_decl_spans: Vec<SimpleSpan>,
    /// File in which this function is declared (for cross-file secondary labels).
    pub fn_file: FileId,
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
    /// Deprecated item messages, keyed by DefId. Value is the user's message string,
    /// or empty string for bare `[Deprecated]` (no message). Only items with the
    /// `[Deprecated]` attribute have entries here.
    pub deprecated_items: FxHashMap<DefId, String>,
    /// [Conditional("name")] functions mapped to their condition string.
    /// Only populated for functions that carry a [Conditional] attribute.
    pub conditional_fns: FxHashMap<DefId, String>,
    /// Maps conditional fn DefId -> fallback fn DefId (same name, same sig, no [Conditional]).
    /// Populated during the fallback verification pass. Empty when no conditions are used.
    pub fallback_for_conditional: FxHashMap<DefId, DefId>,
    /// Prelude enum variant names keyed by type name. Populated at build time
    /// so that LSP completions can look up Option/Result variants without hardcoding.
    pub prelude_enum_variants: FxHashMap<String, Vec<String>>,
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
            deprecated_items: FxHashMap::default(),
            conditional_fns: FxHashMap::default(),
            fallback_for_conditional: FxHashMap::default(),
            prelude_enum_variants: {
                let mut m = FxHashMap::default();
                m.insert("Option".to_string(), vec!["Some".to_string(), "None".to_string()]);
                m.insert("Result".to_string(), vec!["Ok".to_string(), "Err".to_string()]);
                m
            },
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
                DefKind::AttributeDef => {
                    // No type-env entries needed for attribute declarations in this phase.
                    // Plan 02 will add parameter type tracking here.
                }
            }
        }

        // Second pass: populate deprecated_items from [Deprecated] attributes.
        // Runs after the main declaration pass so all DefIds are already registered.
        for decl in &resolved.decls {
            let def_id = env_build::decl_def_id(decl);
            let entry = resolved.def_map.get_entry(def_id);
            let attrs = env_build::find_attrs_for_entry(asts, entry);
            if let Some(msg) = env_build::extract_deprecated_msg(&attrs) {
                env.deprecated_items.insert(def_id, msg);
            }
        }

        // Third pass: populate conditional_fns from [Conditional] attributes.
        // Parallel to the deprecated_items pass above.
        for decl in &resolved.decls {
            let def_id = env_build::decl_def_id(decl);
            let entry = resolved.def_map.get_entry(def_id);
            if !matches!(entry.kind, DefKind::Fn) {
                continue;
            }
            let attrs = env_build::find_attrs_for_entry(asts, entry);
            if let Some(cond_name) = env_build::extract_conditional_name(&attrs) {
                env.conditional_fns.insert(def_id, cond_name);
            }
        }

        // Fourth pass: fallback verification for [Conditional] functions.
        // For each conditional fn, find a non-conditional overload with the same signature.
        // If none found, emit E0009. If found, record in fallback_for_conditional.
        let mut diags = diags;
        {
            // Collect (cond_def_id, cond_name, entry_name, entry_namespace, entry_file_id, entry_name_span, sig)
            // We collect first to avoid borrow conflicts.
            let conditional_entries: Vec<(DefId, String, String, String, FileId, chumsky::span::SimpleSpan)> =
                env.conditional_fns.iter().map(|(&def_id, cond_name)| {
                    let entry = resolved.def_map.get_entry(def_id);
                    (def_id, cond_name.clone(), entry.name.clone(), entry.namespace.clone(), entry.file_id, entry.name_span)
                }).collect();

            for (cond_def_id, cond_name, entry_name, entry_namespace, entry_file_id, entry_name_span) in conditional_entries {
                // Get the overload set for this function's FQN.
                let fqn = if entry_namespace.is_empty() {
                    entry_name.clone()
                } else {
                    format!("{}::{}", entry_namespace, entry_name)
                };

                let all_overloads: Vec<DefId> = if let Some(overloads) = resolved.def_map.fn_overloads.get(&fqn) {
                    overloads.clone()
                } else if let Some(&single_id) = resolved.def_map.by_fqn.get(&fqn) {
                    vec![single_id]
                } else {
                    // Private fn: check file_private
                    let mut found = vec![];
                    if let Some(privs) = resolved.def_map.file_private.get(&entry_file_id) {
                        if let Some(&id) = privs.get(&entry_name) {
                            found.push(id);
                        }
                    }
                    // Also check private overloads by file_private_overloads if available
                    // (fall back to private_fn_overloads key format)
                    let priv_key = format!("{}@{}", entry_name, entry_file_id.0);
                    if let Some(overloads) = resolved.def_map.fn_overloads.get(&priv_key) {
                        found = overloads.clone();
                    }
                    found
                };

                // Get the conditional fn's signature.
                let cond_sig = match env.fn_sigs.get(&cond_def_id) {
                    Some(s) => s.clone(),
                    None => continue, // no sig means parse error; skip
                };

                // Find a non-conditional overload with matching signature.
                let fallback = all_overloads.iter().copied().find(|&other_id| {
                    if other_id == cond_def_id {
                        return false; // skip self
                    }
                    if env.conditional_fns.contains_key(&other_id) {
                        return false; // skip other conditional fns
                    }
                    // Check signature: same param count, types, return type, generic count.
                    match env.fn_sigs.get(&other_id) {
                        Some(other_sig) => {
                            other_sig.params.len() == cond_sig.params.len()
                                && other_sig.ret == cond_sig.ret
                                && other_sig.generics.len() == cond_sig.generics.len()
                                && other_sig.params.iter().zip(cond_sig.params.iter())
                                    .all(|((_, t1), (_, t2))| t1 == t2)
                        }
                        None => false,
                    }
                });

                match fallback {
                    Some(fallback_id) => {
                        env.fallback_for_conditional.insert(cond_def_id, fallback_id);
                    }
                    None => {
                        diags.push(
                            Diagnostic::error(
                                code::E0009,
                                format!(
                                    "[Conditional(\"{cond_name}\")] function '{entry_name}' has no matching non-conditional fallback"
                                ),
                            )
                            .with_primary(entry_file_id, entry_name_span, "conditional function defined here")
                            .build(),
                        );
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
                    bound_decl_spans: vec![],
                    fn_file: FileId(u32::MAX),
                };
                env.fn_sigs.insert(def_id, sig);
            }
        }

        // Inject synthetic FnSig entries for dialogue builtins (say, say_localized, choice, ChoiceOption).
        // These are injected by inject_dialogue_namespace in the resolver — no AST entry exists.
        let int_ty = interner.int();
        let entity_ty = interner.any_entity();
        let fn_void_void = interner.func(vec![], void_ty);
        let array_int_ty = interner.array(int_ty);

        #[allow(clippy::type_complexity)] // dialogue signature table is a static data literal
        let dialogue_sigs: &[(&str, Vec<(&str, Ty)>, Ty)] = &[
            ("say", vec![("speaker", entity_ty), ("text", string_ty)], void_ty),
            ("say_localized", vec![("speaker", entity_ty), ("key", string_ty), ("fallback", string_ty)], void_ty),
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
                    bound_decl_spans: vec![],
                    fn_file: FileId(u32::MAX),
                };
                env.fn_sigs.insert(def_id, sig);
            }
        }

        // Validate that all contract impl blocks implement all required methods.
        let impl_errors = env.validate_contract_impls(&resolved.def_map);
        for err in impl_errors {
            diags.push(err.into());
        }

        (env, diags)
    }

    /// Check every `impl Contract for Type` block for completeness.
    ///
    /// For each impl that has a `contract_def_id`, look up the contract's required
    /// method names and compare against the methods actually present in the impl.
    /// Returns a list of `TypeError::IncompleteContractImpl` for each missing method set.
    pub fn validate_contract_impls(&self, def_map: &DefMap) -> Vec<super::error::TypeError> {
        let mut errors = Vec::new();

        for (target_def_id, impl_entries) in &self.impl_index {
            for impl_entry in impl_entries {
                let contract_def_id = match impl_entry.contract_def_id {
                    Some(id) => id,
                    None => continue, // plain `impl Type {}` blocks have no contract to check
                };

                // Get required methods from the contract definition.
                let required_methods = match self.contract_methods.get(&contract_def_id) {
                    Some(methods) => methods,
                    None => continue, // contract not found in env (parse/resolve error path)
                };

                // Collect method names provided by this impl block.
                let provided: std::collections::HashSet<&str> =
                    impl_entry.methods.iter().map(|(name, _)| name.as_str()).collect();

                // Find missing method names.
                let missing: Vec<String> = required_methods
                    .iter()
                    .filter(|req| !provided.contains(req.name.as_str()))
                    .map(|req| req.name.clone())
                    .collect();

                if missing.is_empty() {
                    continue;
                }

                // Resolve human-readable names from DefMap.
                let impl_entry_entry = def_map.get_entry(impl_entry.impl_def_id);
                let contract_entry = def_map.get_entry(contract_def_id);

                // The target type name comes from the impl_index key (target_def_id),
                // NOT from the impl block's DefEntry (which has a synthetic name "impl#N").
                let ty_name = def_map.get_entry(*target_def_id).name.clone();
                let contract_name = contract_entry.name.clone();

                errors.push(super::error::TypeError::IncompleteContractImpl {
                    ty_name,
                    contract_name,
                    missing_methods: missing,
                    span: impl_entry_entry.span,
                    file: impl_entry_entry.file_id,
                });
            }
        }

        errors
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
