use writ_module::heap;
use writ_module::instruction::Instruction;
use writ_module::module::{MethodBody, Module};
use writ_module::tables::TypeDefKind;
use writ_module::ModuleBuilder;

#[test]
fn test_empty_builder() {
    let module = ModuleBuilder::new("test").build();
    assert_eq!(module.module_defs.len(), 1);
    let name = heap::read_string(&module.string_heap, module.module_defs[0].name).unwrap();
    assert_eq!(name, "test");
}

#[test]
fn test_builder_with_version() {
    let module = ModuleBuilder::new("test").version("2.1.0").build();
    assert_eq!(module.module_defs.len(), 1);
    let version = heap::read_string(&module.string_heap, module.module_defs[0].version).unwrap();
    assert_eq!(version, "2.1.0");
}

#[test]
fn test_builder_with_type_and_fields() {
    let mut builder = ModuleBuilder::new("test_mod");
    let _type_tok = builder.add_type_def("MyStruct", "game", TypeDefKind::Struct.as_u8(), 0);
    let _field1 = builder.add_field_def("x", &[0x00], 0); // int
    let _field2 = builder.add_field_def("y", &[0x00], 0); // int

    let module = builder.build();
    assert_eq!(module.type_defs.len(), 1);
    assert_eq!(module.field_defs.len(), 2);

    let type_name = heap::read_string(&module.string_heap, module.type_defs[0].name).unwrap();
    assert_eq!(type_name, "MyStruct");

    let f1_name = heap::read_string(&module.string_heap, module.field_defs[0].name).unwrap();
    let f2_name = heap::read_string(&module.string_heap, module.field_defs[1].name).unwrap();
    assert_eq!(f1_name, "x");
    assert_eq!(f2_name, "y");
}

#[test]
fn test_builder_with_method_body() {
    let mut builder = ModuleBuilder::new("test_mod");

    // Create a method body
    let mut code = Vec::new();
    Instruction::LoadInt { r_dst: 0, value: 42 }.encode(&mut code).unwrap();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let body = MethodBody {
        register_types: vec![0, 0], // placeholder blob offsets
        code,
        debug_locals: Vec::new(),
        source_spans: Vec::new(),
    };

    let _meth_tok = builder.add_method("main", &[0x01], 0, 2, body);

    let module = builder.build();
    assert_eq!(module.method_defs.len(), 1);
    assert_eq!(module.method_bodies.len(), 1);

    let meth_name = heap::read_string(&module.string_heap, module.method_defs[0].name).unwrap();
    assert_eq!(meth_name, "main");

    // Verify method body has code
    assert!(!module.method_bodies[0].code.is_empty());
}

#[test]
fn test_builder_round_trip_through_serialization() {
    let mut builder = ModuleBuilder::new("round_trip_test");

    // Add a type with a field
    let _type_tok = builder.add_type_def("Point", "math", TypeDefKind::Struct.as_u8(), 0);
    let _field = builder.add_field_def("x", &[0x00], 0);

    // Add a method with body
    let mut code = Vec::new();
    Instruction::LoadInt { r_dst: 0, value: 42 }.encode(&mut code).unwrap();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let body = MethodBody {
        register_types: vec![0],
        code,
        debug_locals: Vec::new(),
        source_spans: Vec::new(),
    };

    let _meth = builder.add_method("init", &[0x01], 0, 1, body);

    let module = builder.build();

    // Serialize
    let bytes1 = module.to_bytes().expect("first to_bytes should succeed");
    // Deserialize
    let module2 = Module::from_bytes(&bytes1).expect("from_bytes should succeed");
    // Serialize again
    let bytes2 = module2.to_bytes().expect("second to_bytes should succeed");

    assert_eq!(bytes1, bytes2, "Builder-produced module round-trip failed");
}

#[test]
fn test_builder_multiple_types() {
    let mut builder = ModuleBuilder::new("multi_type");

    // Type 1 with 2 fields
    let _t1 = builder.add_type_def("Vec2", "math", TypeDefKind::Struct.as_u8(), 0);
    let _f1 = builder.add_field_def("x", &[0x01], 0);
    let _f2 = builder.add_field_def("y", &[0x01], 0);

    // Type 2 with 1 field
    let _t2 = builder.add_type_def("Color", "gfx", TypeDefKind::Struct.as_u8(), 0);
    let _f3 = builder.add_field_def("r", &[0x00], 0);

    let module = builder.build();

    assert_eq!(module.type_defs.len(), 2);
    assert_eq!(module.field_defs.len(), 3);

    // Type 1 field_list should be 1 (first field)
    assert_eq!(module.type_defs[0].field_list, 1);
    // Type 2 field_list should be 3 (third field, after the 2 from type 1)
    assert_eq!(module.type_defs[1].field_list, 3);

    // Verify names
    let t1_name = heap::read_string(&module.string_heap, module.type_defs[0].name).unwrap();
    let t2_name = heap::read_string(&module.string_heap, module.type_defs[1].name).unwrap();
    assert_eq!(t1_name, "Vec2");
    assert_eq!(t2_name, "Color");
}

#[test]
fn test_builder_module_name_in_header() {
    let module = ModuleBuilder::new("my_game").build();
    let name = heap::read_string(&module.string_heap, module.header.module_name).unwrap();
    assert_eq!(name, "my_game");
}

#[test]
fn test_builder_serialization_no_error() {
    let module = ModuleBuilder::new("basic").build();
    let bytes = module.to_bytes();
    assert!(bytes.is_ok(), "Builder-produced module should serialize without error");
    let bytes = bytes.unwrap();
    assert!(bytes.len() >= 200, "Output should have at least a 200-byte header");
    assert_eq!(&bytes[0..4], b"WRIT");
}
