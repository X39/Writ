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
                let contract_key = self.resolve_contract_key_for_impl(mod_idx, impl_def.contract);
                if type_key == u32::MAX || contract_key == u32::MAX {
                    // Inherent impls have a null contract, while malformed or
                    // unresolved metadata has no canonical key. Neither belongs
                    // in the virtual-dispatch index.
                    continue;
                }

                let target_pattern = if impl_def.type_token.table_id() == 4 {
                    let Some(signature) = crate::type_specs::type_spec_signature(
                        mod_idx,
                        impl_def.type_token,
                        &self.modules,
                    ) else {
                        continue;
                    };
                    Some((mod_idx, signature))
                } else {
                    None
                };
                let contract_pattern = if impl_def.contract.table_id() == 4 {
                    let Some(signature) = crate::type_specs::type_spec_signature(
                        mod_idx,
                        impl_def.contract,
                        &self.modules,
                    ) else {
                        continue;
                    };
                    Some((mod_idx, signature))
                } else {
                    None
                };

                // Find the methods explicitly owned by this ImplDef. Ownership is
                // encoded on each MethodDef, so unrelated type or top-level methods
                // cannot leak into this implementation's dispatch slots.
                let mut method_indices = module.impl_method_indices(impl_idx);
                let contract_method_count =
                    self.get_contract_method_count(mod_idx, impl_def.contract);
                if contract_method_count > 0 {
                    method_indices.truncate(contract_method_count);
                }

                // For each method in this impl, slot = sequential ownership order.
                for (slot, method_idx) in method_indices.into_iter().enumerate() {
                    let method_def = &module.method_defs[method_idx];
                    let slot = slot as u16;

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

                    let type_args_hash = crate::type_specs::specialization_hash(
                        mod_idx,
                        impl_def.contract,
                        &self.modules,
                    );
                    table.insert(DispatchKey { type_key, contract_key, slot, type_args_hash }, target);
                    table.insert_pattern(
                        type_key,
                        contract_key,
                        slot,
                        target_pattern.clone(),
                        contract_pattern.clone(),
                        target,
                    );
                }
            }
        }

        table
    }

    /// Resolve a type MetadataToken to a global type_key.
    ///
    /// Encoded as `(module_idx << 16) | typedef_row_idx_0based`.
    fn resolve_type_key(&self, mod_idx: usize, token: MetadataToken) -> u32 {
        crate::type_specs::resolve_type_key(mod_idx, token, &self.modules)
    }

    /// Resolve a contract MetadataToken to a global contract_key for dispatch table building.
    ///
    /// ContractDef tokens use table ID 10. The key is `(module_idx << 16) | contractdef_row_idx`.
    fn resolve_contract_key_for_impl(&self, mod_idx: usize, token: MetadataToken) -> u32 {
        crate::type_specs::resolve_contract_key(mod_idx, token, &self.modules)
    }

    /// Get the number of methods in a contract (from its ContractMethod slots).
    fn get_contract_method_count(&self, mod_idx: usize, contract_token: MetadataToken) -> usize {
        crate::type_specs::contract_method_count(mod_idx, contract_token, &self.modules)
    }

    /// Get the type name for a type MetadataToken (for intrinsic resolution).
    fn get_type_name(&self, mod_idx: usize, token: MetadataToken) -> String {
        crate::type_specs::type_name(mod_idx, token, &self.modules)
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
        ("Type", "fields")               => Some(IntrinsicId::TypeFields),
        ("Type", "methods")              => Some(IntrinsicId::TypeMethods),
        ("Type", "attributes")           => Some(IntrinsicId::TypeAttributes),
        ("Type", "contracts")            => Some(IntrinsicId::TypeContracts),
        ("Type", "implements")           => Some(IntrinsicId::TypeImplements),
        ("Type", "type_get_name")        => Some(IntrinsicId::TypeGetName),
        ("Type", "type_get_namespace")   => Some(IntrinsicId::TypeGetNamespace),
        ("Type", "type_get_kind")        => Some(IntrinsicId::TypeGetKind),
        ("Type", "type_get_is_generic")  => Some(IntrinsicId::TypeGetIsGeneric),
        // Reflection — FieldInfo methods (Phase 103)
        ("FieldInfo", "get")                          => Some(IntrinsicId::FieldInfoGet),
        ("FieldInfo", "fieldinfo_get_name")           => Some(IntrinsicId::FieldInfoGetName),
        ("FieldInfo", "fieldinfo_get_declared_type")  => Some(IntrinsicId::FieldInfoGetDeclaredType),
        ("FieldInfo", "fieldinfo_get_is_mutable")     => Some(IntrinsicId::FieldInfoGetIsMutable),
        ("FieldInfo", "set")                          => Some(IntrinsicId::FieldInfoSet),
        // Reflection — MethodInfo methods (Phase 103, Phase 107)
        ("MethodInfo", "methodinfo_get_name")         => Some(IntrinsicId::MethodInfoGetName),
        ("MethodInfo", "methodinfo_get_return_type")  => Some(IntrinsicId::MethodInfoGetReturnType),
        ("MethodInfo", "methodinfo_get_parameters")   => Some(IntrinsicId::MethodInfoGetParameters),
        ("MethodInfo", "invoke")                      => Some(IntrinsicId::MethodInfoInvoke),
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
        ("Type",       "type_args")             => Some(IntrinsicId::TypeTypeArgs),
        // Reflection — Per-member attributes (Phase 108)
        ("MethodInfo", "attributes")            => Some(IntrinsicId::MethodInfoAttributes),
        ("FieldInfo",  "attributes")            => Some(IntrinsicId::FieldInfoAttributes),
        // Hashable (4) — Phase 116
        ("Int",    "int_hash")    => Some(IntrinsicId::IntHash),
        ("Float",  "float_hash")  => Some(IntrinsicId::FloatHash),
        ("Bool",   "bool_hash")   => Some(IntrinsicId::BoolHash),
        ("String", "string_hash") => Some(IntrinsicId::StringHash),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_reflection_names_resolve_to_intrinsics() {
        let cases = [
            ("Type", "fields", IntrinsicId::TypeFields),
            ("Type", "methods", IntrinsicId::TypeMethods),
            ("Type", "attributes", IntrinsicId::TypeAttributes),
            ("Type", "contracts", IntrinsicId::TypeContracts),
            ("Type", "implements", IntrinsicId::TypeImplements),
            ("Type", "type_args", IntrinsicId::TypeTypeArgs),
            ("FieldInfo", "get", IntrinsicId::FieldInfoGet),
            ("FieldInfo", "set", IntrinsicId::FieldInfoSet),
            (
                "FieldInfo",
                "attributes",
                IntrinsicId::FieldInfoAttributes,
            ),
            ("MethodInfo", "invoke", IntrinsicId::MethodInfoInvoke),
            (
                "MethodInfo",
                "attributes",
                IntrinsicId::MethodInfoAttributes,
            ),
        ];

        for (type_name, method_name, expected) in cases {
            assert_eq!(
                resolve_intrinsic_id(type_name, method_name),
                Some(expected),
                "{type_name}.{method_name}"
            );
        }
    }

    #[test]
    fn obsolete_public_reflection_internal_names_do_not_resolve() {
        for (type_name, method_name) in [
            ("Type", "type_fields"),
            ("Type", "type_methods"),
            ("Type", "type_attributes"),
            ("Type", "type_contracts"),
            ("Type", "type_implements"),
            ("Type", "type_type_args"),
            ("FieldInfo", "fieldinfo_get"),
            ("FieldInfo", "fieldinfo_set"),
            ("FieldInfo", "fieldinfo_attributes"),
            ("MethodInfo", "methodinfo_invoke"),
            ("MethodInfo", "methodinfo_attributes"),
        ] {
            assert_eq!(resolve_intrinsic_id(type_name, method_name), None);
        }
    }
}
