use crate::heap;
use crate::module::{MethodBody, Module, ModuleHeader};
use crate::tables::*;
use crate::token::MetadataToken;

/// Builder for programmatic construction of IL modules.
///
/// Users work with `&str` for string fields and `&[u8]` for blob fields.
/// `build()` interns everything into heaps and produces a complete `Module`.
pub struct ModuleBuilder {
    name: String,
    version: String,
    type_defs: Vec<TypeDefBuilder>,
    type_refs: Vec<TypeRefBuilder>,
    field_defs: Vec<FieldDefBuilder>,
    method_defs: Vec<MethodDefBuilder>,
    method_bodies: Vec<MethodBody>,
    param_defs: Vec<ParamDefBuilder>,
    contract_defs: Vec<ContractDefBuilder>,
    contract_methods: Vec<ContractMethodBuilder>,
    impl_defs: Vec<ImplDefRow>,
    generic_params: Vec<GenericParamBuilder>,
    generic_constraints: Vec<GenericConstraintRow>,
    global_defs: Vec<GlobalDefBuilder>,
    extern_defs: Vec<ExternDefBuilder>,
    module_refs: Vec<ModuleRefBuilder>,
    component_slots: Vec<ComponentSlotRow>,
    locale_defs: Vec<LocaleDefBuilder>,
    export_defs: Vec<ExportDefBuilder>,
    attribute_defs: Vec<AttributeDefBuilder>,
    type_specs: Vec<TypeSpecBuilder>,
    field_refs: Vec<FieldRefBuilder>,
    method_refs: Vec<MethodRefBuilder>,
}

// ── Builder row types ──────────────────────────────────────────

struct TypeDefBuilder {
    name: String,
    namespace: String,
    kind: u8,
    flags: u16,
    field_list: u32,
    method_list: u32,
}

struct TypeRefBuilder {
    scope: MetadataToken,
    name: String,
    namespace: String,
}

struct FieldDefBuilder {
    name: String,
    type_sig: Vec<u8>,
    flags: u16,
}

struct MethodDefBuilder {
    name: String,
    signature: Vec<u8>,
    flags: u16,
    reg_count: u16,
}

struct ParamDefBuilder {
    name: String,
    type_sig: Vec<u8>,
    sequence: u16,
}

struct ContractDefBuilder {
    name: String,
    namespace: String,
    method_list: u32,
    generic_param_list: u32,
}

struct ContractMethodBuilder {
    name: String,
    signature: Vec<u8>,
    slot: u16,
}

struct GenericParamBuilder {
    owner: MetadataToken,
    owner_kind: u8,
    ordinal: u16,
    name: String,
}

struct GlobalDefBuilder {
    name: String,
    type_sig: Vec<u8>,
    flags: u16,
    init_value: Vec<u8>,
}

struct ExternDefBuilder {
    name: String,
    signature: Vec<u8>,
    import_name: String,
    flags: u16,
}

struct ModuleRefBuilder {
    name: String,
    min_version: String,
}

struct LocaleDefBuilder {
    dlg_method: MetadataToken,
    locale: String,
    loc_method: MetadataToken,
}

struct ExportDefBuilder {
    name: String,
    item_kind: u8,
    item: MetadataToken,
}

struct AttributeDefBuilder {
    owner: MetadataToken,
    owner_kind: u8,
    name: String,
    value: Vec<u8>,
}

struct TypeSpecBuilder {
    signature: Vec<u8>,
}

struct FieldRefBuilder {
    parent: MetadataToken,
    name: String,
    type_sig: Vec<u8>,
}

struct MethodRefBuilder {
    parent: MetadataToken,
    name: String,
    signature: Vec<u8>,
}

// ── ModuleBuilder implementation ───────────────────────────────

impl ModuleBuilder {
    /// Create a new builder for a module with the given name.
    pub fn new(name: &str) -> Self {
        ModuleBuilder {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            type_defs: Vec::new(),
            type_refs: Vec::new(),
            field_defs: Vec::new(),
            method_defs: Vec::new(),
            method_bodies: Vec::new(),
            param_defs: Vec::new(),
            contract_defs: Vec::new(),
            contract_methods: Vec::new(),
            impl_defs: Vec::new(),
            generic_params: Vec::new(),
            generic_constraints: Vec::new(),
            global_defs: Vec::new(),
            extern_defs: Vec::new(),
            module_refs: Vec::new(),
            component_slots: Vec::new(),
            locale_defs: Vec::new(),
            export_defs: Vec::new(),
            attribute_defs: Vec::new(),
            type_specs: Vec::new(),
            field_refs: Vec::new(),
            method_refs: Vec::new(),
        }
    }

    /// Set the module version (semver string).
    pub fn version(mut self, v: &str) -> Self {
        self.version = v.to_string();
        self
    }

    /// Add a type definition.
    ///
    /// `field_list` and `method_list` use the "next index" pattern:
    /// the builder records the current counts, so add a type's fields
    /// and methods immediately after adding the type.
    pub fn add_type_def(&mut self, name: &str, namespace: &str, kind: u8, flags: u16) -> MetadataToken {
        let idx = self.type_defs.len() as u32 + 1;
        let field_list = self.field_defs.len() as u32 + 1;
        let method_list = self.method_defs.len() as u32 + 1;
        self.type_defs.push(TypeDefBuilder {
            name: name.to_string(),
            namespace: namespace.to_string(),
            kind,
            flags,
            field_list,
            method_list,
        });
        MetadataToken::new(TableId::TypeDef.as_u8(), idx)
    }

    /// Add a field definition.
    pub fn add_field_def(&mut self, name: &str, type_sig: &[u8], flags: u16) -> MetadataToken {
        let idx = self.field_defs.len() as u32 + 1;
        self.field_defs.push(FieldDefBuilder {
            name: name.to_string(),
            type_sig: type_sig.to_vec(),
            flags,
        });
        MetadataToken::new(TableId::FieldDef.as_u8(), idx)
    }

    /// Add a method with a body. Returns the method's token.
    pub fn add_method(&mut self, name: &str, signature: &[u8], flags: u16, reg_count: u16, body: MethodBody) -> MetadataToken {
        let idx = self.method_defs.len() as u32 + 1;
        self.method_defs.push(MethodDefBuilder {
            name: name.to_string(),
            signature: signature.to_vec(),
            flags,
            reg_count,
        });
        self.method_bodies.push(body);
        MetadataToken::new(TableId::MethodDef.as_u8(), idx)
    }

    /// Add a contract definition.
    pub fn add_contract_def(&mut self, name: &str, namespace: &str) -> MetadataToken {
        let idx = self.contract_defs.len() as u32 + 1;
        let method_list = self.contract_methods.len() as u32 + 1;
        let generic_param_list = self.generic_params.len() as u32 + 1;
        self.contract_defs.push(ContractDefBuilder {
            name: name.to_string(),
            namespace: namespace.to_string(),
            method_list,
            generic_param_list,
        });
        MetadataToken::new(TableId::ContractDef.as_u8(), idx)
    }

    /// Add a contract method slot.
    pub fn add_contract_method(&mut self, name: &str, signature: &[u8], slot: u16) -> MetadataToken {
        let idx = self.contract_methods.len() as u32 + 1;
        self.contract_methods.push(ContractMethodBuilder {
            name: name.to_string(),
            signature: signature.to_vec(),
            slot,
        });
        MetadataToken::new(TableId::ContractMethod.as_u8(), idx)
    }

    /// Add a contract implementation.
    pub fn add_impl_def(&mut self, type_token: MetadataToken, contract: MetadataToken) -> MetadataToken {
        let idx = self.impl_defs.len() as u32 + 1;
        let method_list = self.method_defs.len() as u32 + 1;
        self.impl_defs.push(ImplDefRow {
            type_token,
            contract,
            method_list,
        });
        MetadataToken::new(TableId::ImplDef.as_u8(), idx)
    }

    /// Add a generic parameter.
    pub fn add_generic_param(&mut self, owner: MetadataToken, owner_kind: u8, ordinal: u16, name: &str) -> MetadataToken {
        let idx = self.generic_params.len() as u32 + 1;
        self.generic_params.push(GenericParamBuilder {
            owner,
            owner_kind,
            ordinal,
            name: name.to_string(),
        });
        MetadataToken::new(TableId::GenericParam.as_u8(), idx)
    }

    /// Add a generic constraint.
    pub fn add_generic_constraint(&mut self, param: u32, constraint: MetadataToken) -> MetadataToken {
        let idx = self.generic_constraints.len() as u32 + 1;
        self.generic_constraints.push(GenericConstraintRow { param, constraint });
        MetadataToken::new(TableId::GenericConstraint.as_u8(), idx)
    }

    /// Add a global definition.
    pub fn add_global_def(&mut self, name: &str, type_sig: &[u8], flags: u16, init_value: &[u8]) -> MetadataToken {
        let idx = self.global_defs.len() as u32 + 1;
        self.global_defs.push(GlobalDefBuilder {
            name: name.to_string(),
            type_sig: type_sig.to_vec(),
            flags,
            init_value: init_value.to_vec(),
        });
        MetadataToken::new(TableId::GlobalDef.as_u8(), idx)
    }

    /// Add an extern definition.
    pub fn add_extern_def(&mut self, name: &str, signature: &[u8], import_name: &str, flags: u16) -> MetadataToken {
        let idx = self.extern_defs.len() as u32 + 1;
        self.extern_defs.push(ExternDefBuilder {
            name: name.to_string(),
            signature: signature.to_vec(),
            import_name: import_name.to_string(),
            flags,
        });
        MetadataToken::new(TableId::ExternDef.as_u8(), idx)
    }

    /// Add a module reference (dependency).
    pub fn add_module_ref(&mut self, name: &str, min_version: &str) -> MetadataToken {
        let idx = self.module_refs.len() as u32 + 1;
        self.module_refs.push(ModuleRefBuilder {
            name: name.to_string(),
            min_version: min_version.to_string(),
        });
        MetadataToken::new(TableId::ModuleRef.as_u8(), idx)
    }

    /// Add a component slot binding.
    pub fn add_component_slot(&mut self, owner: MetadataToken, component_type: MetadataToken) -> MetadataToken {
        let idx = self.component_slots.len() as u32 + 1;
        self.component_slots.push(ComponentSlotRow { owner_entity: owner, component_type });
        MetadataToken::new(TableId::ComponentSlot.as_u8(), idx)
    }

    /// Add a locale definition for dialogue dispatch.
    pub fn add_locale_def(&mut self, dlg_method: MetadataToken, locale: &str, loc_method: MetadataToken) -> MetadataToken {
        let idx = self.locale_defs.len() as u32 + 1;
        self.locale_defs.push(LocaleDefBuilder {
            dlg_method,
            locale: locale.to_string(),
            loc_method,
        });
        MetadataToken::new(TableId::LocaleDef.as_u8(), idx)
    }

    /// Add an export definition.
    pub fn add_export_def(&mut self, name: &str, item_kind: u8, item: MetadataToken) -> MetadataToken {
        let idx = self.export_defs.len() as u32 + 1;
        self.export_defs.push(ExportDefBuilder {
            name: name.to_string(),
            item_kind,
            item,
        });
        MetadataToken::new(TableId::ExportDef.as_u8(), idx)
    }

    /// Add an attribute definition.
    pub fn add_attribute_def(&mut self, owner: MetadataToken, owner_kind: u8, name: &str, value: &[u8]) -> MetadataToken {
        let idx = self.attribute_defs.len() as u32 + 1;
        self.attribute_defs.push(AttributeDefBuilder {
            owner,
            owner_kind,
            name: name.to_string(),
            value: value.to_vec(),
        });
        MetadataToken::new(TableId::AttributeDef.as_u8(), idx)
    }

    /// Add a type spec (instantiated generic type).
    pub fn add_type_spec(&mut self, signature: &[u8]) -> MetadataToken {
        let idx = self.type_specs.len() as u32 + 1;
        self.type_specs.push(TypeSpecBuilder {
            signature: signature.to_vec(),
        });
        MetadataToken::new(TableId::TypeSpec.as_u8(), idx)
    }

    /// Add a cross-module field reference.
    pub fn add_field_ref(&mut self, parent: MetadataToken, name: &str, type_sig: &[u8]) -> MetadataToken {
        let idx = self.field_refs.len() as u32 + 1;
        self.field_refs.push(FieldRefBuilder {
            parent,
            name: name.to_string(),
            type_sig: type_sig.to_vec(),
        });
        MetadataToken::new(TableId::FieldRef.as_u8(), idx)
    }

    /// Add a cross-module method reference.
    pub fn add_method_ref(&mut self, parent: MetadataToken, name: &str, signature: &[u8]) -> MetadataToken {
        let idx = self.method_refs.len() as u32 + 1;
        self.method_refs.push(MethodRefBuilder {
            parent,
            name: name.to_string(),
            signature: signature.to_vec(),
        });
        MetadataToken::new(TableId::MethodRef.as_u8(), idx)
    }

    /// Add a cross-module type reference.
    pub fn add_type_ref(&mut self, scope: MetadataToken, name: &str, namespace: &str) -> MetadataToken {
        let idx = self.type_refs.len() as u32 + 1;
        self.type_refs.push(TypeRefBuilder {
            scope,
            name: name.to_string(),
            namespace: namespace.to_string(),
        });
        MetadataToken::new(TableId::TypeRef.as_u8(), idx)
    }

    /// Add a parameter definition.
    pub fn add_param_def(&mut self, name: &str, type_sig: &[u8], sequence: u16) -> MetadataToken {
        let idx = self.param_defs.len() as u32 + 1;
        self.param_defs.push(ParamDefBuilder {
            name: name.to_string(),
            type_sig: type_sig.to_vec(),
            sequence,
        });
        MetadataToken::new(TableId::ParamDef.as_u8(), idx)
    }

    /// Build the final Module, interning all strings and blobs into heaps.
    pub fn build(self) -> Module {
        let mut string_heap = heap::init_string_heap();
        let mut blob_heap = heap::init_blob_heap();

        // Intern module name and version
        let name_off = heap::intern_string(&mut string_heap, &self.name);
        let version_off = heap::intern_string(&mut string_heap, &self.version);

        // Helper closures
        let mut intern_str = |s: &str| -> u32 { heap::intern_string(&mut string_heap, s) };
        let mut intern_blob = |b: &[u8]| -> u32 { heap::write_blob(&mut blob_heap, b) };

        // ModuleDef
        let module_defs = vec![ModuleDefRow {
            name: name_off,
            version: version_off,
            flags: 0,
        }];

        // ModuleRef
        let module_refs: Vec<ModuleRefRow> = self.module_refs.iter().map(|b| {
            ModuleRefRow {
                name: intern_str(&b.name),
                min_version: intern_str(&b.min_version),
            }
        }).collect();

        // TypeDef
        let type_defs: Vec<TypeDefRow> = self.type_defs.iter().map(|b| {
            TypeDefRow {
                name: intern_str(&b.name),
                namespace: intern_str(&b.namespace),
                kind: b.kind,
                flags: b.flags,
                field_list: b.field_list,
                method_list: b.method_list,
            }
        }).collect();

        // TypeRef
        let type_refs: Vec<TypeRefRow> = self.type_refs.iter().map(|b| {
            TypeRefRow {
                scope: b.scope,
                name: intern_str(&b.name),
                namespace: intern_str(&b.namespace),
            }
        }).collect();

        // TypeSpec
        let type_specs: Vec<TypeSpecRow> = self.type_specs.iter().map(|b| {
            TypeSpecRow {
                signature: intern_blob(&b.signature),
            }
        }).collect();

        // FieldDef
        let field_defs: Vec<FieldDefRow> = self.field_defs.iter().map(|b| {
            FieldDefRow {
                name: intern_str(&b.name),
                type_sig: intern_blob(&b.type_sig),
                flags: b.flags,
            }
        }).collect();

        // FieldRef
        let field_refs: Vec<FieldRefRow> = self.field_refs.iter().map(|b| {
            FieldRefRow {
                parent: b.parent,
                name: intern_str(&b.name),
                type_sig: intern_blob(&b.type_sig),
            }
        }).collect();

        // MethodDef
        let method_defs: Vec<MethodDefRow> = self.method_defs.iter().map(|b| {
            MethodDefRow {
                name: intern_str(&b.name),
                signature: intern_blob(&b.signature),
                flags: b.flags,
                body_offset: 0, // writer will compute
                body_size: 1,   // non-zero to indicate body exists
                reg_count: b.reg_count,
                param_count: 0, // builder API does not yet track param_count
            }
        }).collect();

        // MethodRef
        let method_refs: Vec<MethodRefRow> = self.method_refs.iter().map(|b| {
            MethodRefRow {
                parent: b.parent,
                name: intern_str(&b.name),
                signature: intern_blob(&b.signature),
            }
        }).collect();

        // ParamDef
        let param_defs: Vec<ParamDefRow> = self.param_defs.iter().map(|b| {
            ParamDefRow {
                name: intern_str(&b.name),
                type_sig: intern_blob(&b.type_sig),
                sequence: b.sequence,
            }
        }).collect();

        // ContractDef
        let contract_defs: Vec<ContractDefRow> = self.contract_defs.iter().map(|b| {
            ContractDefRow {
                name: intern_str(&b.name),
                namespace: intern_str(&b.namespace),
                method_list: b.method_list,
                generic_param_list: b.generic_param_list,
            }
        }).collect();

        // ContractMethod
        let contract_methods: Vec<ContractMethodRow> = self.contract_methods.iter().map(|b| {
            ContractMethodRow {
                name: intern_str(&b.name),
                signature: intern_blob(&b.signature),
                slot: b.slot,
            }
        }).collect();

        // GenericParam
        let generic_params: Vec<GenericParamRow> = self.generic_params.iter().map(|b| {
            GenericParamRow {
                owner: b.owner,
                owner_kind: b.owner_kind,
                ordinal: b.ordinal,
                name: intern_str(&b.name),
            }
        }).collect();

        // GlobalDef
        let global_defs: Vec<GlobalDefRow> = self.global_defs.iter().map(|b| {
            GlobalDefRow {
                name: intern_str(&b.name),
                type_sig: intern_blob(&b.type_sig),
                flags: b.flags,
                init_value: intern_blob(&b.init_value),
            }
        }).collect();

        // ExternDef
        let extern_defs: Vec<ExternDefRow> = self.extern_defs.iter().map(|b| {
            ExternDefRow {
                name: intern_str(&b.name),
                signature: intern_blob(&b.signature),
                import_name: intern_str(&b.import_name),
                flags: b.flags,
            }
        }).collect();

        // LocaleDef
        let locale_defs: Vec<LocaleDefRow> = self.locale_defs.iter().map(|b| {
            LocaleDefRow {
                dlg_method: b.dlg_method,
                locale: intern_str(&b.locale),
                loc_method: b.loc_method,
            }
        }).collect();

        // ExportDef
        let export_defs: Vec<ExportDefRow> = self.export_defs.iter().map(|b| {
            ExportDefRow {
                name: intern_str(&b.name),
                item_kind: b.item_kind,
                item: b.item,
            }
        }).collect();

        // AttributeDef
        let attribute_defs: Vec<AttributeDefRow> = self.attribute_defs.iter().map(|b| {
            AttributeDefRow {
                owner: b.owner,
                owner_kind: b.owner_kind,
                name: intern_str(&b.name),
                value: intern_blob(&b.value),
            }
        }).collect();

        Module {
            header: ModuleHeader {
                format_version: 1,
                flags: 0,
                module_name: name_off,
                module_version: version_off,
                string_heap_offset: 0, // writer computes
                string_heap_size: 0,   // writer computes
                blob_heap_offset: 0,   // writer computes
                blob_heap_size: 0,     // writer computes
                table_directory: [(0, 0); 21], // writer computes
            },
            string_heap,
            blob_heap,
            module_defs,
            module_refs,
            type_defs,
            type_refs,
            type_specs,
            field_defs,
            field_refs,
            method_defs,
            method_refs,
            param_defs,
            contract_defs,
            contract_methods,
            impl_defs: self.impl_defs,
            generic_params,
            generic_constraints: self.generic_constraints,
            global_defs,
            extern_defs,
            component_slots: self.component_slots,
            locale_defs,
            export_defs,
            attribute_defs,
            method_bodies: self.method_bodies,
        }
    }
}
