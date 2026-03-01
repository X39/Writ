use writ_module::error::DecodeError;
use writ_module::heap;
use writ_module::instruction::Instruction;
use writ_module::module::{MethodBody, Module};
use writ_module::tables::*;
use writ_module::MetadataToken;

/// Assert write -> read -> write produces identical bytes.
fn assert_round_trip(module: &Module) {
    let bytes1 = module.to_bytes().expect("first to_bytes should succeed");
    let module2 = Module::from_bytes(&bytes1).expect("from_bytes should succeed");
    let bytes2 = module2.to_bytes().expect("second to_bytes should succeed");
    assert_eq!(bytes1, bytes2, "Round-trip identity failed: bytes differ");
}

#[test]
fn test_empty_module_round_trip() {
    let module = Module::new();
    let bytes = module.to_bytes().unwrap();
    // Must start with WRIT magic
    assert_eq!(&bytes[0..4], b"WRIT");
    // Must have at least the 200-byte header
    assert!(bytes.len() >= 200);
    assert_round_trip(&module);
}

#[test]
fn test_module_with_strings_round_trip() {
    let mut module = Module::new();

    let hello_off = heap::intern_string(&mut module.string_heap, "hello");
    let world_off = heap::intern_string(&mut module.string_heap, "world");

    module.header.module_name = hello_off;
    module.module_defs.push(ModuleDefRow {
        name: hello_off,
        version: world_off,
        flags: 0,
    });

    assert_round_trip(&module);

    // After round-trip, verify heap contents
    let bytes = module.to_bytes().unwrap();
    let module2 = Module::from_bytes(&bytes).unwrap();
    assert_eq!(
        heap::read_string(&module2.string_heap, hello_off).unwrap(),
        "hello"
    );
    assert_eq!(
        heap::read_string(&module2.string_heap, world_off).unwrap(),
        "world"
    );
}

#[test]
fn test_module_with_typedef_round_trip() {
    let mut module = Module::new();

    let name_off = heap::intern_string(&mut module.string_heap, "MyStruct");
    let ns_off = heap::intern_string(&mut module.string_heap, "game");

    module.type_defs.push(TypeDefRow {
        name: name_off,
        namespace: ns_off,
        kind: TypeDefKind::Struct.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 0,
    });

    let field_name = heap::intern_string(&mut module.string_heap, "x");
    let type_sig = heap::write_blob(&mut module.blob_heap, &[0x00]); // primitive int tag

    module.field_defs.push(FieldDefRow {
        name: field_name,
        type_sig,
        flags: 0,
    });

    assert_round_trip(&module);
}

#[test]
fn test_module_with_method_body_round_trip() {
    let mut module = Module::new();

    // Create method body with some instructions
    let mut code = Vec::new();
    Instruction::LoadInt { r_dst: 0, value: 42 }.encode(&mut code).unwrap();
    Instruction::LoadString { r_dst: 1, string_idx: 100 }.encode(&mut code).unwrap();
    Instruction::AddI { r_dst: 2, r_a: 0, r_b: 0 }.encode(&mut code).unwrap();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let sig_off = heap::write_blob(&mut module.blob_heap, &[0x00]);
    let reg_type1 = heap::write_blob(&mut module.blob_heap, &[0x00]); // int
    let reg_type2 = heap::write_blob(&mut module.blob_heap, &[0x04]); // string

    let method_name = heap::intern_string(&mut module.string_heap, "main");

    module.method_defs.push(MethodDefRow {
        name: method_name,
        signature: sig_off,
        flags: 0,
        body_offset: 0, // writer will set
        body_size: 1,    // non-zero to indicate body exists
        reg_count: 3,
        param_count: 0,
    });

    module.method_bodies.push(MethodBody {
        register_types: vec![reg_type1, reg_type2, reg_type1],
        code,
        debug_locals: Vec::new(),
        source_spans: Vec::new(),
    });

    assert_round_trip(&module);
}

#[test]
fn test_module_with_multiple_tables_round_trip() {
    let mut module = Module::new();

    // ModuleDef
    let mod_name = heap::intern_string(&mut module.string_heap, "test_module");
    let mod_version = heap::intern_string(&mut module.string_heap, "1.0.0");
    module.module_defs.push(ModuleDefRow {
        name: mod_name,
        version: mod_version,
        flags: 0,
    });
    module.header.module_name = mod_name;
    module.header.module_version = mod_version;

    // TypeDef
    let type_name = heap::intern_string(&mut module.string_heap, "Player");
    let ns = heap::intern_string(&mut module.string_heap, "game");
    module.type_defs.push(TypeDefRow {
        name: type_name,
        namespace: ns,
        kind: TypeDefKind::Entity.as_u8(),
        flags: 0x01,
        field_list: 1,
        method_list: 1,
    });

    // FieldDef
    let field_name = heap::intern_string(&mut module.string_heap, "health");
    let field_sig = heap::write_blob(&mut module.blob_heap, &[0x00]);
    module.field_defs.push(FieldDefRow {
        name: field_name,
        type_sig: field_sig,
        flags: 0,
    });

    // MethodDef (no body)
    let meth_name = heap::intern_string(&mut module.string_heap, "update");
    let meth_sig = heap::write_blob(&mut module.blob_heap, &[0x01, 0x00]);
    module.method_defs.push(MethodDefRow {
        name: meth_name,
        signature: meth_sig,
        flags: 0,
        body_offset: 0,
        body_size: 0,
        reg_count: 0,
        param_count: 0,
    });

    // ContractDef
    let contract_name = heap::intern_string(&mut module.string_heap, "Updatable");
    let contract_ns = heap::intern_string(&mut module.string_heap, "game");
    module.contract_defs.push(ContractDefRow {
        name: contract_name,
        namespace: contract_ns,
        method_list: 1,
        generic_param_list: 0,
    });

    // ImplDef
    module.impl_defs.push(ImplDefRow {
        type_token: MetadataToken::new(TableId::TypeDef.as_u8(), 1),
        contract: MetadataToken::new(TableId::ContractDef.as_u8(), 1),
        method_list: 1,
    });

    assert_round_trip(&module);
}

#[test]
fn test_bad_magic_error() {
    let mut bytes = vec![0u8; 200];
    bytes[0] = b'X';
    bytes[1] = b'R';
    bytes[2] = b'I';
    bytes[3] = b'T';

    let result = Module::from_bytes(&bytes);
    assert!(result.is_err());
    match result.unwrap_err() {
        DecodeError::BadMagic(magic) => {
            assert_eq!(&magic, b"XRIT");
        }
        other => panic!("Expected BadMagic, got {other:?}"),
    }
}

#[test]
fn test_truncated_header_error() {
    let mut bytes = vec![0u8; 100];
    bytes[0..4].copy_from_slice(b"WRIT");

    let result = Module::from_bytes(&bytes);
    assert!(result.is_err());
    // Should be UnexpectedEof or an IO error
    match &result.unwrap_err() {
        DecodeError::UnexpectedEof => {}
        DecodeError::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
        other => panic!("Expected EOF-related error, got {other:?}"),
    }
}
