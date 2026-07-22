use writ_module::heap;
use writ_module::instruction::Instruction;
use writ_module::module::{MethodBody, Module};
use writ_module::tables::{TypeDefKind, METHOD_REF_FLAG_HAS_RECEIVER};
use writ_module::{FORMAT_VERSION, MetadataToken, ModuleBuilder};

#[test]
fn test_empty_builder() {
    let module = ModuleBuilder::new("test").build();
    assert_eq!(module.header.format_version, FORMAT_VERSION);
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
    let _type_tok = builder.add_type_def("MyStruct", "game", TypeDefKind::Struct, 0);
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
fn test_builder_derives_method_param_register_counts() {
    fn empty_body(reg_count: usize) -> MethodBody {
        MethodBody {
            register_types: vec![0; reg_count],
            code: Vec::new(),
            debug_locals: Vec::new(),
            source_spans: Vec::new(),
        }
    }

    // One regular int parameter and a void return.
    let signature = [1, 0, 0x01, 0x00];
    let mut builder = ModuleBuilder::new("method_params");
    let owner = builder.add_type_def("Counter", "", TypeDefKind::Class, 0);
    builder.add_type_method(owner, "instance", &signature, 0, 2, empty_body(2));
    builder.add_type_method(owner, "static_method", &signature, 1 << 1, 1, empty_body(1));
    builder.add_method("free_function", &signature, 1 << 1, 1, empty_body(1));

    let module = builder.build();
    assert_eq!(module.method_defs[0].param_count, 2, "r0=self, r1=arg");
    assert_eq!(module.method_defs[1].param_count, 1, "r0=arg");
    assert_eq!(module.method_defs[2].param_count, 1, "r0=arg");
}

#[test]
fn test_builder_round_trip_through_serialization() {
    let mut builder = ModuleBuilder::new("round_trip_test");

    // Add a type with a field
    let _type_tok = builder.add_type_def("Point", "math", TypeDefKind::Struct, 0);
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
fn method_ref_receiver_abi_is_part_of_identity_and_round_trips() {
    let mut builder = ModuleBuilder::new("method_ref_abi");
    let scope = builder.add_module_ref("dependency", "1.0.0");
    let parent = builder.add_type_ref(scope, "Utility", "");
    let signature = [0, 0x01];

    let instance = builder.add_method_ref_with_flags(
        parent,
        "identity",
        &signature,
        METHOD_REF_FLAG_HAS_RECEIVER,
    );
    let static_method = builder.add_method_ref_with_flags(parent, "identity", &signature, 0);
    assert_ne!(instance, static_method, "receiver ABI distinguishes MethodRef rows");

    let bytes = builder.build().to_bytes().unwrap();
    let decoded = Module::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.method_refs.len(), 2);
    assert_eq!(decoded.method_refs[0].flags, METHOD_REF_FLAG_HAS_RECEIVER);
    assert_eq!(decoded.method_refs[1].flags, 0);
    assert_eq!(decoded.to_bytes().unwrap(), bytes);
}

#[test]
fn test_builder_multiple_types() {
    let mut builder = ModuleBuilder::new("multi_type");

    // Type 1 with 2 fields
    let _t1 = builder.add_type_def("Vec2", "math", TypeDefKind::Struct, 0);
    let _f1 = builder.add_field_def("x", &[0x01], 0);
    let _f2 = builder.add_field_def("y", &[0x01], 0);

    // Type 2 with 1 field
    let _t2 = builder.add_type_def("Color", "gfx", TypeDefKind::Struct, 0);
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
fn test_builder_zero_field_types_use_next_field_index() {
    let mut builder = ModuleBuilder::new("empty_field_ranges");

    // Empty types repeat the current 1-based next FieldDef row. This is the
    // sentinel that makes the preceding type's [start, next_start) range exact.
    builder.add_type_def("LeadingEmpty", "", TypeDefKind::Struct, 0);
    builder.add_type_def("HasOne", "", TypeDefKind::Struct, 0);
    builder.add_field_def("one", &[0x01], 0);
    builder.add_type_def("MiddleEmpty", "", TypeDefKind::Struct, 0);
    builder.add_type_def("HasTwo", "", TypeDefKind::Struct, 0);
    builder.add_field_def("first", &[0x01], 0);
    builder.add_field_def("second", &[0x01], 0);
    builder.add_type_def("TrailingEmpty", "", TypeDefKind::Struct, 0);

    let module = builder.build();
    let starts: Vec<u32> = module.type_defs.iter().map(|ty| ty.field_list).collect();

    assert_eq!(starts, vec![1, 1, 2, 2, 4]);
    assert!(starts.iter().all(|start| *start > 0));
    assert!(starts.windows(2).all(|pair| pair[0] <= pair[1]));
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

#[test]
fn test_builder_class_type() {
    let mut builder = ModuleBuilder::new("class_test");
    let _tok = builder.add_type_def("MyClass", "game", TypeDefKind::Class, 0);

    let module = builder.build();
    assert_eq!(module.type_defs.len(), 1);
    assert_eq!(module.type_defs[0].kind, 4, "Class kind should be 4");

    let type_name = heap::read_string(&module.string_heap, module.type_defs[0].name).unwrap();
    assert_eq!(type_name, "MyClass");
}

#[test]
fn test_explicit_method_ownership_queries_round_trip() {
    fn empty_body() -> MethodBody {
        MethodBody {
            register_types: Vec::new(),
            code: Vec::new(),
            debug_locals: Vec::new(),
            source_spans: Vec::new(),
        }
    }

    let mut builder = ModuleBuilder::new("owners");
    let type_owner = builder.add_type_def("Thing", "", TypeDefKind::Struct, 0);
    let impl_owner = builder.add_impl_def(type_owner, MetadataToken::NULL);

    builder.add_type_method(type_owner, "hook", &[0x00], 0, 0, empty_body());
    builder.add_impl_method(impl_owner, "method", &[0x00], 0, 0, empty_body());
    builder.add_method("factory", &[0x00], 0, 0, empty_body());

    let module = builder.build();
    assert_eq!(module.type_method_indices(0), vec![0]);
    assert_eq!(module.impl_method_indices(0), vec![1]);
    assert_eq!(module.top_level_method_indices(), vec![2]);
    assert_eq!(module.method_indices_owned_by(type_owner), vec![0]);
    assert_eq!(module.method_indices_owned_by(impl_owner), vec![1]);

    let bytes = module.to_bytes().unwrap();
    let decoded = Module::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.type_method_indices(0), vec![0]);
    assert_eq!(decoded.impl_method_indices(0), vec![1]);
    assert_eq!(decoded.top_level_method_indices(), vec![2]);
}
