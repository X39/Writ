/// ASM-01 integration tests: assemble .writil with all directives.

#[test]
fn assemble_minimal_module() {
    let src = r#"
.module "test" "1.0.0" {
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.type_defs.len(), 0);
    assert_eq!(module.method_defs.len(), 0);
    assert_eq!(module.contract_defs.len(), 0);
}

#[test]
fn assemble_type_with_fields() {
    let src = r#"
.module "test" "1.0.0" {
    .type "MyStruct" struct {
        .field "x" int pub
        .field "y" float pub
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.type_defs.len(), 1);
    assert_eq!(module.field_defs.len(), 2);
}

#[test]
fn assemble_contract_with_methods() {
    let src = r#"
.module "test" "1.0.0" {
    .contract "IFoo" {
        .method "do_thing" (int) -> void slot 0
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.contract_defs.len(), 1);
    assert_eq!(module.contract_methods.len(), 1);
}

#[test]
fn assemble_method_with_nop_ret() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        NOP
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert_eq!(module.method_defs.len(), 1);
    // Method body should have code bytes
    assert!(!module.method_bodies[0].code.is_empty());
}

#[test]
fn assemble_method_with_registers() {
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
    assert_eq!(module.method_defs.len(), 1);
    assert!(!module.method_bodies[0].code.is_empty());
}

#[test]
fn assemble_impl_block() {
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
    assert_eq!(module.type_defs.len(), 1);
    assert_eq!(module.contract_defs.len(), 1);
    assert_eq!(module.impl_defs.len(), 1);
    assert_eq!(module.method_defs.len(), 1);
}

#[test]
fn assemble_full_module() {
    let src = r#"
.module "game" "1.0.0" {
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
    assert_eq!(module.type_defs.len(), 1, "1 TypeDef");
    assert_eq!(module.field_defs.len(), 2, "2 FieldDefs");
    assert_eq!(module.contract_defs.len(), 1, "1 ContractDef");
    assert_eq!(module.contract_methods.len(), 1, "1 ContractMethod");
    assert_eq!(module.impl_defs.len(), 1, "1 ImplDef");
    assert_eq!(module.method_defs.len(), 2, "2 MethodDefs (impl + global)");
}

#[test]
fn test_typeof_assembles() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        TYPEOF r0, 1
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).unwrap();
    assert_eq!(module.method_bodies.len(), 1);
    // TYPEOF is 8 bytes (RI32 shape) + RET_VOID is 2 bytes = 10 bytes
    assert_eq!(module.method_bodies[0].code.len(), 10);
}

#[test]
fn field_operands_assemble_as_metadata_tokens() {
    use std::io::Cursor;
    use writ_module::Instruction;

    let src = r#"
.module "test" "1.0.0" {
    .type "First" struct {
        .field "first" int pub
    }
    .type "Second" struct {
        .field "second_0" int pub
        .field "second_1" int pub
    }
    .method "main" () -> void {
        .reg r0 int
        .reg r1 int
        GET_FIELD r0, r1, Second::second_1
        SET_FIELD r1, token(83886083), r0
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble field tokens");
    let mut cursor = Cursor::new(module.method_bodies[0].code.as_slice());

    assert_eq!(
        Instruction::decode(&mut cursor).unwrap(),
        Instruction::GetField {
            r_dst: 0,
            r_obj: 1,
            field_token: 0x0500_0003,
        }
    );
    assert_eq!(
        Instruction::decode(&mut cursor).unwrap(),
        Instruction::SetField {
            r_obj: 1,
            field_token: 0x0500_0003,
            r_val: 0,
        }
    );
}

#[test]
fn atomic_construction_operands_assemble_in_wire_order() {
    use std::io::Cursor;
    use writ_module::Instruction;

    let src = r#"
.module "test" "1.0.0" {
    .type "Thing" struct {
        .field "left" int
        .field "right" int
    }
    .type "Actor" entity {
    }
    .method "main" () -> void {
        .reg r0 int
        .reg r1 int
        .reg r2 int
        .reg r3 int
        NEW r0, Thing, 2, r1
        SPAWN_ENTITY r1, Actor, 0, r3
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble atomic constructors");
    let code = &module.method_bodies[0].code;
    assert_eq!(code.len(), 26, "two 12-byte constructors plus RET_VOID");
    let mut cursor = Cursor::new(code.as_slice());

    assert_eq!(
        Instruction::decode(&mut cursor).unwrap(),
        Instruction::New {
            r_dst: 0,
            type_idx: 0x0200_0001,
            field_count: 2,
            r_base: 1,
        }
    );
    assert_eq!(
        Instruction::decode(&mut cursor).unwrap(),
        Instruction::SpawnEntity {
            r_dst: 1,
            type_idx: 0x0200_0002,
            field_count: 0,
            r_base: 3,
        }
    );
}
