use writ_module::error::DecodeError;
use writ_module::heap;
use writ_module::instruction::Instruction;
use writ_module::module::{DebugLocal, MethodBody, Module};
use writ_module::signature::{TypeSignature, encode_type_signature};
use writ_module::tables::*;
use writ_module::{MetadataToken, ModuleBuilder};

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
    Instruction::LoadInt {
        r_dst: 0,
        value: 42,
    }
    .encode(&mut code)
    .unwrap();
    Instruction::LoadString {
        r_dst: 1,
        string_idx: 100,
    }
    .encode(&mut code)
    .unwrap();
    Instruction::AddI {
        r_dst: 2,
        r_a: 0,
        r_b: 0,
    }
    .encode(&mut code)
    .unwrap();
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
        body_size: 1,   // non-zero to indicate body exists
        reg_count: 3,
        param_count: 0,
        owner: MetadataToken::NULL,
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
        owner: MetadataToken::new(TableId::ImplDef.as_u8(), 1),
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

    // TypeSpecs used directly by ImplDef target and contract tokens.
    for (name, argument) in [
        ("Player", TypeSignature::Int),
        ("Updatable", TypeSignature::String),
    ] {
        let signature = encode_type_signature(&TypeSignature::Generic {
            namespace: "game".to_string(),
            name: name.to_string(),
            args: vec![argument],
        })
        .unwrap();
        module.type_specs.push(TypeSpecRow {
            signature: heap::write_blob(&mut module.blob_heap, &signature),
        });
    }

    // ImplDef
    module.impl_defs.push(ImplDefRow {
        type_token: MetadataToken::new(TableId::TypeSpec.as_u8(), 1),
        contract: MetadataToken::new(TableId::TypeSpec.as_u8(), 2),
        method_list: 1,
    });

    assert_round_trip(&module);

    let decoded = Module::from_bytes(&module.to_bytes().unwrap()).unwrap();
    assert_eq!(
        decoded.impl_defs[0].type_token.table_id(),
        TableId::TypeSpec.as_u8()
    );
    assert_eq!(
        decoded.impl_defs[0].contract.table_id(),
        TableId::TypeSpec.as_u8()
    );
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

#[test]
fn test_class_typedef_round_trip() {
    let mut module = Module::new();

    let name_off = heap::intern_string(&mut module.string_heap, "MyClass");
    let ns_off = heap::intern_string(&mut module.string_heap, "game");

    module.type_defs.push(TypeDefRow {
        name: name_off,
        namespace: ns_off,
        kind: TypeDefKind::Class.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 1,
    });

    let field_name = heap::intern_string(&mut module.string_heap, "value");
    let type_sig = heap::write_blob(&mut module.blob_heap, &[0x00]); // primitive int tag

    module.field_defs.push(FieldDefRow {
        name: field_name,
        type_sig,
        flags: 0,
    });

    assert_round_trip(&module);

    // After round-trip, verify kind is still 4
    let bytes = module.to_bytes().unwrap();
    let module2 = Module::from_bytes(&bytes).unwrap();
    assert_eq!(module2.type_defs.len(), 1);
    assert_eq!(
        module2.type_defs[0].kind, 4,
        "Class kind must survive round-trip as 4"
    );
}

#[test]
fn test_format_version_rejection() {
    // Build a valid v9 module, then patch the format_version bytes to stale v8.
    // Version 8 permits raw field ordinals and has the removed SPAWN_DETACHED opcode.
    let module = Module::new();
    let mut bytes = module.to_bytes().expect("to_bytes should succeed");

    // format_version is at bytes 4-5 (little-endian u16)
    bytes[4] = 0x08;
    bytes[5] = 0x00;

    let result = Module::from_bytes(&bytes);
    assert!(result.is_err());
    match result.unwrap_err() {
        DecodeError::UnsupportedVersion(v) => {
            assert_eq!(v, 8, "Expected UnsupportedVersion(8)");
        }
        other => panic!("Expected UnsupportedVersion, got {other:?}"),
    }
}

#[test]
fn test_reserved_methodref_flags_rejected_during_decode() {
    let mut builder = ModuleBuilder::new("invalid_methodref_flags");
    let scope = builder.add_module_ref("dependency", "1.0.0");
    let parent = builder.add_type_ref(scope, "Utility", "");
    builder.add_method_ref(parent, "identity", &[0, 0]);
    let mut bytes = builder.build().to_bytes().unwrap();

    let directory_entry = 32 + TableId::MethodRef.as_u8() as usize * 8;
    let row_offset = u32::from_le_bytes(
        bytes[directory_entry..directory_entry + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    bytes[row_offset + 12..row_offset + 14].copy_from_slice(&0x8000u16.to_le_bytes());

    match Module::from_bytes(&bytes) {
        Err(DecodeError::InvalidMethodRefFlags(0x8000)) => {}
        other => panic!("expected InvalidMethodRefFlags(0x8000), got {other:?}"),
    }
}

#[test]
fn test_debug_local_v6_roundtrip() {
    let mut module = Module::new();
    module.header.flags = 1; // enable debug info

    let name_off = heap::intern_string(&mut module.string_heap, "my_var");
    let type_blob = heap::write_blob(&mut module.blob_heap, &[0x00]); // int type

    let mut code = Vec::new();
    Instruction::LoadInt { r_dst: 0, value: 1 }
        .encode(&mut code)
        .unwrap();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let sig_off = heap::write_blob(&mut module.blob_heap, &[0x00]);
    let reg_type = heap::write_blob(&mut module.blob_heap, &[0x00]);

    module.method_defs.push(writ_module::tables::MethodDefRow {
        name: name_off,
        signature: sig_off,
        flags: 0,
        body_offset: 0,
        body_size: 1,
        reg_count: 1,
        param_count: 0,
        owner: MetadataToken::NULL,
    });

    module.method_bodies.push(MethodBody {
        register_types: vec![reg_type],
        code,
        debug_locals: vec![DebugLocal {
            register: 0,
            name: name_off,
            type_ref: type_blob, // non-zero type_ref
            start_pc: 0,
            end_pc: 100,
        }],
        source_spans: Vec::new(),
    });

    // Round-trip: serialize then deserialize
    let bytes = module.to_bytes().expect("to_bytes should succeed");
    let module2 = Module::from_bytes(&bytes).expect("from_bytes should succeed");

    // Verify DebugLocal was preserved correctly (18 bytes per entry)
    assert_eq!(module2.method_bodies.len(), 1);
    let body2 = &module2.method_bodies[0];
    assert_eq!(body2.debug_locals.len(), 1);
    let dl = &body2.debug_locals[0];
    assert_eq!(dl.register, 0);
    assert_eq!(dl.name, name_off);
    assert_eq!(dl.type_ref, type_blob, "type_ref must survive round-trip");
    assert_eq!(dl.start_pc, 0);
    assert_eq!(dl.end_pc, 100);
}

#[test]
fn test_invalid_typedef_kind_rejection() {
    // Build a valid module with one TypeDef, serialize it, then corrupt the kind byte
    let mut module = Module::new();
    let name_off = heap::intern_string(&mut module.string_heap, "SomeType");
    let ns_off = heap::intern_string(&mut module.string_heap, "ns");
    module.type_defs.push(TypeDefRow {
        name: name_off,
        namespace: ns_off,
        kind: TypeDefKind::Struct.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 1,
    });

    let mut bytes = module.to_bytes().expect("to_bytes should succeed");

    // The table directory starts at offset 32 (after magic+version+flags+name+version+heaps)
    // TypeDef is table 2, so its directory entry is at offset 32 + 2*8 = 48
    // Entry format: (offset: u32, count: u32)
    // Read TypeDef table offset from bytes 48..52
    let typedef_offset = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]) as usize;
    // TypeDef row layout: name(u32) + namespace(u32) + kind(u8) = kind byte at row_start + 8
    let kind_byte_offset = typedef_offset + 8;
    bytes[kind_byte_offset] = 0xFF;

    let result = Module::from_bytes(&bytes);
    assert!(result.is_err());
    match result.unwrap_err() {
        DecodeError::InvalidTypeDefKind(k) => {
            assert_eq!(k, 0xFF, "Expected InvalidTypeDefKind(0xFF)");
        }
        other => panic!("Expected InvalidTypeDefKind, got {other:?}"),
    }
}

#[test]
fn test_unsupported_version_3_rejected() {
    let module = Module::new();
    let mut bytes = module.to_bytes().unwrap();
    // Patch format_version field (bytes 4-5 in LE u16 in the 200-byte header)
    bytes[4] = 3;
    bytes[5] = 0;
    match Module::from_bytes(&bytes) {
        Err(DecodeError::UnsupportedVersion(3)) => {}
        other => panic!("expected UnsupportedVersion(3), got {:?}", other),
    }
}
