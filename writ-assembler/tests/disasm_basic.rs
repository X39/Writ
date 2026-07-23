/// Disassembler unit tests: verify output format of disassemble() and disassemble_verbose().

#[test]
fn disassemble_empty_module() {
    let src = r#"
.module "test" "1.0.0" {
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".module"),
        "output should contain .module directive"
    );
    assert!(
        text.contains("\"test\""),
        "output should contain module name"
    );
    assert!(
        text.contains("\"1.0.0\""),
        "output should contain module version"
    );
    assert!(text.contains('}'), "output should have closing brace");
}

#[test]
fn disassemble_type_with_fields() {
    let src = r#"
.module "test" "1.0.0" {
    .type "MyStruct" struct {
        .field "x" int pub
        .field "y" float pub
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".type"),
        "output should contain .type directive"
    );
    assert!(
        text.contains("\"MyStruct\""),
        "output should contain type name"
    );
    assert!(text.contains("struct"), "output should contain kind");
    assert!(
        text.contains(".field"),
        "output should contain .field directive"
    );
    assert!(text.contains("\"x\""), "output should contain field name x");
    assert!(text.contains("\"y\""), "output should contain field name y");
    assert!(text.contains("int"), "output should contain int type");
    assert!(text.contains("float"), "output should contain float type");
}

#[test]
fn disassemble_uses_table_specific_flag_names() {
    let src = r#"
.module "test" "1.0.0" {
    .type "State" struct pub {
        .field "value" int pub has_default component readonly
        .method "update" () -> void pub mut_self hook_destroy intrinsic dialogue {
            RET_VOID
        }
    }
    .global "counter" int pub mut
    .global "answer" int pub const
    .extern_fn "print" (string) -> void "host_print" pub
    .method "main" () -> void pub static {
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);

    assert!(text.contains(".type \"State\" struct pub {"), "{text}");
    assert!(
        text.contains(".field \"value\" int pub has_default component readonly"),
        "{text}"
    );
    assert!(
        text.contains(
            ".method \"update\" () -> void pub mut_self hook_destroy intrinsic dialogue {"
        ),
        "{text}"
    );
    assert!(text.contains(".global \"counter\" int pub mut"), "{text}");
    assert!(text.contains(".global \"answer\" int pub const"), "{text}");
    assert!(
        text.contains(".extern_fn \"print\" (string) -> void \"host_print\" pub"),
        "{text}"
    );
    assert!(
        text.contains(".method \"main\" () -> void pub static {"),
        "{text}"
    );

    let reassembled =
        writ_assembler::assemble(&text).expect("table-specific flags should round-trip");
    assert_eq!(reassembled.type_defs[0].flags, module.type_defs[0].flags);
    assert_eq!(reassembled.field_defs[0].flags, module.field_defs[0].flags);
    assert_eq!(
        reassembled.method_defs[0].flags,
        module.method_defs[0].flags
    );
    assert_eq!(
        reassembled.global_defs[0].flags,
        module.global_defs[0].flags
    );
    assert_eq!(
        reassembled.global_defs[1].flags,
        module.global_defs[1].flags
    );
    assert_eq!(
        reassembled.extern_defs[0].flags,
        module.extern_defs[0].flags
    );
}

#[test]
fn disassemble_unknown_flags_as_complete_numeric_values() {
    let src = r#"
.module "test" "1.0.0" {
    .type "Opaque" struct 32769 {
        .field "value" int 32776
        .method "run" () -> void 32769 {
            RET_VOID
        }
    }
    .global "state" int 32772
    .extern_fn "native" () -> void "host_native" 32769
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble numeric flags");
    let text = writ_assembler::disassemble(&module);

    assert!(text.contains(".type \"Opaque\" struct 32769 {"), "{text}");
    assert!(text.contains(".field \"value\" int 32776"), "{text}");
    assert!(
        text.contains(".method \"run\" () -> void 32769 {"),
        "{text}"
    );
    assert!(text.contains(".global \"state\" int 32772"), "{text}");
    assert!(
        text.contains(".extern_fn \"native\" () -> void \"host_native\" 32769"),
        "{text}"
    );

    let reassembled =
        writ_assembler::assemble(&text).expect("numeric fallback flags should round-trip");
    assert_eq!(reassembled.type_defs[0].flags, 32769);
    assert_eq!(reassembled.field_defs[0].flags, 32776);
    assert_eq!(reassembled.method_defs[0].flags, 32769);
    assert_eq!(reassembled.global_defs[0].flags, 32772);
    assert_eq!(reassembled.extern_defs[0].flags, 32769);
}

#[test]
fn disassemble_contract_with_methods() {
    let src = r#"
.module "test" "1.0.0" {
    .contract "IFoo" {
        .method "do_thing" (int) -> void slot 0
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".contract"),
        "output should contain .contract directive"
    );
    assert!(
        text.contains("\"IFoo\""),
        "output should contain contract name"
    );
    assert!(
        text.contains("\"do_thing\""),
        "output should contain method name"
    );
    assert!(text.contains("slot"), "output should contain slot keyword");
}

#[test]
fn disassemble_method_with_instructions() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> int {
        .reg r0 int
        LOAD_INT r0, 42
        RET r0
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".method"),
        "output should contain .method directive"
    );
    assert!(
        text.contains("\"main\""),
        "output should contain method name"
    );
    assert!(
        text.contains("LOAD_INT"),
        "output should contain LOAD_INT mnemonic"
    );
    assert!(text.contains("42"), "output should contain integer value");
    assert!(text.contains("RET"), "output should contain RET mnemonic");
    assert!(text.contains("r0"), "output should contain register r0");
}

#[test]
fn disassemble_field_operands_as_tokens() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        GET_FIELD r0, r0, token(83886081)
        SET_FIELD r0, token(100663297), r0
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);

    assert!(text.contains("GET_FIELD r0, r0, token(83886081)"), "{text}");
    assert!(
        text.contains("SET_FIELD r0, token(100663297), r0"),
        "{text}"
    );
    writ_assembler::assemble(&text).expect("disassembled field tokens should reassemble");
}

#[test]
fn disassemble_atomic_construction_operands() {
    let src = r#"
.module "test" "1.0.0" {
    .type "Thing" struct {
        .field "value" int
    }
    .type "Actor" entity {
    }
    .method "main" () -> void {
        .reg r0 int
        .reg r1 int
        .reg r2 int
        NEW r0, Thing, 1, r1
        SPAWN_ENTITY r1, Actor, 0, r2
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);

    assert!(text.contains("NEW r0, 33554433, 1, r1"), "{text}");
    assert!(text.contains("SPAWN_ENTITY r1, 33554434, 0, r2"), "{text}");
    writ_assembler::assemble(&text).expect("disassembled atomic constructors should reassemble");
}

#[test]
fn disassemble_param_names_ignore_implicit_receiver_registers() {
    use writ_module::TypeDefKind;
    use writ_module::builder::ModuleBuilder;
    use writ_module::module::MethodBody;
    use writ_module::signature::{TypeSignature, encode_method_signature, encode_type_signature};

    let void_sig = encode_method_signature(&[], &TypeSignature::Void).unwrap();
    let one_int_sig = encode_method_signature(&[TypeSignature::Int], &TypeSignature::Void).unwrap();
    let int_sig = encode_type_signature(&TypeSignature::Int).unwrap();
    let empty_body = || MethodBody {
        register_types: vec![],
        code: vec![],
        debug_locals: vec![],
        source_spans: vec![],
    };

    let mut builder = ModuleBuilder::new("param-names");
    let owner = builder.add_type_def("Widget", "", TypeDefKind::Class, 0);
    let implementation = builder.add_impl_def(owner, writ_module::MetadataToken::NULL);
    builder.add_impl_method(implementation, "first", &void_sig, 0, 0, empty_body());
    builder.add_impl_method(implementation, "second", &void_sig, 0, 0, empty_body());
    builder.add_method("consume", &one_int_sig, 0, 1, empty_body());
    builder.add_param_def("value", &int_sig, 0);

    let text = writ_assembler::disassemble(&builder.build());

    assert!(text.contains(".method \"first\" () -> void"), "{text}");
    assert!(text.contains(".method \"second\" () -> void"), "{text}");
    assert!(
        text.contains(".method \"consume\" (value: int) -> void"),
        "{text}"
    );
}

#[test]
fn disassemble_impl_block() {
    let src = r#"
.module "test" "1.0.0" {
    .type "MyStruct" struct {
        .field "x" int pub
    }
    .contract "IFoo" {
        .method "do_thing" (int) -> void slot 0
    }
    .impl MyStruct : IFoo {
        .method "do_thing" (r0 int) -> void {
            .reg r0 int
            NOP
            RET_VOID
        }
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".impl"),
        "output should contain .impl directive"
    );
    assert!(
        text.contains("MyStruct"),
        "output should contain type name in impl"
    );
    assert!(
        text.contains("IFoo"),
        "output should contain contract name in impl"
    );
    assert!(text.contains(':'), "output should contain : separator");
}

#[test]
fn disassemble_verbose_includes_offsets() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        NOP
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble_verbose(&module);
    assert!(
        text.contains("// +0x"),
        "verbose output should contain hex offset comments"
    );
}

#[test]
fn disassemble_module_with_extern_ref() {
    let src = r#"
.module "test" "1.0.0" {
    .extern "MyLib" "2.0.0"
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(
        text.contains(".extern"),
        "output should contain .extern directive"
    );
    assert!(
        text.contains("\"MyLib\""),
        "output should contain module ref name"
    );
    assert!(
        text.contains("\"2.0.0\""),
        "output should contain min version"
    );
}

#[test]
fn disassemble_type_signatures() {
    let src = r#"
.module "test" "1.0.0" {
    .type "Container" struct {
        .field "value" int pub
        .field "name" string pub
        .field "flag" bool pub
        .field "ratio" float pub
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(text.contains("int"), "should contain int type");
    assert!(text.contains("string"), "should contain string type");
    assert!(text.contains("bool"), "should contain bool type");
    assert!(text.contains("float"), "should contain float type");
}

#[test]
fn disassemble_all_control_flow() {
    let src = r#"
.module "test" "1.0.0" {
    .method "test_branches" () -> void {
        .reg r0 bool
        .reg r1 bool
        LOAD_TRUE r0
        BR_TRUE r0, 4
        LOAD_FALSE r1
        BR_FALSE r1, -2
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(text.contains("BR_TRUE"), "should contain BR_TRUE mnemonic");
    assert!(
        text.contains("BR_FALSE"),
        "should contain BR_FALSE mnemonic"
    );
    assert!(
        text.contains("LOAD_TRUE"),
        "should contain LOAD_TRUE mnemonic"
    );
}

#[test]
fn disassemble_reg_declarations() {
    let src = r#"
.module "test" "1.0.0" {
    .method "compute" () -> int {
        .reg r0 int
        .reg r1 int
        .reg r2 int
        LOAD_INT r0, 10
        LOAD_INT r1, 20
        ADD_I r2, r0, r1
        RET r2
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(text.contains(".reg"), "should contain .reg declarations");
    assert!(text.contains("ADD_I"), "should contain ADD_I mnemonic");
    assert!(text.contains("r2"), "should contain register r2");
}
