//! ModuleBuilder: accumulates metadata rows and assigns tokens.
//!
//! Two-pass approach per CONTEXT.md locked decision:
//! Pass 1 (collect): call add_* methods to register all definitions.
//! Pass 2 (finalize): assign contiguous row indices, respecting list-ownership.
//!
//! ## SPLIT-08 review (Phase 63)
//!
//! Reviewed for split opportunities at 1,063 lines. Conclusion: no split.
//! `ModuleBuilder` is a single struct with 40+ fields. Its impl block contains
//! constructor, 14 "add" methods (Pass 1), `finalize` (Pass 2), and ~30 query
//! methods (post-finalize). All methods read/write `self` fields directly.
//! Splitting impl blocks across files would add file navigation overhead without
//! reducing complexity — callers still use `builder.add_typedef(...)`,
//! `builder.finalize()`, etc. regardless of file layout.

use rustc_hash::FxHashMap;

use crate::resolve::def_map::DefId;

use super::heaps::{BlobHeap, StringHeap};
// Intentional wildcard: metadata module exports 21 table-row structs that mirror
// the writ-module tables vocabulary — all are used during IL emission.
use super::metadata::*;

/// Provisional handle for a TypeDef entry (index into type_defs Vec).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeDefHandle(pub usize);

/// Provisional handle for a MethodDef entry (index into method_defs Vec).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MethodDefHandle(pub usize);

/// Provisional handle for a ContractDef entry (index into contract_defs Vec).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContractDefHandle(pub usize);

/// Provisional handle for an ImplDef entry (index into impl_defs Vec).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImplDefHandle(pub usize);

/// Internal record tracking parent-child relationships for list-ownership.
#[derive(Debug, Clone)]
struct FieldDefEntry {
    parent: TypeDefHandle,
    row: FieldDefRow,
}

#[derive(Debug, Clone)]
struct MethodDefEntry {
    /// Type used for name-based method lookup during body emission.
    parent: Option<TypeDefHandle>,
    /// Explicit ImplDef owner. When set, this is authoritative over `parent`.
    impl_owner: Option<ImplDefHandle>,
    row: MethodDefRow,
    def_id: Option<DefId>,
}

#[derive(Debug, Clone)]
struct ParamDefEntry {
    parent: MethodDefHandle,
    row: ParamDefRow,
}

#[derive(Debug, Clone)]
struct GenericParamEntry {
    row: GenericParamRow,
    /// Which owner handle (type, method, or contract).
    owner_table: TableId,
    owner_index: usize,
}

#[derive(Debug, Clone)]
struct ContractMethodEntry {
    parent: ContractDefHandle,
    row: ContractMethodRow,
}

fn type_signature_pattern_matches(
    pattern: &writ_module::signature::TypeSignature,
    actual: &writ_module::signature::TypeSignature,
    bindings: &mut FxHashMap<u16, writ_module::signature::TypeSignature>,
) -> bool {
    use writ_module::signature::TypeSignature;
    if let TypeSignature::GenericParam(ordinal) = pattern {
        return match bindings.get(ordinal) {
            Some(bound) => bound == actual,
            None => {
                bindings.insert(*ordinal, actual.clone());
                true
            }
        };
    }
    match (pattern, actual) {
        (TypeSignature::Void, TypeSignature::Void)
        | (TypeSignature::Int, TypeSignature::Int)
        | (TypeSignature::Float, TypeSignature::Float)
        | (TypeSignature::Bool, TypeSignature::Bool)
        | (TypeSignature::String, TypeSignature::String)
        | (TypeSignature::Entity, TypeSignature::Entity) => true,
        (TypeSignature::Named(left), TypeSignature::Named(right)) => left == right,
        (
            TypeSignature::Generic {
                namespace: left_ns,
                name: left_name,
                args: left_args,
            },
            TypeSignature::Generic {
                namespace: right_ns,
                name: right_name,
                args: right_args,
            },
        ) => {
            left_ns == right_ns
                && left_name == right_name
                && left_args.len() == right_args.len()
                && left_args
                    .iter()
                    .zip(right_args)
                    .all(|(left, right)| type_signature_pattern_matches(left, right, bindings))
        }
        (TypeSignature::Array(left), TypeSignature::Array(right)) => {
            type_signature_pattern_matches(left, right, bindings)
        }
        (
            TypeSignature::Function {
                params: left_params,
                ret: left_ret,
            },
            TypeSignature::Function {
                params: right_params,
                ret: right_ret,
            },
        ) => {
            left_params.len() == right_params.len()
                && left_params
                    .iter()
                    .zip(right_params)
                    .all(|(left, right)| type_signature_pattern_matches(left, right, bindings))
                && type_signature_pattern_matches(left_ret, right_ret, bindings)
        }
        _ => false,
    }
}

fn type_signature_specificity(signature: &writ_module::signature::TypeSignature) -> usize {
    use writ_module::signature::TypeSignature;
    match signature {
        TypeSignature::GenericParam(_) => 0,
        TypeSignature::Generic { args, .. } => {
            1 + args.iter().map(type_signature_specificity).sum::<usize>()
        }
        TypeSignature::Array(element) => 1 + type_signature_specificity(element),
        TypeSignature::Function { params, ret } => {
            1 + params.iter().map(type_signature_specificity).sum::<usize>()
                + type_signature_specificity(ret)
        }
        _ => 1,
    }
}

/// The central builder for IL metadata tables.
///
/// Accumulates rows during collection, then assigns final row indices
/// during `finalize()`.
pub struct ModuleBuilder {
    // Heaps
    pub string_heap: StringHeap,
    pub blob_heap: BlobHeap,

    // Table rows (provisional, not yet assigned final indices)
    pub module_def: Option<ModuleDefRow>,
    pub module_refs: Vec<ModuleRefRow>,
    type_defs: Vec<TypeDefRow>,
    type_def_def_ids: Vec<Option<DefId>>,
    type_refs: Vec<TypeRefRow>,
    type_specs: Vec<TypeSpecRow>,
    type_spec_tokens: FxHashMap<crate::check::ty::Ty, MetadataToken>,
    type_spec_signature_tokens: FxHashMap<u32, MetadataToken>,
    field_defs: Vec<FieldDefEntry>,
    field_refs: Vec<FieldRefRow>,
    /// Imported field identity -> FieldRef row index. Field declarations do
    /// not have source-level DefIds, so the owning type DefId and field name
    /// form the stable identity used by checked body emission.
    field_ref_by_owner_name: FxHashMap<(DefId, String), usize>,
    method_defs: Vec<MethodDefEntry>,
    method_refs: Vec<MethodRefRow>,
    /// Compiler-only MethodRef origin metadata used for inherent preference.
    method_ref_inherent: Vec<bool>,
    param_defs: Vec<ParamDefEntry>,
    contract_defs: Vec<ContractDefRow>,
    contract_def_def_ids: Vec<Option<DefId>>,
    contract_methods: Vec<ContractMethodEntry>,
    impl_defs: Vec<ImplDefRow>,
    impl_def_def_ids: Vec<Option<DefId>>,
    generic_params: Vec<GenericParamEntry>,
    generic_constraints: Vec<GenericConstraintRow>,
    /// DefIds of the constraint contracts, parallel to `generic_constraints`.
    /// Resolved to MetadataTokens during finalize.
    generic_constraint_contract_ids: Vec<DefId>,
    pub global_defs: Vec<GlobalDefRow>,
    global_def_def_ids: Vec<Option<DefId>>,
    pub extern_defs: Vec<ExternDefRow>,
    extern_def_def_ids: Vec<Option<DefId>>,
    pub component_slots: Vec<ComponentSlotRow>,
    pub locale_defs: Vec<LocaleDefRow>,
    pub export_defs: Vec<ExportDefRow>,
    pub attribute_defs: Vec<AttributeDefRow>,

    // DefId -> MetadataToken mapping (populated during finalize)
    pub def_token_map: FxHashMap<DefId, MetadataToken>,

    // DefId -> ordered (name, Ty) param list for each Fn and Impl method.
    // Populated during collect_fn / collect_impl before body emission.
    // Used by emit_all_bodies to pre-allocate r0..r(n-1) for parameters.
    pub fn_param_map: FxHashMap<DefId, Vec<(String, crate::check::ty::Ty)>>,

    // MethodDefHandle -> ordered (name, Ty) param list for impl methods.
    // Impl methods share a DefId (the impl block's DefId), so fn_param_map
    // cannot distinguish between them. This secondary map is indexed by
    // MethodDefHandle for unambiguous per-method param lookup.
    pub impl_method_param_map: FxHashMap<usize, Vec<(String, crate::check::ty::Ty)>>,

    // FIX-02: impl method DefId -> contract token mapping.
    // Populated by register_impl_method_contract() when contract impls are collected.
    // Allows contract_token_for_method_def_id() to look up the contract token for a
    // given impl method DefId so CALL_VIRT can emit the correct contract_idx.
    method_to_contract: FxHashMap<DefId, MetadataToken>,

    // Finalized state
    finalized: bool,

    // Final row counts (set after finalize)
    pub final_type_def_count: u32,
    pub final_field_def_count: u32,
    pub final_method_def_count: u32,
    pub final_param_def_count: u32,
    pub final_contract_def_count: u32,
    pub final_contract_method_count: u32,
    pub final_impl_def_count: u32,
    pub final_generic_param_count: u32,
    pub final_generic_constraint_count: u32,
}

impl ModuleBuilder {
    /// Create a new empty ModuleBuilder.
    pub fn new() -> Self {
        Self {
            string_heap: StringHeap::new(),
            blob_heap: BlobHeap::new(),
            module_def: None,
            module_refs: Vec::new(),
            type_defs: Vec::new(),
            type_def_def_ids: Vec::new(),
            type_refs: Vec::new(),
            type_specs: Vec::new(),
            type_spec_tokens: FxHashMap::default(),
            type_spec_signature_tokens: FxHashMap::default(),
            field_defs: Vec::new(),
            field_refs: Vec::new(),
            field_ref_by_owner_name: FxHashMap::default(),
            method_defs: Vec::new(),
            method_refs: Vec::new(),
            method_ref_inherent: Vec::new(),
            param_defs: Vec::new(),
            contract_defs: Vec::new(),
            contract_def_def_ids: Vec::new(),
            contract_methods: Vec::new(),
            impl_defs: Vec::new(),
            impl_def_def_ids: Vec::new(),
            generic_params: Vec::new(),
            generic_constraints: Vec::new(),
            generic_constraint_contract_ids: Vec::new(),
            global_defs: Vec::new(),
            global_def_def_ids: Vec::new(),
            extern_defs: Vec::new(),
            extern_def_def_ids: Vec::new(),
            component_slots: Vec::new(),
            locale_defs: Vec::new(),
            export_defs: Vec::new(),
            attribute_defs: Vec::new(),
            def_token_map: FxHashMap::default(),
            fn_param_map: FxHashMap::default(),
            impl_method_param_map: FxHashMap::default(),
            method_to_contract: FxHashMap::default(),
            finalized: false,
            final_type_def_count: 0,
            final_field_def_count: 0,
            final_method_def_count: 0,
            final_param_def_count: 0,
            final_contract_def_count: 0,
            final_contract_method_count: 0,
            final_impl_def_count: 0,
            final_generic_param_count: 0,
            final_generic_constraint_count: 0,
        }
    }

    // =========================================================================
    // Add methods (Pass 1)
    // =========================================================================

    /// Set the ModuleDef row (always exactly 1).
    pub fn set_module_def(&mut self, name: &str, version: &str, flags: u32) {
        let name_offset = self.string_heap.intern(name);
        let version_offset = self.string_heap.intern(version);
        self.module_def = Some(ModuleDefRow {
            name: name_offset,
            version: version_offset,
            flags,
        });
    }

    /// Add a ModuleRef row.
    pub fn add_module_ref(&mut self, name: &str, min_version: &str) -> usize {
        let name_offset = self.string_heap.intern(name);
        let ver_offset = self.string_heap.intern(min_version);
        self.module_refs.push(ModuleRefRow {
            name: name_offset,
            min_version: ver_offset,
        });
        self.module_refs.len() - 1
    }

    /// Add a TypeRef row (cross-module type reference). Returns the 0-based row index.
    pub fn add_type_ref(&mut self, module_ref_idx: usize, name: &str, namespace: &str) -> usize {
        let scope = MetadataToken::new(TableId::ModuleRef, (module_ref_idx + 1) as u32);
        let name_offset = self.string_heap.intern(name);
        let ns_offset = self.string_heap.intern(namespace);
        self.type_refs.push(TypeRefRow {
            scope,
            name: name_offset,
            namespace: ns_offset,
        });
        self.type_refs.len() - 1
    }

    /// Add a MethodRef row for a method owned by a local or referenced type.
    /// Returns the 0-based MethodRef row index.
    pub fn add_method_ref(&mut self, parent: MetadataToken, name: &str, signature: &[u8]) -> usize {
        self.add_method_ref_with_origin(parent, name, signature, true, true)
    }

    pub fn add_method_ref_with_origin(
        &mut self,
        parent: MetadataToken,
        name: &str,
        signature: &[u8],
        inherent: bool,
        has_receiver: bool,
    ) -> usize {
        let flags = if has_receiver {
            writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER
        } else {
            0
        };
        if let Some(index) = self.method_refs.iter().position(|method_ref| {
            method_ref.parent == parent
                && self.string_heap.get_str(method_ref.name) == name
                && writ_module::heap::read_blob(self.blob_heap.data(), method_ref.signature)
                    .is_ok_and(|existing| existing == signature)
                && method_ref.flags == flags
        }) {
            self.method_ref_inherent[index] |= inherent;
            return index;
        }

        let name = self.string_heap.intern(name);
        let signature = self.blob_heap.intern(signature);
        self.method_refs.push(MethodRefRow {
            parent,
            name,
            signature,
            flags,
        });
        self.method_ref_inherent.push(inherent);
        self.method_refs.len() - 1
    }

    /// Add a TypeDef row. Returns a handle for child relationships.
    pub fn add_typedef(
        &mut self,
        name: &str,
        namespace: &str,
        kind: TypeDefKind,
        flags: u16,
        def_id: Option<DefId>,
    ) -> TypeDefHandle {
        let name_offset = self.string_heap.intern(name);
        let ns_offset = self.string_heap.intern(namespace);
        self.type_defs.push(TypeDefRow {
            name: name_offset,
            namespace: ns_offset,
            kind: kind as u8,
            flags,
            field_list: 0,  // set during finalize
            method_list: 0, // set during finalize
        });
        self.type_def_def_ids.push(def_id);
        TypeDefHandle(self.type_defs.len() - 1)
    }

    /// Add or reuse an addressable generic type specialization.
    ///
    /// TypeSpec rows are not reordered during finalization, so the returned
    /// token is stable and can be stored directly in ImplDef rows and method
    /// bodies collected later in the same emission pass.
    pub fn add_type_spec(&mut self, ty: crate::check::ty::Ty, signature: u32) -> MetadataToken {
        if let Some(token) = self.type_spec_tokens.get(&ty) {
            return *token;
        }
        if let Some(token) = self.type_spec_signature_tokens.get(&signature).copied() {
            self.type_spec_tokens.insert(ty, token);
            return token;
        }

        let token = self.add_type_spec_signature(signature);
        self.type_spec_tokens.insert(ty, token);
        token
    }

    /// Add an addressable descriptor that has no compiler `Ty` identity.
    ///
    /// This is used when remapping a library TypeSpec into a consuming module.
    pub fn add_type_spec_signature(&mut self, signature: u32) -> MetadataToken {
        if let Some(token) = self.type_spec_signature_tokens.get(&signature) {
            return *token;
        }
        let token = MetadataToken::new(TableId::TypeSpec, (self.type_specs.len() + 1) as u32);
        self.type_specs.push(TypeSpecRow { signature });
        self.type_spec_signature_tokens.insert(signature, token);
        token
    }

    /// Return the TypeSpec token previously registered for `ty`.
    pub fn type_spec_token_for_ty(&self, ty: crate::check::ty::Ty) -> Option<MetadataToken> {
        self.type_spec_tokens.get(&ty).copied()
    }

    /// Resolve a TypeSpec by its canonical descriptor, following inference
    /// bindings recursively through nested generic arguments.
    pub fn type_spec_token_for_encoded_ty(
        &self,
        ty: crate::check::ty::Ty,
        interner: &crate::check::ty::TyInterner,
    ) -> Option<MetadataToken> {
        let token_for_def = |def_id| {
            self.def_token_map
                .get(&def_id)
                .copied()
                .unwrap_or(MetadataToken::NULL)
        };
        let signature = crate::emit::type_sig::encode_type_bytes(ty, interner, &token_for_def);
        let signature = self.blob_heap.offset_of(&signature)?;
        self.type_spec_signature_tokens.get(&signature).copied()
    }

    /// Add a FieldDef row under a parent TypeDef.
    pub fn add_fielddef(
        &mut self,
        parent: TypeDefHandle,
        name: &str,
        type_sig: u32,
        flags: u16,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.field_defs.push(FieldDefEntry {
            parent,
            row: FieldDefRow {
                name: name_offset,
                type_sig,
                flags,
            },
        });
        self.field_defs.len() - 1
    }

    /// Add or reuse an imported FieldRef and bind it to its source-level owner.
    ///
    /// FieldRef identity includes the exact parent, name, and canonical type
    /// signature. The `(owner_def_id, name)` map lets body emission recover the
    /// corresponding metadata token without confusing it with a local ordinal.
    pub fn add_field_ref(
        &mut self,
        owner_def_id: DefId,
        parent: MetadataToken,
        name: &str,
        type_signature: &[u8],
    ) -> usize {
        let row = self.add_field_ref_row(parent, name, type_signature);

        let identity = (owner_def_id, name.to_owned());
        if let Some(previous) = self.field_ref_by_owner_name.insert(identity, row) {
            assert_eq!(
                previous, row,
                "ambiguous imported field identity for `{name}`"
            );
        }
        row
    }

    /// Add or reuse a FieldRef row without binding a source-level owner.
    ///
    /// Runtime-provided lowering paths that identify a field by its metadata
    /// parent and name use this directly.
    pub fn add_field_ref_row(
        &mut self,
        parent: MetadataToken,
        name: &str,
        type_signature: &[u8],
    ) -> usize {
        self.field_refs
            .iter()
            .position(|field_ref| {
                field_ref.parent == parent
                    && self.string_heap.get_str(field_ref.name) == name
                    && writ_module::heap::read_blob(self.blob_heap.data(), field_ref.type_sig)
                        .is_ok_and(|existing| existing == type_signature)
            })
            .unwrap_or_else(|| {
                let name = self.string_heap.intern(name);
                let type_sig = self.blob_heap.intern(type_signature);
                self.field_refs.push(FieldRefRow {
                    parent,
                    name,
                    type_sig,
                });
                self.field_refs.len() - 1
            })
    }

    /// Add a MethodDef row, optionally under a parent TypeDef.
    pub fn add_methoddef(
        &mut self,
        parent: Option<TypeDefHandle>,
        name: &str,
        signature: u32,
        flags: u16,
        def_id: Option<DefId>,
        param_count: u16,
    ) -> MethodDefHandle {
        let name_offset = self.string_heap.intern(name);
        self.method_defs.push(MethodDefEntry {
            parent,
            impl_owner: None,
            row: MethodDefRow {
                name: name_offset,
                signature,
                flags,
                body_offset: 0,
                body_size: 0,
                reg_count: 0,
                param_count,
                owner: MetadataToken::NULL,
            },
            def_id,
        });
        MethodDefHandle(self.method_defs.len() - 1)
    }

    /// Add a ParamDef row under a parent MethodDef.
    pub fn add_paramdef(
        &mut self,
        parent: MethodDefHandle,
        name: &str,
        type_sig: u32,
        sequence: u16,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.param_defs.push(ParamDefEntry {
            parent,
            row: ParamDefRow {
                name: name_offset,
                type_sig,
                sequence,
            },
        });
        self.param_defs.len() - 1
    }

    /// Add a GenericParam row.
    pub fn add_generic_param(
        &mut self,
        owner_table: TableId,
        owner_index: usize,
        ordinal: u16,
        name: &str,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.generic_params.push(GenericParamEntry {
            row: GenericParamRow {
                owner: MetadataToken::NULL, // set during finalize
                owner_kind: match owner_table {
                    TableId::TypeDef => 0,
                    TableId::MethodDef => 1,
                    TableId::ContractDef => 2,
                    _ => 0,
                },
                ordinal,
                name: name_offset,
            },
            owner_table,
            owner_index,
        });
        self.generic_params.len() - 1
    }

    /// Add a GenericConstraint row.
    pub fn add_generic_constraint(
        &mut self,
        param_index: usize,
        constraint_def_id: DefId,
    ) -> usize {
        self.generic_constraints.push(GenericConstraintRow {
            param_row: param_index as u32, // provisional 0-based; remapped to 1-based in finalize
            constraint: MetadataToken::NULL, // resolved in finalize via def_token_map
        });
        self.generic_constraint_contract_ids.push(constraint_def_id);
        self.generic_constraints.len() - 1
    }

    /// Add a ContractDef row.
    pub fn add_contract_def(
        &mut self,
        name: &str,
        namespace: &str,
        def_id: Option<DefId>,
    ) -> ContractDefHandle {
        let name_offset = self.string_heap.intern(name);
        let ns_offset = self.string_heap.intern(namespace);
        self.contract_defs.push(ContractDefRow {
            name: name_offset,
            namespace: ns_offset,
            method_list: 0,        // set during finalize
            generic_param_list: 0, // set during finalize
        });
        self.contract_def_def_ids.push(def_id);
        ContractDefHandle(self.contract_defs.len() - 1)
    }

    /// Add a ContractMethod row under a parent ContractDef.
    pub fn add_contract_method(
        &mut self,
        parent: ContractDefHandle,
        name: &str,
        signature: u32,
        slot: u16,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.contract_methods.push(ContractMethodEntry {
            parent,
            row: ContractMethodRow {
                name: name_offset,
                signature,
                slot,
            },
        });
        self.contract_methods.len() - 1
    }

    /// Add an ImplDef row.
    pub fn add_impl_def(
        &mut self,
        type_token: MetadataToken,
        contract_token: MetadataToken,
        method_list: u32,
        def_id: Option<DefId>,
    ) -> ImplDefHandle {
        self.impl_defs.push(ImplDefRow {
            type_token,
            contract_token,
            method_list,
        });
        self.impl_def_def_ids.push(def_id);
        ImplDefHandle(self.impl_defs.len() - 1)
    }

    /// Update an ImplDef row's method_list field after finalize.
    ///
    /// Used by the Reflectable auto-impl post-finalize fixup to set the correct
    /// 1-based MethodDef row index for each synthetic get_type() method.
    pub fn set_impl_def_method_list(&mut self, handle: ImplDefHandle, method_list: u32) {
        self.impl_defs[handle.0].method_list = method_list;
    }

    /// Mark a MethodDef as owned by an ImplDef while retaining its target type
    /// for name-based lookup during emission.
    pub fn set_method_impl_owner(&mut self, method: MethodDefHandle, owner: ImplDefHandle) {
        self.method_defs[method.0].impl_owner = Some(owner);
    }

    /// Get the method_list value for a TypeDef (after finalize).
    ///
    /// Used to find the 1-based row index of the first MethodDef parented to a TypeDef.
    /// Returns 0 if no methods are parented to this TypeDef.
    pub fn typedef_method_list_by_handle(&self, handle: TypeDefHandle) -> u32 {
        self.type_defs[handle.0].method_list
    }

    /// Add a GlobalDef row.
    pub fn add_global_def(
        &mut self,
        name: &str,
        type_sig: u32,
        flags: u16,
        init_value: u32,
        def_id: Option<DefId>,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.global_defs.push(GlobalDefRow {
            name: name_offset,
            type_sig,
            flags,
            init_value,
        });
        self.global_def_def_ids.push(def_id);
        self.global_defs.len() - 1
    }

    /// Add an ExternDef row.
    pub fn add_extern_def(
        &mut self,
        name: &str,
        signature: u32,
        import_name: &str,
        flags: u16,
        def_id: Option<DefId>,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        let import_offset = self.string_heap.intern(import_name);
        self.extern_defs.push(ExternDefRow {
            name: name_offset,
            signature,
            import_name: import_offset,
            flags,
        });
        self.extern_def_def_ids.push(def_id);
        self.extern_defs.len() - 1
    }

    /// Add a ComponentSlot row.
    pub fn add_component_slot(
        &mut self,
        owner_entity: MetadataToken,
        component_type: MetadataToken,
    ) -> usize {
        self.component_slots.push(ComponentSlotRow {
            owner_entity,
            component_type,
        });
        self.component_slots.len() - 1
    }

    /// Add a LocaleDef row.
    pub fn add_locale_def(
        &mut self,
        dlg_method: MetadataToken,
        locale: &str,
        loc_method: MetadataToken,
    ) -> usize {
        let locale_offset = self.string_heap.intern(locale);
        self.locale_defs.push(LocaleDefRow {
            dlg_method,
            locale: locale_offset,
            loc_method,
        });
        self.locale_defs.len() - 1
    }

    /// Add an ExportDef row.
    pub fn add_export_def(&mut self, name: &str, item_kind: u8, item: MetadataToken) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.export_defs.push(ExportDefRow {
            name: name_offset,
            item_kind,
            item,
        });
        self.export_defs.len() - 1
    }

    /// Add an AttributeDef row.
    pub fn add_attribute_def(
        &mut self,
        owner: MetadataToken,
        owner_kind: u8,
        name: &str,
        value: u32,
    ) -> usize {
        let name_offset = self.string_heap.intern(name);
        self.attribute_defs.push(AttributeDefRow {
            owner,
            owner_kind,
            name: name_offset,
            value,
        });
        self.attribute_defs.len() - 1
    }

    // =========================================================================
    // Finalize (Pass 2)
    // =========================================================================

    /// Assign contiguous row indices respecting list-ownership.
    ///
    /// After calling this, `def_token_map` is populated.
    pub fn finalize(&mut self) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        // 1. TypeDef: assign 1-based row indices in collection order.
        self.final_type_def_count = self.type_defs.len() as u32;
        for (i, def_id) in self.type_def_def_ids.iter().enumerate() {
            let token = MetadataToken::new(TableId::TypeDef, (i + 1) as u32);
            if let Some(id) = def_id {
                self.def_token_map.insert(*id, token);
            }
        }

        // 2. FieldDef: group by parent, assign contiguous rows.
        // Sort field_defs by parent index to group children.
        self.field_defs.sort_by_key(|f| f.parent.0);
        self.final_field_def_count = self.field_defs.len() as u32;

        // Set every TypeDef.field_list to its first child row (1-based), using
        // the metadata-table "next index" convention. A type with no fields
        // repeats the next type's start row; a trailing empty type stores
        // FieldDef.len() + 1. Zero is not a valid finalized list index.
        {
            let mut field_idx = 0usize;
            for (type_idx, type_def) in self.type_defs.iter_mut().enumerate() {
                type_def.field_list = (field_idx + 1) as u32;
                while field_idx < self.field_defs.len()
                    && self.field_defs[field_idx].parent.0 == type_idx
                {
                    field_idx += 1;
                }
            }
            debug_assert_eq!(
                field_idx,
                self.field_defs.len(),
                "every FieldDef must have a TypeDef parent"
            );
        }

        // 3. MethodDef: preserve collection order so provisional handles remain stable.
        // Version 6 records an explicit owner on every row, so list contiguity is no
        // longer needed to distinguish type, impl, and top-level methods.
        self.final_method_def_count = self.method_defs.len() as u32;

        // Resolve explicit owners and retain method_list as a derived first-row index.
        {
            for (i, entry) in self.method_defs.iter_mut().enumerate() {
                let row_idx = (i + 1) as u32;
                entry.row.owner = if let Some(owner) = entry.impl_owner {
                    if self.impl_defs[owner.0].method_list == 0 {
                        self.impl_defs[owner.0].method_list = row_idx;
                    }
                    MetadataToken::new(TableId::ImplDef, (owner.0 + 1) as u32)
                } else if let Some(parent) = entry.parent {
                    if self.type_defs[parent.0].method_list == 0 {
                        self.type_defs[parent.0].method_list = row_idx;
                    }
                    MetadataToken::new(TableId::TypeDef, (parent.0 + 1) as u32)
                } else {
                    MetadataToken::NULL
                };
                // Map DefId -> token
                let token = MetadataToken::new(TableId::MethodDef, row_idx);
                if let Some(id) = entry.def_id {
                    self.def_token_map.insert(id, token);
                }
            }
        }

        // 4. ParamDef: group by parent MethodDef.
        self.param_defs.sort_by_key(|p| p.parent.0);
        self.final_param_def_count = self.param_defs.len() as u32;

        // 5. ContractDef: assign row indices.
        self.final_contract_def_count = self.contract_defs.len() as u32;
        for (i, def_id) in self.contract_def_def_ids.iter().enumerate() {
            let token = MetadataToken::new(TableId::ContractDef, (i + 1) as u32);
            if let Some(id) = def_id {
                self.def_token_map.insert(*id, token);
            }
        }

        // 6. ContractMethod: group by parent ContractDef.
        self.contract_methods.sort_by_key(|cm| cm.parent.0);
        self.final_contract_method_count = self.contract_methods.len() as u32;

        // Set every ContractDef.method_list to its first child row (1-based),
        // using the metadata-table "next index" convention. Empty contracts
        // repeat the next contract's start, and a trailing empty contract stores
        // ContractMethod.len() + 1. Zero is not a valid finalized list index.
        {
            let mut method_idx = 0usize;
            for (contract_idx, contract_def) in self.contract_defs.iter_mut().enumerate() {
                contract_def.method_list = (method_idx + 1) as u32;
                while method_idx < self.contract_methods.len()
                    && self.contract_methods[method_idx].parent.0 == contract_idx
                {
                    method_idx += 1;
                }
            }
            debug_assert_eq!(
                method_idx,
                self.contract_methods.len(),
                "every ContractMethod must have a ContractDef parent"
            );
        }

        // 7. ImplDef: assign row indices.
        self.final_impl_def_count = self.impl_defs.len() as u32;
        for (i, def_id) in self.impl_def_def_ids.iter().enumerate() {
            let token = MetadataToken::new(TableId::ImplDef, (i + 1) as u32);
            if let Some(id) = def_id {
                self.def_token_map.insert(*id, token);
            }
        }

        // 8. GenericParam: group by owner, assign rows.
        self.generic_params
            .sort_by_key(|g| (g.owner_table as u8, g.owner_index));
        self.final_generic_param_count = self.generic_params.len() as u32;

        // Resolve GenericParam.owner tokens.
        for entry in &mut self.generic_params {
            entry.row.owner = match entry.owner_table {
                TableId::TypeDef => {
                    MetadataToken::new(TableId::TypeDef, (entry.owner_index + 1) as u32)
                }
                TableId::MethodDef => {
                    // MethodDef collection order is preserved, so handles map directly
                    // to final 1-based rows.
                    MetadataToken::new(TableId::MethodDef, (entry.owner_index + 1) as u32)
                }
                TableId::ContractDef => {
                    MetadataToken::new(TableId::ContractDef, (entry.owner_index + 1) as u32)
                }
                _ => MetadataToken::NULL,
            };
        }

        // Set every ContractDef.generic_param_list to the first row owned by
        // that contract. GenericParam rows belonging to earlier owner tables
        // remain part of the absolute row index. Empty contracts repeat the
        // next contract's start, including the row after the table for a
        // trailing empty contract.
        {
            let contract_table_id = TableId::ContractDef as u8;
            let mut generic_idx = self
                .generic_params
                .iter()
                .position(|entry| entry.owner_table as u8 >= contract_table_id)
                .unwrap_or(self.generic_params.len());

            for (contract_idx, contract_def) in self.contract_defs.iter_mut().enumerate() {
                contract_def.generic_param_list = (generic_idx + 1) as u32;
                while generic_idx < self.generic_params.len()
                    && self.generic_params[generic_idx].owner_table == TableId::ContractDef
                    && self.generic_params[generic_idx].owner_index == contract_idx
                {
                    generic_idx += 1;
                }
            }

            debug_assert!(
                generic_idx == self.generic_params.len()
                    || self.generic_params[generic_idx].owner_table as u8 > contract_table_id,
                "every contract-owned GenericParam must have a ContractDef parent"
            );
        }

        // 9. GenericConstraint: resolve param_row to 1-based and constraint to MetadataToken.
        for (i, row) in self.generic_constraints.iter_mut().enumerate() {
            // Remap provisional 0-based GenericParam index to 1-based row index (spec section 2.16.5).
            row.param_row = row.param_row + 1;
            // Resolve contract DefId to MetadataToken via def_token_map.
            if i < self.generic_constraint_contract_ids.len() {
                let contract_def_id = self.generic_constraint_contract_ids[i];
                row.constraint = self
                    .def_token_map
                    .get(&contract_def_id)
                    .copied()
                    .unwrap_or(MetadataToken::NULL);
            }
        }
        self.final_generic_constraint_count = self.generic_constraints.len() as u32;

        // 10. GlobalDef: map DefIds.
        for (i, def_id) in self.global_def_def_ids.iter().enumerate() {
            let token = MetadataToken::new(TableId::GlobalDef, (i + 1) as u32);
            if let Some(id) = def_id {
                self.def_token_map.insert(*id, token);
            }
        }

        // 11. ExternDef: map DefIds.
        for (i, def_id) in self.extern_def_def_ids.iter().enumerate() {
            let token = MetadataToken::new(TableId::ExternDef, (i + 1) as u32);
            if let Some(id) = def_id {
                self.def_token_map.insert(*id, token);
            }
        }
    }

    // =========================================================================
    // Query methods (after finalize)
    // =========================================================================

    /// Get the MetadataToken for a DefId. Returns None if not registered.
    pub fn token_for_def(&self, def_id: DefId) -> Option<MetadataToken> {
        self.def_token_map.get(&def_id).copied()
    }

    /// Get the (name, Ty) parameter list for a function/method DefId.
    ///
    /// Returns None if not registered (lambdas, consts, and globals have no params).
    /// Parameters are in declaration order, excluding self.
    pub fn get_fn_params(&self, def_id: DefId) -> Option<&Vec<(String, crate::check::ty::Ty)>> {
        self.fn_param_map.get(&def_id)
    }

    /// Get function parameters for an impl method by MethodDefHandle index.
    /// Used when DefId-based lookup is ambiguous (all impl methods share impl_def_id).
    pub fn get_fn_params_by_handle(
        &self,
        handle_idx: usize,
    ) -> Option<&Vec<(String, crate::check::ty::Ty)>> {
        self.impl_method_param_map.get(&handle_idx)
    }

    /// Find the MethodDefHandle index for the Nth method of an impl block.
    ///
    /// Impl methods share a DefId, so we identify them by: finding all MethodDefs
    /// whose def_id matches impl_def_id, then returning the Nth one's handle index.
    pub fn find_impl_method_handle(&self, impl_def_id: DefId, method_idx: usize) -> Option<usize> {
        let mut count = 0;
        for (i, md) in self.method_defs.iter().enumerate() {
            if md.def_id == Some(impl_def_id) {
                if count == method_idx {
                    return Some(i);
                }
                count += 1;
            }
        }
        None
    }

    /// Whether the method's declared return signature is `Option<T>`.
    ///
    /// Return emission uses this to apply the nullable-value lift required by
    /// expression-bodied functions: a bare `T` tail in a `T?` method is encoded
    /// as `Some(T)`, while an existing `Option<T>` is returned unchanged.
    pub fn method_returns_option(&self, method_handle_idx: usize) -> bool {
        let Some(method) = self.method_defs.get(method_handle_idx) else {
            return false;
        };
        let Ok(signature) =
            writ_module::heap::read_blob(self.blob_heap.data(), method.row.signature)
        else {
            return false;
        };
        writ_module::signature::decode_method_signature(signature).is_ok_and(|(_, ret)| {
            matches!(
                ret,
                writ_module::signature::TypeSignature::Generic {
                    ref namespace,
                    ref name,
                    ref args,
                } if (namespace.is_empty() || namespace == "writ")
                    && name == "Option"
                    && args.len() == 1
            )
        })
    }

    /// Find a finalized MethodDef row by its source definition id.
    pub fn find_method_handle(&self, def_id: DefId) -> Option<usize> {
        self.method_defs
            .iter()
            .position(|method| method.def_id == Some(def_id))
    }

    /// Get the number of TypeDef rows.
    pub fn type_def_count(&self) -> usize {
        self.type_defs.len()
    }

    /// Get the number of FieldDef rows.
    pub fn field_def_count(&self) -> usize {
        self.field_defs.len()
    }

    /// Get the number of MethodDef rows.
    pub fn method_def_count(&self) -> usize {
        self.method_defs.len()
    }

    /// Get the number of ParamDef rows.
    pub fn param_def_count(&self) -> usize {
        self.param_defs.len()
    }

    /// Get the number of ContractDef rows.
    pub fn contract_def_count(&self) -> usize {
        self.contract_defs.len()
    }

    /// Get the number of ContractMethod rows.
    pub fn contract_method_count(&self) -> usize {
        self.contract_methods.len()
    }

    /// Get a ContractMethod's slot value by index.
    pub fn contract_method_slot(&self, index: usize) -> u16 {
        self.contract_methods[index].row.slot
    }

    /// Set a ContractMethod's slot value by index.
    pub fn set_contract_method_slot(&mut self, index: usize, slot: u16) {
        self.contract_methods[index].row.slot = slot;
    }

    /// Get the ContractDef method_list range for iteration.
    pub fn contract_method_range(&self, contract_idx: usize) -> std::ops::Range<usize> {
        let start = self
            .contract_methods
            .iter()
            .position(|cm| cm.parent.0 == contract_idx)
            .unwrap_or(self.contract_methods.len());
        let end = self
            .contract_methods
            .iter()
            .rposition(|cm| cm.parent.0 == contract_idx)
            .map(|p| p + 1)
            .unwrap_or(start);
        start..end
    }

    /// Get the number of ImplDef rows.
    pub fn impl_def_count(&self) -> usize {
        self.impl_defs.len()
    }

    /// Get the number of GenericParam rows.
    pub fn generic_param_count(&self) -> usize {
        self.generic_params.len()
    }

    /// Get a TypeDef row's field_list value.
    pub fn typedef_field_list(&self, idx: usize) -> u32 {
        self.type_defs[idx].field_list
    }

    /// Get a TypeDef row's method_list value.
    pub fn typedef_method_list(&self, idx: usize) -> u32 {
        self.type_defs[idx].method_list
    }

    /// Get a MethodDef row's flags by its handle.
    pub fn methoddef_flags(&self, handle: MethodDefHandle) -> u16 {
        self.method_defs[handle.0].row.flags
    }

    /// Whether a finalized MethodDef token names an instance method with a receiver.
    pub fn methoddef_has_receiver(&self, token: MetadataToken) -> Option<bool> {
        self.method_has_receiver(token)
    }

    /// Whether a MethodDef or MethodRef uses an implicit instance receiver.
    pub fn method_has_receiver(&self, token: MetadataToken) -> Option<bool> {
        match token.table() {
            TableId::MethodDef => {
                let index = token.row().checked_sub(1)? as usize;
                let method = self.method_defs.get(index)?;
                Some(
                    (method.parent.is_some() || method.impl_owner.is_some())
                        && method.row.flags & writ_module::tables::METHOD_FLAG_STATIC == 0,
                )
            }
            TableId::MethodRef => {
                let index = token.row().checked_sub(1)? as usize;
                self.method_refs.get(index).map(|method| {
                    method.flags & writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER != 0
                })
            }
            _ => None,
        }
    }

    /// Get the number of GlobalDef rows.
    pub fn global_def_count(&self) -> usize {
        self.global_defs.len()
    }

    /// Get the number of ExternDef rows.
    pub fn extern_def_count(&self) -> usize {
        self.extern_defs.len()
    }

    /// Get the number of ExportDef rows.
    pub fn export_def_count(&self) -> usize {
        self.export_defs.len()
    }

    /// Get the number of AttributeDef rows.
    pub fn attribute_def_count(&self) -> usize {
        self.attribute_defs.len()
    }

    /// Get the number of ComponentSlot rows.
    pub fn component_slot_count(&self) -> usize {
        self.component_slots.len()
    }

    /// Get the TypeDef kind by index.
    pub fn typedef_kind(&self, idx: usize) -> u8 {
        self.type_defs[idx].kind
    }

    // =========================================================================
    // Body emission helpers (used by call.rs and expr.rs)
    // =========================================================================

    /// Look up a field operand by parent type DefId and field name.
    ///
    /// Local fields return an encoded table-5 FieldDef token. Imported fields
    /// return an encoded table-6 FieldRef token.
    ///
    /// Returns None if the type is not registered or the field is not found.
    pub fn field_token_by_name(&self, parent_def_id: DefId, field_name: &str) -> Option<u32> {
        if let Some(parent_idx) = self
            .type_def_def_ids
            .iter()
            .position(|id| id.as_ref() == Some(&parent_def_id))
        {
            let parent_handle = TypeDefHandle(parent_idx);
            for (field_idx, entry) in self.field_defs.iter().enumerate() {
                if entry.parent == parent_handle {
                    let name_in_heap = self.string_heap.get_str(entry.row.name);
                    if name_in_heap == field_name {
                        return Some(
                            MetadataToken::new(TableId::FieldDef, (field_idx + 1) as u32).0,
                        );
                    }
                }
            }
        }

        self.field_ref_by_owner_name
            .get(&(parent_def_id, field_name.to_owned()))
            .map(|row| MetadataToken::new(TableId::FieldRef, (*row + 1) as u32).0)
    }

    /// Look up an extern def token by DefId.
    ///
    /// Returns the encoded MetadataToken value (used as extern_idx in CALL_EXTERN).
    pub fn extern_token_by_def_id(&self, def_id: DefId) -> Option<u32> {
        self.token_for_def(def_id).map(|t| t.0)
    }

    /// Look up the TypeDef token by name (for synthetic closure TypeDefs).
    ///
    /// Searches the string heap for a TypeDef with the given name and returns its
    /// encoded MetadataToken value. Used by `closure::emit_lambda` to locate
    /// the capture struct TypeDef registered by `pre_scan_lambdas`.
    ///
    /// Returns None if no TypeDef with that name is found.
    pub fn typedef_token_by_name(&self, name: &str) -> Option<u32> {
        for (i, td) in self.type_defs.iter().enumerate() {
            let name_in_heap = self.string_heap.get_str(td.name);
            if name_in_heap == name {
                let row_idx = (i + 1) as u32;
                return Some(MetadataToken::new(TableId::TypeDef, row_idx).0);
            }
        }
        None
    }

    /// Look up the MethodDef token by name (for synthetic closure invoke methods).
    ///
    /// Searches the string heap for a MethodDef with the given name and returns its
    /// encoded MetadataToken value. Used by `closure::emit_lambda` to locate
    /// the invoke method registered by `pre_scan_lambdas`.
    ///
    /// Returns None if no MethodDef with that name is found.
    pub fn methoddef_token_by_name(&self, name: &str) -> Option<u32> {
        for (i, md) in self.method_defs.iter().enumerate() {
            let name_in_heap = self.string_heap.get_str(md.row.name);
            if name_in_heap == name {
                let row_idx = (i + 1) as u32;
                return Some(MetadataToken::new(TableId::MethodDef, row_idx).0);
            }
        }
        None
    }

    /// Resolve a method by parent, name, and declaration signature using the
    /// same policy as the checker: inherent methods win first, then the most
    /// specific matching specialization, and remaining ambiguity fails closed.
    pub fn method_token_by_parent_name_and_signature(
        &self,
        exact_parent: MetadataToken,
        base_parent: Option<MetadataToken>,
        method_name: &str,
        signature: &[u8],
        has_receiver: bool,
    ) -> Option<u32> {
        let mut candidates =
            self.method_candidates_for_parent(exact_parent, method_name, signature, has_receiver);
        candidates.extend(self.method_pattern_candidates(
            exact_parent,
            method_name,
            signature,
            has_receiver,
        ));
        if let Some(base_parent) = base_parent.filter(|parent| *parent != exact_parent) {
            candidates.extend(self.method_candidates_for_parent(
                base_parent,
                method_name,
                signature,
                has_receiver,
            ));
        }
        let mut deduplicated = Vec::new();
        for candidate in candidates {
            if !deduplicated
                .iter()
                .any(|(token, _, _)| *token == candidate.0)
            {
                deduplicated.push(candidate);
            }
        }
        if deduplicated.iter().any(|(_, inherent, _)| *inherent) {
            deduplicated.retain(|(_, inherent, _)| *inherent);
        }
        if let Some(max_specificity) = deduplicated.iter().map(|(_, _, score)| *score).max() {
            deduplicated.retain(|(_, _, score)| *score == max_specificity);
        }
        (deduplicated.len() == 1).then(|| deduplicated[0].0)
    }

    fn method_pattern_candidates(
        &self,
        actual_parent: MetadataToken,
        method_name: &str,
        signature: &[u8],
        has_receiver: bool,
    ) -> Vec<(u32, bool, usize)> {
        let Some(_actual) = self.type_spec_signature(actual_parent) else {
            return Vec::new();
        };
        let mut candidates = Vec::new();
        for (index, method) in self.method_defs.iter().enumerate() {
            let token = MetadataToken::new(TableId::MethodDef, (index + 1) as u32);
            let (parent, inherent) = if let Some(owner) = method.impl_owner {
                let implementation = &self.impl_defs[owner.0];
                (
                    implementation.type_token,
                    implementation.contract_token.is_null(),
                )
            } else {
                continue;
            };
            let Some(pattern) = self.type_spec_signature(parent) else {
                continue;
            };
            if self.method_has_receiver(token) != Some(has_receiver)
                || self.string_heap.get_str(method.row.name) != method_name
                || !self.method_signature_pattern_matches(
                    parent,
                    actual_parent,
                    method.row.signature,
                    signature,
                )
            {
                continue;
            }
            candidates.push((token.0, inherent, type_signature_specificity(&pattern)));
        }
        for (index, method) in self.method_refs.iter().enumerate() {
            let token = MetadataToken::new(TableId::MethodRef, (index + 1) as u32);
            let Some(pattern) = self.type_spec_signature(method.parent) else {
                continue;
            };
            if self.method_has_receiver(token) != Some(has_receiver)
                || self.string_heap.get_str(method.name) != method_name
                || !self.method_signature_pattern_matches(
                    method.parent,
                    actual_parent,
                    method.signature,
                    signature,
                )
            {
                continue;
            }
            candidates.push((
                token.0,
                self.method_ref_inherent[index],
                type_signature_specificity(&pattern),
            ));
        }
        candidates
    }

    fn type_spec_signature(
        &self,
        token: MetadataToken,
    ) -> Option<writ_module::signature::TypeSignature> {
        if token.table() != TableId::TypeSpec {
            return None;
        }
        let row = token.row().checked_sub(1)? as usize;
        let type_spec = self.type_specs.get(row)?;
        let blob = writ_module::heap::read_blob(self.blob_heap.data(), type_spec.signature).ok()?;
        writ_module::signature::decode_type_signature(blob).ok()
    }

    /// Match an open declaration signature against a concrete call signature.
    /// Parent and method generic ordinals share one binding map because impl
    /// generics precede method generics in emitted metadata.
    fn method_signature_pattern_matches(
        &self,
        declaration_parent: MetadataToken,
        actual_parent: MetadataToken,
        declaration_signature_offset: u32,
        actual_signature: &[u8],
    ) -> bool {
        let mut bindings = FxHashMap::default();
        match (
            self.type_spec_signature(declaration_parent),
            self.type_spec_signature(actual_parent),
        ) {
            (Some(pattern), Some(actual)) => {
                if !type_signature_pattern_matches(&pattern, &actual, &mut bindings) {
                    return false;
                }
            }
            (None, None) if declaration_parent == actual_parent => {}
            _ => return false,
        }

        let Some((declaration_params, declaration_ret)) =
            writ_module::heap::read_blob(self.blob_heap.data(), declaration_signature_offset)
                .ok()
                .and_then(|blob| writ_module::signature::decode_method_signature(blob).ok())
        else {
            return false;
        };
        let Ok((actual_params, actual_ret)) =
            writ_module::signature::decode_method_signature(actual_signature)
        else {
            return false;
        };

        declaration_params.len() == actual_params.len()
            && declaration_params
                .iter()
                .zip(&actual_params)
                .all(|(pattern, actual)| {
                    type_signature_pattern_matches(pattern, actual, &mut bindings)
                })
            && type_signature_pattern_matches(&declaration_ret, &actual_ret, &mut bindings)
    }

    fn method_candidates_for_parent(
        &self,
        parent: MetadataToken,
        method_name: &str,
        signature: &[u8],
        has_receiver: bool,
    ) -> Vec<(u32, bool, usize)> {
        let mut candidates = Vec::new();
        for (index, method) in self.method_defs.iter().enumerate() {
            let token = MetadataToken::new(TableId::MethodDef, (index + 1) as u32);
            let (method_parent, inherent) = if let Some(owner) = method.impl_owner {
                let implementation = &self.impl_defs[owner.0];
                (
                    implementation.type_token,
                    implementation.contract_token.is_null(),
                )
            } else if let Some(owner) = method.parent {
                (
                    MetadataToken::new(TableId::TypeDef, (owner.0 + 1) as u32),
                    true,
                )
            } else {
                continue;
            };
            if self.method_has_receiver(token) != Some(has_receiver)
                || method_parent != parent
                || self.string_heap.get_str(method.row.name) != method_name
                || !self.method_signature_pattern_matches(
                    method_parent,
                    parent,
                    method.row.signature,
                    signature,
                )
            {
                continue;
            }
            candidates.push((
                token.0,
                inherent,
                self.type_spec_signature(method_parent)
                    .as_ref()
                    .map(type_signature_specificity)
                    .unwrap_or(0),
            ));
        }
        for (index, method) in self.method_refs.iter().enumerate() {
            let token = MetadataToken::new(TableId::MethodRef, (index + 1) as u32);
            if self.method_has_receiver(token) != Some(has_receiver)
                || method.parent != parent
                || self.string_heap.get_str(method.name) != method_name
                || !self.method_signature_pattern_matches(
                    method.parent,
                    parent,
                    method.signature,
                    signature,
                )
            {
                continue;
            }
            candidates.push((
                token.0,
                self.method_ref_inherent[index],
                self.type_spec_signature(method.parent)
                    .as_ref()
                    .map(type_signature_specificity)
                    .unwrap_or(0),
            ));
        }
        candidates
    }

    /// Look up the MethodDef token by parent type DefId and method name.
    ///
    /// Used by the emitter to resolve impl method calls like `obj.method()` where
    /// `callee_def_id` is None (type checker did not propagate a DefId for the method).
    /// Finds the MethodDef whose parent TypeDef was registered with `parent_def_id`
    /// and whose name matches `method_name`.
    ///
    /// Returns None if not found.
    pub fn methoddef_token_by_type_and_name(
        &self,
        parent_def_id: DefId,
        method_name: &str,
    ) -> Option<u32> {
        // Find the TypeDef index for the parent type.
        let parent_idx = self
            .type_def_def_ids
            .iter()
            .position(|id| id.as_ref() == Some(&parent_def_id))?;
        let parent_handle = TypeDefHandle(parent_idx);

        // Find a MethodDef with this parent and matching name.
        for (i, md) in self.method_defs.iter().enumerate() {
            if md.parent == Some(parent_handle) {
                let name_in_heap = self.string_heap.get_str(md.row.name);
                if name_in_heap == method_name {
                    let row_idx = (i + 1) as u32;
                    return Some(MetadataToken::new(TableId::MethodDef, row_idx).0);
                }
            }
        }
        None
    }

    /// Look up a cross-module MethodRef token by its parent type DefId and name.
    pub fn methodref_token_by_type_and_name(
        &self,
        parent_def_id: DefId,
        method_name: &str,
    ) -> Option<u32> {
        let parent = self.def_token_map.get(&parent_def_id)?;
        for (index, method_ref) in self.method_refs.iter().enumerate() {
            if method_ref.parent == *parent
                && self.string_heap.get_str(method_ref.name) == method_name
            {
                return Some(MetadataToken::new(TableId::MethodRef, (index + 1) as u32).0);
            }
        }
        None
    }

    /// Look up a FieldDef token by closure struct name and field name.
    ///
    /// This is the closure-specific version of `field_token_by_name`. Since closure
    /// TypeDefs use synthetic names (not DefIds), we look up the parent by name.
    pub fn field_token_by_name_on_closure(
        &self,
        closure_type_name: &str,
        field_name: &str,
    ) -> Option<u32> {
        // Find the TypeDef by name
        let parent_idx = self
            .type_defs
            .iter()
            .position(|td| self.string_heap.get_str(td.name) == closure_type_name)?;
        let parent_handle = TypeDefHandle(parent_idx);

        // Return the absolute, 1-based FieldDef metadata token.
        for (field_idx, entry) in self.field_defs.iter().enumerate() {
            if entry.parent == parent_handle {
                let name_in_heap = self.string_heap.get_str(entry.row.name);
                if name_in_heap == field_name {
                    return Some(MetadataToken::new(TableId::FieldDef, (field_idx + 1) as u32).0);
                }
            }
        }
        None
    }

    /// Look up an imported field token by the referenced type and field names.
    ///
    /// This is used by lowering for runtime-provided types such as `Range<T>`,
    /// whose fields have no source-level `DefId` at the lowering site.
    pub fn imported_field_token_by_type_name(
        &self,
        type_name: &str,
        field_name: &str,
    ) -> Option<u32> {
        let parent_row = self
            .type_refs
            .iter()
            .position(|type_ref| self.string_heap.get_str(type_ref.name) == type_name)?;
        let parent = MetadataToken::new(TableId::TypeRef, (parent_row + 1) as u32);
        self.field_refs
            .iter()
            .position(|field_ref| {
                field_ref.parent == parent && self.string_heap.get_str(field_ref.name) == field_name
            })
            .map(|field_row| MetadataToken::new(TableId::FieldRef, (field_row + 1) as u32).0)
    }

    // =========================================================================
    // Serialization accessors (for serialize.rs)
    // =========================================================================

    /// Get all finalized TypeDef rows (for serialization).
    pub fn finalized_type_defs(&self) -> impl Iterator<Item = &TypeDefRow> {
        self.type_defs.iter()
    }

    /// Get all finalized FieldDef rows (sorted by parent during finalize).
    pub fn finalized_field_defs(&self) -> impl Iterator<Item = &FieldDefRow> {
        self.field_defs.iter().map(|e| &e.row)
    }

    /// Get all finalized FieldRef rows.
    pub fn finalized_field_refs(&self) -> &[FieldRefRow] {
        &self.field_refs
    }

    /// Get all finalized MethodDef rows (sorted by parent during finalize).
    pub fn finalized_method_defs(&self) -> impl Iterator<Item = &MethodDefRow> {
        self.method_defs.iter().map(|e| &e.row)
    }

    /// Get all finalized MethodDef entries with their DefIds (for body matching).
    pub fn finalized_method_def_entries(
        &self,
    ) -> impl Iterator<Item = (Option<DefId>, &MethodDefRow)> {
        self.method_defs.iter().map(|e| (e.def_id, &e.row))
    }

    /// Get all finalized MethodRef rows.
    pub fn finalized_method_refs(&self) -> &[MethodRefRow] {
        &self.method_refs
    }

    /// Get all finalized ParamDef rows (sorted by parent during finalize).
    pub fn finalized_param_defs(&self) -> impl Iterator<Item = &ParamDefRow> {
        self.param_defs.iter().map(|e| &e.row)
    }

    /// Get all finalized ContractDef rows.
    pub fn finalized_contract_defs(&self) -> &[ContractDefRow] {
        &self.contract_defs
    }

    /// Get all finalized ContractMethod rows (sorted by parent during finalize).
    pub fn finalized_contract_methods(&self) -> impl Iterator<Item = &ContractMethodRow> {
        self.contract_methods.iter().map(|e| &e.row)
    }

    /// Get all finalized ImplDef rows.
    pub fn finalized_impl_defs(&self) -> &[ImplDefRow] {
        &self.impl_defs
    }

    /// Get all finalized GenericParam rows.
    pub fn finalized_generic_params(&self) -> impl Iterator<Item = &GenericParamRow> {
        self.generic_params.iter().map(|e| &e.row)
    }

    /// Get all finalized GenericConstraint rows.
    pub fn finalized_generic_constraints(&self) -> &[GenericConstraintRow] {
        &self.generic_constraints
    }

    /// Get all finalized TypeRef rows.
    pub fn finalized_type_refs(&self) -> &[TypeRefRow] {
        &self.type_refs
    }

    /// Get all finalized TypeSpec rows.
    pub fn finalized_type_specs(&self) -> &[TypeSpecRow] {
        &self.type_specs
    }

    /// Look up the contract method slot (vtable slot) for a contract method DefId.
    ///
    /// This is used to determine the slot field of CALL_VIRT. The slot is assigned
    /// from the ContractMethod table during the `slots::assign_vtable_slots` pass.
    ///
    /// Note: Since ContractMethod rows don't have a separate DefId mapping, we use
    /// the name-based approach: find the ContractMethod entry that matches the
    /// method's string name.
    ///
    /// Returns None if not found.
    pub fn contract_method_slot_by_def_id(&self, def_id: DefId) -> Option<u16> {
        // Look up the MethodDef entry that has this def_id in its token
        // The DefId maps to a MethodDef token via def_token_map.
        // For virtual dispatch, the ContractMethod entries hold slots.
        // We search by finding which contract method matches.
        // For now, search by comparing the method token against ContractMethod names.
        // Note: In the full pipeline, each impl method has a corresponding contract method.
        // For body emission tests, we just need basic slot lookup.
        let _ = def_id;
        // ContractMethods don't map to DefIds directly.
        // Return slot 0 as fallback for test purposes.
        // Full contract method slot resolution requires Phase 24 context (impl->contract mapping).
        None
    }

    /// Register a mapping from an impl method DefId to its contract's MetadataToken.
    ///
    /// Called from the collect phase when emitting ImplDef entries. This allows
    /// `contract_token_for_method_def_id` to return the correct contract token
    /// for a given impl method, enabling CALL_VIRT to emit non-zero contract_idx.
    ///
    /// FIX-02: Once the full pipeline registers all impl method → contract token
    /// mappings via this method, compiler-emitted CALL_VIRT instructions will carry
    /// the correct specialization contract token instead of the 0 placeholder.
    pub fn register_impl_method_contract(
        &mut self,
        method_def_id: DefId,
        contract_token: MetadataToken,
    ) {
        self.method_to_contract
            .insert(method_def_id, contract_token);
    }

    /// Look up the contract token for an impl method DefId.
    ///
    /// Returns the MetadataToken of the contract that the impl method belongs to,
    /// or None if no mapping has been registered for this DefId.
    ///
    /// The returned token is the contract's ContractDef token — the same value stored
    /// in ImplDefRow.contract_token — which equals impl_def.contract.0 in the built
    /// module's dispatch table entry type_args_hash field.
    ///
    /// Used by CALL_VIRT emission in call.rs to produce a non-zero contract_idx that
    /// the runtime's dispatch lookup can match against the table's type_args_hash.
    pub fn contract_token_for_method_def_id(&self, def_id: DefId) -> Option<MetadataToken> {
        self.method_to_contract.get(&def_id).copied()
    }

    /// Look up the CALL_VIRT slot for a contract method by contract DefId and method name.
    ///
    /// Searches the ContractMethod entries for the ContractDef registered with `contract_def_id`,
    /// returning the 0-based slot index of the entry whose name matches `method_name`.
    /// Slots are assigned by `assign_vtable_slots` in declaration order (0, 1, 2, ...).
    ///
    /// Returns None if the contract is not registered or the method is not found.
    pub fn contract_method_slot_by_name(
        &self,
        contract_def_id: DefId,
        method_name: &str,
    ) -> Option<u16> {
        // Find the ContractDef index for this DefId
        let contract_idx = self
            .contract_def_def_ids
            .iter()
            .position(|id| id.as_ref() == Some(&contract_def_id))?;
        // Iterate the contract's methods in range order; slot = 0-based position
        let range = self.contract_method_range(contract_idx);
        for (slot, cm_idx) in range.enumerate() {
            let name_in_heap = self
                .string_heap
                .get_str(self.contract_methods[cm_idx].row.name);
            if name_in_heap == method_name {
                return Some(slot as u16);
            }
        }
        None
    }

    /// Returns the type_idx token for the Range<T> type from the writ-runtime module.
    ///
    /// Range<T> is defined in the writ-runtime virtual module as the 3rd TypeDef
    /// (0-indexed: 2), after Option<T> and Result<T, E>. In user modules, Range is
    /// referenced via a cross-module TypeRef entry.
    ///
    /// Look up a TypeRef token by name. Returns the encoded MetadataToken value, or 0
    /// if no TypeRef with that name has been registered.
    ///
    /// Used by typeof() emission to resolve type_idx for primitives and runtime types.
    pub fn type_ref_token_by_name(&self, name: &str) -> u32 {
        for (i, tr) in self.type_refs.iter().enumerate() {
            let heap_name = self.string_heap.get_str(tr.name);
            if heap_name == name {
                return MetadataToken::new(TableId::TypeRef, (i + 1) as u32).0;
            }
        }
        0
    }

    /// If a TypeRef for "Range" from the writ-runtime module has been registered in
    /// this builder, its encoded token is returned. Otherwise falls back to 0, which
    /// is acceptable for Phase 28 since the instruction SEQUENCE (New + SetField) is
    /// what matters for correctness; the exact type_idx is wired in a later pass.
    pub fn range_type_token(&self) -> u32 {
        self.type_ref_token_by_name("Range")
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ModuleBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleBuilder")
            .field("type_defs", &self.type_defs.len())
            .field("field_defs", &self.field_defs.len())
            .field("method_defs", &self.method_defs.len())
            .field("param_defs", &self.param_defs.len())
            .field("contract_defs", &self.contract_defs.len())
            .field("impl_defs", &self.impl_defs.len())
            .field("generic_params", &self.generic_params.len())
            .field("finalized", &self.finalized)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::ty::{InferVar, TyInterner, TyKind};
    use crate::resolve::def_map::{DefEntry, DefKind, DefMap, DefVis};
    use chumsky::span::{SimpleSpan, Span as _};
    use writ_diagnostics::FileId;

    #[test]
    fn encoded_typespec_lookup_normalizes_nested_inference() {
        let span = SimpleSpan::new((), 0..0);
        let mut def_map = DefMap::new();
        let crate_id = def_map.arena.alloc(DefEntry {
            id: None,
            kind: DefKind::Class,
            vis: DefVis::Pub,
            file_id: FileId(0),
            namespace: "test".to_string(),
            name: "Crate".to_string(),
            name_span: span,
            generics: vec!["T".to_string()],
            span,
        });

        let mut interner = TyInterner::new();
        let int_ty = interner.int();
        let inferred_ty = interner.intern(TyKind::Infer(InferVar(0)));
        interner.record_infer_resolution(InferVar(0), int_ty);
        let base = interner.intern(TyKind::Class(crate_id));
        let unresolved = interner.intern(TyKind::GenericInstance {
            base,
            namespace: "test".to_string(),
            name: "Crate".to_string(),
            args: vec![inferred_ty],
        });
        let resolved = interner.intern(TyKind::GenericInstance {
            base,
            namespace: "test".to_string(),
            name: "Crate".to_string(),
            args: vec![int_ty],
        });

        let mut builder = ModuleBuilder::new();
        builder.add_typedef("Crate", "test", TypeDefKind::Class, 0, Some(crate_id));
        builder.finalize();
        let token_for_def = |def_id| builder.token_for_def(def_id).unwrap_or(MetadataToken::NULL);
        let descriptor =
            crate::emit::type_sig::encode_type_bytes(unresolved, &interner, &token_for_def);
        let descriptor = builder.blob_heap.intern(&descriptor);
        let token = builder.add_type_spec(unresolved, descriptor);

        assert_eq!(
            builder.type_spec_token_for_encoded_ty(resolved, &interner),
            Some(token)
        );
    }

    #[test]
    fn method_lookup_filters_same_identity_by_receiver_abi() {
        let mut builder = ModuleBuilder::new();
        let module_ref = builder.add_module_ref("dependency", "1.0.0");
        let type_ref = builder.add_type_ref(module_ref, "Utility", "");
        let parent = MetadataToken::new(TableId::TypeRef, (type_ref + 1) as u32);
        let signature = writ_module::signature::encode_method_signature(
            &[writ_module::signature::TypeSignature::Int],
            &writ_module::signature::TypeSignature::Int,
        )
        .unwrap();
        let instance =
            builder.add_method_ref_with_origin(parent, "identity", &signature, true, true);
        let static_method =
            builder.add_method_ref_with_origin(parent, "identity", &signature, true, false);

        assert_ne!(instance, static_method);
        assert_eq!(
            builder.method_token_by_parent_name_and_signature(
                parent, None, "identity", &signature, true,
            ),
            Some(MetadataToken::new(TableId::MethodRef, (instance + 1) as u32).0)
        );
        assert_eq!(
            builder.method_token_by_parent_name_and_signature(
                parent, None, "identity", &signature, false,
            ),
            Some(MetadataToken::new(TableId::MethodRef, (static_method + 1) as u32).0)
        );
    }

    #[test]
    fn local_field_lookup_returns_absolute_fielddef_tokens() {
        let span = SimpleSpan::new((), 0..0);
        let mut def_map = DefMap::new();
        let first_id = def_map.arena.alloc(DefEntry {
            id: None,
            kind: DefKind::Struct,
            vis: DefVis::Pub,
            file_id: FileId(0),
            namespace: "test".to_string(),
            name: "First".to_string(),
            name_span: span,
            generics: vec![],
            span,
        });
        let second_id = def_map.arena.alloc(DefEntry {
            id: None,
            kind: DefKind::Struct,
            vis: DefVis::Pub,
            file_id: FileId(0),
            namespace: "test".to_string(),
            name: "Second".to_string(),
            name_span: span,
            generics: vec![],
            span,
        });

        let mut builder = ModuleBuilder::new();
        let first = builder.add_typedef("First", "test", TypeDefKind::Struct, 0, Some(first_id));
        builder.add_fielddef(first, "first", 0, 0);
        let second = builder.add_typedef("Second", "test", TypeDefKind::Struct, 0, Some(second_id));
        builder.add_fielddef(second, "second_0", 0, 0);
        builder.add_fielddef(second, "second_1", 0, 0);
        builder.finalize();

        assert_eq!(
            builder.field_token_by_name(first_id, "first"),
            Some(MetadataToken::new(TableId::FieldDef, 1).0)
        );
        assert_eq!(
            builder.field_token_by_name(second_id, "second_0"),
            Some(MetadataToken::new(TableId::FieldDef, 2).0)
        );
        assert_eq!(
            builder.field_token_by_name(second_id, "second_1"),
            Some(MetadataToken::new(TableId::FieldDef, 3).0)
        );
    }
}
