//! Dispatch table construction for the Domain.
//!
//! This module contains `Domain::build_dispatch_table()` and the associated
//! private helpers, plus the `resolve_intrinsic_id` free function that maps
//! (type_name, method_name) pairs to IntrinsicId values.
//!
//! These functions are split from domain.rs because dispatch table construction
//! is a distinct concern from cross-module reference resolution.

use writ_module::heap::read_string;
use writ_module::token::MetadataToken;

use crate::dispatch::{DispatchKey, DispatchTable, DispatchTarget, IntrinsicId};

use crate::domain::Domain;

impl Domain {
    /// Build the dispatch table from ImplDef rows across all loaded modules.
    ///
    /// Iterates every ImplDef in every module, resolves type/contract tokens
    /// to global keys, and inserts entries. Methods with the intrinsic flag
    /// (0x80) are mapped to the corresponding `IntrinsicId`.
    pub fn build_dispatch_table(&self) -> DispatchTable {
        let mut table = DispatchTable::new();

        for (mod_idx, loaded) in self.modules.iter().enumerate() {
            let module = &loaded.module;

            for (impl_idx, impl_def) in module.impl_defs.iter().enumerate() {
                // Resolve type_token to a global type_key
                let type_key = self.resolve_type_key(mod_idx, impl_def.type_token);
                // Resolve the contract token to a global contract_key.
                // Uses ContractDef-based keys for standard virtual dispatch:
                // CALL_VIRT provides a contract identifier, and the runtime type
                // determines which implementation to use.
                //
                // Note: When a type implements the same contract with different
                // generic specializations (e.g., Int: Into<Float> vs Int: Into<String>),
                // only the last-registered implementation will be in the table.
                // Full generic dispatch requires a future phase.
                let contract_key = self.resolve_contract_key_for_impl(mod_idx, impl_def.contract);

                // Find the method range for this ImplDef.
                // Use the contract's method count to bound the range, rather than
                // extending to the next ImplDef's method_list (which may include
                // unrelated methods from other types).
                let method_start = impl_def.method_list.saturating_sub(1) as usize;
                let contract_method_count = Self::get_contract_method_count(module, impl_def.contract);
                let method_end_from_next = if impl_idx + 1 < module.impl_defs.len() {
                    module.impl_defs[impl_idx + 1].method_list.saturating_sub(1) as usize
                } else {
                    module.method_defs.len()
                };
                // Use the smaller of: contract method count, or next ImplDef boundary
                let method_end = if contract_method_count > 0 {
                    (method_start + contract_method_count).min(method_end_from_next)
                } else {
                    method_end_from_next
                };

                // For each method in this impl, slot = sequential offset from start
                for method_idx in method_start..method_end {
                    let method_def = &module.method_defs[method_idx];
                    let slot = (method_idx - method_start) as u16;

                    let target = if method_def.flags & 0x80 != 0 {
                        // Intrinsic method -- resolve to IntrinsicId
                        let type_name = self.get_type_name(mod_idx, impl_def.type_token);
                        let method_name = read_string(
                            &module.string_heap, method_def.name
                        ).unwrap_or("");
                        match resolve_intrinsic_id(&type_name, method_name) {
                            Some(intrinsic) => DispatchTarget::Intrinsic(intrinsic),
                            None => {
                                // Unknown intrinsic -- treat as IL method (shouldn't happen with
                                // correct virtual module, but avoids panic)
                                DispatchTarget::Method { module_idx: mod_idx, method_idx }
                            }
                        }
                    } else {
                        DispatchTarget::Method { module_idx: mod_idx, method_idx }
                    };

                    // FIX-02: Use impl_def.contract.0 as the type_args_hash discriminator.
                    // Each generic specialization (e.g. Into<Float>, Into<String>) has its own
                    // synthetic ContractDef token in the virtual module, so their raw token values
                    // differ. This produces distinct DispatchKeys for each specialization, eliminating
                    // the 4 collisions that occurred when all specializations shared the same base
                    // contract token. CALL_VIRT carries contract_idx (which must match this value)
                    // to perform the lookup.
                    let type_args_hash = impl_def.contract.0;
                    table.insert(DispatchKey { type_key, contract_key, slot, type_args_hash }, target);
                }
            }
        }

        table
    }

    /// Resolve a type MetadataToken to a global type_key.
    ///
    /// Encoded as `(module_idx << 16) | typedef_row_idx_0based`.
    fn resolve_type_key(&self, mod_idx: usize, token: MetadataToken) -> u32 {
        let table_id = token.table_id();
        let row = match token.row_index() {
            Some(r) => r - 1, // convert to 0-based
            None => return u32::MAX,
        };

        match table_id {
            2 => {
                // Local TypeDef
                ((mod_idx as u32) << 16) | row
            }
            3 => {
                // TypeRef -- resolve via cross-module resolution
                if let Some(resolved) = self.modules[mod_idx].resolved_refs.types.get(&row) {
                    ((resolved.module_idx as u32) << 16) | (resolved.typedef_idx as u32)
                } else {
                    u32::MAX
                }
            }
            _ => u32::MAX,
        }
    }

    /// Resolve a contract MetadataToken to a global contract_key for dispatch table building.
    ///
    /// ContractDef tokens use table ID 10. The key is `(module_idx << 16) | contractdef_row_idx`.
    fn resolve_contract_key_for_impl(&self, mod_idx: usize, token: MetadataToken) -> u32 {
        let table_id = token.table_id();
        let row = match token.row_index() {
            Some(r) => r - 1, // convert to 0-based
            None => return u32::MAX,
        };

        match table_id {
            10 => {
                // Local ContractDef
                ((mod_idx as u32) << 16) | row
            }
            3 => {
                // TypeRef pointing to a contract in another module.
                // Check the contracts map first (TypeRef resolved to ContractDef).
                if let Some(resolved) = self.modules[mod_idx].resolved_refs.contracts.get(&row) {
                    ((resolved.module_idx as u32) << 16) | (resolved.contractdef_idx as u32)
                } else {
                    u32::MAX
                }
            }
            _ => u32::MAX,
        }
    }

    /// Get the number of methods in a contract (from its ContractMethod slots).
    fn get_contract_method_count(module: &writ_module::Module, contract_token: MetadataToken) -> usize {
        let table_id = contract_token.table_id();
        if table_id != 10 {
            return 0; // Cross-module contract -- can't count methods locally
        }
        let row = match contract_token.row_index() {
            Some(r) => (r - 1) as usize,
            None => return 0,
        };
        if row >= module.contract_defs.len() {
            return 0;
        }
        let cd = &module.contract_defs[row];
        let method_start = cd.method_list.saturating_sub(1) as usize;
        let method_end = if row + 1 < module.contract_defs.len() {
            module.contract_defs[row + 1].method_list.saturating_sub(1) as usize
        } else {
            module.contract_methods.len()
        };
        method_end.saturating_sub(method_start)
    }

    /// Get the type name for a type MetadataToken (for intrinsic resolution).
    fn get_type_name(&self, mod_idx: usize, token: MetadataToken) -> String {
        let table_id = token.table_id();
        let row = match token.row_index() {
            Some(r) => (r - 1) as usize,
            None => return String::new(),
        };

        match table_id {
            2 => {
                // Local TypeDef
                let module = &self.modules[mod_idx].module;
                if row < module.type_defs.len() {
                    read_string(&module.string_heap, module.type_defs[row].name)
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                }
            }
            3 => {
                // TypeRef -- resolve to target module and get name from there
                if let Some(resolved) = self.modules[mod_idx].resolved_refs.types.get(&(row as u32)) {
                    let target_module = &self.modules[resolved.module_idx].module;
                    if resolved.typedef_idx < target_module.type_defs.len() {
                        read_string(&target_module.string_heap, target_module.type_defs[resolved.typedef_idx].name)
                            .unwrap_or("")
                            .to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }
}

/// Map a (type_name, method_name) pair to the corresponding IntrinsicId.
///
/// Returns None for unknown combinations (e.g., user-defined types that happen
/// to have the intrinsic flag set incorrectly).
pub fn resolve_intrinsic_id(type_name: &str, method_name: &str) -> Option<IntrinsicId> {
    match (type_name, method_name) {
        // Int (13)
        ("Int", "int_add") => Some(IntrinsicId::IntAdd),
        ("Int", "int_sub") => Some(IntrinsicId::IntSub),
        ("Int", "int_mul") => Some(IntrinsicId::IntMul),
        ("Int", "int_div") => Some(IntrinsicId::IntDiv),
        ("Int", "int_mod") => Some(IntrinsicId::IntMod),
        ("Int", "int_neg") => Some(IntrinsicId::IntNeg),
        ("Int", "int_not") => Some(IntrinsicId::IntNot),
        ("Int", "int_eq") => Some(IntrinsicId::IntEq),
        ("Int", "int_ord") => Some(IntrinsicId::IntOrd),
        ("Int", "int_bitand") => Some(IntrinsicId::IntBitAnd),
        ("Int", "int_bitor") => Some(IntrinsicId::IntBitOr),
        ("Int", "int_into_float") => Some(IntrinsicId::IntIntoFloat),
        ("Int", "int_into_string") => Some(IntrinsicId::IntIntoString),
        // Float (10)
        ("Float", "float_add") => Some(IntrinsicId::FloatAdd),
        ("Float", "float_sub") => Some(IntrinsicId::FloatSub),
        ("Float", "float_mul") => Some(IntrinsicId::FloatMul),
        ("Float", "float_div") => Some(IntrinsicId::FloatDiv),
        ("Float", "float_mod") => Some(IntrinsicId::FloatMod),
        ("Float", "float_neg") => Some(IntrinsicId::FloatNeg),
        ("Float", "float_eq") => Some(IntrinsicId::FloatEq),
        ("Float", "float_ord") => Some(IntrinsicId::FloatOrd),
        ("Float", "float_into_int") => Some(IntrinsicId::FloatIntoInt),
        ("Float", "float_into_string") => Some(IntrinsicId::FloatIntoString),
        // Bool (3)
        ("Bool", "bool_eq") => Some(IntrinsicId::BoolEq),
        ("Bool", "bool_not") => Some(IntrinsicId::BoolNot),
        ("Bool", "bool_into_string") => Some(IntrinsicId::BoolIntoString),
        // String (6)
        ("String", "string_add") => Some(IntrinsicId::StringAdd),
        ("String", "string_eq") => Some(IntrinsicId::StringEq),
        ("String", "string_ord") => Some(IntrinsicId::StringOrd),
        ("String", "string_index_int") => Some(IntrinsicId::StringIndexChar),
        ("String", "string_index_range") => Some(IntrinsicId::StringIndexRange),
        ("String", "string_into_string") => Some(IntrinsicId::StringIntoString),
        // Array (4)
        ("Array", "array_index") => Some(IntrinsicId::ArrayIndex),
        ("Array", "array_index_set") => Some(IntrinsicId::ArrayIndexSet),
        ("Array", "array_index_range") => Some(IntrinsicId::ArraySlice),
        ("Array", "array_iterable") => Some(IntrinsicId::ArrayIterable),
        // Reflection get_type (4)
        ("Int",    "int_get_type")    => Some(IntrinsicId::IntGetType),
        ("Float",  "float_get_type")  => Some(IntrinsicId::FloatGetType),
        ("Bool",   "bool_get_type")   => Some(IntrinsicId::BoolGetType),
        ("String", "string_get_type") => Some(IntrinsicId::StringGetType),
        // Reflection — Type methods (Phase 103)
        ("Type", "type_fields")          => Some(IntrinsicId::TypeFields),
        ("Type", "type_methods")         => Some(IntrinsicId::TypeMethods),
        ("Type", "type_attributes")      => Some(IntrinsicId::TypeAttributes),
        ("Type", "type_contracts")       => Some(IntrinsicId::TypeContracts),
        ("Type", "type_implements")      => Some(IntrinsicId::TypeImplements),
        ("Type", "type_get_name")        => Some(IntrinsicId::TypeGetName),
        ("Type", "type_get_namespace")   => Some(IntrinsicId::TypeGetNamespace),
        ("Type", "type_get_kind")        => Some(IntrinsicId::TypeGetKind),
        ("Type", "type_get_is_generic")  => Some(IntrinsicId::TypeGetIsGeneric),
        // Reflection — FieldInfo methods (Phase 103)
        ("FieldInfo", "fieldinfo_get")                => Some(IntrinsicId::FieldInfoGet),
        ("FieldInfo", "fieldinfo_get_name")           => Some(IntrinsicId::FieldInfoGetName),
        ("FieldInfo", "fieldinfo_get_declared_type")  => Some(IntrinsicId::FieldInfoGetDeclaredType),
        ("FieldInfo", "fieldinfo_get_is_mutable")     => Some(IntrinsicId::FieldInfoGetIsMutable),
        ("FieldInfo", "fieldinfo_set")                => Some(IntrinsicId::FieldInfoSet),
        // Reflection — MethodInfo methods (Phase 103, Phase 107)
        ("MethodInfo", "methodinfo_get_name")         => Some(IntrinsicId::MethodInfoGetName),
        ("MethodInfo", "methodinfo_get_return_type")  => Some(IntrinsicId::MethodInfoGetReturnType),
        ("MethodInfo", "methodinfo_get_parameters")   => Some(IntrinsicId::MethodInfoGetParameters),
        ("MethodInfo", "methodinfo_invoke")           => Some(IntrinsicId::MethodInfoInvoke),
        // Reflection — ParameterInfo methods (Phase 103)
        ("ParameterInfo", "paraminfo_get_name") => Some(IntrinsicId::ParameterInfoGetName),
        ("ParameterInfo", "paraminfo_get_type") => Some(IntrinsicId::ParameterInfoGetType),
        // Reflection — AttributeInfo methods (Phase 103)
        ("AttributeInfo", "attrinfo_get_name") => Some(IntrinsicId::AttributeInfoGetName),
        ("AttributeInfo", "attrinfo_get_args") => Some(IntrinsicId::AttributeInfoGetArgs),
        // Reflection — ContractInfo methods (Phase 103)
        ("ContractInfo", "contractinfo_get_name") => Some(IntrinsicId::ContractInfoGetName),
        ("ContractInfo", "contractinfo_get_type") => Some(IntrinsicId::ContractInfoGetType),
        // Reflection — Generic type queries (Phase 108)
        ("Type",       "type_type_args")        => Some(IntrinsicId::TypeTypeArgs),
        // Reflection — Per-member attributes (Phase 108)
        ("MethodInfo", "methodinfo_attributes") => Some(IntrinsicId::MethodInfoAttributes),
        ("FieldInfo",  "fieldinfo_attributes")  => Some(IntrinsicId::FieldInfoAttributes),
        // Hashable (4) — Phase 116
        ("Int",    "int_hash")    => Some(IntrinsicId::IntHash),
        ("Float",  "float_hash")  => Some(IntrinsicId::FloatHash),
        ("Bool",   "bool_hash")   => Some(IntrinsicId::BoolHash),
        ("String", "string_hash") => Some(IntrinsicId::StringHash),
        _ => None,
    }
}
