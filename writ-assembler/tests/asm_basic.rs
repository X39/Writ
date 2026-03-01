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
