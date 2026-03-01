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
    pub by_fqn: FxHashMap<String, DefId>,
    /// Per-file private definitions indexed by simple name.
    pub file_private: FxHashMap<FileId, FxHashMap<String, DefId>>,
    /// Namespace to list of public member DefIds.
    pub namespace_members: FxHashMap<String, Vec<DefId>>,
    /// All impl block DefIds (for later association in Pass 2).
    pub impl_blocks: Vec<DefId>,
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
        }
    }

    /// Insert a definition into the map.
    ///
    /// - For `Pub` visibility: inserts into `by_fqn` by FQN. If duplicate, emits E0001 diagnostic.
    /// - For `Private` visibility: inserts into `file_private` by simple name.
    /// - Impl blocks are also pushed onto `impl_blocks`.
    pub fn insert(
        &mut self,
        fqn: String,
        mut entry: DefEntry,
        diags: &mut Vec<Diagnostic>,
    ) -> DefId {
        let is_impl = matches!(entry.kind, DefKind::Impl);

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
                self.file_private
                    .entry(entry.file_id)
                    .or_default()
                    .insert(entry.name.clone(), id);
            }
        }

        id
    }

    /// Look up a public definition by fully-qualified name.
    pub fn get(&self, fqn: &str) -> Option<DefId> {
        self.by_fqn.get(fqn).copied()
    }

    /// Get the entry for a DefId.
    pub fn get_entry(&self, id: DefId) -> &DefEntry {
        &self.arena[id]
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
    Entity,
    Enum,
    Contract,
    Impl,
    Component,
    ExternFn,
    ExternStruct,
    ExternComponent,
    Const,
    Global,
}

/// Visibility of a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefVis {
    /// Visible to all files (pub).
    Pub,
    /// Visible only within the defining file.
    Private,
}
