/// Disassembler round-trip tests: assemble -> disassemble -> reassemble -> compare.
///
/// Each test: assemble text -> Module (m1) -> disassemble -> text2 -> assemble -> Module (m2)
/// Then assert m1 and m2 have the same table counts.

fn round_trip(src: &str) -> (writ_module::Module, writ_module::Module, String) {
    let m1 = writ_assembler::assemble(src).unwrap_or_else(|errs| {
        panic!("Initial assembly failed: {:?}", errs);
    });
    let text = writ_assembler::disassemble(&m1);
    let m2 = writ_assembler::assemble(&text).unwrap_or_else(|errs| {
        panic!(
            "Reassembly of disassembled text failed.\nErrors: {:?}\nDisassembled text:\n{}",
            errs, text
        );
    });
    (m1, m2, text)
}

#[test]
fn round_trip_minimal_module() {
    let src = r#"
.module "test" "1.0.0" {
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.type_defs.len(), m2.type_defs.len(), "type_defs count must match");
    assert_eq!(m1.method_defs.len(), m2.method_defs.len(), "method_defs count must match");
    assert_eq!(m1.contract_defs.len(), m2.contract_defs.len(), "contract_defs count must match");
}

#[test]
fn round_trip_type_with_fields() {
    let src = r#"
.module "test" "1.0.0" {
    .type "Player" struct {
        .field "name" string pub
        .field "health" int pub
    }
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.type_defs.len(), m2.type_defs.len(), "type_defs count must match");
    assert_eq!(m1.field_defs.len(), m2.field_defs.len(), "field_defs count must match");
}

#[test]
fn round_trip_method_with_body() {
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
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.method_defs.len(), m2.method_defs.len(), "method_defs count must match");
    // Verify the method body is also preserved
    assert!(!m2.method_bodies.is_empty(), "method bodies should exist");
    assert!(!m2.method_bodies[0].code.is_empty(), "method body code should be non-empty");
}

#[test]
fn round_trip_complex_module() {
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
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.type_defs.len(), m2.type_defs.len(), "type_defs count must match");
    assert_eq!(m1.field_defs.len(), m2.field_defs.len(), "field_defs count must match");
    assert_eq!(m1.contract_defs.len(), m2.contract_defs.len(), "contract_defs count must match");
    assert_eq!(m1.contract_methods.len(), m2.contract_methods.len(), "contract_methods count must match");
    assert_eq!(m1.impl_defs.len(), m2.impl_defs.len(), "impl_defs count must match");
    assert_eq!(m1.method_defs.len(), m2.method_defs.len(), "method_defs count must match");
}

#[test]
fn round_trip_contract_with_generic() {
    let src = r#"
.module "test" "1.0.0" {
    .contract "IComparable" <T> {
        .method "compare" (int) -> int slot 0
    }
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.contract_defs.len(), m2.contract_defs.len(), "contract_defs must match");
    assert_eq!(m1.contract_methods.len(), m2.contract_methods.len(), "contract_methods must match");
    assert_eq!(m1.generic_params.len(), m2.generic_params.len(), "generic_params must match");
}

#[test]
fn round_trip_entity_type() {
    let src = r#"
.module "test" "1.0.0" {
    .type "NPC" entity {
        .field "name" string pub
    }
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.type_defs.len(), m2.type_defs.len(), "type_defs count must match");
    assert_eq!(m1.field_defs.len(), m2.field_defs.len(), "field_defs count must match");
}

#[test]
fn round_trip_module_with_extern_ref() {
    let src = r#"
.module "test" "1.0.0" {
    .extern "MyLib" "2.0.0"
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.module_refs.len(), m2.module_refs.len(), "module_refs count must match");
}

#[test]
fn round_trip_branch_instructions() {
    let src = r#"
.module "test" "1.0.0" {
    .method "loop" () -> void {
        .reg r0 bool
        LOAD_TRUE r0
        BR -2
        RET_VOID
    }
}
"#;
    let (m1, m2, text) = round_trip(src);
    assert_eq!(m1.method_defs.len(), m2.method_defs.len(), "method_defs count must match");
    // Verify BR instruction is present in output
    assert!(text.contains("BR"), "disassembled text should contain BR");
}

#[test]
fn round_trip_global_def() {
    let src = r#"
.module "test" "1.0.0" {
    .global "counter" int pub
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.global_defs.len(), m2.global_defs.len(), "global_defs count must match");
}

#[test]
fn round_trip_multiple_types_and_impls() {
    let src = r#"
.module "test" "1.0.0" {
    .type "TypeA" struct {
        .field "val" int pub
    }
    .type "TypeB" struct {
        .field "name" string pub
        .field "active" bool pub
    }
    .contract "ISerializable" {
        .method "serialize" () -> void slot 0
    }
    .impl TypeA : ISerializable {
        .method "serialize" () -> void {
            NOP
            RET_VOID
        }
    }
    .method "init" () -> void {
        RET_VOID
    }
}
"#;
    let (m1, m2, _) = round_trip(src);
    assert_eq!(m1.type_defs.len(), m2.type_defs.len(), "type_defs count must match");
    assert_eq!(m1.field_defs.len(), m2.field_defs.len(), "field_defs count must match");
    assert_eq!(m1.contract_defs.len(), m2.contract_defs.len(), "contract_defs count must match");
    assert_eq!(m1.impl_defs.len(), m2.impl_defs.len(), "impl_defs count must match");
    assert_eq!(m1.method_defs.len(), m2.method_defs.len(), "method_defs count must match");
}
