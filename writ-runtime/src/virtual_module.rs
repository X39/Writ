//! Programmatic construction of the `writ-runtime` virtual module.
//!
//! The virtual module provides all standard library types and contracts
//! defined in spec section 2.18, constructed in memory without reading
//! any file from disk. It contains:
//!
//! - 17 contracts (Add, Sub, Mul, Div, Mod, Neg, Not, Eq, Ord, Index,
//!   IndexSet, BitAnd, BitOr, Iterable, Iterator, Into, Error)
//! - Core types: Option<T>, Result<T,E>, Range<T>
//! - Primitive pseudo-TypeDefs: Int, Float, Bool, String
//! - Array<T> with methods and contract implementations
//! - Entity base type with static methods
//!
//! All primitive and array contract implementations are marked as intrinsic
//! (flag 0x80) since they map to native operations, not IL method bodies.

use writ_module::module::MethodBody;
use writ_module::token::MetadataToken;
use writ_module::{Module, ModuleBuilder};

/// The intrinsic method flag (bit 7).
const INTRINSIC_FLAG: u16 = 0x80;

/// An empty method body for intrinsic methods (no IL code).
fn empty_body() -> MethodBody {
    MethodBody {
        register_types: vec![],
        code: vec![],
        debug_locals: vec![],
        source_spans: vec![],
    }
}

/// Add an intrinsic implementation method to the builder.
///
/// Returns the method's metadata token. The method has an empty body
/// and the intrinsic flag set (0x80).
fn add_intrinsic_method(builder: &mut ModuleBuilder, name: &str) -> MetadataToken {
    builder.add_method(name, &[], INTRINSIC_FLAG, 0, empty_body())
}

/// Build the complete `writ-runtime` virtual module.
///
/// This module is constructed programmatically in memory and provides all
/// standard library metadata required for contract dispatch, type resolution,
/// and intrinsic method routing.
pub fn build_writ_runtime_module() -> Module {
    let mut builder = ModuleBuilder::new("writ-runtime");

    // ────────────────────────────────────────────────────────────────
    // Section 1: Define all 17 contracts (spec section 2.18.3)
    // ────────────────────────────────────────────────────────────────
    //
    // Each contract gets:
    //   1. add_contract_def(name, namespace) -- creates the ContractDef row
    //   2. add_contract_method(name, sig, slot) -- the single required method at slot 0
    //   3. add_generic_param(owner, owner_kind=0, ordinal, name) -- for generic contracts
    //
    // Note: Generic params for contracts use owner_kind=0 (treating contracts
    // as type-like entities). The ModuleBuilder accepts arbitrary tokens as owners.
    // The generic_param_list in ContractDefBuilder is set at the time add_contract_def
    // is called, so we must add generic params immediately after the contract def
    // (before the next contract def is added) to maintain correct list ownership.

    // Contract 1: Add<T, R>
    let add_contract = builder.add_contract_def("Add", "writ");
    builder.add_contract_method("op_add", &[], 0);
    builder.add_generic_param(add_contract, 0, 0, "T");
    builder.add_generic_param(add_contract, 0, 1, "R");

    // Contract 2: Sub<T, R>
    let sub_contract = builder.add_contract_def("Sub", "writ");
    builder.add_contract_method("op_sub", &[], 0);
    builder.add_generic_param(sub_contract, 0, 0, "T");
    builder.add_generic_param(sub_contract, 0, 1, "R");

    // Contract 3: Mul<T, R>
    let mul_contract = builder.add_contract_def("Mul", "writ");
    builder.add_contract_method("op_mul", &[], 0);
    builder.add_generic_param(mul_contract, 0, 0, "T");
    builder.add_generic_param(mul_contract, 0, 1, "R");

    // Contract 4: Div<T, R>
    let div_contract = builder.add_contract_def("Div", "writ");
    builder.add_contract_method("op_div", &[], 0);
    builder.add_generic_param(div_contract, 0, 0, "T");
    builder.add_generic_param(div_contract, 0, 1, "R");

    // Contract 5: Mod<T, R>
    let mod_contract = builder.add_contract_def("Mod", "writ");
    builder.add_contract_method("op_mod", &[], 0);
    builder.add_generic_param(mod_contract, 0, 0, "T");
    builder.add_generic_param(mod_contract, 0, 1, "R");

    // Contract 6: Neg<R>
    let neg_contract = builder.add_contract_def("Neg", "writ");
    builder.add_contract_method("op_neg", &[], 0);
    builder.add_generic_param(neg_contract, 0, 0, "R");

    // Contract 7: Not<R>
    let not_contract = builder.add_contract_def("Not", "writ");
    builder.add_contract_method("op_not", &[], 0);
    builder.add_generic_param(not_contract, 0, 0, "R");

    // Contract 8: Eq<T>
    let eq_contract = builder.add_contract_def("Eq", "writ");
    builder.add_contract_method("op_eq", &[], 0);
    builder.add_generic_param(eq_contract, 0, 0, "T");

    // Contract 9: Ord<T>
    let ord_contract = builder.add_contract_def("Ord", "writ");
    builder.add_contract_method("op_lt", &[], 0);
    builder.add_generic_param(ord_contract, 0, 0, "T");

    // Contract 10: Index<K, V>
    let index_contract = builder.add_contract_def("Index", "writ");
    builder.add_contract_method("op_index", &[], 0);
    builder.add_generic_param(index_contract, 0, 0, "K");
    builder.add_generic_param(index_contract, 0, 1, "V");

    // Contract 11: IndexSet<K, V>
    let indexset_contract = builder.add_contract_def("IndexSet", "writ");
    builder.add_contract_method("op_index_set", &[], 0);
    builder.add_generic_param(indexset_contract, 0, 0, "K");
    builder.add_generic_param(indexset_contract, 0, 1, "V");

    // Contract 12: BitAnd<T, R>
    let bitand_contract = builder.add_contract_def("BitAnd", "writ");
    builder.add_contract_method("op_bitand", &[], 0);
    builder.add_generic_param(bitand_contract, 0, 0, "T");
    builder.add_generic_param(bitand_contract, 0, 1, "R");

    // Contract 13: BitOr<T, R>
    let bitor_contract = builder.add_contract_def("BitOr", "writ");
    builder.add_contract_method("op_bitor", &[], 0);
    builder.add_generic_param(bitor_contract, 0, 0, "T");
    builder.add_generic_param(bitor_contract, 0, 1, "R");

    // Contract 14: Iterable<T>
    let iterable_contract = builder.add_contract_def("Iterable", "writ");
    builder.add_contract_method("iterator", &[], 0);
    builder.add_generic_param(iterable_contract, 0, 0, "T");

    // Contract 15: Iterator<T>
    let iterator_contract = builder.add_contract_def("Iterator", "writ");
    builder.add_contract_method("next", &[], 0);
    builder.add_generic_param(iterator_contract, 0, 0, "T");

    // Contract 16: Into<T> (base generic contract)
    let into_contract = builder.add_contract_def("Into", "writ");
    builder.add_contract_method("into", &[], 0);
    builder.add_generic_param(into_contract, 0, 0, "T");

    // Contract 17: Error (no generic params)
    let _error_contract = builder.add_contract_def("Error", "writ");
    builder.add_contract_method("message", &[], 0);

    // Specialization contracts 18-22 for generic dispatch (FIX-02).
    // Each represents a monomorphized specialization of a generic contract.
    // Distinct tokens allow build_dispatch_table to assign distinct type_args_hash
    // values (= impl_def.contract.0) per specialization, eliminating DispatchKey
    // collisions between e.g. Int:Into<Float> and Int:Into<String>.
    let into_float_spec = builder.add_contract_def("Into<Float>", "writ");
    builder.add_contract_method("into", &[], 0);

    let into_int_spec = builder.add_contract_def("Into<Int>", "writ");
    builder.add_contract_method("into", &[], 0);

    let into_string_spec = builder.add_contract_def("Into<String>", "writ");
    builder.add_contract_method("into", &[], 0);

    let index_int_spec = builder.add_contract_def("Index<Int>", "writ");
    builder.add_contract_method("op_index", &[], 0);

    let index_range_spec = builder.add_contract_def("Index<Range>", "writ");
    builder.add_contract_method("op_index", &[], 0);

    // ────────────────────────────────────────────────────────────────
    // Section 2: Core types (spec section 2.18.1 - 2.18.2)
    // ────────────────────────────────────────────────────────────────

    // Option<T> (kind=Enum=1)
    let option_type = builder.add_type_def("Option", "writ", 1, 0);
    builder.add_generic_param(option_type, 0, 0, "T");

    // Result<T, E> (kind=Enum=1)
    let result_type = builder.add_type_def("Result", "writ", 1, 0);
    builder.add_generic_param(result_type, 0, 0, "T");
    builder.add_generic_param(result_type, 0, 1, "E");

    // Range<T> (kind=Struct=0) with 4 fields
    let range_type = builder.add_type_def("Range", "writ", 0, 0);
    builder.add_field_def("start", &[0x12, 0x00, 0x00], 0);         // GenericParam ordinal 0 = T
    builder.add_field_def("end", &[0x12, 0x00, 0x00], 0);
    builder.add_field_def("start_inclusive", &[0x03], 0);             // bool
    builder.add_field_def("end_inclusive", &[0x03], 0);
    builder.add_generic_param(range_type, 0, 0, "T");

    // ────────────────────────────────────────────────────────────────
    // Section 3: Primitive pseudo-TypeDefs (spec section 2.18.4)
    // ────────────────────────────────────────────────────────────────
    // These are anchor types for ImplDef entries that map primitives
    // to their contract implementations.

    let int_type = builder.add_type_def("Int", "writ", 0, 0);
    let float_type = builder.add_type_def("Float", "writ", 0, 0);
    let bool_type = builder.add_type_def("Bool", "writ", 0, 0);
    let string_type = builder.add_type_def("String", "writ", 0, 0);

    // ────────────────────────────────────────────────────────────────
    // Section 4: Primitive contract implementations (spec section 2.18.5)
    // ────────────────────────────────────────────────────────────────
    // Each ImplDef links a type to a contract, and needs an intrinsic
    // method added immediately after (so the method_list ownership works).

    // --- Int implementations (13) ---
    builder.add_impl_def(int_type, add_contract);
    add_intrinsic_method(&mut builder, "int_add");

    builder.add_impl_def(int_type, sub_contract);
    add_intrinsic_method(&mut builder, "int_sub");

    builder.add_impl_def(int_type, mul_contract);
    add_intrinsic_method(&mut builder, "int_mul");

    builder.add_impl_def(int_type, div_contract);
    add_intrinsic_method(&mut builder, "int_div");

    builder.add_impl_def(int_type, mod_contract);
    add_intrinsic_method(&mut builder, "int_mod");

    builder.add_impl_def(int_type, neg_contract);
    add_intrinsic_method(&mut builder, "int_neg");

    builder.add_impl_def(int_type, not_contract);
    add_intrinsic_method(&mut builder, "int_not");

    builder.add_impl_def(int_type, eq_contract);
    add_intrinsic_method(&mut builder, "int_eq");

    builder.add_impl_def(int_type, ord_contract);
    add_intrinsic_method(&mut builder, "int_ord");

    builder.add_impl_def(int_type, bitand_contract);
    add_intrinsic_method(&mut builder, "int_bitand");

    builder.add_impl_def(int_type, bitor_contract);
    add_intrinsic_method(&mut builder, "int_bitor");

    builder.add_impl_def(int_type, into_float_spec);   // Int:Into<Float>
    add_intrinsic_method(&mut builder, "int_into_float");

    builder.add_impl_def(int_type, into_string_spec);  // Int:Into<String>
    add_intrinsic_method(&mut builder, "int_into_string");

    // --- Float implementations (10) ---
    builder.add_impl_def(float_type, add_contract);
    add_intrinsic_method(&mut builder, "float_add");

    builder.add_impl_def(float_type, sub_contract);
    add_intrinsic_method(&mut builder, "float_sub");

    builder.add_impl_def(float_type, mul_contract);
    add_intrinsic_method(&mut builder, "float_mul");

    builder.add_impl_def(float_type, div_contract);
    add_intrinsic_method(&mut builder, "float_div");

    builder.add_impl_def(float_type, mod_contract);
    add_intrinsic_method(&mut builder, "float_mod");

    builder.add_impl_def(float_type, neg_contract);
    add_intrinsic_method(&mut builder, "float_neg");

    builder.add_impl_def(float_type, eq_contract);
    add_intrinsic_method(&mut builder, "float_eq");

    builder.add_impl_def(float_type, ord_contract);
    add_intrinsic_method(&mut builder, "float_ord");

    builder.add_impl_def(float_type, into_int_spec);    // Float:Into<Int>
    add_intrinsic_method(&mut builder, "float_into_int");

    builder.add_impl_def(float_type, into_string_spec); // Float:Into<String>
    add_intrinsic_method(&mut builder, "float_into_string");

    // --- Bool implementations (3) ---
    builder.add_impl_def(bool_type, eq_contract);
    add_intrinsic_method(&mut builder, "bool_eq");

    builder.add_impl_def(bool_type, not_contract);
    add_intrinsic_method(&mut builder, "bool_not");

    builder.add_impl_def(bool_type, into_string_spec);  // Bool:Into<String>
    add_intrinsic_method(&mut builder, "bool_into_string");

    // --- String implementations (6) ---
    builder.add_impl_def(string_type, add_contract);
    add_intrinsic_method(&mut builder, "string_add");

    builder.add_impl_def(string_type, eq_contract);
    add_intrinsic_method(&mut builder, "string_eq");

    builder.add_impl_def(string_type, ord_contract);
    add_intrinsic_method(&mut builder, "string_ord");

    builder.add_impl_def(string_type, index_int_spec);   // String:Index<Int>
    add_intrinsic_method(&mut builder, "string_index_int");

    builder.add_impl_def(string_type, index_range_spec); // String:Index<Range>
    add_intrinsic_method(&mut builder, "string_index_range");

    builder.add_impl_def(string_type, into_string_spec); // String:Into<String>
    add_intrinsic_method(&mut builder, "string_into_string");

    // ────────────────────────────────────────────────────────────────
    // Section 5: Array<T> TypeDef and methods (spec section 2.18.6)
    // ────────────────────────────────────────────────────────────────

    let array_type = builder.add_type_def("Array", "writ", 0, 0);
    builder.add_field_def("length", &[0x01], 0x01);  // int type, read-only flag
    builder.add_generic_param(array_type, 0, 0, "T");

    // Array intrinsic instance methods
    add_intrinsic_method(&mut builder, "array_add");
    add_intrinsic_method(&mut builder, "array_remove_at");
    add_intrinsic_method(&mut builder, "array_insert");
    add_intrinsic_method(&mut builder, "array_contains");
    add_intrinsic_method(&mut builder, "array_slice");
    add_intrinsic_method(&mut builder, "array_iterator");

    // Array contract implementations (4 ImplDef entries)
    builder.add_impl_def(array_type, index_int_spec);       // Array:Index<Int>
    add_intrinsic_method(&mut builder, "array_index");

    builder.add_impl_def(array_type, indexset_contract);    // IndexSet<int, T>
    add_intrinsic_method(&mut builder, "array_index_set");

    builder.add_impl_def(array_type, index_range_spec);     // Array:Index<Range>
    add_intrinsic_method(&mut builder, "array_index_range");

    builder.add_impl_def(array_type, iterable_contract);    // Iterable<T>
    add_intrinsic_method(&mut builder, "array_iterable");

    // ────────────────────────────────────────────────────────────────
    // Section 6: Entity base TypeDef (spec section 2.18.7)
    // ────────────────────────────────────────────────────────────────

    let _entity_type = builder.add_type_def("Entity", "writ", 2, 0); // kind=Entity=2

    // Entity intrinsic static methods
    add_intrinsic_method(&mut builder, "entity_destroy");
    add_intrinsic_method(&mut builder, "entity_is_alive");
    add_intrinsic_method(&mut builder, "entity_get_or_create");
    add_intrinsic_method(&mut builder, "entity_find_all");

    // ────────────────────────────────────────────────────────────────
    // Section 7: Build and return
    // ────────────────────────────────────────────────────────────────

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::heap::read_string;

    /// Helper to read a string from the module's string heap.
    fn str_from_heap(module: &Module, offset: u32) -> &str {
        read_string(&module.string_heap, offset).expect("valid string")
    }

    #[test]
    fn module_name_is_writ_runtime() {
        let module = build_writ_runtime_module();
        assert_eq!(module.module_defs.len(), 1);
        let name = str_from_heap(&module, module.module_defs[0].name);
        assert_eq!(name, "writ-runtime");
    }

    #[test]
    fn has_exactly_22_contract_defs() {
        let module = build_writ_runtime_module();
        // 17 base contracts + 5 specialization contracts (Into<Float>, Into<Int>,
        // Into<String>, Index<Int>, Index<Range>) = 22
        assert_eq!(module.contract_defs.len(), 22);
    }

    #[test]
    fn contract_names_are_resolvable() {
        let module = build_writ_runtime_module();
        let names: Vec<&str> = module.contract_defs.iter()
            .map(|c| str_from_heap(&module, c.name))
            .collect();
        assert!(names.contains(&"Add"));
        assert!(names.contains(&"Sub"));
        assert!(names.contains(&"Mul"));
        assert!(names.contains(&"Div"));
        assert!(names.contains(&"Mod"));
        assert!(names.contains(&"Neg"));
        assert!(names.contains(&"Not"));
        assert!(names.contains(&"Eq"));
        assert!(names.contains(&"Ord"));
        assert!(names.contains(&"Index"));
        assert!(names.contains(&"IndexSet"));
        assert!(names.contains(&"BitAnd"));
        assert!(names.contains(&"BitOr"));
        assert!(names.contains(&"Iterable"));
        assert!(names.contains(&"Iterator"));
        assert!(names.contains(&"Into"));
        assert!(names.contains(&"Error"));
    }

    #[test]
    fn contract_namespaces_are_writ() {
        let module = build_writ_runtime_module();
        for contract in &module.contract_defs {
            let ns = str_from_heap(&module, contract.namespace);
            assert_eq!(ns, "writ", "contract namespace should be 'writ'");
        }
    }

    #[test]
    fn each_contract_has_one_method() {
        let module = build_writ_runtime_module();
        // 22 contracts (17 base + 5 specializations), each with exactly one method.
        assert_eq!(module.contract_methods.len(), 22);

        // Verify slot assignments are all 0
        for cm in &module.contract_methods {
            assert_eq!(cm.slot, 0, "all contract methods should have slot 0");
        }
    }

    #[test]
    fn contract_method_names_match_spec() {
        let module = build_writ_runtime_module();
        let method_names: Vec<&str> = module.contract_methods.iter()
            .map(|m| str_from_heap(&module, m.name))
            .collect();
        let expected = [
            "op_add", "op_sub", "op_mul", "op_div", "op_mod",
            "op_neg", "op_not", "op_eq", "op_lt",
            "op_index", "op_index_set", "op_bitand", "op_bitor",
            "iterator", "next", "into", "message",
        ];
        for name in &expected {
            assert!(method_names.contains(name), "missing contract method: {}", name);
        }
    }

    #[test]
    fn type_defs_include_all_nine_types() {
        let module = build_writ_runtime_module();
        let type_names: Vec<&str> = module.type_defs.iter()
            .map(|t| str_from_heap(&module, t.name))
            .collect();
        assert_eq!(module.type_defs.len(), 9);
        assert!(type_names.contains(&"Option"));
        assert!(type_names.contains(&"Result"));
        assert!(type_names.contains(&"Range"));
        assert!(type_names.contains(&"Int"));
        assert!(type_names.contains(&"Float"));
        assert!(type_names.contains(&"Bool"));
        assert!(type_names.contains(&"String"));
        assert!(type_names.contains(&"Array"));
        assert!(type_names.contains(&"Entity"));
    }

    #[test]
    fn option_is_enum_with_one_generic_param() {
        let module = build_writ_runtime_module();
        let option = module.type_defs.iter()
            .find(|t| str_from_heap(&module, t.name) == "Option")
            .expect("Option type exists");
        assert_eq!(option.kind, 1, "Option should be Enum (kind=1)");

        // Find generic params owned by the Option TypeDef token
        let option_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Option")
            .unwrap();
        let option_token = MetadataToken::new(2, option_idx as u32 + 1); // table 2 = TypeDef
        let params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == option_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Option should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn result_is_enum_with_two_generic_params() {
        let module = build_writ_runtime_module();
        let result_type = module.type_defs.iter()
            .find(|t| str_from_heap(&module, t.name) == "Result")
            .expect("Result type exists");
        assert_eq!(result_type.kind, 1, "Result should be Enum (kind=1)");

        let result_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Result")
            .unwrap();
        let result_token = MetadataToken::new(2, result_idx as u32 + 1);
        let params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == result_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 2, "Result should have 2 generic params");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
        assert_eq!(str_from_heap(&module, params[1].name), "E");
    }

    #[test]
    fn range_is_struct_with_four_fields_and_one_generic_param() {
        let module = build_writ_runtime_module();
        let range_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Range")
            .expect("Range type exists");
        let range = &module.type_defs[range_idx];
        assert_eq!(range.kind, 0, "Range should be Struct (kind=0)");

        // Fields owned by Range: from field_list to the next type's field_list
        let field_start = range.field_list as usize - 1; // 0-based
        let field_end = if range_idx + 1 < module.type_defs.len() {
            module.type_defs[range_idx + 1].field_list as usize - 1
        } else {
            module.field_defs.len()
        };
        let field_count = field_end - field_start;
        assert_eq!(field_count, 4, "Range should have 4 fields");

        let field_names: Vec<&str> = module.field_defs[field_start..field_end].iter()
            .map(|f| str_from_heap(&module, f.name))
            .collect();
        assert!(field_names.contains(&"start"));
        assert!(field_names.contains(&"end"));
        assert!(field_names.contains(&"start_inclusive"));
        assert!(field_names.contains(&"end_inclusive"));

        // Generic param
        let range_token = MetadataToken::new(2, range_idx as u32 + 1);
        let params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == range_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Range should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn primitive_types_are_structs() {
        let module = build_writ_runtime_module();
        for name in &["Int", "Float", "Bool", "String"] {
            let t = module.type_defs.iter()
                .find(|t| str_from_heap(&module, t.name) == *name)
                .unwrap_or_else(|| panic!("{} type should exist", name));
            assert_eq!(t.kind, 0, "{} should be Struct (kind=0)", name);
        }
    }

    #[test]
    fn impl_defs_count_is_at_least_36() {
        let module = build_writ_runtime_module();
        // 13 int + 10 float + 3 bool + 6 string + 4 array = 36
        assert!(
            module.impl_defs.len() >= 36,
            "expected at least 36 ImplDef rows, got {}",
            module.impl_defs.len()
        );
    }

    #[test]
    fn all_intrinsic_methods_have_intrinsic_flag() {
        let module = build_writ_runtime_module();
        for method in &module.method_defs {
            // All methods in the virtual module should be intrinsic
            assert!(
                method.flags & INTRINSIC_FLAG != 0,
                "method should have intrinsic flag set, flags=0x{:04x}",
                method.flags
            );
        }
    }

    #[test]
    fn array_type_has_one_field_and_one_generic_param() {
        let module = build_writ_runtime_module();
        let array_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Array")
            .expect("Array type exists");
        let array = &module.type_defs[array_idx];
        assert_eq!(array.kind, 0, "Array should be Struct (kind=0)");

        // Fields: from field_list to next type's field_list
        let field_start = array.field_list as usize - 1;
        let field_end = if array_idx + 1 < module.type_defs.len() {
            module.type_defs[array_idx + 1].field_list as usize - 1
        } else {
            module.field_defs.len()
        };
        let field_count = field_end - field_start;
        assert_eq!(field_count, 1, "Array should have 1 field (length)");
        assert_eq!(
            str_from_heap(&module, module.field_defs[field_start].name),
            "length"
        );

        // Generic param
        let array_token = MetadataToken::new(2, array_idx as u32 + 1);
        let params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == array_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Array should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn array_has_six_instance_methods() {
        let module = build_writ_runtime_module();
        let array_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Array")
            .expect("Array type exists");
        let array = &module.type_defs[array_idx];

        // Methods owned by Array: from method_list to the next type's method_list
        let method_start = array.method_list as usize - 1;
        let method_end = if array_idx + 1 < module.type_defs.len() {
            module.type_defs[array_idx + 1].method_list as usize - 1
        } else {
            module.method_defs.len()
        };
        let method_count = method_end - method_start;
        // 6 instance methods + 4 impl methods = 10 methods total on Array
        assert!(
            method_count >= 6,
            "Array should have at least 6 methods, got {}",
            method_count
        );
    }

    #[test]
    fn entity_type_is_entity_kind() {
        let module = build_writ_runtime_module();
        let entity = module.type_defs.iter()
            .find(|t| str_from_heap(&module, t.name) == "Entity")
            .expect("Entity type exists");
        assert_eq!(entity.kind, 2, "Entity should be Entity (kind=2)");
    }

    #[test]
    fn entity_has_four_static_methods() {
        let module = build_writ_runtime_module();
        let entity_idx = module.type_defs.iter()
            .position(|t| str_from_heap(&module, t.name) == "Entity")
            .expect("Entity type exists");
        let entity = &module.type_defs[entity_idx];

        let method_start = entity.method_list as usize - 1;
        let method_end = if entity_idx + 1 < module.type_defs.len() {
            module.type_defs[entity_idx + 1].method_list as usize - 1
        } else {
            module.method_defs.len()
        };
        let method_names: Vec<&str> = module.method_defs[method_start..method_end].iter()
            .map(|m| str_from_heap(&module, m.name))
            .collect();
        assert_eq!(method_names.len(), 4, "Entity should have 4 static methods");
        assert!(method_names.contains(&"entity_destroy"));
        assert!(method_names.contains(&"entity_is_alive"));
        assert!(method_names.contains(&"entity_get_or_create"));
        assert!(method_names.contains(&"entity_find_all"));
    }

    #[test]
    fn generic_params_for_contracts_are_correct() {
        let module = build_writ_runtime_module();

        // Add contract (index 0, token row 1) should have 2 generic params: T, R
        let add_token = MetadataToken::new(10, 1); // table 10 = ContractDef
        let add_params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == add_token)
            .collect();
        assert_eq!(add_params.len(), 2, "Add should have 2 generic params");

        // Error contract (index 16, token row 17) should have 0 generic params
        let error_token = MetadataToken::new(10, 17);
        let error_params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == error_token)
            .collect();
        assert_eq!(error_params.len(), 0, "Error should have 0 generic params");

        // Neg contract (index 5, token row 6) should have 1 generic param: R
        let neg_token = MetadataToken::new(10, 6);
        let neg_params: Vec<_> = module.generic_params.iter()
            .filter(|p| p.owner == neg_token)
            .collect();
        assert_eq!(neg_params.len(), 1, "Neg should have 1 generic param");
        assert_eq!(str_from_heap(&module, neg_params[0].name), "R");
    }
}
