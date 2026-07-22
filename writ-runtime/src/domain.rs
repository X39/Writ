//! Multi-module domain with cross-module name resolution.
//!
//! A `Domain` holds all loaded modules (the virtual `writ-runtime` module
//! plus any user modules) and resolves cross-module TypeRef, MethodRef,
//! and FieldRef entries by name matching at load time.
//!
//! Resolution results are stored per-module in `ResolvedRefs` maps for
//! O(1) lookup at runtime.

use rustc_hash::FxHashMap;
use writ_module::heap::{read_blob, read_string};
use writ_module::signature::TypeSignature;
use writ_module::token::MetadataToken;

use crate::error::RuntimeError;
use crate::loader::LoadedModule;

// ──── Resolution result types ─────────────────────────────────────────

/// Resolved cross-module type reference: points to a TypeDef in a specific module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedType {
    /// Index into Domain::modules.
    pub module_idx: usize,
    /// 0-based index into the target module's type_defs table.
    pub typedef_idx: usize,
}

/// Resolved cross-module method reference: points to a MethodDef in a specific module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMethod {
    /// Index into Domain::modules.
    pub module_idx: usize,
    /// 0-based index into the target module's method_defs / decoded_bodies.
    pub method_idx: usize,
}

/// Resolved cross-module field reference: points to a FieldDef in a specific module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedField {
    /// Index into Domain::modules.
    pub module_idx: usize,
    /// 0-based index into the target module's field_defs table.
    pub field_idx: usize,
}

/// Resolved cross-module contract reference: points to a ContractDef in a specific module.
///
/// TypeRefs can point to contracts (not just TypeDefs). When a TypeRef resolves
/// to a ContractDef, it is stored here instead of in the `types` map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedContract {
    /// Index into Domain::modules.
    pub module_idx: usize,
    /// 0-based index into the target module's contract_defs table.
    pub contractdef_idx: usize,
}

#[derive(Debug, Clone)]
enum MethodRefParent {
    Module {
        module_idx: usize,
    },
    Bare {
        module_idx: usize,
        type_idx: usize,
    },
    Specialized {
        source_module_idx: usize,
        signature: TypeSignature,
        base_module_idx: usize,
        base_type_idx: usize,
    },
}

#[derive(Debug, Clone, Copy)]
struct MethodCandidate {
    module_idx: usize,
    method_idx: usize,
    inherent: bool,
    specificity: usize,
}

fn type_signature_specificity(signature: &TypeSignature) -> usize {
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

/// Per-module resolution results for cross-module references.
///
/// Maps are keyed by the 0-based row index in the source module's
/// TypeRef/MethodRef/FieldRef tables.
#[derive(Debug, Clone, Default)]
pub struct ResolvedRefs {
    /// TypeRef row index (0-based) -> resolved (module_idx, typedef_idx).
    pub types: FxHashMap<u32, ResolvedType>,
    /// TypeRef row index (0-based) -> resolved (module_idx, contractdef_idx).
    /// For TypeRefs that resolve to a ContractDef rather than a TypeDef.
    pub contracts: FxHashMap<u32, ResolvedContract>,
    /// MethodRef row index (0-based) -> resolved (module_idx, method_idx).
    pub methods: FxHashMap<u32, ResolvedMethod>,
    /// FieldRef row index (0-based) -> resolved (module_idx, field_idx).
    pub fields: FxHashMap<u32, ResolvedField>,
}

impl ResolvedRefs {
    /// Create empty resolution maps.
    pub fn new() -> Self {
        Self::default()
    }
}

// ──── Domain ──────────────────────────────────────────────────────────

/// A domain holds all loaded modules and provides cross-module resolution.
///
/// Module index 0 is conventionally the `writ-runtime` virtual module,
/// loaded first via `Domain::with_virtual_module()`.
pub struct Domain {
    pub modules: Vec<LoadedModule>,
}

impl Default for Domain {
    fn default() -> Self {
        Self::new()
    }
}

impl Domain {
    /// Create an empty domain with no modules.
    pub fn new() -> Self {
        Domain {
            modules: Vec::new(),
        }
    }

    /// Add a module to the domain. Returns the module's index.
    pub fn add_module(&mut self, module: writ_module::Module) -> Result<usize, RuntimeError> {
        let loaded = LoadedModule::from_module(module)?;
        let idx = self.modules.len();
        self.modules.push(loaded);
        Ok(idx)
    }

    /// Resolve all cross-module references across all loaded modules.
    ///
    /// For each module, every TypeRef, MethodRef, and FieldRef row is resolved
    /// by name matching against the target module's definitions. Results are
    /// stored in each module's `resolved_refs` field.
    ///
    /// Returns an error for the first unresolvable reference encountered.
    pub fn resolve_refs(&mut self) -> Result<(), RuntimeError> {
        let module_count = self.modules.len();
        for src_idx in 0..module_count {
            let resolved = self.resolve_type_refs(src_idx)?;
            self.modules[src_idx].resolved_refs = resolved;
        }
        for src_idx in 0..module_count {
            let resolved = self.resolve_module_refs(src_idx)?;
            self.modules[src_idx].resolved_refs = resolved;
        }
        Ok(())
    }

    // ── Resolution implementation ─────────────────────────────────

    /// Resolve TypeRefs for every module before signatures or TypeSpecs use
    /// them during member resolution.
    fn resolve_type_refs(&self, src_idx: usize) -> Result<ResolvedRefs, RuntimeError> {
        let mut resolved = ResolvedRefs::new();
        let src_module = &self.modules[src_idx].module;
        for (ref_idx, type_ref) in src_module.type_refs.iter().enumerate() {
            let scope_row = type_ref.scope.row_index().ok_or_else(|| {
                RuntimeError::ExecutionError("TypeRef has null scope token".into())
            })?;
            let scope = (scope_row - 1) as usize;
            let module_ref = src_module.module_refs.get(scope).ok_or_else(|| {
                RuntimeError::ExecutionError(format!(
                    "TypeRef scope index {} out of range (module has {} ModuleRef rows)",
                    scope,
                    src_module.module_refs.len()
                ))
            })?;
            let target_name = read_string(&src_module.string_heap, module_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid ModuleRef name".into()))?;
            let target_idx = self.find_module_by_name(target_name).ok_or_else(|| {
                RuntimeError::ExecutionError(format!(
                    "unresolved module reference: '{}'",
                    target_name
                ))
            })?;
            let name = read_string(&src_module.string_heap, type_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid TypeRef name".into()))?;
            let namespace = read_string(&src_module.string_heap, type_ref.namespace)
                .map_err(|_| RuntimeError::ExecutionError("invalid TypeRef namespace".into()))?;
            let target = &self.modules[target_idx].module;
            if let Some(type_idx) = Self::find_type_def_by_name(target, namespace, name) {
                resolved.types.insert(
                    ref_idx as u32,
                    ResolvedType {
                        module_idx: target_idx,
                        typedef_idx: type_idx,
                    },
                );
            } else if let Some(contract_idx) =
                Self::find_contract_def_by_name(target, namespace, name)
            {
                resolved.contracts.insert(
                    ref_idx as u32,
                    ResolvedContract {
                        module_idx: target_idx,
                        contractdef_idx: contract_idx,
                    },
                );
            } else {
                return Err(RuntimeError::ExecutionError(format!(
                    "unresolved type reference: '{}::{}' in module '{}'",
                    namespace, name, target_name
                )));
            }
        }
        Ok(resolved)
    }

    /// Resolve all cross-module references for a single module.
    fn resolve_module_refs(&self, src_idx: usize) -> Result<ResolvedRefs, RuntimeError> {
        let mut resolved = self.modules[src_idx].resolved_refs.clone();
        let src_module = &self.modules[src_idx].module;

        // ── TypeRef resolution ────────────────────────────────────
        for (ref_idx, type_ref) in src_module.type_refs.iter().enumerate() {
            // Get the ModuleRef scope to find the target module
            let scope_row = type_ref.scope.row_index()
                .ok_or_else(|| RuntimeError::ExecutionError(
                    "TypeRef has null scope token".into()
                ))?;
            let scope_0based = (scope_row - 1) as usize;
            if scope_0based >= src_module.module_refs.len() {
                return Err(RuntimeError::ExecutionError(format!(
                    "TypeRef scope index {} out of range (module has {} ModuleRef rows)",
                    scope_0based, src_module.module_refs.len()
                )));
            }
            let mod_ref = &src_module.module_refs[scope_0based];
            let target_mod_name = read_string(&src_module.string_heap, mod_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid ModuleRef name".into()))?;

            let target_mod_idx = self.find_module_by_name(target_mod_name)
                .ok_or_else(|| RuntimeError::ExecutionError(format!(
                    "unresolved module reference: '{}'", target_mod_name
                )))?;

            let ref_name = read_string(&src_module.string_heap, type_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid TypeRef name".into()))?;
            let ref_ns = read_string(&src_module.string_heap, type_ref.namespace)
                .map_err(|_| RuntimeError::ExecutionError("invalid TypeRef namespace".into()))?;

            let target_module = &self.modules[target_mod_idx].module;
            if let Some(typedef_idx) = Self::find_type_def_by_name(target_module, ref_ns, ref_name)
            {
                resolved.types.insert(
                    ref_idx as u32,
                    ResolvedType {
                    module_idx: target_mod_idx,
                    typedef_idx,
                    },
                );
            } else if let Some(contractdef_idx) =
                Self::find_contract_def_by_name(target_module, ref_ns, ref_name)
            {
                // TypeRef points to a ContractDef, not a TypeDef.
                // Store in the contracts map for dispatch table resolution.
                resolved.contracts.insert(
                    ref_idx as u32,
                    ResolvedContract {
                    module_idx: target_mod_idx,
                    contractdef_idx,
                    },
                );
            } else {
                return Err(RuntimeError::ExecutionError(format!(
                    "unresolved type reference: '{}::{}' in module '{}'",
                    ref_ns, ref_name, target_mod_name
                )));
            }
        }

        // ── MethodRef resolution ──────────────────────────────────
        for (ref_idx, method_ref) in src_module.method_refs.iter().enumerate() {
            let unknown_flags =
                method_ref.flags & !writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER;
            if unknown_flags != 0 {
                return Err(RuntimeError::ExecutionError(format!(
                    "invalid MethodRef flags: 0x{:04X}",
                    method_ref.flags
                )));
            }
            let has_receiver =
                method_ref.flags & writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER != 0;
            let parent = self.resolve_method_parent(src_idx, method_ref.parent)?;
            let method_name = read_string(&src_module.string_heap, method_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid MethodRef name".into()))?;
            let signature_blob =
                read_blob(&src_module.blob_heap, method_ref.signature).map_err(|_| {
                    RuntimeError::ExecutionError("invalid MethodRef signature blob offset".into())
                })?;
            let signature = writ_module::signature::decode_method_signature(signature_blob)
                .map_err(|_| RuntimeError::ExecutionError("invalid MethodRef signature".into()))?;
            let candidate = self.resolve_method_candidate(
                &parent,
                src_idx,
                method_name,
                &signature,
                has_receiver,
            )?;
            resolved.methods.insert(
                ref_idx as u32,
                ResolvedMethod {
                    module_idx: candidate.module_idx,
                    method_idx: candidate.method_idx,
                },
            );
        }

        // ── FieldRef resolution ───────────────────────────────────
        for (ref_idx, field_ref) in src_module.field_refs.iter().enumerate() {
            let (target_mod_idx, type_idx) = self.resolve_parent_type(
                src_idx, field_ref.parent, &resolved
            )?;

            let field_name = read_string(&src_module.string_heap, field_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid FieldRef name".into()))?;

            let target_module = &self.modules[target_mod_idx].module;
            let field_idx = Self::find_field_in_type(target_module, type_idx, field_name)
                .ok_or_else(|| {
                    let type_name = read_string(
                        &target_module.string_heap,
                        target_module.type_defs[type_idx].name,
                    )
                    .unwrap_or("<unknown>");
                    RuntimeError::ExecutionError(format!(
                        "unresolved field reference: '{}' on type '{}'",
                        field_name, type_name
                    ))
                })?;

            resolved.fields.insert(
                ref_idx as u32,
                ResolvedField {
                module_idx: target_mod_idx,
                field_idx,
                },
            );
        }

        Ok(resolved)
    }

    /// Resolve a parent MetadataToken (from MethodRef/FieldRef) to a
    /// (module_idx, typedef_idx) pair.
    ///
    /// The parent can be either:
    /// - Table 2 (TypeDef): a local type in the same module
    /// - Table 3 (TypeRef): a cross-module reference already resolved
    fn resolve_parent_type(
        &self,
        src_idx: usize,
        parent: MetadataToken,
        resolved: &ResolvedRefs,
    ) -> Result<(usize, usize), RuntimeError> {
        let table_id = parent.table_id();
        let row_idx = parent
            .row_index()
            .ok_or_else(|| RuntimeError::ExecutionError("parent token is null".into()))?;
        let row_0based = row_idx - 1;

        match table_id {
            2 => {
                // TypeDef -- local type in the same module
                Ok((src_idx, row_0based as usize))
            }
            3 => {
                // TypeRef -- look up in already-resolved types map
                let rt = resolved.types.get(&row_0based)
                    .ok_or_else(|| RuntimeError::ExecutionError(format!(
                        "parent TypeRef row {} not yet resolved", row_0based
                    )))?;
                Ok((rt.module_idx, rt.typedef_idx))
            }
            _ => Err(RuntimeError::ExecutionError(format!(
                "unexpected parent token table ID: {}", table_id
            )))
        }
    }

    // ── Name-matching helpers ─────────────────────────────────────

    fn resolve_method_parent(
        &self,
        src_idx: usize,
        parent: MetadataToken,
    ) -> Result<MethodRefParent, RuntimeError> {
        match parent.table_id() {
            1 => {
                let row = parent
                    .row_index()
                    .and_then(|row| row.checked_sub(1))
                    .ok_or_else(|| {
                        RuntimeError::ExecutionError("invalid MethodRef ModuleRef parent".into())
                    })? as usize;
                let source = &self.modules[src_idx].module;
                let module_ref = source.module_refs.get(row).ok_or_else(|| {
                    RuntimeError::ExecutionError(
                        "MethodRef ModuleRef parent is out of range".into(),
                    )
                })?;
                let name = read_string(&source.string_heap, module_ref.name).map_err(|_| {
                    RuntimeError::ExecutionError("invalid MethodRef ModuleRef name".into())
                })?;
                let module_idx = self.find_module_by_name(name).ok_or_else(|| {
                    RuntimeError::ExecutionError(format!(
                        "unresolved MethodRef module parent: '{}'",
                        name
                    ))
                })?;
                Ok(MethodRefParent::Module { module_idx })
            }
            2 | 3 => {
                let (module_idx, type_idx) =
                    crate::type_specs::resolve_type_location(src_idx, parent, &self.modules)
                        .ok_or_else(|| {
                            RuntimeError::ExecutionError("unresolved MethodRef parent type".into())
                        })?;
                Ok(MethodRefParent::Bare {
                    module_idx,
                    type_idx,
                })
            }
            4 => {
                let signature =
                    crate::type_specs::type_spec_signature(src_idx, parent, &self.modules)
                        .ok_or_else(|| {
                            RuntimeError::ExecutionError("invalid MethodRef parent TypeSpec".into())
                        })?;
                let (base_module_idx, base_type_idx) =
                    crate::type_specs::resolve_type_location(src_idx, parent, &self.modules)
                        .ok_or_else(|| {
                            RuntimeError::ExecutionError(
                                "unresolved MethodRef parent TypeSpec".into(),
                            )
                        })?;
                Ok(MethodRefParent::Specialized {
                    source_module_idx: src_idx,
                    signature,
                    base_module_idx,
                    base_type_idx,
                })
            }
            table => Err(RuntimeError::ExecutionError(format!(
                "unexpected MethodRef parent token table ID: {}",
                table
            ))),
        }
    }

    fn resolve_method_candidate(
        &self,
        parent: &MethodRefParent,
        reference_module_idx: usize,
        method_name: &str,
        reference_signature: &(Vec<TypeSignature>, TypeSignature),
        reference_has_receiver: bool,
    ) -> Result<MethodCandidate, RuntimeError> {
        let mut candidates = Vec::new();
        match parent {
            MethodRefParent::Module { module_idx } => {
                self.append_method_candidates(
                    *module_idx,
                    self.modules[*module_idx].module.top_level_method_indices(),
                    true,
                    0,
                    None,
                    None,
                    reference_module_idx,
                    method_name,
                    reference_signature,
                    reference_has_receiver,
                    &mut candidates,
                );
            }
            MethodRefParent::Bare {
                module_idx,
                type_idx,
            } => {
                self.append_method_candidates(
                    *module_idx,
                    self.modules[*module_idx]
                        .module
                        .type_method_indices(*type_idx),
                    true,
                    0,
                    None,
                    None,
                    reference_module_idx,
                    method_name,
                    reference_signature,
                    reference_has_receiver,
                    &mut candidates,
                );
                for (impl_module_idx, loaded) in self.modules.iter().enumerate() {
                    for (impl_idx, implementation) in loaded.module.impl_defs.iter().enumerate() {
                        if !matches!(implementation.type_token.table_id(), 2 | 3)
                            || crate::type_specs::resolve_type_location(
                                impl_module_idx,
                                implementation.type_token,
                                &self.modules,
                            ) != Some((*module_idx, *type_idx))
                        {
                            continue;
                        }
                        self.append_method_candidates(
                            impl_module_idx,
                            loaded.module.impl_method_indices(impl_idx),
                            implementation.contract.is_null(),
                            0,
                            None,
                            None,
                            reference_module_idx,
                            method_name,
                            reference_signature,
                            reference_has_receiver,
                            &mut candidates,
                        );
                    }
                }
            }
            MethodRefParent::Specialized {
                source_module_idx,
                signature,
                base_module_idx,
                base_type_idx,
            } => {
                for (impl_module_idx, loaded) in self.modules.iter().enumerate() {
                    for (impl_idx, implementation) in loaded.module.impl_defs.iter().enumerate() {
                        if implementation.type_token.table_id() != 4
                            || crate::type_specs::resolve_type_location(
                                impl_module_idx,
                                implementation.type_token,
                                &self.modules,
                            ) != Some((*base_module_idx, *base_type_idx))
                        {
                            continue;
                        }
                        let Some(target_signature) = crate::type_specs::type_spec_signature(
                            impl_module_idx,
                            implementation.type_token,
                            &self.modules,
                        ) else {
                            continue;
                        };
                        if !crate::type_specs::matches_impl_specialization(
                            Some((impl_module_idx, &target_signature)),
                            Some((*source_module_idx, signature)),
                            None,
                            None,
                            &self.modules,
                        ) {
                            continue;
                        }
                        self.append_method_candidates(
                            impl_module_idx,
                            loaded.module.impl_method_indices(impl_idx),
                            implementation.contract.is_null(),
                            type_signature_specificity(&target_signature),
                            Some((impl_module_idx, &target_signature)),
                            Some((*source_module_idx, signature)),
                            reference_module_idx,
                            method_name,
                            reference_signature,
                            reference_has_receiver,
                            &mut candidates,
                        );
                    }
                }
            }
        }
        if candidates.iter().any(|candidate| candidate.inherent) {
            candidates.retain(|candidate| candidate.inherent);
        }
        if let Some(max_specificity) = candidates
            .iter()
            .map(|candidate| candidate.specificity)
            .max()
        {
            candidates.retain(|candidate| candidate.specificity == max_specificity);
        }
        match candidates.as_slice() {
            [candidate] => Ok(*candidate),
            [] => Err(RuntimeError::ExecutionError(format!(
                "unresolved method reference: '{}'",
                method_name
            ))),
            _ => Err(RuntimeError::ExecutionError(format!(
                "ambiguous method reference: '{}' matched {} definitions",
                method_name,
                candidates.len()
            ))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn append_method_candidates(
        &self,
        module_idx: usize,
        method_indices: Vec<usize>,
        inherent: bool,
        specificity: usize,
        pattern_parent: Option<(usize, &TypeSignature)>,
        actual_parent: Option<(usize, &TypeSignature)>,
        reference_module_idx: usize,
        method_name: &str,
        reference_signature: &(Vec<TypeSignature>, TypeSignature),
        reference_has_receiver: bool,
        candidates: &mut Vec<MethodCandidate>,
    ) {
        let module = &self.modules[module_idx].module;
        for method_idx in method_indices {
            let Some(method) = module.method_defs.get(method_idx) else {
                continue;
            };
            let definition_has_receiver = !method.owner.is_null()
                && method.flags & writ_module::tables::METHOD_FLAG_STATIC == 0;
            if definition_has_receiver != reference_has_receiver
                || read_string(&module.string_heap, method.name).ok() != Some(method_name)
                || !self.method_signature_matches(
                    pattern_parent,
                    actual_parent,
                    reference_module_idx,
                    reference_signature,
                    module_idx,
                    method.signature,
                )
            {
                continue;
            }
            candidates.push(MethodCandidate {
                module_idx,
                method_idx,
                inherent,
                specificity,
            });
        }
    }

    fn method_signature_matches(
        &self,
        pattern_parent: Option<(usize, &TypeSignature)>,
        actual_parent: Option<(usize, &TypeSignature)>,
        reference_module_idx: usize,
        reference: &(Vec<TypeSignature>, TypeSignature),
        definition_module_idx: usize,
        definition_offset: u32,
    ) -> bool {
        let (reference_params, reference_ret) = reference;
        let module = &self.modules[definition_module_idx].module;
        let Some((definition_params, definition_ret)) =
            read_blob(&module.blob_heap, definition_offset)
                .ok()
                .and_then(|blob| writ_module::signature::decode_method_signature(blob).ok())
        else {
            return false;
        };
        crate::type_specs::matches_method_specialization(
            pattern_parent,
            actual_parent,
            &definition_params,
            &definition_ret,
            definition_module_idx,
            reference_params,
            reference_ret,
            reference_module_idx,
            &self.modules,
        )
    }

    /// Find a module in the domain by its name.
    fn find_module_by_name(&self, name: &str) -> Option<usize> {
        for (idx, m) in self.modules.iter().enumerate() {
            let mod_name = read_string(
                &m.module.string_heap,
                m.module.module_defs.first().map_or(0, |d| d.name),
            ).unwrap_or("");
            if mod_name == name {
                return Some(idx);
            }
        }
        None
    }

    /// Find a TypeDef by (namespace, name) in a module.
    fn find_type_def_by_name(
        module: &writ_module::Module,
        namespace: &str,
        name: &str,
    ) -> Option<usize> {
        for (idx, td) in module.type_defs.iter().enumerate() {
            let td_name = read_string(&module.string_heap, td.name).unwrap_or("");
            let td_ns = read_string(&module.string_heap, td.namespace).unwrap_or("");
            if td_name == name && td_ns == namespace {
                return Some(idx);
            }
        }
        None
    }

    /// Find a ContractDef by (namespace, name) in a module.
    fn find_contract_def_by_name(
        module: &writ_module::Module,
        namespace: &str,
        name: &str,
    ) -> Option<usize> {
        for (idx, cd) in module.contract_defs.iter().enumerate() {
            let cd_name = read_string(&module.string_heap, cd.name).unwrap_or("");
            let cd_ns = read_string(&module.string_heap, cd.namespace).unwrap_or("");
            if cd_name == name && cd_ns == namespace {
                return Some(idx);
            }
        }
        None
    }

    /// Find a FieldDef by name within a type's field range.
    fn find_field_in_type(
        module: &writ_module::Module,
        type_idx: usize,
        field_name: &str,
    ) -> Option<usize> {
        let td = &module.type_defs[type_idx];
        let field_start = td.field_list.saturating_sub(1) as usize;
        let field_end = if type_idx + 1 < module.type_defs.len() {
            module.type_defs[type_idx + 1].field_list.saturating_sub(1) as usize
        } else {
            module.field_defs.len()
        };
        for idx in field_start..field_end {
            let fd_name = read_string(&module.string_heap, module.field_defs[idx].name).unwrap_or("");
            if fd_name == field_name {
                return Some(idx);
            }
        }
        None
    }

}

// ──── Attribute Query API ──────────────────────────────────────────────

/// A single attribute match returned by Domain query methods.
///
/// Carries the module index so callers can locate the owning module,
/// the decoded arguments, and the owner token identifying the definition
/// the attribute was applied to.
#[derive(Debug, Clone)]
pub struct DomainAttributeMatch {
    /// Index into Domain::modules for the module that owns this attribute row.
    pub module_idx: usize,
    /// Attribute name (from the string heap of the owning module).
    pub name: String,
    /// Decoded argument list (empty vec when the attribute has no arguments).
    pub args: Vec<writ_module::attr::AttrValue>,
    /// Metadata token of the owner (type, method, field, etc.).
    pub owner: writ_module::token::MetadataToken,
    /// Owner kind discriminant: 0 = type, 1 = method, 2 = field/global.
    /// Never 3 (ATTR_OWNER_KIND_DECL) — declaration rows are always filtered out.
    pub owner_kind: u8,
}

impl Domain {
    /// Return all attribute application rows across all loaded modules whose
    /// name matches `attr_name`.
    ///
    /// Declaration rows (`owner_kind == ATTR_OWNER_KIND_DECL`) are always excluded.
    /// Returns an empty vec if no matches are found.
    pub fn query_attributes(&self, attr_name: &str) -> Vec<DomainAttributeMatch> {
        use writ_module::tables::ATTR_OWNER_KIND_DECL;

        let mut result = Vec::new();
        for (module_idx, loaded) in self.modules.iter().enumerate() {
            let module = &loaded.module;
            for row in &module.attribute_defs {
                if row.owner_kind == ATTR_OWNER_KIND_DECL {
                    continue;
                }
                if writ_module::heap::read_string(&module.string_heap, row.name).ok()
                    != Some(attr_name)
                {
                    continue;
                }
                result.push(DomainAttributeMatch {
                    module_idx,
                    name: attr_name.to_owned(),
                    args: decode_row_args(module, row),
                    owner: row.owner,
                    owner_kind: row.owner_kind,
                });
            }
        }
        result
    }

    /// Return all attribute application rows on the TypeDef at 0-based `typedef_idx`
    /// in the module at `module_idx`.
    ///
    /// Declaration rows are excluded. Returns an empty vec if `module_idx` is out of
    /// range or if the typedef has no attributes.
    pub fn query_attributes_on(
        &self,
        module_idx: usize,
        typedef_idx: usize,
    ) -> Vec<DomainAttributeMatch> {
        use writ_module::tables::{ATTR_OWNER_KIND_DECL, TableId};

        if module_idx >= self.modules.len() {
            return Vec::new();
        }
        let module = &self.modules[module_idx].module;
        let target_row = (typedef_idx + 1) as u32; // 0-based to 1-based

        let mut result = Vec::new();
        for row in &module.attribute_defs {
            if row.owner_kind == ATTR_OWNER_KIND_DECL {
                continue;
            }
            if row.owner.table_id() != TableId::TypeDef.as_u8() {
                continue;
            }
            if row.owner.row_index() != Some(target_row) {
                continue;
            }
            let name = writ_module::heap::read_string(&module.string_heap, row.name)
                .unwrap_or("<unknown>")
                .to_owned();
            result.push(DomainAttributeMatch {
                module_idx,
                name,
                args: decode_row_args(module, row),
                owner: row.owner,
                owner_kind: row.owner_kind,
            });
        }
        result
    }

    /// Return the decoded arguments for the first attribute matching `attr_name`
    /// on `owner_token` in the module at `module_idx`, or `None` if not found.
    ///
    /// Declaration rows are excluded. Returns `None` if `module_idx` is out of range.
    pub fn query_attribute_value(
        &self,
        module_idx: usize,
        owner_token: writ_module::token::MetadataToken,
        attr_name: &str,
    ) -> Option<Vec<writ_module::attr::AttrValue>> {
        use writ_module::tables::ATTR_OWNER_KIND_DECL;

        if module_idx >= self.modules.len() {
            return None;
        }
        let module = &self.modules[module_idx].module;

        module
            .attribute_defs
            .iter()
            .find(|row| {
            row.owner_kind != ATTR_OWNER_KIND_DECL
                && row.owner == owner_token
                && writ_module::heap::read_string(&module.string_heap, row.name).ok()
                    == Some(attr_name)
            })
            .map(|row| decode_row_args(module, row))
    }
}

/// Decode attribute args from a single AttributeDefRow.
///
/// Blob offset 0 (null blob) means no args — returns empty vec without touching read_blob.
/// On any decode failure, returns empty vec (never panics).
fn decode_row_args(
    module: &writ_module::Module,
    row: &writ_module::tables::AttributeDefRow,
) -> Vec<writ_module::attr::AttrValue> {
    if row.value == 0 {
        return Vec::new();
    }
    match writ_module::heap::read_blob(&module.blob_heap, row.value) {
        Ok(blob) => writ_module::attr::decode_attr_args(blob).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{DispatchTarget, IntrinsicId};
    use writ_module::module::MethodBody;
    use writ_module::tables::TypeDefKind;
    use writ_module::{Instruction, ModuleBuilder};

    fn empty_body() -> MethodBody {
        MethodBody {
            register_types: vec![],
            code: vec![],
            debug_locals: vec![],
            source_spans: vec![],
        }
    }

    fn void_signature() -> Vec<u8> {
        writ_module::signature::encode_method_signature(
            &[],
            &writ_module::signature::TypeSignature::Void,
        )
        .unwrap()
    }

    #[test]
    fn domain_new_is_empty() {
        let domain = Domain::new();
        assert!(domain.modules.is_empty());
    }

    #[test]
    fn add_module_returns_index_zero() {
        let mut domain = Domain::new();
        let module = ModuleBuilder::new("test").build();
        let idx = domain.add_module(module).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(domain.modules.len(), 1);
    }

    #[test]
    fn add_two_modules_returns_sequential_indices() {
        let mut domain = Domain::new();
        let m1 = ModuleBuilder::new("mod-a").build();
        let m2 = ModuleBuilder::new("mod-b").build();
        let idx1 = domain.add_module(m1).unwrap();
        let idx2 = domain.add_module(m2).unwrap();
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(domain.modules.len(), 2);
    }

    #[test]
    fn resolve_refs_on_empty_domain_succeeds() {
        let mut domain = Domain::new();
        domain.resolve_refs().unwrap();
    }

    #[test]
    fn resolve_refs_on_module_with_no_refs_succeeds() {
        let mut domain = Domain::new();
        let module = ModuleBuilder::new("test").build();
        domain.add_module(module).unwrap();
        domain.resolve_refs().unwrap();
        let resolved = &domain.modules[0].resolved_refs;
        assert!(resolved.types.is_empty());
        assert!(resolved.methods.is_empty());
        assert!(resolved.fields.is_empty());
    }

    #[test]
    fn typeref_resolves_to_typedef_in_target_module() {
        let mut domain = Domain::new();

        // Module A: has a TypeDef "Foo" in namespace "ns"
        let mut builder_a = ModuleBuilder::new("mod-a");
        builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        domain.add_module(builder_a.build()).unwrap();

        // Module B: has a ModuleRef to "mod-a" and a TypeRef to "Foo" in "ns"
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        builder_b.add_type_ref(mod_ref, "Foo", "ns");
        domain.add_module(builder_b.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[1].resolved_refs;
        assert_eq!(resolved.types.len(), 1);
        let rt = resolved
            .types
            .get(&0)
            .expect("TypeRef 0 should be resolved");
        assert_eq!(rt.module_idx, 0, "should point to mod-a");
        assert_eq!(rt.typedef_idx, 0, "should point to first TypeDef");
    }

    #[test]
    fn methodref_resolves_to_methoddef_in_target_module() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo" with method "bar"
        let mut builder_a = ModuleBuilder::new("mod-a");
        let foo = builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        let signature = void_signature();
        builder_a.add_type_method(foo, "bar", &signature, 0, 0, empty_body());
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references "bar" on Foo from mod-a
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        let type_ref = builder_b.add_type_ref(mod_ref, "Foo", "ns");
        builder_b.add_method_ref(type_ref, "bar", &signature);
        domain.add_module(builder_b.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[1].resolved_refs;
        assert_eq!(resolved.methods.len(), 1);
        let rm = resolved
            .methods
            .get(&0)
            .expect("MethodRef 0 should be resolved");
        assert_eq!(rm.module_idx, 0, "should point to mod-a");
        assert_eq!(rm.method_idx, 0, "should point to first MethodDef");
    }

    #[test]
    fn methodref_overloads_resolve_by_canonical_signature() {
        use writ_module::signature::{TypeSignature, encode_method_signature};

        let mut library = ModuleBuilder::new("overload-library");
        let argument = library.add_type_def("Argument", "lib", TypeDefKind::Class, 0);
        let picker = library.add_type_def("Picker", "lib", TypeDefKind::Class, 0);
        let implementation = library.add_impl_def(picker, MetadataToken::NULL);
        let named_definition =
            encode_method_signature(&[TypeSignature::Named(argument)], &TypeSignature::Int)
                .unwrap();
        let bool_signature =
            encode_method_signature(&[TypeSignature::Bool], &TypeSignature::Int).unwrap();
        let named_method = library.add_impl_method(
            implementation,
            "choose",
            &named_definition,
            0,
            2,
            empty_body(),
        );
        let bool_method = library.add_impl_method(
            implementation,
            "choose",
            &bool_signature,
            0,
            2,
            empty_body(),
        );

        let mut user = ModuleBuilder::new("overload-user");
        let library_ref = user.add_module_ref("overload-library", "1.0.0");
        let argument_ref = user.add_type_ref(library_ref, "Argument", "lib");
        let picker_ref = user.add_type_ref(library_ref, "Picker", "lib");
        let named_reference =
            encode_method_signature(&[TypeSignature::Named(argument_ref)], &TypeSignature::Int)
                .unwrap();
        user.add_method_ref(picker_ref, "choose", &named_reference);
        user.add_method_ref(picker_ref, "choose", &bool_signature);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        domain.resolve_refs().unwrap();
        let resolved = &domain.modules[1].resolved_refs.methods;
        assert_eq!(
            resolved[&0].method_idx,
            named_method.row_index().unwrap() as usize - 1
        );
        assert_eq!(
            resolved[&1].method_idx,
            bool_method.row_index().unwrap() as usize - 1
        );
    }

    #[test]
    fn methodref_receiver_abi_selects_static_and_instance_definitions() {
        use writ_module::tables::{METHOD_FLAG_STATIC, METHOD_REF_FLAG_HAS_RECEIVER};

        let signature = void_signature();
        let mut library = ModuleBuilder::new("receiver-library");
        let utility = library.add_type_def("Utility", "lib", TypeDefKind::Class, 0);
        let implementation = library.add_impl_def(utility, MetadataToken::NULL);
        let instance = library.add_impl_method(
            implementation,
            "identity",
            &signature,
            0,
            1,
            empty_body(),
        );
        let static_method = library.add_impl_method(
            implementation,
            "identity",
            &signature,
            METHOD_FLAG_STATIC,
            0,
            empty_body(),
        );

        let mut user = ModuleBuilder::new("receiver-user");
        let library_ref = user.add_module_ref("receiver-library", "1.0.0");
        let utility_ref = user.add_type_ref(library_ref, "Utility", "lib");
        user.add_method_ref_with_flags(
            utility_ref,
            "identity",
            &signature,
            METHOD_REF_FLAG_HAS_RECEIVER,
        );
        user.add_method_ref_with_flags(utility_ref, "identity", &signature, 0);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        domain.resolve_refs().unwrap();
        let resolved = &domain.modules[1].resolved_refs.methods;
        assert_eq!(
            resolved[&0].method_idx,
            instance.row_index().unwrap() as usize - 1
        );
        assert_eq!(
            resolved[&1].method_idx,
            static_method.row_index().unwrap() as usize - 1
        );
    }

    #[test]
    fn stale_static_methodref_does_not_bind_instance_definition() {
        let signature = void_signature();
        let mut library = ModuleBuilder::new("receiver-library");
        let utility = library.add_type_def("Utility", "lib", TypeDefKind::Class, 0);
        let implementation = library.add_impl_def(utility, MetadataToken::NULL);
        library.add_impl_method(
            implementation,
            "identity",
            &signature,
            0,
            1,
            empty_body(),
        );

        let mut user = ModuleBuilder::new("receiver-user");
        let library_ref = user.add_module_ref("receiver-library", "1.0.0");
        let utility_ref = user.add_type_ref(library_ref, "Utility", "lib");
        user.add_method_ref_with_flags(utility_ref, "identity", &signature, 0);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        let error = domain.resolve_refs().unwrap_err().to_string();
        assert!(error.contains("unresolved method reference: 'identity'"), "{error}");
    }

    #[test]
    fn equally_specific_inherent_methodrefs_remain_ambiguous() {
        let signature = void_signature();
        let mut library = ModuleBuilder::new("ambiguous-library");
        let utility = library.add_type_def("Utility", "lib", TypeDefKind::Class, 0);
        for _ in 0..2 {
            let implementation = library.add_impl_def(utility, MetadataToken::NULL);
            library.add_impl_method(
                implementation,
                "identity",
                &signature,
                0,
                1,
                empty_body(),
            );
        }

        let mut user = ModuleBuilder::new("ambiguous-user");
        let library_ref = user.add_module_ref("ambiguous-library", "1.0.0");
        let utility_ref = user.add_type_ref(library_ref, "Utility", "lib");
        user.add_method_ref(utility_ref, "identity", &signature);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        let error = domain.resolve_refs().unwrap_err().to_string();
        assert!(
            error.contains("ambiguous method reference: 'identity' matched 2 definitions"),
            "{error}"
        );
    }

    #[test]
    fn reserved_methodref_flags_fail_closed() {
        let signature = void_signature();
        let mut library = ModuleBuilder::new("flag-library");
        library.add_type_def("Utility", "lib", TypeDefKind::Class, 0);
        let mut user = ModuleBuilder::new("receiver-user");
        let library_ref = user.add_module_ref("flag-library", "1.0.0");
        let utility_ref = user.add_type_ref(library_ref, "Utility", "lib");
        user.add_method_ref_with_flags(utility_ref, "identity", &signature, 1 << 15);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        let error = domain.resolve_refs().unwrap_err().to_string();
        assert!(error.contains("invalid MethodRef flags: 0x8000"), "{error}");
    }

    #[test]
    fn malformed_methodref_signature_fails_closed() {
        use writ_module::signature::{TypeSignature, encode_method_signature};

        let mut library = ModuleBuilder::new("ambiguous-library");
        let picker = library.add_type_def("Picker", "lib", TypeDefKind::Class, 0);
        let implementation = library.add_impl_def(picker, MetadataToken::NULL);
        let signature =
            encode_method_signature(&[TypeSignature::Int], &TypeSignature::Int).unwrap();
        library.add_impl_method(implementation, "choose", &signature, 0, 2, empty_body());
        let mut user = ModuleBuilder::new("ambiguous-user");
        let module_ref = user.add_module_ref("ambiguous-library", "1.0.0");
        let picker_ref = user.add_type_ref(module_ref, "Picker", "lib");
        user.add_method_ref(picker_ref, "choose", &[]);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        let error = domain.resolve_refs().unwrap_err().to_string();
        assert!(error.contains("invalid MethodRef signature"), "{error}");
    }

    #[test]
    fn methodref_typespec_parent_selects_exact_specialized_impl() {
        use writ_module::signature::{
            TypeSignature, encode_method_signature, encode_type_signature,
        };

        let method_signature = encode_method_signature(&[], &TypeSignature::Int).unwrap();
        let int_parent = TypeSignature::Generic {
            namespace: "lib".into(),
            name: "Crate".into(),
            args: vec![TypeSignature::Int],
        };
        let string_parent = TypeSignature::Generic {
            namespace: "lib".into(),
            name: "Crate".into(),
            args: vec![TypeSignature::String],
        };
        let mut library = ModuleBuilder::new("specialized-library");
        library.add_type_def("Crate", "lib", TypeDefKind::Class, 0);
        let int_spec = library.add_type_spec(&encode_type_signature(&int_parent).unwrap());
        let string_spec = library.add_type_spec(&encode_type_signature(&string_parent).unwrap());
        let int_impl = library.add_impl_def(int_spec, MetadataToken::NULL);
        let int_method =
            library.add_impl_method(int_impl, "marker", &method_signature, 0, 1, empty_body());
        let string_impl = library.add_impl_def(string_spec, MetadataToken::NULL);
        library.add_impl_method(string_impl, "marker", &method_signature, 0, 1, empty_body());

        let mut user = ModuleBuilder::new("specialized-user");
        let module_ref = user.add_module_ref("specialized-library", "1.0.0");
        user.add_type_ref(module_ref, "Crate", "lib");
        let int_spec = user.add_type_spec(&encode_type_signature(&int_parent).unwrap());
        user.add_method_ref(int_spec, "marker", &method_signature);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        domain.resolve_refs().unwrap();
        assert_eq!(
            domain.modules[1].resolved_refs.methods[&0].method_idx,
            int_method.row_index().unwrap() as usize - 1
        );
    }

    #[test]
    fn methodref_open_impl_substitutes_parent_and_method_generics() {
        use writ_module::signature::{
            TypeSignature, encode_method_signature, encode_type_signature,
        };

        let open_parent = TypeSignature::Generic {
            namespace: "lib".into(),
            name: "Crate".into(),
            args: vec![TypeSignature::GenericParam(0)],
        };
        let concrete_parent = TypeSignature::Generic {
            namespace: "lib".into(),
            name: "Crate".into(),
            args: vec![TypeSignature::Int],
        };
        let definition_signature = encode_method_signature(
            &[TypeSignature::GenericParam(0), TypeSignature::GenericParam(1)],
            &TypeSignature::GenericParam(1),
        )
        .unwrap();
        let reference_signature = encode_method_signature(
            &[TypeSignature::Int, TypeSignature::String],
            &TypeSignature::String,
        )
        .unwrap();

        let mut library = ModuleBuilder::new("generic-library");
        library.add_type_def("Crate", "lib", TypeDefKind::Class, 0);
        let open_spec = library.add_type_spec(&encode_type_signature(&open_parent).unwrap());
        let implementation = library.add_impl_def(open_spec, MetadataToken::NULL);
        let selected = library.add_impl_method(
            implementation,
            "choose",
            &definition_signature,
            0,
            3,
            empty_body(),
        );

        let mut user = ModuleBuilder::new("generic-user");
        let module_ref = user.add_module_ref("generic-library", "1.0.0");
        user.add_type_ref(module_ref, "Crate", "lib");
        let concrete_spec =
            user.add_type_spec(&encode_type_signature(&concrete_parent).unwrap());
        user.add_method_ref(concrete_spec, "choose", &reference_signature);

        let mut domain = Domain::new();
        domain.add_module(library.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        domain.resolve_refs().unwrap();
        assert_eq!(
            domain.modules[1].resolved_refs.methods[&0].method_idx,
            selected.row_index().unwrap() as usize - 1
        );
    }

    #[test]
    fn methodref_discovers_extension_impl_in_another_module() {
        use writ_module::signature::{TypeSignature, encode_method_signature};

        let signature = encode_method_signature(&[], &TypeSignature::Int).unwrap();
        let mut base = ModuleBuilder::new("base-library");
        base.add_type_def("Foreign", "lib", TypeDefKind::Class, 0);
        let mut extension = ModuleBuilder::new("extension-library");
        let base_ref = extension.add_module_ref("base-library", "1.0.0");
        let foreign_ref = extension.add_type_ref(base_ref, "Foreign", "lib");
        let implementation = extension.add_impl_def(foreign_ref, MetadataToken::NULL);
        let extension_method =
            extension.add_impl_method(implementation, "marker", &signature, 0, 1, empty_body());
        let mut user = ModuleBuilder::new("extension-user");
        let base_ref = user.add_module_ref("base-library", "1.0.0");
        let foreign_ref = user.add_type_ref(base_ref, "Foreign", "lib");
        user.add_method_ref(foreign_ref, "marker", &signature);

        let mut domain = Domain::new();
        domain.add_module(base.build()).unwrap();
        domain.add_module(extension.build()).unwrap();
        domain.add_module(user.build()).unwrap();
        domain.resolve_refs().unwrap();
        let resolved = domain.modules[2].resolved_refs.methods[&0];
        assert_eq!(resolved.module_idx, 1);
        assert_eq!(
            resolved.method_idx,
            extension_method.row_index().unwrap() as usize - 1
        );
    }

    #[test]
    fn call_methodref_executes_impl_method_in_library_module() {
        fn body(instructions: &[Instruction], reg_count: usize) -> MethodBody {
            let mut code = Vec::new();
            for instruction in instructions {
                instruction.encode(&mut code).unwrap();
            }
            MethodBody {
                register_types: vec![0; reg_count],
                code,
                debug_locals: vec![],
                source_spans: vec![],
            }
        }

        let mut library = ModuleBuilder::new("test-library");
        let counter = library.add_type_def("Counter", "lib", TypeDefKind::Class, 0);
        let counter_impl = library.add_impl_def(counter, MetadataToken::NULL);
        let get_signature = writ_module::signature::encode_method_signature(
            &[],
            &writ_module::signature::TypeSignature::Int,
        )
        .unwrap();
        library.add_impl_method(
            counter_impl,
            "get",
            &get_signature,
            0,
            2,
            body(
                &[
                    Instruction::LoadInt {
                        r_dst: 1,
                        value: 42,
                    },
                    Instruction::Ret { r_src: 1 },
                ],
                2,
            ),
        );
        let library = library.build();

        let mut user = ModuleBuilder::new("test-user");
        let library_ref = user.add_module_ref("test-library", "1.0.0");
        let counter_ref = user.add_type_ref(library_ref, "Counter", "lib");
        let get_ref = user.add_method_ref(counter_ref, "get", &get_signature);
        user.add_method(
            "main",
            &get_signature,
            0,
            2,
            body(
                &[
                    Instruction::New {
                        r_dst: 0,
                        type_idx: counter_ref.0,
                    },
                    Instruction::Call {
                        r_dst: 1,
                        method_idx: get_ref.0,
                        r_base: 0,
                        argc: 1,
                    },
                    Instruction::Ret { r_src: 1 },
                ],
                2,
            ),
        );

        let mut runtime = crate::RuntimeBuilder::new(user.build())
            .with_library(library)
            .build()
            .unwrap();
        let task_id = runtime.spawn_task(0, vec![]).unwrap();
        runtime.tick(0.0, crate::ExecutionLimit::None);

        assert_eq!(
            runtime.task_state(task_id),
            Some(crate::TaskState::Completed)
        );
        assert_eq!(runtime.return_value(task_id), Some(crate::Value::Int(42)));
    }

    #[test]
    fn compiled_xmod_static_generic_and_top_level_calls_execute() {
        let library_source: &'static str = r#"
            pub class Utility {}
            impl Utility {
                pub fn identity(value: int) -> int { return value; }
            }
            pub class Crate<T> { pub value: T }
            impl<T> Crate<T> {
                pub fn choose<U>(self, value: U) -> U { return value; }
            }
            pub fn make_crate() -> Crate<int> {
                return new Crate<int> { value: 0 };
            }
            pub fn add_ints(left: int, right: int) -> int {
                return left + right;
            }
        "#;
        let library_bytes = writ_compiler::compile_source(library_source).unwrap();
        let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
        let user_source: &'static str = r#"
            pub fn main() -> int {
                let crate = make_crate();
                let chosen = crate.choose(9);
                let total = add_ints(chosen, 1);
                let utility = new Utility {};
                let result = utility.identity(total);
                return result;
            }
        "#;
        let user_bytes = writ_compiler::compile_with_libraries(user_source, &[&library])
            .expect("cross-module calls should compile");
        let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
        let main_idx = user
            .top_level_method_indices()
            .into_iter()
            .find(|index| {
                read_string(&user.string_heap, user.method_defs[*index].name).ok() == Some("main")
            })
            .unwrap();
        let body = &user.method_bodies[main_idx];
        let mut cursor = std::io::Cursor::new(&body.code);
        let calls: Vec<_> = std::iter::from_fn(|| {
            ((cursor.position() as usize) < body.code.len())
                .then(|| Instruction::decode(&mut cursor).unwrap())
        })
        .filter_map(|instruction| match instruction {
            Instruction::Call {
                method_idx,
                r_base,
                argc,
                ..
            } => Some((method_idx, r_base, argc)),
            _ => None,
        })
        .collect();
        assert!(calls.iter().all(|(method, _, _)| *method != 0));
        assert!(calls.iter().any(|(_, _, argc)| *argc == 1));

        let mut runtime = crate::RuntimeBuilder::new(user)
            .with_library(library)
            .build()
            .unwrap();
        let task_id = runtime.spawn_task(main_idx, vec![]).unwrap();
        runtime.tick(0.0, crate::ExecutionLimit::None);
        assert_eq!(
            runtime.task_state(task_id),
            Some(crate::TaskState::Completed)
        );
        assert_eq!(runtime.return_value(task_id), Some(crate::Value::Int(10)));
    }

    #[test]
    fn fieldref_resolves_to_fielddef_in_target_module() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo" with field "x"
        let mut builder_a = ModuleBuilder::new("mod-a");
        builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        builder_a.add_field_def("x", &[0x01], 0);
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references field "x" on Foo from mod-a
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        let type_ref = builder_b.add_type_ref(mod_ref, "Foo", "ns");
        builder_b.add_field_ref(type_ref, "x", &[0x01]);
        domain.add_module(builder_b.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[1].resolved_refs;
        assert_eq!(resolved.fields.len(), 1);
        let rf = resolved
            .fields
            .get(&0)
            .expect("FieldRef 0 should be resolved");
        assert_eq!(rf.module_idx, 0, "should point to mod-a");
        assert_eq!(rf.field_idx, 0, "should point to first FieldDef");
    }

    #[test]
    fn unresolvable_typeref_produces_error() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo"
        let builder_a = ModuleBuilder::new("mod-a");
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references non-existent "Bar" in mod-a
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        builder_b.add_type_ref(mod_ref, "Bar", "ns");
        domain.add_module(builder_b.build()).unwrap();

        let err = domain.resolve_refs().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("unresolved type reference"), "error: {}", msg);
        assert!(
            msg.contains("Bar"),
            "error should mention type name: {}",
            msg
        );
        assert!(
            msg.contains("mod-a"),
            "error should mention module name: {}",
            msg
        );
    }

    #[test]
    fn unresolvable_methodref_produces_error() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo" with no methods
        let mut builder_a = ModuleBuilder::new("mod-a");
        builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references non-existent method "baz" on Foo
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        let type_ref = builder_b.add_type_ref(mod_ref, "Foo", "ns");
        builder_b.add_method_ref(type_ref, "baz", &void_signature());
        domain.add_module(builder_b.build()).unwrap();

        let err = domain.resolve_refs().unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("unresolved method reference"),
            "error: {}",
            msg
        );
        assert!(
            msg.contains("baz"),
            "error should mention method name: {}",
            msg
        );
    }

    #[test]
    fn unresolvable_module_reference_produces_error() {
        let mut domain = Domain::new();

        // Module B references non-existent "mod-c"
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-c", "1.0.0");
        builder_b.add_type_ref(mod_ref, "Foo", "ns");
        domain.add_module(builder_b.build()).unwrap();

        let err = domain.resolve_refs().unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("unresolved module reference"),
            "error: {}",
            msg
        );
        assert!(
            msg.contains("mod-c"),
            "error should mention module name: {}",
            msg
        );
    }

    #[test]
    fn virtual_module_types_resolvable_from_user_module() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();

        // Add virtual module at index 0
        domain.add_module(build_writ_runtime_module()).unwrap();

        // User module references "Int" from writ-runtime
        let mut builder = ModuleBuilder::new("user-module");
        let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
        builder.add_type_ref(mod_ref, "Int", "writ");
        builder.add_type_ref(mod_ref, "Option", "writ");
        builder.add_type_ref(mod_ref, "Array", "writ");
        domain.add_module(builder.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[1].resolved_refs;
        assert_eq!(resolved.types.len(), 3);

        // Int should resolve to writ-runtime module
        let int_ref = resolved.types.get(&0).unwrap();
        assert_eq!(int_ref.module_idx, 0);

        let option_ref = resolved.types.get(&1).unwrap();
        assert_eq!(option_ref.module_idx, 0);

        let array_ref = resolved.types.get(&2).unwrap();
        assert_eq!(array_ref.module_idx, 0);
    }

    #[test]
    fn methodref_on_local_typedef_resolves() {
        let mut domain = Domain::new();

        // Single module with a TypeDef and a MethodRef pointing to a local method
        let mut builder = ModuleBuilder::new("self-contained");
        let type_token = builder.add_type_def("MyType", "app", TypeDefKind::Struct, 0);
        let signature = void_signature();
        builder.add_type_method(type_token, "do_thing", &signature, 0, 0, empty_body());
        // MethodRef with parent pointing to local TypeDef
        builder.add_method_ref(type_token, "do_thing", &signature);
        domain.add_module(builder.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[0].resolved_refs;
        assert_eq!(resolved.methods.len(), 1);
        let rm = resolved.methods.get(&0).unwrap();
        assert_eq!(rm.module_idx, 0);
        assert_eq!(rm.method_idx, 0);
    }

    #[test]
    fn unresolvable_fieldref_produces_error() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo" with field "x" only
        let mut builder_a = ModuleBuilder::new("mod-a");
        builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        builder_a.add_field_def("x", &[0x01], 0);
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references non-existent field "y" on Foo
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        let type_ref = builder_b.add_type_ref(mod_ref, "Foo", "ns");
        builder_b.add_field_ref(type_ref, "y", &[0x01]);
        domain.add_module(builder_b.build()).unwrap();

        let err = domain.resolve_refs().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("unresolved field reference"), "error: {}", msg);
        assert!(msg.contains("y"), "error should mention field name: {}", msg);
    }

    // ── Dispatch table tests ──────────────────────────────────────

    #[test]
    fn dispatch_table_virtual_module_has_36_intrinsic_entries() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();
        domain.add_module(build_writ_runtime_module()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();
        // Synthetic specialization contracts in the virtual module have distinct
        // canonical contract keys. Compiler TypeSpecs are additionally indexed by
        // structural target and contract patterns.
        // 36 original + 4 Reflectable impls + 22 Phase-103 reflection method impls
        // + 2 Phase 107 dynamic invocation impls (FieldInfo.set, MethodInfo.invoke) = 64
        // + 3 Phase 108 generic reflection impls (Type.type_args, MethodInfo.attributes, FieldInfo.attributes) = 67
        // + 4 Phase 116 Hashable impls (int, float, bool, string) = 71
        assert_eq!(
            table.len(),
            71,
            "expected 71 dispatch entries (no generic collisions)"
        );
    }

    #[test]
    fn dispatch_table_int_add_resolves_to_intrinsic() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();
        domain.add_module(build_writ_runtime_module()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        // Find type_key for Int
        let module = &domain.modules[0].module;
        let int_idx = module
            .type_defs
            .iter()
            .enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Int")
            .map(|(i, _)| i)
            .expect("Int type should exist");
        let type_key = (0u32 << 16) | (int_idx as u32);

        // Find Add contract index for ContractDef-based key
        let add_idx = module
            .contract_defs
            .iter()
            .enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Add")
            .map(|(i, _)| i)
            .expect("Add contract should exist");
        let contract_key = (0u32 << 16) | (add_idx as u32);

        // Compatibility lookup ignores the legacy TypeSpec shape hash.
        let target = table
            .get_any(type_key, contract_key, 0)
            .expect("should have dispatch entry for Int:Add");
        match target {
            DispatchTarget::Intrinsic(IntrinsicId::IntAdd) => {} // expected
            other => panic!("expected Intrinsic(IntAdd), got {:?}", other),
        }
    }

    #[test]
    fn dispatch_table_bool_eq_resolves_to_intrinsic() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();
        domain.add_module(build_writ_runtime_module()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        let module = &domain.modules[0].module;
        let bool_idx = module
            .type_defs
            .iter()
            .enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Bool")
            .map(|(i, _)| i)
            .expect("Bool type should exist");
        let type_key = (0u32 << 16) | (bool_idx as u32);

        // Find Eq contract index
        let eq_idx = module
            .contract_defs
            .iter()
            .enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Eq")
            .map(|(i, _)| i)
            .expect("Eq contract should exist");
        let contract_key = (0u32 << 16) | (eq_idx as u32);

        // Compatibility lookup ignores the legacy TypeSpec shape hash.
        let target = table
            .get_any(type_key, contract_key, 0)
            .expect("should have dispatch entry for Bool:Eq");
        match target {
            DispatchTarget::Intrinsic(IntrinsicId::BoolEq) => {} // expected
            other => panic!("expected Intrinsic(BoolEq), got {:?}", other),
        }
    }

    #[test]
    fn dispatch_table_nonexistent_returns_none() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();
        domain.add_module(build_writ_runtime_module()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        // Use an impossible key (also try get_any for completeness)
        assert!(
            table.get_any(0xFFFF_FFFF, 0xFFFF_FFFF, 99).is_none(),
            "non-existent key should return None"
        );
    }

    #[test]
    fn dispatch_table_user_impl_produces_method_target() {
        let mut domain = Domain::new();

        // Module with a type, a contract, a contract method, and a non-intrinsic impl
        let mut builder = ModuleBuilder::new("test-module");
        let my_type = builder.add_type_def("MyType", "app", TypeDefKind::Struct, 0);
        let my_contract = builder.add_contract_def("MyContract", "app");
        builder.add_contract_method("do_it", &[], 0);

        let impl_token = builder.add_impl_def(my_type, my_contract);
        // Non-intrinsic method (flags=0)
        builder.add_impl_method(impl_token, "do_it", &[], 0, 0, empty_body());

        domain.add_module(builder.build()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();
        assert_eq!(table.len(), 1, "should have exactly 1 dispatch entry");

        // type_key = (0 << 16) | typedef_idx(0)
        // contract_key = (0 << 16) | contractdef_idx(0)
        // Compatibility lookup ignores the legacy TypeSpec shape hash.
        let target = table.get_any(0, 0, 0).expect("should have dispatch entry");
        match target {
            DispatchTarget::Method {
                module_idx,
                method_idx,
            } => {
                assert_eq!(*module_idx, 0);
                assert_eq!(*method_idx, 0);
            }
            other => panic!("expected Method target, got {:?}", other),
        }
    }

    #[test]
    fn dispatch_table_all_intrinsic_types_covered() {
        use crate::virtual_module::build_writ_runtime_module;

        let mut domain = Domain::new();
        domain.add_module(build_writ_runtime_module()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        // 71 unique entries (FIX-02: distinct specialization contract tokens eliminate collisions)
        // 36 original + 4 Reflectable primitive impls (Phase 102) + 22 reflection method impls (Phase 103)
        // + 2 Phase 107 dynamic invocation impls (FieldInfo.set, MethodInfo.invoke) = 64
        // + 3 Phase 108 generic reflection impls (Type.type_args, MethodInfo.attributes, FieldInfo.attributes) = 67
        // + 4 Phase 116 Hashable impls (int, float, bool, string) = 71
        assert_eq!(table.len(), 71);

        // Spot check specific entries using ContractDef-based keys
        let module = &domain.modules[0].module;

        // Float:Mul
        let float_idx = module
            .type_defs
            .iter()
            .enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Float")
            .map(|(i, _)| i)
            .unwrap();
        let mul_idx = module
            .contract_defs
            .iter()
            .enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Mul")
            .map(|(i, _)| i)
            .unwrap();
        // Compatibility lookup ignores the legacy TypeSpec shape hash.
        match table.get_any(float_idx as u32, mul_idx as u32, 0) {
            Some(DispatchTarget::Intrinsic(IntrinsicId::FloatMul)) => {}
            other => panic!("expected Intrinsic(FloatMul), got {:?}", other),
        }

        // String:Eq
        let string_idx = module
            .type_defs
            .iter()
            .enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "String")
            .map(|(i, _)| i)
            .unwrap();
        let eq_idx = module
            .contract_defs
            .iter()
            .enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Eq")
            .map(|(i, _)| i)
            .unwrap();
        match table.get_any(string_idx as u32, eq_idx as u32, 0) {
            Some(DispatchTarget::Intrinsic(IntrinsicId::StringEq)) => {}
            other => panic!("expected Intrinsic(StringEq), got {:?}", other),
        }
    }

    // ── FIX-02: Generic dispatch key tests ───────────────────

    /// Two ImplDefs for the SAME type referencing the SAME ContractDef but
    /// registered with different method-level discriminators must produce
    /// distinct dispatch table entries (not overwrite each other).
    ///
    /// This mirrors the virtual module's `Int:Into<Float>` vs `Int:Into<String>`
    /// situation: both use the `into` contract but represent different specializations.
    ///
    /// The virtual module uses distinct synthetic contract definitions for its
    /// built-in monomorphizations. Compiler-emitted TypeSpecs instead use the
    /// structural dispatch index exercised by integration tests.
    #[test]
    fn two_same_contract_different_token_specializations_produce_two_entries() {
        let mut domain = Domain::new();

        let mut builder = ModuleBuilder::new("test-module");
        let my_type = builder.add_type_def("MyType", "app", TypeDefKind::Struct, 0);
        // Base "Into" contract
        let into_contract = builder.add_contract_def("Into", "app");
        builder.add_contract_method("into", &[], 0);

        // For compiler-generated specializations, each specialization gets its own
        // TypeRef token (pointing to Into<Float> and Into<String> respectively).
        // We simulate this with distinct ContractDefs (each has a unique token).
        let into_float = builder.add_contract_def("Into_Float_spec", "app");
        builder.add_contract_method("into", &[], 0);

        let into_string = builder.add_contract_def("Into_String_spec", "app");
        builder.add_contract_method("into", &[], 0);

        // ImplDef 1: MyType implements Into<Float> (distinct contract token)
        let into_float_impl = builder.add_impl_def(my_type, into_float);
        builder.add_impl_method(into_float_impl, "into_float_impl", &[], 0, 0, empty_body());

        // ImplDef 2: MyType implements Into<String> (distinct contract token)
        let into_string_impl = builder.add_impl_def(my_type, into_string);
        builder.add_impl_method(
            into_string_impl,
            "into_string_impl",
            &[],
            0,
            0,
            empty_body(),
        );

        // The base into_contract is unused in impls above but exists for reference
        let _ = into_contract;

        domain.add_module(builder.build()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        // Two distinct contract tokens -> two distinct dispatch entries
        assert_eq!(
            table.len(),
            2,
            "two distinct contract tokens must produce 2 dispatch entries; got {}",
            table.len()
        );
    }

    /// Non-generic ImplDef should continue to work correctly after FIX-02.
    #[test]
    fn non_generic_impl_still_works_after_fix02() {
        let mut domain = Domain::new();

        let mut builder = ModuleBuilder::new("test-module");
        let my_type = builder.add_type_def("MyType", "app", TypeDefKind::Struct, 0);
        let my_contract = builder.add_contract_def("Eq", "app");
        builder.add_contract_method("eq", &[], 0);

        let impl_token = builder.add_impl_def(my_type, my_contract);
        builder.add_impl_method(impl_token, "eq_impl", &[], 0, 0, empty_body());

        domain.add_module(builder.build()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();

        assert_eq!(
            table.len(),
            1,
            "single non-generic impl must produce exactly one dispatch entry; got {}",
            table.len()
        );
    }
}
