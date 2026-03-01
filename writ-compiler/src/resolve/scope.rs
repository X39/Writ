//! Scope chain for name resolution.
//!
//! The ScopeChain implements layered name lookup during Pass 2 resolution.
//! Lookup order: generics -> primitives -> prelude -> current namespace -> usings -> root -> fail.

use std::cell::Cell;

use chumsky::span::SimpleSpan;
use writ_diagnostics::{Diagnostic, FileId};

use crate::resolve::def_map::{DefId, DefKind, DefMap, DefVis};
use crate::resolve::error::ResolutionError;
use crate::resolve::ir::PrimitiveTag;
use crate::resolve::prelude;

/// A using import entry.
#[derive(Debug)]
pub struct UsingEntry {
    /// Local name (alias or terminal path segment).
    pub alias: String,
    /// Target namespace for namespace imports (e.g., "survival").
    pub target_ns: Option<String>,
    /// Target FQN for specific imports (e.g., "survival::HealthPotion").
    pub target_fqn: Option<String>,
    /// Span of the using declaration.
    pub span: SimpleSpan,
    /// Whether this import was used (for W0001 detection).
    pub used: Cell<bool>,
}

/// A scope layer in the scope chain.
#[derive(Debug, Clone)]
pub enum ScopeLayer {
    /// Generic type parameters (e.g., T, U from `fn foo<T, U>`).
    GenericParams(Vec<(String, SimpleSpan)>),
    /// Local variable bindings (from let, for, fn params).
    Locals(Vec<(String, SimpleSpan)>),
}

/// Result of a name lookup.
#[derive(Debug, Clone)]
pub enum LookupResult {
    /// Resolved to a DefId.
    Def(DefId),
    /// Resolved to a primitive type.
    Primitive(PrimitiveTag),
    /// Resolved to a generic type parameter.
    GenericParam(String),
    /// Resolved to a prelude type (Option, Result, etc.) -- represented by name since
    /// these don't have DefIds in the DefMap (they're built-in).
    PreludeType(String),
    /// Resolved to a prelude contract.
    PreludeContract(String),
    /// Not found.
    NotFound,
    /// Ambiguous (multiple candidates).
    Ambiguous(Vec<(DefId, String)>),
    /// Visibility violation.
    VisibilityError(DefId),
}

/// The scope chain for resolving names during Pass 2.
pub struct ScopeChain<'def> {
    /// Reference to the DefMap from Pass 1.
    pub def_map: &'def DefMap,
    /// The file being resolved.
    pub current_file: FileId,
    /// The current namespace (e.g., "survival::combat").
    pub current_ns: String,
    /// Active using imports.
    pub active_usings: Vec<UsingEntry>,
    /// Stack of scope layers.
    pub layers: Vec<ScopeLayer>,
    /// Current self type (inside impl/entity method).
    pub self_type: Option<DefId>,
}

impl<'def> ScopeChain<'def> {
    /// Create a new scope chain for a file.
    pub fn new(def_map: &'def DefMap, current_file: FileId, current_ns: String) -> Self {
        Self {
            def_map,
            current_file,
            current_ns,
            active_usings: Vec::new(),
            layers: Vec::new(),
            self_type: None,
        }
    }

    /// Push a generic params layer.
    pub fn push_generics(&mut self, params: Vec<(String, SimpleSpan)>) {
        self.layers.push(ScopeLayer::GenericParams(params));
    }

    /// Push a locals layer.
    pub fn push_locals(&mut self) {
        self.layers.push(ScopeLayer::Locals(Vec::new()));
    }

    /// Add a local binding to the current locals layer.
    pub fn add_local(&mut self, name: String, span: SimpleSpan) {
        for layer in self.layers.iter_mut().rev() {
            if let ScopeLayer::Locals(locals) = layer {
                locals.push((name, span));
                return;
            }
        }
        // No locals layer -- push one
        self.layers
            .push(ScopeLayer::Locals(vec![(name, span)]));
    }

    /// Pop the top scope layer.
    pub fn pop_layer(&mut self) {
        self.layers.pop();
    }

    /// Resolve an unqualified type name.
    pub fn resolve_type(&self, name: &str) -> LookupResult {
        // 1. Check generic params (innermost first)
        for layer in self.layers.iter().rev() {
            if let ScopeLayer::GenericParams(params) = layer {
                if params.iter().any(|(n, _)| n == name) {
                    return LookupResult::GenericParam(name.to_string());
                }
            }
        }

        // 2. Check primitives
        if let Some(tag) = primitive_tag(name) {
            return LookupResult::Primitive(tag);
        }

        // 3. Check prelude types
        if prelude::PRELUDE_TYPE_NAMES.contains(&name) {
            return LookupResult::PreludeType(name.to_string());
        }

        // 3b. Check prelude contracts
        if prelude::PRELUDE_CONTRACT_NAMES.contains(&name) {
            return LookupResult::PreludeContract(name.to_string());
        }

        // 4. Check current namespace (FQN)
        if !self.current_ns.is_empty() {
            let fqn = format!("{}::{name}", self.current_ns);
            if let Some(def_id) = self.def_map.get(&fqn) {
                let entry = self.def_map.get_entry(def_id);
                if entry.vis == DefVis::Private && entry.file_id != self.current_file {
                    return LookupResult::VisibilityError(def_id);
                }
                return LookupResult::Def(def_id);
            }
        }

        // 4b. Check file-private in current file
        if let Some(privates) = self.def_map.file_private.get(&self.current_file) {
            if let Some(&def_id) = privates.get(name) {
                return LookupResult::Def(def_id);
            }
        }

        // 5. Check using imports
        let mut using_matches: Vec<(DefId, String)> = Vec::new();
        for entry in &self.active_usings {
            if let Some(ref target_fqn) = entry.target_fqn {
                // Specific import: using survival::HealthPotion;
                if entry.alias == name {
                    if let Some(def_id) = self.def_map.get(target_fqn) {
                        let def = self.def_map.get_entry(def_id);
                        if def.vis == DefVis::Private && def.file_id != self.current_file {
                            continue; // Skip private defs
                        }
                        entry.used.set(true);
                        using_matches.push((def_id, target_fqn.clone()));
                    }
                }
            } else if let Some(ref target_ns) = entry.target_ns {
                // Namespace import: using survival;
                let fqn = format!("{target_ns}::{name}");
                if let Some(def_id) = self.def_map.get(&fqn) {
                    let def = self.def_map.get_entry(def_id);
                    if def.vis == DefVis::Private && def.file_id != self.current_file {
                        continue;
                    }
                    entry.used.set(true);
                    using_matches.push((def_id, fqn));
                }
            }
        }

        match using_matches.len() {
            0 => {}
            1 => return LookupResult::Def(using_matches[0].0),
            _ => return LookupResult::Ambiguous(using_matches),
        }

        // 6. Check root namespace
        if let Some(def_id) = self.def_map.get(name) {
            let entry = self.def_map.get_entry(def_id);
            if entry.vis == DefVis::Private && entry.file_id != self.current_file {
                return LookupResult::VisibilityError(def_id);
            }
            return LookupResult::Def(def_id);
        }

        // 7. Not found
        LookupResult::NotFound
    }

    /// Resolve a qualified path like `survival::HealthPotion` or `::root::Name`.
    pub fn resolve_qualified_path(&self, segments: &[String]) -> LookupResult {
        if segments.is_empty() {
            return LookupResult::NotFound;
        }

        // Handle root-anchored path (first segment is empty string from `::`)
        let (segments, _root_anchored) = if segments.first().map(|s| s.is_empty()).unwrap_or(false) {
            (&segments[1..], true)
        } else {
            (segments, false)
        };

        if segments.is_empty() {
            return LookupResult::NotFound;
        }

        // Try as full FQN
        let fqn = segments.join("::");
        if let Some(def_id) = self.def_map.get(&fqn) {
            let entry = self.def_map.get_entry(def_id);
            if entry.vis == DefVis::Private && entry.file_id != self.current_file {
                return LookupResult::VisibilityError(def_id);
            }
            return LookupResult::Def(def_id);
        }

        // Try prefix as namespace + terminal as name
        if segments.len() >= 2 {
            let ns = segments[..segments.len() - 1].join("::");
            let terminal = &segments[segments.len() - 1];

            // Check if this is an enum variant path (e.g., Direction::North)
            if let Some(enum_id) = self.def_map.get(&ns) {
                let entry = self.def_map.get_entry(enum_id);
                if entry.kind == DefKind::Enum {
                    // Enum variant access -- return the enum DefId
                    // (variant validation happens in type checking)
                    return LookupResult::Def(enum_id);
                }
            }

            // Try with current namespace prefix
            if !self.current_ns.is_empty() {
                let fqn_with_ns = format!("{}::{}", self.current_ns, fqn);
                if let Some(def_id) = self.def_map.get(&fqn_with_ns) {
                    let entry = self.def_map.get_entry(def_id);
                    if entry.vis == DefVis::Private && entry.file_id != self.current_file {
                        return LookupResult::VisibilityError(def_id);
                    }
                    return LookupResult::Def(def_id);
                }
            }

            // Try prefix as a namespace, terminal as the name
            let fqn_candidate = format!("{ns}::{terminal}");
            if let Some(def_id) = self.def_map.get(&fqn_candidate) {
                let entry = self.def_map.get_entry(def_id);
                if entry.vis == DefVis::Private && entry.file_id != self.current_file {
                    return LookupResult::VisibilityError(def_id);
                }
                return LookupResult::Def(def_id);
            }
        }

        LookupResult::NotFound
    }

    /// Resolve a value name (variables, functions, constants).
    pub fn resolve_value(&self, name: &str) -> LookupResult {
        // 1. Check local bindings
        for layer in self.layers.iter().rev() {
            if let ScopeLayer::Locals(locals) = layer {
                if locals.iter().any(|(n, _)| n == name) {
                    // Local binding found -- we don't have DefIds for locals,
                    // but we signal it's resolved
                    return LookupResult::GenericParam(name.to_string()); // Reuse for now
                }
            }
        }

        // 2. Check generic params (type params can be used as values in some contexts)
        for layer in self.layers.iter().rev() {
            if let ScopeLayer::GenericParams(params) = layer {
                if params.iter().any(|(n, _)| n == name) {
                    return LookupResult::GenericParam(name.to_string());
                }
            }
        }

        // Fall through to type resolution (which checks def_map)
        self.resolve_type(name)
    }

    /// Collect all visible names in the current scope (for fuzzy suggestion).
    pub fn visible_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // Generic params
        for layer in &self.layers {
            if let ScopeLayer::GenericParams(params) = layer {
                for (name, _) in params {
                    names.push(name.clone());
                }
            }
        }

        // Current namespace members
        if !self.current_ns.is_empty() {
            for def_id in self.def_map.pub_members_of(&self.current_ns) {
                let entry = self.def_map.get_entry(*def_id);
                names.push(entry.name.clone());
            }
        }

        // File-private names
        if let Some(privates) = self.def_map.file_private.get(&self.current_file) {
            for name in privates.keys() {
                names.push(name.clone());
            }
        }

        // Root namespace names
        for (fqn, _) in &self.def_map.by_fqn {
            if !fqn.contains("::") {
                names.push(fqn.clone());
            }
        }

        // Using imports
        for entry in &self.active_usings {
            if let Some(ref target_ns) = entry.target_ns {
                for def_id in self.def_map.pub_members_of(target_ns) {
                    let entry = self.def_map.get_entry(*def_id);
                    names.push(entry.name.clone());
                }
            }
            if entry.target_fqn.is_some() {
                names.push(entry.alias.clone());
            }
        }

        names.sort();
        names.dedup();
        names
    }

    /// Emit W0001 warnings for unused imports.
    pub fn emit_unused_import_warnings(&self, diags: &mut Vec<Diagnostic>) {
        for entry in &self.active_usings {
            if !entry.used.get() {
                diags.push(
                    ResolutionError::UnusedImport {
                        alias: entry.alias.clone(),
                        span: entry.span,
                        file: self.current_file,
                    }
                    .into(),
                );
            }
        }
    }
}

/// Map a type name to a PrimitiveTag.
fn primitive_tag(name: &str) -> Option<PrimitiveTag> {
    match name {
        "int" => Some(PrimitiveTag::Int),
        "float" => Some(PrimitiveTag::Float),
        "bool" => Some(PrimitiveTag::Bool),
        "string" => Some(PrimitiveTag::String),
        "void" => Some(PrimitiveTag::Void),
        _ => None,
    }
}
