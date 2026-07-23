//! Programmatic construction of the `writ-runtime` virtual module.
//!
//! The virtual module provides all standard library types and contracts
//! defined in spec section 2.18, constructed in memory without reading
//! any file from disk. It contains:
//!
//! - The normative operator/runtime contracts plus synthetic reflection
//!   dispatch contracts
//! - Core types: Option<T>, Result<T,E>, Range<T>
//! - Primitive pseudo-TypeDefs: Int, Float, Bool, String
//! - Array<T> with methods and contract implementations
//! - Entity base type with static methods
//!
//! All primitive and array contract implementations are marked as intrinsic
//! (flag 0x80) since they map to native operations, not IL method bodies.

use crate::module::MethodBody;
use crate::signature::{TypeSignature, encode_method_signature, encode_type_signature};
use crate::tables::{
    FIELD_FLAG_PUBLIC, FIELD_FLAG_READONLY, METHOD_FLAG_INTRINSIC, TableId, TypeDefKind,
};
use crate::token::MetadataToken;
use crate::{Module, ModuleBuilder};

const PUBLIC_FLAG: u16 = 1 << 0;
const STATIC_FLAG: u16 = 1 << 1;
const MUT_SELF_FLAG: u16 = 1 << 2;
const TYPE_GENERIC_OWNER_KIND: u8 = 0;
const METHOD_GENERIC_OWNER_KIND: u8 = 1;
const CONTRACT_GENERIC_OWNER_KIND: u8 = 2;

const TYPE_TYPEDEF_ROW: u32 = 10;
const PARAMETER_INFO_TYPEDEF_ROW: u32 = 11;
const ATTRIBUTE_INFO_TYPEDEF_ROW: u32 = 12;
const CONTRACT_INFO_TYPEDEF_ROW: u32 = 13;
const FIELD_INFO_TYPEDEF_ROW: u32 = 14;
const METHOD_INFO_TYPEDEF_ROW: u32 = 15;
const BOX_TYPEDEF_ROW: u32 = 16;
const ENTITY_LIST_TYPEDEF_ROW: u32 = 17;

fn type_def_token(row: u32) -> MetadataToken {
    MetadataToken::new(TableId::TypeDef.as_u8(), row)
}

fn add_typed_field(builder: &mut ModuleBuilder, name: &str, signature: TypeSignature) {
    let signature = encode_type_signature(&signature)
        .expect("writ-runtime field signature must fit the module format");
    builder.add_field_def(name, &signature, FIELD_FLAG_PUBLIC);
}

fn generic_type(name: &str, args: Vec<TypeSignature>) -> TypeSignature {
    TypeSignature::Generic {
        namespace: "writ".to_string(),
        name: name.to_string(),
        args,
    }
}

fn range_of(element: TypeSignature) -> TypeSignature {
    generic_type("Range", vec![element])
}

fn iterator_of(element: TypeSignature) -> TypeSignature {
    generic_type("Iterator", vec![element])
}

fn option_of(element: TypeSignature) -> TypeSignature {
    generic_type("Option", vec![element])
}

fn entity_list_of(element: TypeSignature) -> TypeSignature {
    generic_type("EntityList", vec![element])
}

fn array_of(element: TypeSignature) -> TypeSignature {
    TypeSignature::Array(Box::new(element))
}

fn named_type(row: u32) -> TypeSignature {
    TypeSignature::Named(type_def_token(row))
}

fn method_signature(params: &[TypeSignature], ret: &TypeSignature) -> Vec<u8> {
    encode_method_signature(params, ret)
        .expect("writ-runtime method signature must fit the module format")
}

fn intrinsic_register_count(params: &[TypeSignature], flags: u16) -> u16 {
    let regular_params = u16::try_from(params.len())
        .expect("writ-runtime intrinsic parameter count must fit the module format");
    regular_params
        .checked_add(u16::from(flags & STATIC_FLAG == 0))
        .expect("writ-runtime intrinsic register count must fit the module format")
}

fn add_typed_contract_method(
    builder: &mut ModuleBuilder,
    name: &str,
    params: &[TypeSignature],
    ret: TypeSignature,
) -> MetadataToken {
    let signature = method_signature(params, &ret);
    builder.add_contract_method(name, &signature, 0)
}

fn intrinsic_method_metadata(name: &str) -> (Vec<TypeSignature>, TypeSignature, u16) {
    let generic_param = || TypeSignature::GenericParam(0);
    let type_type = || named_type(TYPE_TYPEDEF_ROW);
    let box_type = || named_type(BOX_TYPEDEF_ROW);
    let attribute_info = || named_type(ATTRIBUTE_INFO_TYPEDEF_ROW);
    let contract_info = || named_type(CONTRACT_INFO_TYPEDEF_ROW);
    let field_info = || named_type(FIELD_INFO_TYPEDEF_ROW);
    let method_info = || named_type(METHOD_INFO_TYPEDEF_ROW);
    let parameter_info = || named_type(PARAMETER_INFO_TYPEDEF_ROW);

    let (params, ret, extra_flags) = match name {
        "int_add" | "int_sub" | "int_mul" | "int_div" | "int_mod" | "int_bitand" | "int_bitor" => {
            (vec![TypeSignature::Int], TypeSignature::Int, 0)
        }
        "int_neg" | "int_not" => (vec![], TypeSignature::Int, 0),
        "int_eq" | "int_ord" => (vec![TypeSignature::Int], TypeSignature::Bool, 0),
        "int_into_float" => (vec![], TypeSignature::Float, 0),
        "int_into_string" => (vec![], TypeSignature::String, 0),

        "float_add" | "float_sub" | "float_mul" | "float_div" | "float_mod" => {
            (vec![TypeSignature::Float], TypeSignature::Float, 0)
        }
        "float_neg" => (vec![], TypeSignature::Float, 0),
        "float_eq" | "float_ord" => (vec![TypeSignature::Float], TypeSignature::Bool, 0),
        "float_into_int" => (vec![], TypeSignature::Int, 0),
        "float_into_string" => (vec![], TypeSignature::String, 0),

        "bool_eq" => (vec![TypeSignature::Bool], TypeSignature::Bool, 0),
        "bool_not" => (vec![], TypeSignature::Bool, 0),
        "bool_into_string" => (vec![], TypeSignature::String, 0),

        "string_add" => (vec![TypeSignature::String], TypeSignature::String, 0),
        "string_eq" | "string_ord" => (vec![TypeSignature::String], TypeSignature::Bool, 0),
        "string_index_int" => (vec![TypeSignature::Int], TypeSignature::String, 0),
        "string_index_range" => (vec![range_of(TypeSignature::Int)], TypeSignature::String, 0),
        "string_into_string" => (vec![], TypeSignature::String, 0),

        "int_get_type" | "float_get_type" | "bool_get_type" | "string_get_type" => {
            (vec![], type_type(), 0)
        }
        "int_hash" | "float_hash" | "bool_hash" | "string_hash" => (vec![], TypeSignature::Int, 0),

        "len" => (vec![], TypeSignature::Int, 0),
        "slice" => (
            vec![TypeSignature::Int, TypeSignature::Int],
            array_of(generic_param()),
            0,
        ),
        "resize" => (vec![TypeSignature::Int], TypeSignature::Void, MUT_SELF_FLAG),
        "copy_from" => (
            vec![
                array_of(generic_param()),
                TypeSignature::Int,
                TypeSignature::Int,
                TypeSignature::Int,
            ],
            TypeSignature::Void,
            MUT_SELF_FLAG,
        ),
        "array_iterable" => (vec![], iterator_of(generic_param()), 0),
        "array_index" => (vec![TypeSignature::Int], generic_param(), 0),
        "array_index_set" => (
            vec![TypeSignature::Int, generic_param()],
            TypeSignature::Void,
            MUT_SELF_FLAG,
        ),
        "array_index_range" => (
            vec![range_of(TypeSignature::Int)],
            array_of(generic_param()),
            0,
        ),

        "destroy" => (
            vec![TypeSignature::Entity],
            TypeSignature::Void,
            STATIC_FLAG,
        ),
        "isAlive" => (
            vec![TypeSignature::Entity],
            TypeSignature::Bool,
            STATIC_FLAG,
        ),
        "getOrCreate" => (vec![], generic_param(), STATIC_FLAG),
        "findAll" => (vec![], entity_list_of(generic_param()), STATIC_FLAG),

        "fields" => (vec![], array_of(field_info()), PUBLIC_FLAG),
        "methods" => (vec![], array_of(method_info()), PUBLIC_FLAG),
        "attributes" => (vec![], array_of(attribute_info()), PUBLIC_FLAG),
        "contracts" => (vec![], array_of(contract_info()), PUBLIC_FLAG),
        "implements" => (vec![type_type()], TypeSignature::Bool, PUBLIC_FLAG),
        "type_get_name" | "type_get_namespace" | "type_get_kind" => {
            (vec![], TypeSignature::String, 0)
        }
        "type_get_is_generic" => (vec![], TypeSignature::Bool, 0),
        "type_args" => (vec![], array_of(type_type()), PUBLIC_FLAG),

        "get" => (vec![box_type()], box_type(), PUBLIC_FLAG),
        "fieldinfo_get_name" => (vec![], TypeSignature::String, 0),
        "fieldinfo_get_declared_type" => (vec![], type_type(), 0),
        "fieldinfo_get_is_mutable" => (vec![], TypeSignature::Bool, 0),
        "set" => (
            vec![box_type(), box_type()],
            TypeSignature::Void,
            PUBLIC_FLAG,
        ),

        "methodinfo_get_name" => (vec![], TypeSignature::String, 0),
        "methodinfo_get_return_type" => (vec![], type_type(), 0),
        "methodinfo_get_parameters" => (vec![], array_of(parameter_info()), 0),
        "invoke" => (
            vec![box_type(), array_of(box_type())],
            box_type(),
            PUBLIC_FLAG,
        ),

        "paraminfo_get_name" | "attrinfo_get_name" | "contractinfo_get_name" => {
            (vec![], TypeSignature::String, 0)
        }
        "paraminfo_get_type" | "contractinfo_get_type" => (vec![], type_type(), 0),
        "attrinfo_get_args" => (vec![], array_of(box_type()), 0),
        other => panic!("missing writ-runtime intrinsic signature for {other}"),
    };

    (params, ret, extra_flags)
}

/// An empty method body for intrinsic methods (no IL code).
fn empty_body() -> MethodBody {
    MethodBody {
        register_types: vec![],
        code: vec![],
        debug_locals: vec![],
        source_spans: vec![],
    }
}

/// Add an intrinsic implementation and its explicitly owned method.
fn add_intrinsic_impl(
    builder: &mut ModuleBuilder,
    type_token: MetadataToken,
    contract: MetadataToken,
    name: &str,
) -> MetadataToken {
    let impl_token = builder.add_impl_def(type_token, contract);
    let (params, ret, flags) = intrinsic_method_metadata(name);
    let signature = method_signature(&params, &ret);
    let reg_count = intrinsic_register_count(&params, flags);
    builder.add_impl_method(
        impl_token,
        name,
        &signature,
        METHOD_FLAG_INTRINSIC | flags,
        reg_count,
        empty_body(),
    )
}

fn add_intrinsic_generic_impl(
    builder: &mut ModuleBuilder,
    type_token: MetadataToken,
    contract_name: &str,
    contract_args: Vec<TypeSignature>,
    name: &str,
) -> MetadataToken {
    let contract = generic_type(contract_name, contract_args);
    let contract = encode_type_signature(&contract)
        .expect("writ-runtime contract specialization must fit the module format");
    let contract = builder.add_type_spec(&contract);
    add_intrinsic_impl(builder, type_token, contract, name)
}

fn add_intrinsic_contract_method(builder: &mut ModuleBuilder, name: &str) -> MetadataToken {
    let (params, ret, _) = intrinsic_method_metadata(name);
    add_typed_contract_method(builder, name, &params, ret)
}

/// Add an intrinsic method owned directly by a type.
fn add_intrinsic_type_method(
    builder: &mut ModuleBuilder,
    owner: MetadataToken,
    name: &str,
) -> MetadataToken {
    let (params, ret, flags) = intrinsic_method_metadata(name);
    let signature = method_signature(&params, &ret);
    let reg_count = intrinsic_register_count(&params, flags);
    builder.add_type_method(
        owner,
        name,
        &signature,
        PUBLIC_FLAG | METHOD_FLAG_INTRINSIC | flags,
        reg_count,
        empty_body(),
    )
}

/// Build the complete `writ-runtime` virtual module.
///
/// This module is constructed programmatically in memory and provides all
/// standard library metadata required for contract dispatch, type resolution,
/// and intrinsic method routing.
pub fn build_writ_runtime_module() -> Module {
    let mut builder = ModuleBuilder::new("writ-runtime");

    // ────────────────────────────────────────────────────────────────
    // Section 1: Define the normative contracts (spec section 2.18.3 and extensions)
    // ────────────────────────────────────────────────────────────────
    //
    // Each contract gets:
    //   1. add_contract_def(name, namespace) -- creates the ContractDef row
    //   2. add_contract_method(name, sig, slot) -- the single required method at slot 0
    //   3. add_generic_param(owner, owner_kind=2, ordinal, name) -- for generic contracts
    //
    // Contract generic parameters use the format-defined ContractDef owner kind.
    // The generic_param_list in ContractDefBuilder is set at the time add_contract_def
    // is called, so we must add generic params immediately after the contract def
    // (before the next contract def is added) to maintain correct list ownership.

    // Contract 1: Add<T, R>
    let add_contract = builder.add_contract_def("Add", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_add",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(add_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(add_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 2: Sub<T, R>
    let sub_contract = builder.add_contract_def("Sub", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_sub",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(sub_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(sub_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 3: Mul<T, R>
    let mul_contract = builder.add_contract_def("Mul", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_mul",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(mul_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(mul_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 4: Div<T, R>
    let div_contract = builder.add_contract_def("Div", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_div",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(div_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(div_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 5: Mod<T, R>
    let mod_contract = builder.add_contract_def("Mod", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_mod",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(mod_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(mod_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 6: Neg<R>
    let neg_contract = builder.add_contract_def("Neg", "writ");
    add_typed_contract_method(&mut builder, "op_neg", &[], TypeSignature::GenericParam(0));
    builder.add_generic_param(neg_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "R");

    // Contract 7: Not<R>
    let not_contract = builder.add_contract_def("Not", "writ");
    add_typed_contract_method(&mut builder, "op_not", &[], TypeSignature::GenericParam(0));
    builder.add_generic_param(not_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "R");

    // Contract 8: Eq<T>
    let eq_contract = builder.add_contract_def("Eq", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_eq",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::Bool,
    );
    builder.add_generic_param(eq_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");

    // Contract 9: Ord<T>
    let ord_contract = builder.add_contract_def("Ord", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_lt",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::Bool,
    );
    builder.add_generic_param(ord_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");

    // Contract 10: Index<K, V>
    let index_contract = builder.add_contract_def("Index", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_index",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(index_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "K");
    builder.add_generic_param(index_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "V");

    // Contract 11: IndexSet<K, V>
    let indexset_contract = builder.add_contract_def("IndexSet", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_index_set",
        &[
            TypeSignature::GenericParam(0),
            TypeSignature::GenericParam(1),
        ],
        TypeSignature::Void,
    );
    builder.add_generic_param(indexset_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "K");
    builder.add_generic_param(indexset_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "V");

    // Contract 12: BitAnd<T, R>
    let bitand_contract = builder.add_contract_def("BitAnd", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_bitand",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(bitand_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(bitand_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 13: BitOr<T, R>
    let bitor_contract = builder.add_contract_def("BitOr", "writ");
    add_typed_contract_method(
        &mut builder,
        "op_bitor",
        &[TypeSignature::GenericParam(0)],
        TypeSignature::GenericParam(1),
    );
    builder.add_generic_param(bitor_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(bitor_contract, CONTRACT_GENERIC_OWNER_KIND, 1, "R");

    // Contract 14: Iterable<T>
    let iterable_contract = builder.add_contract_def("Iterable", "writ");
    add_typed_contract_method(
        &mut builder,
        "iterator",
        &[],
        iterator_of(TypeSignature::GenericParam(0)),
    );
    builder.add_generic_param(iterable_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");

    // Contract 15: Iterator<T>
    let iterator_contract = builder.add_contract_def("Iterator", "writ");
    add_typed_contract_method(
        &mut builder,
        "next",
        &[],
        option_of(TypeSignature::GenericParam(0)),
    );
    builder.add_generic_param(iterator_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");

    // Contract 16: Into<T> (base generic contract)
    let into_contract = builder.add_contract_def("Into", "writ");
    add_typed_contract_method(&mut builder, "into", &[], TypeSignature::GenericParam(0));
    builder.add_generic_param(into_contract, CONTRACT_GENERIC_OWNER_KIND, 0, "T");

    // Contract 17: Error (no generic params)
    let _error_contract = builder.add_contract_def("Error", "writ");
    add_typed_contract_method(&mut builder, "message", &[], TypeSignature::String);

    // Contract 18: Speaker (no generic params) — optional display name override for dialogue
    let _speaker_contract = builder.add_contract_def("Speaker", "writ");
    add_typed_contract_method(&mut builder, "speaker_name", &[], TypeSignature::String);

    // Contract 19: Reflectable (no generic params) — runtime type query
    let reflectable_contract = builder.add_contract_def("Reflectable", "writ");
    add_typed_contract_method(&mut builder, "get_type", &[], named_type(TYPE_TYPEDEF_ROW));

    // Contract 20: Hashable (no generic params) — deterministic hash for primitive types (Phase 116)
    let hashable_contract = builder.add_contract_def("Hashable", "writ");
    add_typed_contract_method(&mut builder, "hash", &[], TypeSignature::Int);

    // ────────────────────────────────────────────────────────────────
    // Section 2: Core types (spec section 2.18.1 - 2.18.2)
    // ────────────────────────────────────────────────────────────────

    // Option<T> (kind=Enum=1)
    let option_type = builder.add_type_def("Option", "writ", TypeDefKind::Enum, 0);
    builder.add_generic_param(option_type, TYPE_GENERIC_OWNER_KIND, 0, "T");

    // Result<T, E> (kind=Enum=1)
    let result_type = builder.add_type_def("Result", "writ", TypeDefKind::Enum, 0);
    builder.add_generic_param(result_type, TYPE_GENERIC_OWNER_KIND, 0, "T");
    builder.add_generic_param(result_type, TYPE_GENERIC_OWNER_KIND, 1, "E");

    // Range<T> (kind=Struct=0) with 4 fields
    let range_type = builder.add_type_def("Range", "writ", TypeDefKind::Struct, 0);
    builder.add_field_def("start", &[0x12, 0x00, 0x00], FIELD_FLAG_READONLY); // GenericParam ordinal 0 = T
    builder.add_field_def("end", &[0x12, 0x00, 0x00], FIELD_FLAG_READONLY);
    builder.add_field_def("start_inclusive", &[0x03], FIELD_FLAG_READONLY); // bool
    builder.add_field_def("end_inclusive", &[0x03], FIELD_FLAG_READONLY);
    builder.add_generic_param(range_type, TYPE_GENERIC_OWNER_KIND, 0, "T");

    // ────────────────────────────────────────────────────────────────
    // Section 3: Primitive pseudo-TypeDefs (spec section 2.18.4)
    // ────────────────────────────────────────────────────────────────
    // These are anchor types for ImplDef entries that map primitives
    // to their contract implementations.

    let int_type = builder.add_type_def("Int", "writ", TypeDefKind::Struct, 0);
    let float_type = builder.add_type_def("Float", "writ", TypeDefKind::Struct, 0);
    let bool_type = builder.add_type_def("Bool", "writ", TypeDefKind::Struct, 0);
    let string_type = builder.add_type_def("String", "writ", TypeDefKind::Struct, 0);

    // ────────────────────────────────────────────────────────────────
    // Section 4: Primitive contract implementations (spec section 2.18.5)
    // ────────────────────────────────────────────────────────────────
    // Each intrinsic method records its ImplDef owner explicitly.

    // --- Int implementations (13) ---
    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Add",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_add",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Sub",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_sub",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Mul",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_mul",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Div",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_div",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Mod",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_mod",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Neg",
        vec![TypeSignature::Int],
        "int_neg",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Not",
        vec![TypeSignature::Int],
        "int_not",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Eq",
        vec![TypeSignature::Int],
        "int_eq",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Ord",
        vec![TypeSignature::Int],
        "int_ord",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "BitAnd",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_bitand",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "BitOr",
        vec![TypeSignature::Int, TypeSignature::Int],
        "int_bitor",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Into",
        vec![TypeSignature::Float],
        "int_into_float",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        int_type,
        "Into",
        vec![TypeSignature::String],
        "int_into_string",
    );

    // --- Float implementations (10) ---
    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Add",
        vec![TypeSignature::Float, TypeSignature::Float],
        "float_add",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Sub",
        vec![TypeSignature::Float, TypeSignature::Float],
        "float_sub",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Mul",
        vec![TypeSignature::Float, TypeSignature::Float],
        "float_mul",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Div",
        vec![TypeSignature::Float, TypeSignature::Float],
        "float_div",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Mod",
        vec![TypeSignature::Float, TypeSignature::Float],
        "float_mod",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Neg",
        vec![TypeSignature::Float],
        "float_neg",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Eq",
        vec![TypeSignature::Float],
        "float_eq",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Ord",
        vec![TypeSignature::Float],
        "float_ord",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Into",
        vec![TypeSignature::Int],
        "float_into_int",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        float_type,
        "Into",
        vec![TypeSignature::String],
        "float_into_string",
    );

    // --- Bool implementations (3) ---
    add_intrinsic_generic_impl(
        &mut builder,
        bool_type,
        "Eq",
        vec![TypeSignature::Bool],
        "bool_eq",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        bool_type,
        "Not",
        vec![TypeSignature::Bool],
        "bool_not",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        bool_type,
        "Into",
        vec![TypeSignature::String],
        "bool_into_string",
    );

    // --- String implementations (6) ---
    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Add",
        vec![TypeSignature::String, TypeSignature::String],
        "string_add",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Eq",
        vec![TypeSignature::String],
        "string_eq",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Ord",
        vec![TypeSignature::String],
        "string_ord",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Index",
        vec![TypeSignature::Int, TypeSignature::String],
        "string_index_int",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Index",
        vec![range_of(TypeSignature::Int), TypeSignature::String],
        "string_index_range",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        string_type,
        "Into",
        vec![TypeSignature::String],
        "string_into_string",
    );

    // --- Primitive Reflectable implementations (4) ---
    add_intrinsic_impl(&mut builder, int_type, reflectable_contract, "int_get_type");

    add_intrinsic_impl(
        &mut builder,
        float_type,
        reflectable_contract,
        "float_get_type",
    );

    add_intrinsic_impl(
        &mut builder,
        bool_type,
        reflectable_contract,
        "bool_get_type",
    );

    add_intrinsic_impl(
        &mut builder,
        string_type,
        reflectable_contract,
        "string_get_type",
    );

    // --- Primitive Hashable implementations (4) — Phase 116 ---
    add_intrinsic_impl(&mut builder, int_type, hashable_contract, "int_hash");

    add_intrinsic_impl(&mut builder, float_type, hashable_contract, "float_hash");

    add_intrinsic_impl(&mut builder, bool_type, hashable_contract, "bool_hash");

    add_intrinsic_impl(&mut builder, string_type, hashable_contract, "string_hash");

    // ────────────────────────────────────────────────────────────────
    // Section 5: Array<T> TypeDef and methods (spec section 2.18.6)
    // ────────────────────────────────────────────────────────────────

    let array_type = builder.add_type_def("Array", "writ", TypeDefKind::Struct, 0);
    builder.add_field_def("length", &[0x01], FIELD_FLAG_PUBLIC | FIELD_FLAG_READONLY); // public int, read-only after construction
    builder.add_generic_param(array_type, TYPE_GENERIC_OWNER_KIND, 0, "T");

    // Array intrinsic instance methods
    add_intrinsic_type_method(&mut builder, array_type, "len");
    add_intrinsic_type_method(&mut builder, array_type, "slice");
    add_intrinsic_type_method(&mut builder, array_type, "resize");
    add_intrinsic_type_method(&mut builder, array_type, "copy_from");

    // Array contract implementations (4 ImplDef entries)
    add_intrinsic_generic_impl(
        &mut builder,
        array_type,
        "Index",
        vec![TypeSignature::Int, TypeSignature::GenericParam(0)],
        "array_index",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        array_type,
        "IndexSet",
        vec![TypeSignature::Int, TypeSignature::GenericParam(0)],
        "array_index_set",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        array_type,
        "Index",
        vec![
            range_of(TypeSignature::Int),
            array_of(TypeSignature::GenericParam(0)),
        ],
        "array_index_range",
    );

    add_intrinsic_generic_impl(
        &mut builder,
        array_type,
        "Iterable",
        vec![TypeSignature::GenericParam(0)],
        "array_iterable",
    );

    // ────────────────────────────────────────────────────────────────
    // Section 6: Entity base TypeDef (spec section 2.18.7)
    // ────────────────────────────────────────────────────────────────

    let entity_type = builder.add_type_def("Entity", "writ", TypeDefKind::Entity, 0);

    // Entity intrinsic static methods
    add_intrinsic_type_method(&mut builder, entity_type, "destroy");
    add_intrinsic_type_method(&mut builder, entity_type, "isAlive");
    let get_or_create = add_intrinsic_type_method(&mut builder, entity_type, "getOrCreate");
    builder.add_generic_param(get_or_create, METHOD_GENERIC_OWNER_KIND, 0, "T");
    let find_all = add_intrinsic_type_method(&mut builder, entity_type, "findAll");
    builder.add_generic_param(find_all, METHOD_GENERIC_OWNER_KIND, 0, "T");

    // ────────────────────────────────────────────────────────────────
    // Section 7: Builtin Attribute Declarations (UATTR-03)
    // ────────────────────────────────────────────────────────────────
    //
    // Declares the parameter signature for each builtin attribute so the host
    // can enumerate available attribute types via the query API. Each row uses
    // owner_kind = ATTR_OWNER_KIND_DECL (3) and owner = MetadataToken::NULL.
    //
    // Signature blob format: u16 param count (LE) + one tag byte per param.
    // ATTR_TAG_STRING (0x01), ATTR_TAG_INT (0x02), ATTR_TAG_BOOL (0x03).

    use crate::attr::ATTR_TAG_STRING;
    use crate::tables::ATTR_OWNER_KIND_DECL;

    // Deprecated(msg: string) — 1 string param
    {
        let mut sig = Vec::new();
        sig.extend_from_slice(&1u16.to_le_bytes());
        sig.push(ATTR_TAG_STRING);
        builder.add_attribute_def(
            MetadataToken::NULL,
            ATTR_OWNER_KIND_DECL,
            "Deprecated",
            &sig,
        );
    }

    // Conditional(name: string) — 1 string param
    {
        let mut sig = Vec::new();
        sig.extend_from_slice(&1u16.to_le_bytes());
        sig.push(ATTR_TAG_STRING);
        builder.add_attribute_def(
            MetadataToken::NULL,
            ATTR_OWNER_KIND_DECL,
            "Conditional",
            &sig,
        );
    }

    // Singleton — 0 params
    {
        let sig = 0u16.to_le_bytes();
        builder.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, "Singleton", &sig);
    }

    // Locale(tag: string) — 1 string param
    {
        let mut sig = Vec::new();
        sig.extend_from_slice(&1u16.to_le_bytes());
        sig.push(ATTR_TAG_STRING);
        builder.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, "Locale", &sig);
    }

    // ────────────────────────────────────────────────────────────────
    // Section 8: Reflection contracts (Phase 103)
    // ────────────────────────────────────────────────────────────────
    //
    // Each reflection method gets its own synthetic single-method contract,
    // so the dispatch table can resolve CALL_VIRT on Type/FieldInfo/etc.
    // methods to the correct IntrinsicId via the standard ImplDef pathway.
    //
    // Synthetic reflection contracts follow the 20 normative/base contracts.

    // Type method contracts (5)
    let type_fields_contract = builder.add_contract_def("Type.fields", "writ");
    add_intrinsic_contract_method(&mut builder, "fields");

    let type_methods_contract = builder.add_contract_def("Type.methods", "writ");
    add_intrinsic_contract_method(&mut builder, "methods");

    let type_attrs_contract = builder.add_contract_def("Type.attributes", "writ");
    add_intrinsic_contract_method(&mut builder, "attributes");

    let type_contracts_contract = builder.add_contract_def("Type.contracts", "writ");
    add_intrinsic_contract_method(&mut builder, "contracts");

    let type_impl_contract = builder.add_contract_def("Type.implements", "writ");
    add_intrinsic_contract_method(&mut builder, "implements");

    // Type field accessor contracts (4)
    let type_get_name_contract = builder.add_contract_def("Type.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "type_get_name");

    let type_get_ns_contract = builder.add_contract_def("Type.get_namespace", "writ");
    add_intrinsic_contract_method(&mut builder, "type_get_namespace");

    let type_get_kind_contract = builder.add_contract_def("Type.get_kind", "writ");
    add_intrinsic_contract_method(&mut builder, "type_get_kind");

    let type_get_is_generic_contract = builder.add_contract_def("Type.get_is_generic", "writ");
    add_intrinsic_contract_method(&mut builder, "type_get_is_generic");

    // FieldInfo method contracts (4)
    let fieldinfo_get_contract = builder.add_contract_def("FieldInfo.get", "writ");
    add_intrinsic_contract_method(&mut builder, "get");

    let fieldinfo_get_name_contract = builder.add_contract_def("FieldInfo.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "fieldinfo_get_name");

    let fieldinfo_get_type_contract =
        builder.add_contract_def("FieldInfo.get_declared_type", "writ");
    add_intrinsic_contract_method(&mut builder, "fieldinfo_get_declared_type");

    let fieldinfo_get_mut_contract = builder.add_contract_def("FieldInfo.get_is_mutable", "writ");
    add_intrinsic_contract_method(&mut builder, "fieldinfo_get_is_mutable");

    // MethodInfo method contracts (3)
    let methodinfo_get_name_contract = builder.add_contract_def("MethodInfo.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "methodinfo_get_name");

    let methodinfo_get_ret_contract =
        builder.add_contract_def("MethodInfo.get_return_type", "writ");
    add_intrinsic_contract_method(&mut builder, "methodinfo_get_return_type");

    let methodinfo_get_params_contract =
        builder.add_contract_def("MethodInfo.get_parameters", "writ");
    add_intrinsic_contract_method(&mut builder, "methodinfo_get_parameters");

    // ParameterInfo method contracts (2)
    let paraminfo_get_name_contract = builder.add_contract_def("ParameterInfo.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "paraminfo_get_name");

    let paraminfo_get_type_contract = builder.add_contract_def("ParameterInfo.get_type", "writ");
    add_intrinsic_contract_method(&mut builder, "paraminfo_get_type");

    // AttributeInfo method contracts (2)
    let attrinfo_get_name_contract = builder.add_contract_def("AttributeInfo.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "attrinfo_get_name");

    let attrinfo_get_args_contract = builder.add_contract_def("AttributeInfo.get_args", "writ");
    add_intrinsic_contract_method(&mut builder, "attrinfo_get_args");

    // ContractInfo method contracts (2)
    let contractinfo_get_name_contract = builder.add_contract_def("ContractInfo.get_name", "writ");
    add_intrinsic_contract_method(&mut builder, "contractinfo_get_name");

    let contractinfo_get_type_contract = builder.add_contract_def("ContractInfo.get_type", "writ");
    add_intrinsic_contract_method(&mut builder, "contractinfo_get_type");

    // Phase 107: Dynamic invocation contracts (2)
    let fieldinfo_set_contract = builder.add_contract_def("FieldInfo.set", "writ");
    add_intrinsic_contract_method(&mut builder, "set");

    let methodinfo_invoke_contract = builder.add_contract_def("MethodInfo.invoke", "writ");
    add_intrinsic_contract_method(&mut builder, "invoke");

    // Phase 108: Generic reflection + per-member attributes (3)
    let type_type_args_contract = builder.add_contract_def("Type.type_args", "writ");
    add_intrinsic_contract_method(&mut builder, "type_args");

    let methodinfo_attrs_contract = builder.add_contract_def("MethodInfo.attributes", "writ");
    add_intrinsic_contract_method(&mut builder, "attributes");

    let fieldinfo_attrs_contract = builder.add_contract_def("FieldInfo.attributes", "writ");
    add_intrinsic_contract_method(&mut builder, "attributes");

    // ────────────────────────────────────────────────────────────────
    // Section 9: Reflection TypeDefs (spec section 2.18.9)
    // ────────────────────────────────────────────────────────────────
    //
    // 6 class-kind TypeDefs for the reflection API. Added in dependency
    // order so cross-reference fields can encode the correct TypeDef index.
    //
    // TypeDef indices (0-based): Type=9, ParameterInfo=10, AttributeInfo=11,
    // ContractInfo=12, FieldInfo=13, MethodInfo=14

    // Type (row 10) — 5 fields: name(string), namespace(string), kind(string), is_generic(bool), type_args(Array<Type>)
    let type_type = builder.add_type_def("Type", "writ", TypeDefKind::Class, 0);
    assert_eq!(type_type, type_def_token(TYPE_TYPEDEF_ROW));
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(&mut builder, "namespace", TypeSignature::String);
    add_typed_field(&mut builder, "kind", TypeSignature::String);
    add_typed_field(&mut builder, "is_generic", TypeSignature::Bool);
    add_typed_field(
        &mut builder,
        "type_args",
        TypeSignature::Array(Box::new(TypeSignature::Named(type_type))),
    );

    // ParameterInfo (row 11) — 2 fields: name(string), declared_type(Type)
    let param_info_type = builder.add_type_def("ParameterInfo", "writ", TypeDefKind::Class, 0);
    assert_eq!(param_info_type, type_def_token(PARAMETER_INFO_TYPEDEF_ROW));
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(
        &mut builder,
        "declared_type",
        TypeSignature::Named(type_type),
    );

    // AttributeInfo (row 12) — 2 fields: name(string), args(Array<Box>).
    // Box is append-only row 16, so this is the only forward field token.
    let attr_info_type = builder.add_type_def("AttributeInfo", "writ", TypeDefKind::Class, 0);
    assert_eq!(attr_info_type, type_def_token(ATTRIBUTE_INFO_TYPEDEF_ROW));
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(
        &mut builder,
        "args",
        TypeSignature::Array(Box::new(TypeSignature::Named(type_def_token(
            BOX_TYPEDEF_ROW,
        )))),
    );

    // ContractInfo (row 13) — 2 fields: name(string), type(Type)
    let contract_info_type = builder.add_type_def("ContractInfo", "writ", TypeDefKind::Class, 0);
    assert_eq!(
        contract_info_type,
        type_def_token(CONTRACT_INFO_TYPEDEF_ROW)
    );
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(&mut builder, "type", TypeSignature::Named(type_type));

    // FieldInfo (row 14) — 3 fields: name(string), declared_type(Type), is_mutable(bool)
    let field_info_type = builder.add_type_def("FieldInfo", "writ", TypeDefKind::Class, 0);
    assert_eq!(field_info_type, type_def_token(FIELD_INFO_TYPEDEF_ROW));
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(
        &mut builder,
        "declared_type",
        TypeSignature::Named(type_type),
    );
    add_typed_field(&mut builder, "is_mutable", TypeSignature::Bool);

    // MethodInfo (row 15) — 3 fields: name(string), return_type(Type), parameters(Array<ParameterInfo>)
    let method_info_type = builder.add_type_def("MethodInfo", "writ", TypeDefKind::Class, 0);
    assert_eq!(method_info_type, type_def_token(METHOD_INFO_TYPEDEF_ROW));
    add_typed_field(&mut builder, "name", TypeSignature::String);
    add_typed_field(&mut builder, "return_type", TypeSignature::Named(type_type));
    add_typed_field(
        &mut builder,
        "parameters",
        TypeSignature::Array(Box::new(TypeSignature::Named(param_info_type))),
    );

    // Append-only pseudo types keep the established rows 1..15 stable while
    // making the spec's dynamic Box and EntityList<T> signatures resolvable.
    let box_type = builder.add_type_def("Box", "writ", TypeDefKind::Class, 0);
    assert_eq!(box_type, type_def_token(BOX_TYPEDEF_ROW));
    let entity_list_type = builder.add_type_def("EntityList", "writ", TypeDefKind::Class, 0);
    assert_eq!(entity_list_type, type_def_token(ENTITY_LIST_TYPEDEF_ROW));
    builder.add_generic_param(entity_list_type, TYPE_GENERIC_OWNER_KIND, 0, "T");

    // ────────────────────────────────────────────────────────────────
    // Section 10: Reflection ImplDef entries (Phase 103)
    // ────────────────────────────────────────────────────────────────
    //
    // Link each reflection TypeDef to its synthetic method contracts.

    // --- Type implementations (9) ---
    add_intrinsic_impl(&mut builder, type_type, type_fields_contract, "fields");

    add_intrinsic_impl(&mut builder, type_type, type_methods_contract, "methods");

    add_intrinsic_impl(&mut builder, type_type, type_attrs_contract, "attributes");

    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_contracts_contract,
        "contracts",
    );

    add_intrinsic_impl(&mut builder, type_type, type_impl_contract, "implements");

    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_get_name_contract,
        "type_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_get_ns_contract,
        "type_get_namespace",
    );

    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_get_kind_contract,
        "type_get_kind",
    );

    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_get_is_generic_contract,
        "type_get_is_generic",
    );

    // --- ParameterInfo implementations (2) ---
    add_intrinsic_impl(
        &mut builder,
        param_info_type,
        paraminfo_get_name_contract,
        "paraminfo_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        param_info_type,
        paraminfo_get_type_contract,
        "paraminfo_get_type",
    );

    // --- AttributeInfo implementations (2) ---
    add_intrinsic_impl(
        &mut builder,
        attr_info_type,
        attrinfo_get_name_contract,
        "attrinfo_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        attr_info_type,
        attrinfo_get_args_contract,
        "attrinfo_get_args",
    );

    // --- ContractInfo implementations (2) ---
    add_intrinsic_impl(
        &mut builder,
        contract_info_type,
        contractinfo_get_name_contract,
        "contractinfo_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        contract_info_type,
        contractinfo_get_type_contract,
        "contractinfo_get_type",
    );

    // --- FieldInfo implementations (4) ---
    add_intrinsic_impl(&mut builder, field_info_type, fieldinfo_get_contract, "get");

    add_intrinsic_impl(
        &mut builder,
        field_info_type,
        fieldinfo_get_name_contract,
        "fieldinfo_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        field_info_type,
        fieldinfo_get_type_contract,
        "fieldinfo_get_declared_type",
    );

    add_intrinsic_impl(
        &mut builder,
        field_info_type,
        fieldinfo_get_mut_contract,
        "fieldinfo_get_is_mutable",
    );

    // --- MethodInfo implementations (3) ---
    add_intrinsic_impl(
        &mut builder,
        method_info_type,
        methodinfo_get_name_contract,
        "methodinfo_get_name",
    );

    add_intrinsic_impl(
        &mut builder,
        method_info_type,
        methodinfo_get_ret_contract,
        "methodinfo_get_return_type",
    );

    add_intrinsic_impl(
        &mut builder,
        method_info_type,
        methodinfo_get_params_contract,
        "methodinfo_get_parameters",
    );

    // --- Phase 107: Dynamic invocation implementations (2) ---
    add_intrinsic_impl(&mut builder, field_info_type, fieldinfo_set_contract, "set");

    add_intrinsic_impl(
        &mut builder,
        method_info_type,
        methodinfo_invoke_contract,
        "invoke",
    );

    // --- Phase 108: Generic reflection + per-member attributes (3) ---
    add_intrinsic_impl(
        &mut builder,
        type_type,
        type_type_args_contract,
        "type_args",
    );

    add_intrinsic_impl(
        &mut builder,
        method_info_type,
        methodinfo_attrs_contract,
        "attributes",
    );

    add_intrinsic_impl(
        &mut builder,
        field_info_type,
        fieldinfo_attrs_contract,
        "attributes",
    );

    // ────────────────────────────────────────────────────────────────
    // Section 11: Build and return
    // ────────────────────────────────────────────────────────────────

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::{read_blob, read_string};
    use crate::signature::{decode_method_signature, decode_type_signature};

    /// Helper to read a string from the module's string heap.
    fn str_from_heap(module: &Module, offset: u32) -> &str {
        read_string(&module.string_heap, offset).expect("valid string")
    }

    fn field_signature(module: &Module, type_name: &str, field_name: &str) -> TypeSignature {
        let type_idx = module
            .type_defs
            .iter()
            .position(|row| str_from_heap(module, row.name) == type_name)
            .expect("TypeDef exists");
        let start = module.type_defs[type_idx].field_list.saturating_sub(1) as usize;
        let end = module
            .type_defs
            .get(type_idx + 1)
            .map(|row| row.field_list.saturating_sub(1) as usize)
            .unwrap_or(module.field_defs.len());
        let field = module.field_defs[start..end]
            .iter()
            .find(|row| str_from_heap(module, row.name) == field_name)
            .expect("FieldDef exists");
        let blob = read_blob(&module.blob_heap, field.type_sig).expect("valid field blob");
        decode_type_signature(blob).expect("valid field signature")
    }

    fn decoded_method_signature(
        module: &Module,
        signature_offset: u32,
    ) -> (Vec<TypeSignature>, TypeSignature) {
        let blob = read_blob(&module.blob_heap, signature_offset).expect("valid method blob");
        decode_method_signature(blob).expect("valid canonical method signature")
    }

    fn contract_signature(
        module: &Module,
        contract_name: &str,
    ) -> (Vec<TypeSignature>, TypeSignature) {
        let contract_idx = module
            .contract_defs
            .iter()
            .position(|row| str_from_heap(module, row.name) == contract_name)
            .expect("ContractDef exists");
        let start = module.contract_defs[contract_idx]
            .method_list
            .saturating_sub(1) as usize;
        let end = module
            .contract_defs
            .get(contract_idx + 1)
            .map(|row| row.method_list.saturating_sub(1) as usize)
            .unwrap_or(module.contract_methods.len());
        assert_eq!(end - start, 1, "{contract_name} must have one method");
        decoded_method_signature(module, module.contract_methods[start].signature)
    }

    fn type_method<'a>(
        module: &'a Module,
        type_name: &str,
        method_name: &str,
    ) -> &'a crate::tables::MethodDefRow {
        let type_idx = module
            .type_defs
            .iter()
            .position(|row| str_from_heap(module, row.name) == type_name)
            .expect("TypeDef exists");
        module
            .type_method_indices(type_idx)
            .into_iter()
            .map(|idx| &module.method_defs[idx])
            .find(|row| str_from_heap(module, row.name) == method_name)
            .expect("direct MethodDef exists")
    }

    fn impl_method<'a>(
        module: &'a Module,
        type_name: &str,
        method_name: &str,
    ) -> &'a crate::tables::MethodDefRow {
        let type_idx = module
            .type_defs
            .iter()
            .position(|row| str_from_heap(module, row.name) == type_name)
            .expect("TypeDef exists");
        let type_token = type_def_token(type_idx as u32 + 1);
        module
            .impl_defs
            .iter()
            .enumerate()
            .filter(|(_, row)| row.type_token == type_token)
            .flat_map(|(idx, _)| module.impl_method_indices(idx))
            .map(|idx| &module.method_defs[idx])
            .find(|row| str_from_heap(module, row.name) == method_name)
            .expect("Impl MethodDef exists")
    }

    fn method_signature_for(
        module: &Module,
        method: &crate::tables::MethodDefRow,
    ) -> (Vec<TypeSignature>, TypeSignature) {
        decoded_method_signature(module, method.signature)
    }

    fn contract_specialization_for_method(module: &Module, method_name: &str) -> TypeSignature {
        let method = module
            .method_defs
            .iter()
            .find(|row| str_from_heap(module, row.name) == method_name)
            .expect("MethodDef exists");
        assert_eq!(method.owner.table_id(), TableId::ImplDef.as_u8());
        let impl_idx = method.owner.row_index().expect("ImplDef row") as usize - 1;
        let contract = module.impl_defs[impl_idx].contract;
        assert_eq!(contract.table_id(), TableId::TypeSpec.as_u8());
        let type_spec_idx = contract.row_index().expect("TypeSpec row") as usize - 1;
        let blob = read_blob(
            &module.blob_heap,
            module.type_specs[type_spec_idx].signature,
        )
        .expect("valid TypeSpec blob");
        decode_type_signature(blob).expect("canonical contract TypeSpec")
    }

    #[test]
    fn module_name_is_writ_runtime() {
        let module = build_writ_runtime_module();
        assert_eq!(module.module_defs.len(), 1);
        let name = str_from_heap(&module, module.module_defs[0].name);
        assert_eq!(name, "writ-runtime");
    }

    #[test]
    fn has_exactly_47_contract_defs() {
        let module = build_writ_runtime_module();
        // 20 normative/base contracts + 22 reflection accessor contracts
        // + 2 dynamic invocation contracts + 3 generic/member reflection contracts.
        assert_eq!(module.contract_defs.len(), 47);
    }

    #[test]
    fn contract_names_are_resolvable() {
        let module = build_writ_runtime_module();
        let names: Vec<&str> = module
            .contract_defs
            .iter()
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
        assert!(names.contains(&"Speaker"));
        assert!(names.contains(&"Reflectable"));
        assert!(names.contains(&"Hashable"));
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
        assert_eq!(module.contract_methods.len(), 47);

        // Verify slot assignments are all 0
        for cm in &module.contract_methods {
            assert_eq!(cm.slot, 0, "all contract methods should have slot 0");
        }
    }

    #[test]
    fn contract_method_names_match_spec() {
        let module = build_writ_runtime_module();
        let method_names: Vec<&str> = module
            .contract_methods
            .iter()
            .map(|m| str_from_heap(&module, m.name))
            .collect();
        let expected = [
            "op_add",
            "op_sub",
            "op_mul",
            "op_div",
            "op_mod",
            "op_neg",
            "op_not",
            "op_eq",
            "op_lt",
            "op_index",
            "op_index_set",
            "op_bitand",
            "op_bitor",
            "iterator",
            "next",
            "into",
            "message",
            "speaker_name",
        ];
        for name in &expected {
            assert!(
                method_names.contains(name),
                "missing contract method: {}",
                name
            );
        }
    }

    #[test]
    fn normative_contract_signatures_match_spec() {
        let module = build_writ_runtime_module();
        let gp0 = TypeSignature::GenericParam(0);
        let gp1 = TypeSignature::GenericParam(1);
        let binary = || (vec![gp0.clone()], gp1.clone());

        for name in ["Add", "Sub", "Mul", "Div", "Mod", "BitAnd", "BitOr"] {
            assert_eq!(contract_signature(&module, name), binary(), "{name}");
        }
        for name in ["Neg", "Not"] {
            assert_eq!(
                contract_signature(&module, name),
                (vec![], gp0.clone()),
                "{name}"
            );
        }
        for name in ["Eq", "Ord"] {
            assert_eq!(
                contract_signature(&module, name),
                (vec![gp0.clone()], TypeSignature::Bool),
                "{name}"
            );
        }
        assert_eq!(
            contract_signature(&module, "Index"),
            (vec![gp0.clone()], gp1.clone())
        );
        assert_eq!(
            contract_signature(&module, "IndexSet"),
            (vec![gp0.clone(), gp1], TypeSignature::Void)
        );
        assert_eq!(
            contract_signature(&module, "Iterable"),
            (vec![], iterator_of(gp0.clone()))
        );
        assert_eq!(
            contract_signature(&module, "Iterator"),
            (vec![], option_of(gp0.clone()))
        );
        assert_eq!(contract_signature(&module, "Into"), (vec![], gp0));
        assert_eq!(
            contract_signature(&module, "Error"),
            (vec![], TypeSignature::String)
        );
        assert_eq!(
            contract_signature(&module, "Speaker"),
            (vec![], TypeSignature::String)
        );
        assert_eq!(
            contract_signature(&module, "Reflectable"),
            (vec![], named_type(TYPE_TYPEDEF_ROW))
        );
        assert_eq!(
            contract_signature(&module, "Hashable"),
            (vec![], TypeSignature::Int)
        );
    }

    #[test]
    fn every_contract_and_intrinsic_method_has_a_canonical_signature() {
        let module = build_writ_runtime_module();
        for method in &module.contract_methods {
            decoded_method_signature(&module, method.signature);
        }
        for method in &module.method_defs {
            decoded_method_signature(&module, method.signature);
        }
    }

    #[test]
    fn intrinsic_parameter_and_register_counts_follow_the_calling_convention() {
        let module = build_writ_runtime_module();
        for method in &module.method_defs {
            let name = str_from_heap(&module, method.name);
            let (params, _) = decoded_method_signature(&module, method.signature);
            let has_receiver = !method.owner.is_null() && method.flags & STATIC_FLAG == 0;
            let expected = params.len() + usize::from(has_receiver);
            assert_eq!(method.param_count as usize, expected, "{name} param_count");
            assert!(
                method.param_count <= method.reg_count,
                "{name} has param_count {} greater than reg_count {}",
                method.param_count,
                method.reg_count
            );
        }

        let exact = [
            ("Array", "len", 1),
            ("Array", "slice", 3),
            ("Array", "resize", 2),
            ("Array", "copy_from", 5),
            ("Entity", "destroy", 1),
            ("Entity", "isAlive", 1),
            ("Entity", "getOrCreate", 0),
            ("Entity", "findAll", 0),
        ];
        for (type_name, method_name, expected) in exact {
            let method = type_method(&module, type_name, method_name);
            assert_eq!(
                method.param_count, expected,
                "{type_name}.{method_name} param_count"
            );
            assert_eq!(
                method.reg_count, expected,
                "{type_name}.{method_name} reg_count"
            );
        }
    }

    #[test]
    fn generic_impls_use_canonical_contract_typespecs() {
        let module = build_writ_runtime_module();
        let contract_names: Vec<_> = module
            .contract_defs
            .iter()
            .map(|row| str_from_heap(&module, row.name))
            .collect();
        for removed in [
            "Into<Float>",
            "Into<Int>",
            "Into<String>",
            "Index<Int>",
            "Index<Range>",
        ] {
            assert!(
                !contract_names.contains(&removed),
                "removed fake contract {removed}"
            );
        }

        let specialized_impls: Vec<_> = module
            .impl_defs
            .iter()
            .filter(|row| row.contract.table_id() == TableId::TypeSpec.as_u8())
            .collect();
        assert_eq!(specialized_impls.len(), 36);

        assert_eq!(
            contract_specialization_for_method(&module, "int_into_float"),
            generic_type("Into", vec![TypeSignature::Float])
        );
        assert_eq!(
            contract_specialization_for_method(&module, "float_into_int"),
            generic_type("Into", vec![TypeSignature::Int])
        );
        assert_eq!(
            contract_specialization_for_method(&module, "string_index_range"),
            generic_type(
                "Index",
                vec![range_of(TypeSignature::Int), TypeSignature::String],
            )
        );
        assert_eq!(
            contract_specialization_for_method(&module, "array_index"),
            generic_type(
                "Index",
                vec![TypeSignature::Int, TypeSignature::GenericParam(0)],
            )
        );
        assert_eq!(
            contract_specialization_for_method(&module, "array_index_range"),
            generic_type(
                "Index",
                vec![
                    range_of(TypeSignature::Int),
                    array_of(TypeSignature::GenericParam(0)),
                ],
            )
        );
    }

    #[test]
    fn type_defs_include_all_seventeen_types() {
        let module = build_writ_runtime_module();
        let type_names: Vec<&str> = module
            .type_defs
            .iter()
            .map(|t| str_from_heap(&module, t.name))
            .collect();
        assert_eq!(module.type_defs.len(), 17);
        assert!(type_names.contains(&"Option"));
        assert!(type_names.contains(&"Result"));
        assert!(type_names.contains(&"Range"));
        assert!(type_names.contains(&"Int"));
        assert!(type_names.contains(&"Float"));
        assert!(type_names.contains(&"Bool"));
        assert!(type_names.contains(&"String"));
        assert!(type_names.contains(&"Array"));
        assert!(type_names.contains(&"Entity"));
        assert!(type_names.contains(&"Type"));
        assert!(type_names.contains(&"ParameterInfo"));
        assert!(type_names.contains(&"AttributeInfo"));
        assert!(type_names.contains(&"ContractInfo"));
        assert!(type_names.contains(&"FieldInfo"));
        assert!(type_names.contains(&"MethodInfo"));
        assert!(type_names.contains(&"Box"));
        assert!(type_names.contains(&"EntityList"));
    }

    #[test]
    fn reflection_field_signatures_use_canonical_type_tokens() {
        let module = build_writ_runtime_module();
        let type_ty = TypeSignature::Named(type_def_token(TYPE_TYPEDEF_ROW));
        let box_ty = TypeSignature::Named(type_def_token(BOX_TYPEDEF_ROW));
        let parameter_info_ty = TypeSignature::Named(type_def_token(PARAMETER_INFO_TYPEDEF_ROW));

        let expected = [
            ("Type", "name", TypeSignature::String),
            ("Type", "namespace", TypeSignature::String),
            ("Type", "kind", TypeSignature::String),
            ("Type", "is_generic", TypeSignature::Bool),
            (
                "Type",
                "type_args",
                TypeSignature::Array(Box::new(type_ty.clone())),
            ),
            ("ParameterInfo", "name", TypeSignature::String),
            ("ParameterInfo", "declared_type", type_ty.clone()),
            ("AttributeInfo", "name", TypeSignature::String),
            (
                "AttributeInfo",
                "args",
                TypeSignature::Array(Box::new(box_ty)),
            ),
            ("ContractInfo", "name", TypeSignature::String),
            ("ContractInfo", "type", type_ty.clone()),
            ("FieldInfo", "name", TypeSignature::String),
            ("FieldInfo", "declared_type", type_ty.clone()),
            ("FieldInfo", "is_mutable", TypeSignature::Bool),
            ("MethodInfo", "name", TypeSignature::String),
            ("MethodInfo", "return_type", type_ty),
            (
                "MethodInfo",
                "parameters",
                TypeSignature::Array(Box::new(parameter_info_ty)),
            ),
        ];

        for (type_name, field_name, signature) in expected {
            assert_eq!(
                field_signature(&module, type_name, field_name),
                signature,
                "{type_name}.{field_name}"
            );
        }
    }

    #[test]
    fn reflection_fields_are_public() {
        let module = build_writ_runtime_module();
        for row in TYPE_TYPEDEF_ROW..=METHOD_INFO_TYPEDEF_ROW {
            let type_idx = row as usize - 1;
            let type_name = str_from_heap(&module, module.type_defs[type_idx].name);
            let start = module.type_defs[type_idx].field_list.saturating_sub(1) as usize;
            let end = module
                .type_defs
                .get(type_idx + 1)
                .map(|next| next.field_list.saturating_sub(1) as usize)
                .unwrap_or(module.field_defs.len());
            assert!(start < end, "{type_name} must define reflection fields");
            for field in &module.field_defs[start..end] {
                let field_name = str_from_heap(&module, field.name);
                assert_ne!(
                    field.flags & FIELD_FLAG_PUBLIC,
                    0,
                    "{type_name}.{field_name} must be public"
                );
            }
        }
    }

    #[test]
    fn appended_pseudo_types_keep_existing_rows_stable() {
        let module = build_writ_runtime_module();
        assert_eq!(
            str_from_heap(
                &module,
                module.type_defs[TYPE_TYPEDEF_ROW as usize - 1].name
            ),
            "Type"
        );
        assert_eq!(
            str_from_heap(&module, module.type_defs[BOX_TYPEDEF_ROW as usize - 1].name),
            "Box"
        );
        assert_eq!(
            str_from_heap(
                &module,
                module.type_defs[ENTITY_LIST_TYPEDEF_ROW as usize - 1].name,
            ),
            "EntityList"
        );
        let entity_list_token = type_def_token(ENTITY_LIST_TYPEDEF_ROW);
        let params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|row| row.owner == entity_list_token && row.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].ordinal, 0);
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn option_is_enum_with_one_generic_param() {
        let module = build_writ_runtime_module();
        let option = module
            .type_defs
            .iter()
            .find(|t| str_from_heap(&module, t.name) == "Option")
            .expect("Option type exists");
        assert_eq!(option.kind, 1, "Option should be Enum (kind=1)");

        // Find generic params owned by the Option TypeDef token
        let option_idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "Option")
            .unwrap();
        let option_token = MetadataToken::new(2, option_idx as u32 + 1); // table 2 = TypeDef
        let params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == option_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Option should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn result_is_enum_with_two_generic_params() {
        let module = build_writ_runtime_module();
        let result_type = module
            .type_defs
            .iter()
            .find(|t| str_from_heap(&module, t.name) == "Result")
            .expect("Result type exists");
        assert_eq!(result_type.kind, 1, "Result should be Enum (kind=1)");

        let result_idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "Result")
            .unwrap();
        let result_token = MetadataToken::new(2, result_idx as u32 + 1);
        let params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == result_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 2, "Result should have 2 generic params");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
        assert_eq!(str_from_heap(&module, params[1].name), "E");
    }

    #[test]
    fn range_is_struct_with_four_fields_and_one_generic_param() {
        let module = build_writ_runtime_module();
        let range_idx = module
            .type_defs
            .iter()
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

        let field_names: Vec<&str> = module.field_defs[field_start..field_end]
            .iter()
            .map(|f| str_from_heap(&module, f.name))
            .collect();
        assert!(field_names.contains(&"start"));
        assert!(field_names.contains(&"end"));
        assert!(field_names.contains(&"start_inclusive"));
        assert!(field_names.contains(&"end_inclusive"));
        assert!(
            module.field_defs[field_start..field_end]
                .iter()
                .all(|field| field.flags & FIELD_FLAG_READONLY != 0),
            "Range fields should be read-only after construction"
        );

        // Generic param
        let range_token = MetadataToken::new(2, range_idx as u32 + 1);
        let params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == range_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Range should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn primitive_types_are_structs() {
        let module = build_writ_runtime_module();
        for name in &["Int", "Float", "Bool", "String"] {
            let t = module
                .type_defs
                .iter()
                .find(|t| str_from_heap(&module, t.name) == *name)
                .unwrap_or_else(|| panic!("{} type should exist", name));
            assert_eq!(t.kind, 0, "{} should be Struct (kind=0)", name);
        }
    }

    #[test]
    fn impl_defs_count_is_at_least_40() {
        let module = build_writ_runtime_module();
        // 13 int + 10 float + 3 bool + 6 string + 4 array + 4 Reflectable = 40
        assert!(
            module.impl_defs.len() >= 40,
            "expected at least 40 ImplDef rows, got {}",
            module.impl_defs.len()
        );
    }

    #[test]
    fn all_intrinsic_methods_have_intrinsic_flag() {
        let module = build_writ_runtime_module();
        for method in &module.method_defs {
            // All methods in the virtual module should be intrinsic
            assert!(
                method.flags & METHOD_FLAG_INTRINSIC != 0,
                "method should have intrinsic flag set, flags=0x{:04x}",
                method.flags
            );
        }
    }

    #[test]
    fn array_type_has_one_field_and_one_generic_param() {
        let module = build_writ_runtime_module();
        let array_idx = module
            .type_defs
            .iter()
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
        assert_ne!(
            module.field_defs[field_start].flags & FIELD_FLAG_PUBLIC,
            0,
            "Array.length should be public"
        );
        assert_ne!(
            module.field_defs[field_start].flags & FIELD_FLAG_READONLY,
            0,
            "Array.length should be read-only after construction"
        );

        // Generic param
        let array_token = MetadataToken::new(2, array_idx as u32 + 1);
        let params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == array_token && p.owner_kind == 0)
            .collect();
        assert_eq!(params.len(), 1, "Array should have 1 generic param");
        assert_eq!(str_from_heap(&module, params[0].name), "T");
    }

    #[test]
    fn array_has_compiler_known_instance_methods() {
        let module = build_writ_runtime_module();
        let array_idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "Array")
            .expect("Array type exists");
        let method_indices = module.type_method_indices(array_idx);
        assert_eq!(
            method_indices.len(),
            4,
            "Array should have exactly 4 directly owned methods"
        );
        let methods: Vec<_> = method_indices
            .into_iter()
            .map(|idx| &module.method_defs[idx])
            .collect();
        let names: Vec<_> = methods
            .iter()
            .map(|row| str_from_heap(&module, row.name))
            .collect();
        assert_eq!(names, ["len", "slice", "resize", "copy_from"]);
        for method in methods {
            let name = str_from_heap(&module, method.name);
            assert_ne!(method.flags & PUBLIC_FLAG, 0, "Array.{name} must be public");
            assert_ne!(
                method.flags & METHOD_FLAG_INTRINSIC,
                0,
                "Array.{name} must be intrinsic"
            );
            let expected_mut = matches!(name, "resize" | "copy_from");
            assert_eq!(
                method.flags & MUT_SELF_FLAG != 0,
                expected_mut,
                "Array.{name} mut-self flag"
            );
        }
    }

    #[test]
    fn entity_type_is_entity_kind() {
        let module = build_writ_runtime_module();
        let entity = module
            .type_defs
            .iter()
            .find(|t| str_from_heap(&module, t.name) == "Entity")
            .expect("Entity type exists");
        assert_eq!(entity.kind, 2, "Entity should be Entity (kind=2)");
    }

    #[test]
    fn entity_has_four_static_methods() {
        let module = build_writ_runtime_module();
        let entity_idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "Entity")
            .expect("Entity type exists");
        let method_names: Vec<&str> = module
            .type_method_indices(entity_idx)
            .into_iter()
            .map(|idx| str_from_heap(&module, module.method_defs[idx].name))
            .collect();
        assert_eq!(method_names.len(), 4, "Entity should have 4 static methods");
        assert_eq!(
            method_names,
            ["destroy", "isAlive", "getOrCreate", "findAll"]
        );
        for name in method_names {
            let method = type_method(&module, "Entity", name);
            assert_ne!(
                method.flags & PUBLIC_FLAG,
                0,
                "Entity.{name} must be public"
            );
            assert_ne!(
                method.flags & STATIC_FLAG,
                0,
                "Entity.{name} must be static"
            );
            assert_ne!(
                method.flags & METHOD_FLAG_INTRINSIC,
                0,
                "Entity.{name} must be intrinsic"
            );
        }
    }

    #[test]
    fn array_and_entity_method_signatures_match_spec() {
        let module = build_writ_runtime_module();
        let gp0 = TypeSignature::GenericParam(0);

        let array_cases = [
            ("len", vec![], TypeSignature::Int),
            (
                "slice",
                vec![TypeSignature::Int, TypeSignature::Int],
                array_of(gp0.clone()),
            ),
            ("resize", vec![TypeSignature::Int], TypeSignature::Void),
            (
                "copy_from",
                vec![
                    array_of(gp0.clone()),
                    TypeSignature::Int,
                    TypeSignature::Int,
                    TypeSignature::Int,
                ],
                TypeSignature::Void,
            ),
        ];
        for (name, params, ret) in array_cases {
            assert_eq!(
                method_signature_for(&module, type_method(&module, "Array", name)),
                (params, ret),
                "Array.{name}"
            );
        }

        let entity_cases = [
            ("destroy", vec![TypeSignature::Entity], TypeSignature::Void),
            ("isAlive", vec![TypeSignature::Entity], TypeSignature::Bool),
            ("getOrCreate", vec![], gp0.clone()),
            ("findAll", vec![], entity_list_of(gp0)),
        ];
        for (name, params, ret) in entity_cases {
            assert_eq!(
                method_signature_for(&module, type_method(&module, "Entity", name)),
                (params, ret),
                "Entity.{name}"
            );
        }
    }

    #[test]
    fn entity_generic_methods_own_their_type_parameter() {
        let module = build_writ_runtime_module();
        for name in ["getOrCreate", "findAll"] {
            let method = type_method(&module, "Entity", name);
            let direct_owner = module
                .method_defs
                .iter()
                .position(|row| std::ptr::eq(row, method))
                .map(|idx| MetadataToken::new(TableId::MethodDef.as_u8(), idx as u32 + 1))
                .expect("MethodDef index");
            let params: Vec<_> = module
                .generic_params
                .iter()
                .filter(|row| row.owner == direct_owner)
                .collect();
            assert_eq!(params.len(), 1, "Entity.{name} generic parameter count");
            assert_eq!(params[0].owner_kind, METHOD_GENERIC_OWNER_KIND);
            assert_eq!(params[0].ordinal, 0);
            assert_eq!(str_from_heap(&module, params[0].name), "T");
        }
    }

    #[test]
    fn reflection_public_methods_use_spec_names_signatures_and_visibility() {
        let module = build_writ_runtime_module();
        let type_ty = named_type(TYPE_TYPEDEF_ROW);
        let box_ty = named_type(BOX_TYPEDEF_ROW);
        let attr_ty = named_type(ATTRIBUTE_INFO_TYPEDEF_ROW);

        let cases = [
            (
                "Type",
                "fields",
                vec![],
                array_of(named_type(FIELD_INFO_TYPEDEF_ROW)),
            ),
            (
                "Type",
                "methods",
                vec![],
                array_of(named_type(METHOD_INFO_TYPEDEF_ROW)),
            ),
            ("Type", "attributes", vec![], array_of(attr_ty.clone())),
            (
                "Type",
                "contracts",
                vec![],
                array_of(named_type(CONTRACT_INFO_TYPEDEF_ROW)),
            ),
            (
                "Type",
                "implements",
                vec![type_ty.clone()],
                TypeSignature::Bool,
            ),
            ("Type", "type_args", vec![], array_of(type_ty)),
            ("FieldInfo", "get", vec![box_ty.clone()], box_ty.clone()),
            (
                "FieldInfo",
                "set",
                vec![box_ty.clone(), box_ty.clone()],
                TypeSignature::Void,
            ),
            ("FieldInfo", "attributes", vec![], array_of(attr_ty.clone())),
            (
                "MethodInfo",
                "invoke",
                vec![box_ty.clone(), array_of(box_ty.clone())],
                box_ty,
            ),
            ("MethodInfo", "attributes", vec![], array_of(attr_ty)),
        ];

        for (type_name, name, params, ret) in cases {
            let method = impl_method(&module, type_name, name);
            assert_eq!(
                method_signature_for(&module, method),
                (params, ret),
                "{type_name}.{name}"
            );
            assert_ne!(
                method.flags & PUBLIC_FLAG,
                0,
                "{type_name}.{name} must be public"
            );
            assert_ne!(
                method.flags & METHOD_FLAG_INTRINSIC,
                0,
                "{type_name}.{name} must be intrinsic"
            );
        }

        let internal_getters = [
            ("Type", "type_get_name"),
            ("Type", "type_get_namespace"),
            ("Type", "type_get_kind"),
            ("Type", "type_get_is_generic"),
            ("FieldInfo", "fieldinfo_get_name"),
            ("FieldInfo", "fieldinfo_get_declared_type"),
            ("FieldInfo", "fieldinfo_get_is_mutable"),
            ("MethodInfo", "methodinfo_get_name"),
            ("MethodInfo", "methodinfo_get_return_type"),
            ("MethodInfo", "methodinfo_get_parameters"),
            ("ParameterInfo", "paraminfo_get_name"),
            ("ParameterInfo", "paraminfo_get_type"),
            ("AttributeInfo", "attrinfo_get_name"),
            ("AttributeInfo", "attrinfo_get_args"),
            ("ContractInfo", "contractinfo_get_name"),
            ("ContractInfo", "contractinfo_get_type"),
        ];
        for (type_name, name) in internal_getters {
            let method = impl_method(&module, type_name, name);
            assert_eq!(
                method.flags & PUBLIC_FLAG,
                0,
                "{type_name}.{name} must remain internal"
            );
        }
    }

    #[test]
    fn generic_params_for_contracts_are_correct() {
        let module = build_writ_runtime_module();

        // Add contract (index 0, token row 1) should have 2 generic params: T, R
        let add_token = MetadataToken::new(10, 1); // table 10 = ContractDef
        let add_params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == add_token)
            .collect();
        assert_eq!(add_params.len(), 2, "Add should have 2 generic params");

        // Error contract (index 16, token row 17) should have 0 generic params
        // (Speaker at index 17 also has 0 generic params)
        let error_token = MetadataToken::new(10, 17);
        let error_params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == error_token)
            .collect();
        assert_eq!(error_params.len(), 0, "Error should have 0 generic params");

        // Neg contract (index 5, token row 6) should have 1 generic param: R
        let neg_token = MetadataToken::new(10, 6);
        let neg_params: Vec<_> = module
            .generic_params
            .iter()
            .filter(|p| p.owner == neg_token)
            .collect();
        assert_eq!(neg_params.len(), 1, "Neg should have 1 generic param");
        assert_eq!(str_from_heap(&module, neg_params[0].name), "R");

        for param in module
            .generic_params
            .iter()
            .filter(|param| param.owner.table_id() == TableId::ContractDef.as_u8())
        {
            assert_eq!(
                param.owner_kind, CONTRACT_GENERIC_OWNER_KIND,
                "ContractDef generic parameters must use owner_kind=2"
            );
        }
    }

    // -- Reflection type tests (TYPE-01 through TYPE-08) --

    /// Helper: get fields owned by a TypeDef at 0-based index.
    fn get_type_fields<'a>(module: &'a Module, type_idx: usize) -> Vec<&'a str> {
        let td = &module.type_defs[type_idx];
        let field_start = td.field_list as usize - 1;
        let field_end = if type_idx + 1 < module.type_defs.len() {
            module.type_defs[type_idx + 1].field_list as usize - 1
        } else {
            module.field_defs.len()
        };
        module.field_defs[field_start..field_end]
            .iter()
            .map(|f| str_from_heap(module, f.name))
            .collect()
    }

    #[test]
    fn type_typedef_is_class_with_five_fields() {
        // TYPE-01: Type class has 5 fields (Phase 108 added type_args)
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "Type")
            .expect("Type typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "Type should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            5,
            "Type should have 5 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"namespace"));
        assert!(fields.contains(&"kind"));
        assert!(fields.contains(&"is_generic"));
        assert!(fields.contains(&"type_args"));
    }

    #[test]
    fn fieldinfo_typedef_is_class_with_three_fields() {
        // TYPE-02: FieldInfo class has 3 fields
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "FieldInfo")
            .expect("FieldInfo typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "FieldInfo should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            3,
            "FieldInfo should have 3 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"declared_type"));
        assert!(fields.contains(&"is_mutable"));
    }

    #[test]
    fn methodinfo_typedef_is_class_with_three_fields() {
        // TYPE-03: MethodInfo class has 3 fields
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "MethodInfo")
            .expect("MethodInfo typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "MethodInfo should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            3,
            "MethodInfo should have 3 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"return_type"));
        assert!(fields.contains(&"parameters"));
    }

    #[test]
    fn parameterinfo_typedef_is_class_with_two_fields() {
        // TYPE-04: ParameterInfo class has 2 fields
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "ParameterInfo")
            .expect("ParameterInfo typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "ParameterInfo should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            2,
            "ParameterInfo should have 2 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"declared_type"));
    }

    #[test]
    fn attributeinfo_typedef_is_class_with_two_fields() {
        // TYPE-05: AttributeInfo class has 2 fields
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "AttributeInfo")
            .expect("AttributeInfo typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "AttributeInfo should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            2,
            "AttributeInfo should have 2 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"args"));
    }

    #[test]
    fn contractinfo_typedef_is_class_with_two_fields() {
        // TYPE-06: ContractInfo class has 2 fields
        let module = build_writ_runtime_module();
        let idx = module
            .type_defs
            .iter()
            .position(|t| str_from_heap(&module, t.name) == "ContractInfo")
            .expect("ContractInfo typedef exists");
        assert_eq!(
            module.type_defs[idx].kind, 4,
            "ContractInfo should be Class (kind=4)"
        );
        let fields = get_type_fields(&module, idx);
        assert_eq!(
            fields.len(),
            2,
            "ContractInfo should have 2 fields, got {:?}",
            fields
        );
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"type"));
    }

    #[test]
    fn reflectable_contract_at_index_18_with_get_type() {
        // TYPE-07: Reflectable at 0-based index 18, with get_type at slot 0
        let module = build_writ_runtime_module();
        assert!(
            module.contract_defs.len() > 18,
            "need at least 19 contract_defs"
        );
        let name = str_from_heap(&module, module.contract_defs[18].name);
        assert_eq!(
            name, "Reflectable",
            "contract at index 18 should be Reflectable"
        );

        // Find the contract method for Reflectable (1-based method_list -> 0-based index)
        let method_start = module.contract_defs[18].method_list as usize - 1;
        let method_end = if 19 < module.contract_defs.len() {
            module.contract_defs[19].method_list as usize - 1
        } else {
            module.contract_methods.len()
        };
        assert_eq!(
            method_end - method_start,
            1,
            "Reflectable should have exactly 1 method"
        );
        let method = &module.contract_methods[method_start];
        assert_eq!(str_from_heap(&module, method.name), "get_type");
        assert_eq!(method.slot, 0, "get_type should be at slot 0");
    }

    #[test]
    fn primitive_reflectable_impl_defs_exist() {
        // TYPE-08: Int, Float, Bool, String each have a Reflectable ImplDef
        let module = build_writ_runtime_module();

        // Reflectable is at 0-based contract index 18 => 1-based token row 19
        let reflectable_token = MetadataToken::new(10, 19);

        let primitive_names = ["Int", "Float", "Bool", "String"];
        let mut found = 0usize;

        for prim_name in &primitive_names {
            let type_idx = module
                .type_defs
                .iter()
                .position(|t| str_from_heap(&module, t.name) == *prim_name)
                .unwrap_or_else(|| panic!("{} type not found", prim_name));
            // 1-based TypeDef token
            let type_token = MetadataToken::new(2, type_idx as u32 + 1);

            let has_impl = module
                .impl_defs
                .iter()
                .any(|imp| imp.type_token == type_token && imp.contract == reflectable_token);
            assert!(has_impl, "{} should have a Reflectable ImplDef", prim_name);
            found += 1;
        }
        assert_eq!(found, 4, "expected 4 primitive Reflectable ImplDefs");
    }
}
