/// ASM-01/ASM-02/ASM-03/ASM-04 tests: assembled binary -> Module::from_bytes() -> no error.
/// Also covers .class directive round-trip (kind=4 TypeDef), all 5 new directives, and
/// register type blob offset correctness.

#[test]
fn assembled_binary_is_valid() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        NOP
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let bytes = module.to_bytes().expect("should encode to bytes");
    let _reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode from bytes");
}

#[test]
fn round_trip_preserves_structure() {
    let src = r#"
.module "game" "2.0.0" {
    .type "Player" struct {
        .field "name" string pub
        .field "health" int pub
    }
    .contract "IUpdatable" {
        .method "update" () -> void slot 0
    }
    .impl Player : IUpdatable {
        .method "update" () -> void {
            NOP
            RET_VOID
        }
    }
    .method "main" () -> int {
        .reg r0 int
        LOAD_INT r0, 0
        RET r0
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");

    assert_eq!(reloaded.type_defs.len(), 1, "1 TypeDef preserved");
    assert_eq!(reloaded.field_defs.len(), 2, "2 FieldDefs preserved");
    assert_eq!(reloaded.contract_defs.len(), 1, "1 ContractDef preserved");
    assert_eq!(reloaded.contract_methods.len(), 1, "1 ContractMethod preserved");
    assert_eq!(reloaded.impl_defs.len(), 1, "1 ImplDef preserved");
    assert_eq!(reloaded.method_defs.len(), 2, "2 MethodDefs preserved");
}

#[test]
fn round_trip_method_body_intact() {
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
    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");

    // Decode instructions from the reloaded method body
    let code = &reloaded.method_bodies[0].code;
    let mut cursor = std::io::Cursor::new(code.as_slice());
    let mut instrs = Vec::new();
    while (cursor.position() as usize) < code.len() {
        instrs.push(writ_module::Instruction::decode(&mut cursor).expect("decode ok"));
    }

    assert_eq!(instrs.len(), 4, "LOAD_INT + LOAD_INT + ADD_I + RET");

    // Verify instruction types
    assert!(matches!(instrs[0], writ_module::Instruction::LoadInt { r_dst: 0, value: 10 }));
    assert!(matches!(instrs[1], writ_module::Instruction::LoadInt { r_dst: 1, value: 20 }));
    assert!(matches!(instrs[2], writ_module::Instruction::AddI { r_dst: 2, r_a: 0, r_b: 1 }));
    assert!(matches!(instrs[3], writ_module::Instruction::Ret { r_src: 2 }));
}

#[test]
fn round_trip_with_labels() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        BR .end
        NOP
        .end:
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");

    // Verify the branch offset survived the round-trip
    let code = &reloaded.method_bodies[0].code;
    let mut cursor = std::io::Cursor::new(code.as_slice());
    let first = writ_module::Instruction::decode(&mut cursor).expect("decode ok");
    if let writ_module::Instruction::Br { offset } = first {
        assert_eq!(offset, 2, "forward branch offset preserved through round-trip");
    } else {
        panic!("expected BR instruction after round-trip");
    }
}

// ── New directive round-trip tests (ASM-01) ──────────────────────────────────

#[test]
fn round_trip_export() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        RET_VOID
    }
    .export "main" method 1
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.export_defs.len(), 1, "1 export def");
    assert_eq!(module.export_defs[0].item_kind, 0, "item_kind=0 (method)");

    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");
    assert_eq!(reloaded.export_defs.len(), 1, "1 export def after round-trip");

    let text = writ_assembler::disassemble(&reloaded);
    assert!(text.contains(".export \"main\" method"), "disassembled output contains .export \"main\" method, got:\n{}", text);
}

#[test]
fn round_trip_extern_fn() {
    let src = r#"
.module "test" "1.0.0" {
    .extern_fn "print" (string) -> void "host_print"
    .method "main" () -> void {
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert!(module.extern_defs.len() >= 1, "at least 1 extern def");

    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");

    let text = writ_assembler::disassemble(&reloaded);
    assert!(text.contains(".extern_fn \"print\""), "disassembled output contains .extern_fn \"print\", got:\n{}", text);
}

#[test]
fn round_trip_component_slot() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        RET_VOID
    }
    .component_slot 1 2
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.component_slots.len(), 1, "1 component slot");

    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");
    assert_eq!(reloaded.component_slots.len(), 1, "1 component slot after round-trip");

    let text = writ_assembler::disassemble(&reloaded);
    assert!(text.contains(".component_slot 1 2"), "disassembled output contains .component_slot 1 2, got:\n{}", text);
}

#[test]
fn round_trip_locale() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        RET_VOID
    }
    .locale 1 "en-US" 2
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.locale_defs.len(), 1, "1 locale def");

    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");
    assert_eq!(reloaded.locale_defs.len(), 1, "1 locale def after round-trip");

    let text = writ_assembler::disassemble(&reloaded);
    assert!(text.contains(".locale 1 \"en-US\" 2"), "disassembled output contains .locale 1 \"en-US\" 2, got:\n{}", text);
}

#[test]
fn round_trip_attribute() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        RET_VOID
    }
    .attribute 1 3 "deprecated"
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.attribute_defs.len(), 1, "1 attribute def");
    assert_eq!(module.attribute_defs[0].owner_kind, 3, "owner_kind=3");

    let bytes = module.to_bytes().expect("should encode");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode");
    assert_eq!(reloaded.attribute_defs.len(), 1, "1 attribute def after round-trip");

    let text = writ_assembler::disassemble(&reloaded);
    assert!(text.contains(".attribute 1 3 \"deprecated\""), "disassembled output contains .attribute 1 3 \"deprecated\", got:\n{}", text);
}

#[test]
fn round_trip_all_new_directives() {
    let src = r#"
.module "test" "1.0.0" {
    .extern_fn "print" (string) -> void "host_print"
    .method "main" () -> void {
        RET_VOID
    }
    .export "main" method 1
    .component_slot 1 2
    .locale 1 "en-US" 2
    .attribute 1 3 "deprecated"
}
"#;
    let module1 = writ_assembler::assemble(src).expect("first assemble");

    assert!(module1.extern_defs.len() >= 1, "extern_defs present");
    assert_eq!(module1.export_defs.len(), 1, "1 export def");
    assert_eq!(module1.component_slots.len(), 1, "1 component slot");
    assert_eq!(module1.locale_defs.len(), 1, "1 locale def");
    assert_eq!(module1.attribute_defs.len(), 1, "1 attribute def");

    // disassemble -> re-assemble -> check structural equivalence
    let text = writ_assembler::disassemble(&module1);
    let module2 = writ_assembler::assemble(&text).expect("re-assemble from disassembly");

    assert_eq!(module1.extern_defs.len(), module2.extern_defs.len(), "extern_defs count preserved");
    assert_eq!(module1.export_defs.len(), module2.export_defs.len(), "export_defs count preserved");
    assert_eq!(module1.component_slots.len(), module2.component_slots.len(), "component_slots count preserved");
    assert_eq!(module1.locale_defs.len(), module2.locale_defs.len(), "locale_defs count preserved");
    assert_eq!(module1.attribute_defs.len(), module2.attribute_defs.len(), "attribute_defs count preserved");
    assert_eq!(module1.method_defs.len(), module2.method_defs.len(), "method_defs count preserved");
}

#[test]
fn register_types_real_offsets() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        .reg r1 string
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.method_bodies.len(), 1, "1 method body");
    let body = &module.method_bodies[0];
    assert_eq!(body.register_types.len(), 2, "2 register types");
    assert_ne!(body.register_types[0], 0, "r0 (int) blob offset should be non-zero");
    assert_ne!(body.register_types[1], 0, "r1 (string) blob offset should be non-zero");
    assert_ne!(body.register_types[0], body.register_types[1], "int and string encode differently");

    // Verify blob contents are valid type signatures
    let int_blob = writ_module::heap::read_blob(&module.blob_heap, body.register_types[0])
        .expect("should read int type blob");
    assert_eq!(int_blob, &[0x01], "int type encodes as 0x01");

    let str_blob = writ_module::heap::read_blob(&module.blob_heap, body.register_types[1])
        .expect("should read string type blob");
    assert_eq!(str_blob, &[0x04], "string type encodes as 0x04");
}

// ── Pre-existing class round-trip test ───────────────────────────────────────

#[test]
fn test_class_round_trip() {
    // Assemble a .writil source containing a .class type definition,
    // then disassemble back to text and confirm the output contains ".class MyClass".
    let src = r#"
.module "test_class" "1.0.0" {
    .type "MyClass" class {
        .field "value" int pub
    }
    .method "main" () -> void {
        RET_VOID
    }
}
"#;
    // Parse + assemble -> binary
    let module = writ_assembler::assemble(src).expect("should assemble .class directive");

    // Verify the TypeDef was encoded with kind=4 (Class)
    assert_eq!(module.type_defs.len(), 1, "expected 1 TypeDef");
    assert_eq!(module.type_defs[0].kind, 4, "expected kind=4 (Class)");

    // Round-trip: binary -> bytes -> Module
    let bytes = module.to_bytes().expect("should encode to bytes");
    let reloaded = writ_module::Module::from_bytes(&bytes).expect("should decode from bytes");
    assert_eq!(reloaded.type_defs[0].kind, 4, "kind=4 preserved through binary round-trip");

    // Disassemble and confirm "class" appears in output
    let text = writ_assembler::disassemble(&reloaded);
    assert!(
        text.contains(".type \"MyClass\" class"),
        "disassembled output should contain '.type \"MyClass\" class', got:\n{}",
        text
    );
}
