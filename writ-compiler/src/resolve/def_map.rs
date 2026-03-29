//! DefMap: the central symbol table for name resolution.
//!
//! All top-level declarations are collected into the DefMap during Pass 1.
//! Each definition gets a unique `DefId` from an arena allocator.

use chumsky::span::SimpleSpan;
use id_arena::Arena;
use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::resolve::error::ResolutionError;

/// A unique identifier for a definition, allocated from an arena.
pub type DefId = id_arena::Id<DefEntry>;

/// The central symbol table for all top-level declarations.
#[derive(Debug)]
pub struct DefMap {
    /// Arena storing all definition entries.
    pub arena: Arena<DefEntry>,
    /// Public definitions indexed by fully-qualified name (e.g., "survival::Potion").
    /// For overloaded functions, stores the *first* overload; use `fn_overloads` for the full set.
    pub by_fqn: FxHashMap<String, DefId>,
    /// Per-file private definitions indexed by simple name.
    pub file_private: FxHashMap<FileId, FxHashMap<String, DefId>>,
    /// Namespace to list of public member DefIds.
    pub namespace_members: FxHashMap<String, Vec<DefId>>,
    /// All impl block DefIds (for later association in Pass 2).
    pub impl_blocks: Vec<DefId>,
    /// Function overload sets indexed by FQN. Present only when a name has 2+ overloads.
    pub fn_overloads: FxHashMap<String, Vec<DefId>>,
}

impl DefMap {
    /// Create an empty DefMap.
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            by_fqn: FxHashMap::default(),
            file_private: FxHashMap::default(),
            namespace_members: FxHashMap::default(),
            impl_blocks: Vec::new(),
            fn_overloads: FxHashMap::default(),
        }
    }

    /// Insert a definition into the map.
    ///
    /// - For `Pub` visibility: inserts into `by_fqn` by FQN. If duplicate, emits E0001 diagnostic
    ///   (unless both are functions, in which case they form an overload set).
    /// - For `Private` visibility: inserts into `file_private` by simple name.
    /// - Impl blocks are also pushed onto `impl_blocks`.
    pub fn insert(
        &mut self,
        fqn: String,
        mut entry: DefEntry,
        diags: &mut Vec<Diagnostic>,
    ) -> DefId {
        let is_impl = matches!(entry.kind, DefKind::Impl);
        let is_fn = matches!(entry.kind, DefKind::Fn | DefKind::ExternFn);

        // Allocate arena slot
        let id = self.arena.alloc(entry.clone());

        // Update the entry's knowledge of its own id (stored externally; the arena copy is separate)
        entry.id = Some(id);

        // For impl blocks, always track them
        if is_impl {
            self.impl_blocks.push(id);
        }

        match entry.vis {
            DefVis::Pub => {
                if let Some(&existing_id) = self.by_fqn.get(&fqn) {
                    let existing = &self.arena[existing_id];
                    let existing_is_fn =
                        matches!(existing.kind, DefKind::Fn | DefKind::ExternFn);

                    if is_fn && existing_is_fn {
                        // Function overloading: add to overload set
                        let overloads = self.fn_overloads
                            .entry(fqn.clone())
                            .or_insert_with(|| vec![existing_id]);
                        overloads.push(id);
                        // Track namespace membership for the new overload
                        self.namespace_members
                            .entry(entry.namespace.clone())
                            .or_default()
                            .push(id);
                    } else {
                        diags.push(
                            ResolutionError::DuplicateDefinition {
                                name: fqn.clone(),
                                first_file: existing.file_id,
                                first_span: existing.name_span,
                                second_file: entry.file_id,
                                second_span: entry.name_span,
                            }
                            .into(),
                        );
                    }
                } else {
                    self.by_fqn.insert(fqn.clone(), id);
                    // Track namespace membership
                    self.namespace_members
                        .entry(entry.namespace.clone())
                        .or_default()
                        .push(id);
                }
            }
            DefVis::Private => {
                if is_fn {
                    // For private functions, also support overloading via fn_overloads.
                    // file_private stores one DefId per name; overloads go in fn_overloads.
                    let privates = self.file_private
                        .entry(entry.file_id)
                        .or_default();
                    if let Some(&existing_id) = privates.get(&entry.name) {
                        let existing = &self.arena[existing_id];
                        if matches!(existing.kind, DefKind::Fn | DefKind::ExternFn) {
                            // Private function overload
                            let key = format!("{}@{}", entry.name, entry.file_id.0);
                            let overloads = self.fn_overloads
                                .entry(key)
                                .or_insert_with(|| vec![existing_id]);
                            overloads.push(id);
                        }
                    } else {
                        privates.insert(entry.name.clone(), id);
                    }
                } else {
                    self.file_private
                        .entry(entry.file_id)
                        .or_default()
                        .insert(entry.name.clone(), id);
                }
            }
        }

        id
    }

    /// Look up a public definition by fully-qualified name.
    pub fn get(&self, fqn: &str) -> Option<DefId> {
        self.by_fqn.get(fqn).copied()
    }

    /// Look up a public definition by FQN, disambiguating overloads by name_span.
    pub fn get_by_span(&self, fqn: &str, name_span: SimpleSpan) -> Option<DefId> {
        if let Some(overloads) = self.fn_overloads.get(fqn) {
            for &id in overloads {
                if self.arena[id].name_span == name_span {
                    return Some(id);
                }
            }
        }
        if let Some(&id) = self.by_fqn.get(fqn) {
            if self.arena[id].name_span == name_span {
                return Some(id);
            }
        }
        None
    }

    /// Get the entry for a DefId.
    pub fn get_entry(&self, id: DefId) -> &DefEntry {
        &self.arena[id]
    }

    /// Get all function candidates for a given FQN (supports overloading).
    /// Returns a single-element slice for non-overloaded functions,
    /// or the full overload set if present.
    pub fn get_fn_candidates(&self, fqn: &str) -> Vec<DefId> {
        if let Some(overloads) = self.fn_overloads.get(fqn) {
            overloads.clone()
        } else if let Some(&id) = self.by_fqn.get(fqn) {
            vec![id]
        } else {
            vec![]
        }
    }

    /// Get all private function candidates for a given name in a file.
    pub fn get_private_fn_candidates(&self, file_id: FileId, name: &str) -> Vec<DefId> {
        let key = format!("{}@{}", name, file_id.0);
        if let Some(overloads) = self.fn_overloads.get(&key) {
            overloads.clone()
        } else if let Some(privates) = self.file_private.get(&file_id) {
            if let Some(&id) = privates.get(name) {
                vec![id]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    /// Get all public members of a namespace.
    pub fn pub_members_of(&self, namespace: &str) -> &[DefId] {
        self.namespace_members
            .get(namespace)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

impl Default for DefMap {
    fn default() -> Self {
        Self::new()
    }
}

/// A definition entry in the DefMap.
#[derive(Debug, Clone)]
pub struct DefEntry {
    /// The arena-assigned ID (set after insertion).
    pub id: Option<DefId>,
    /// What kind of definition this is.
    pub kind: DefKind,
    /// Visibility: public or file-private.
    pub vis: DefVis,
    /// The file this definition appears in.
    pub file_id: FileId,
    /// The namespace this definition belongs to (e.g., "survival::combat").
    pub namespace: String,
    /// The simple name of the definition (e.g., "Potion").
    pub name: String,
    /// The span of just the name identifier.
    pub name_span: SimpleSpan,
    /// Generic type parameter names (e.g., ["T", "U"]).
    pub generics: Vec<String>,
    /// The span of the entire definition.
    pub span: SimpleSpan,
}

/// The kind of a top-level definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefKind {
    Fn,
    Struct,
    /// Class declaration (reference type, heap-allocated).
    Class,
    Entity,
    Enum,
    Contract,
    Impl,
    Component,
    ExternFn,
    ExternComponent,
    Const,
    Global,
    /// User-defined attribute declaration.
    AttributeDef,
}

/// Visibility of a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefVis {
    /// Visible to all files (pub).
    Pub,
    /// Visible only within the defining file.
    Private,
}
