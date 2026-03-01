/// ASM-03/ASM-04 tests: assembled binary -> Module::from_bytes() -> no error.

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
