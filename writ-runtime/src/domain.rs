//! Multi-module domain with cross-module name resolution.
//!
//! A `Domain` holds all loaded modules (the virtual `writ-runtime` module
//! plus any user modules) and resolves cross-module TypeRef, MethodRef,
//! and FieldRef entries by name matching at load time.
//!
//! Resolution results are stored per-module in `ResolvedRefs` maps for
//! O(1) lookup at runtime.

use rustc_hash::FxHashMap;
use writ_module::heap::read_string;
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
            let resolved = self.resolve_module_refs(src_idx)?;
            self.modules[src_idx].resolved_refs = resolved;
        }
        Ok(())
    }

    // ── Resolution implementation ─────────────────────────────────

    /// Resolve all cross-module references for a single module.
    fn resolve_module_refs(&self, src_idx: usize) -> Result<ResolvedRefs, RuntimeError> {
        let mut resolved = ResolvedRefs::new();
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
            if let Some(typedef_idx) = Self::find_type_def_by_name(target_module, ref_ns, ref_name) {
                resolved.types.insert(ref_idx as u32, ResolvedType {
                    module_idx: target_mod_idx,
                    typedef_idx,
                });
            } else if let Some(contractdef_idx) = Self::find_contract_def_by_name(target_module, ref_ns, ref_name) {
                // TypeRef points to a ContractDef, not a TypeDef.
                // Store in the contracts map for dispatch table resolution.
                resolved.contracts.insert(ref_idx as u32, ResolvedContract {
                    module_idx: target_mod_idx,
                    contractdef_idx,
                });
            } else {
                return Err(RuntimeError::ExecutionError(format!(
                    "unresolved type reference: '{}::{}' in module '{}'",
                    ref_ns, ref_name, target_mod_name
                )));
            }
        }

        // ── MethodRef resolution ──────────────────────────────────
        for (ref_idx, method_ref) in src_module.method_refs.iter().enumerate() {
            let (target_mod_idx, type_idx) = self.resolve_parent_type(
                src_idx, method_ref.parent, &resolved
            )?;

            let method_name = read_string(&src_module.string_heap, method_ref.name)
                .map_err(|_| RuntimeError::ExecutionError("invalid MethodRef name".into()))?;

            let target_module = &self.modules[target_mod_idx].module;
            let method_idx = Self::find_method_in_type(target_module, type_idx, method_name)
                .ok_or_else(|| {
                    let type_name = read_string(
                        &target_module.string_heap,
                        target_module.type_defs[type_idx].name,
                    ).unwrap_or("<unknown>");
                    RuntimeError::ExecutionError(format!(
                        "unresolved method reference: '{}' on type '{}'",
                        method_name, type_name
                    ))
                })?;

            resolved.methods.insert(ref_idx as u32, ResolvedMethod {
                module_idx: target_mod_idx,
                method_idx,
            });
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
                    ).unwrap_or("<unknown>");
                    RuntimeError::ExecutionError(format!(
                        "unresolved field reference: '{}' on type '{}'",
                        field_name, type_name
                    ))
                })?;

            resolved.fields.insert(ref_idx as u32, ResolvedField {
                module_idx: target_mod_idx,
                field_idx,
            });
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
        let row_idx = parent.row_index()
            .ok_or_else(|| RuntimeError::ExecutionError(
                "parent token is null".into()
            ))?;
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
    fn find_type_def_by_name(module: &writ_module::Module, namespace: &str, name: &str) -> Option<usize> {
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
    fn find_contract_def_by_name(module: &writ_module::Module, namespace: &str, name: &str) -> Option<usize> {
        for (idx, cd) in module.contract_defs.iter().enumerate() {
            let cd_name = read_string(&module.string_heap, cd.name).unwrap_or("");
            let cd_ns = read_string(&module.string_heap, cd.namespace).unwrap_or("");
            if cd_name == name && cd_ns == namespace {
                return Some(idx);
            }
        }
        None
    }

    /// Find a MethodDef by name within a type's method range.
    fn find_method_in_type(module: &writ_module::Module, type_idx: usize, method_name: &str) -> Option<usize> {
        let td = &module.type_defs[type_idx];
        let method_start = td.method_list.saturating_sub(1) as usize;
        let method_end = if type_idx + 1 < module.type_defs.len() {
            module.type_defs[type_idx + 1].method_list.saturating_sub(1) as usize
        } else {
            module.method_defs.len()
        };
        for idx in method_start..method_end {
            let md_name = read_string(&module.string_heap, module.method_defs[idx].name).unwrap_or("");
            if md_name == method_name {
                return Some(idx);
            }
        }
        None
    }

    /// Find a FieldDef by name within a type's field range.
    fn find_field_in_type(module: &writ_module::Module, type_idx: usize, field_name: &str) -> Option<usize> {
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
        use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};

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

        module.attribute_defs.iter().find(|row| {
            row.owner_kind != ATTR_OWNER_KIND_DECL
                && row.owner == owner_token
                && writ_module::heap::read_string(&module.string_heap, row.name).ok()
                    == Some(attr_name)
        }).map(|row| decode_row_args(module, row))
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
    use writ_module::ModuleBuilder;

    fn empty_body() -> MethodBody {
        MethodBody {
            register_types: vec![],
            code: vec![],
            debug_locals: vec![],
            source_spans: vec![],
        }
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
        let rt = resolved.types.get(&0).expect("TypeRef 0 should be resolved");
        assert_eq!(rt.module_idx, 0, "should point to mod-a");
        assert_eq!(rt.typedef_idx, 0, "should point to first TypeDef");
    }

    #[test]
    fn methodref_resolves_to_methoddef_in_target_module() {
        let mut domain = Domain::new();

        // Module A: has TypeDef "Foo" with method "bar"
        let mut builder_a = ModuleBuilder::new("mod-a");
        builder_a.add_type_def("Foo", "ns", TypeDefKind::Struct, 0);
        builder_a.add_method("bar", &[], 0, 0, empty_body());
        domain.add_module(builder_a.build()).unwrap();

        // Module B: references "bar" on Foo from mod-a
        let mut builder_b = ModuleBuilder::new("mod-b");
        let mod_ref = builder_b.add_module_ref("mod-a", "1.0.0");
        let type_ref = builder_b.add_type_ref(mod_ref, "Foo", "ns");
        builder_b.add_method_ref(type_ref, "bar", &[]);
        domain.add_module(builder_b.build()).unwrap();

        domain.resolve_refs().unwrap();

        let resolved = &domain.modules[1].resolved_refs;
        assert_eq!(resolved.methods.len(), 1);
        let rm = resolved.methods.get(&0).expect("MethodRef 0 should be resolved");
        assert_eq!(rm.module_idx, 0, "should point to mod-a");
        assert_eq!(rm.method_idx, 0, "should point to first MethodDef");
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
        let rf = resolved.fields.get(&0).expect("FieldRef 0 should be resolved");
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
        assert!(msg.contains("Bar"), "error should mention type name: {}", msg);
        assert!(msg.contains("mod-a"), "error should mention module name: {}", msg);
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
        builder_b.add_method_ref(type_ref, "baz", &[]);
        domain.add_module(builder_b.build()).unwrap();

        let err = domain.resolve_refs().unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("unresolved method reference"), "error: {}", msg);
        assert!(msg.contains("baz"), "error should mention method name: {}", msg);
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
        assert!(msg.contains("unresolved module reference"), "error: {}", msg);
        assert!(msg.contains("mod-c"), "error should mention module name: {}", msg);
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
        builder.add_method("do_thing", &[], 0, 0, empty_body());
        // MethodRef with parent pointing to local TypeDef
        builder.add_method_ref(type_token, "do_thing", &[]);
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
        // FIX-02: Specialization-specific contract tokens in the virtual module assign
        // distinct type_args_hash values per generic specialization. All ImplDef entries
        // now produce distinct DispatchKeys (no collisions).
        // 36 original + 4 Reflectable impls + 22 Phase-103 reflection method impls
        // + 2 Phase 107 dynamic invocation impls (FieldInfo.set, MethodInfo.invoke) = 64
        // + 3 Phase 108 generic reflection impls (Type.type_args, MethodInfo.attributes, FieldInfo.attributes) = 67
        // + 4 Phase 116 Hashable impls (int, float, bool, string) = 71
        assert_eq!(table.len(), 71, "expected 71 dispatch entries (no generic collisions)");
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
        let int_idx = module.type_defs.iter().enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Int")
            .map(|(i, _)| i)
            .expect("Int type should exist");
        let type_key = (0u32 << 16) | (int_idx as u32);

        // Find Add contract index for ContractDef-based key
        let add_idx = module.contract_defs.iter().enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Add")
            .map(|(i, _)| i)
            .expect("Add contract should exist");
        let contract_key = (0u32 << 16) | (add_idx as u32);

        // Use get_any() since type_args_hash = impl_def.contract.0 (non-zero after FIX-02)
        let target = table.get_any(type_key, contract_key, 0)
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
        let bool_idx = module.type_defs.iter().enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Bool")
            .map(|(i, _)| i)
            .expect("Bool type should exist");
        let type_key = (0u32 << 16) | (bool_idx as u32);

        // Find Eq contract index
        let eq_idx = module.contract_defs.iter().enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Eq")
            .map(|(i, _)| i)
            .expect("Eq contract should exist");
        let contract_key = (0u32 << 16) | (eq_idx as u32);

        // Use get_any() since type_args_hash = impl_def.contract.0 (non-zero after FIX-02)
        let target = table.get_any(type_key, contract_key, 0)
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
        assert!(table.get_any(0xFFFF_FFFF, 0xFFFF_FFFF, 99).is_none(), "non-existent key should return None");
    }

    #[test]
    fn dispatch_table_user_impl_produces_method_target() {
        let mut domain = Domain::new();

        // Module with a type, a contract, a contract method, and a non-intrinsic impl
        let mut builder = ModuleBuilder::new("test-module");
        let my_type = builder.add_type_def("MyType", "app", TypeDefKind::Struct, 0);
        let my_contract = builder.add_contract_def("MyContract", "app");
        builder.add_contract_method("do_it", &[], 0);

        builder.add_impl_def(my_type, my_contract);
        // Non-intrinsic method (flags=0)
        builder.add_method("do_it", &[], 0, 0, empty_body());

        domain.add_module(builder.build()).unwrap();
        domain.resolve_refs().unwrap();

        let table = domain.build_dispatch_table();
        assert_eq!(table.len(), 1, "should have exactly 1 dispatch entry");

        // type_key = (0 << 16) | typedef_idx(0)
        // contract_key = (0 << 16) | contractdef_idx(0)
        // Use get_any() since type_args_hash = impl_def.contract.0 (non-zero after FIX-02)
        let target = table.get_any(0, 0, 0).expect("should have dispatch entry");
        match target {
            DispatchTarget::Method { module_idx, method_idx } => {
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
        let float_idx = module.type_defs.iter().enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "Float")
            .map(|(i, _)| i).unwrap();
        let mul_idx = module.contract_defs.iter().enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Mul")
            .map(|(i, _)| i).unwrap();
        // Use get_any() since type_args_hash = impl_def.contract.0 (non-zero after FIX-02)
        match table.get_any(float_idx as u32, mul_idx as u32, 0) {
            Some(DispatchTarget::Intrinsic(IntrinsicId::FloatMul)) => {}
            other => panic!("expected Intrinsic(FloatMul), got {:?}", other),
        }

        // String:Eq
        let string_idx = module.type_defs.iter().enumerate()
            .find(|(_, td)| read_string(&module.string_heap, td.name).unwrap_or("") == "String")
            .map(|(i, _)| i).unwrap();
        let eq_idx = module.contract_defs.iter().enumerate()
            .find(|(_, cd)| read_string(&module.string_heap, cd.name).unwrap_or("") == "Eq")
            .map(|(i, _)| i).unwrap();
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
    /// FIX-02 adds `type_args_hash` to DispatchKey using `impl_def.contract.0` as
    /// the discriminator. Since both ImplDefs have the same contract token, the
    /// virtual module needs to assign distinct contract tokens per specialization.
    ///
    /// For user code with a proper compiler: the compiler emits distinct TypeRef
    /// tokens per specialization (e.g., `Into<Float>` and `Into<String>` are
    /// different tokens), so `impl_def.contract.0` differs and they produce
    /// distinct keys.
    ///
    /// This test uses distinct contract tokens (simulating compiler-generated output)
    /// and verifies the two-entry result.
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
        builder.add_impl_def(my_type, into_float);
        builder.add_method("into_float_impl", &[], 0, 0, empty_body());

        // ImplDef 2: MyType implements Into<String> (distinct contract token)
        builder.add_impl_def(my_type, into_string);
        builder.add_method("into_string_impl", &[], 0, 0, empty_body());

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

        builder.add_impl_def(my_type, my_contract);
        builder.add_method("eq_impl", &[], 0, 0, empty_body());

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
