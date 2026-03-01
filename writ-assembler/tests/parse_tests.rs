use writ_assembler::ast::*;
use writ_assembler::lexer::tokenize;
use writ_assembler::parser::parse;

fn parse_str(src: &str) -> AsmModule {
    let tokens = tokenize(src).expect("tokenize failed");
    parse(&tokens).expect("parse failed")
}

fn parse_str_err(src: &str) -> Vec<writ_assembler::AssembleError> {
    let tokens = match tokenize(src) {
        Ok(t) => t,
        Err(e) => return e,
    };
    match parse(&tokens) {
        Ok(_) => panic!("expected parse error"),
        Err(e) => e,
    }
}

// ── Module parsing ──────────────────────────────────────────

#[test]
fn parse_empty_module() {
    let m = parse_str(r#".module "test" "1.0.0" { }"#);
    assert_eq!(m.name, "test");
    assert_eq!(m.version, "1.0.0");
    assert!(m.types.is_empty());
    assert!(m.contracts.is_empty());
    assert!(m.impls.is_empty());
    assert!(m.methods.is_empty());
}

#[test]
fn parse_module_with_newlines() {
    let m = parse_str(
        r#"
.module "test" "1.0.0" {
}
"#,
    );
    assert_eq!(m.name, "test");
    assert_eq!(m.version, "1.0.0");
}

// ── Type parsing ────────────────────────────────────────────

#[test]
fn parse_type_with_fields() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "MyStruct" struct {
        .field "x" int pub
        .field "y" float pub
    }
}"#,
    );
    assert_eq!(m.types.len(), 1);
    let t = &m.types[0];
    assert_eq!(t.name, "MyStruct");
    assert_eq!(t.kind, AsmTypeKind::Struct);
    assert_eq!(t.fields.len(), 2);
    assert_eq!(t.fields[0].name, "x");
    assert_eq!(t.fields[0].type_ref, AsmTypeRef::Int);
    assert_eq!(t.fields[0].flags, 0x0001); // pub
    assert_eq!(t.fields[1].name, "y");
    assert_eq!(t.fields[1].type_ref, AsmTypeRef::Float);
}

#[test]
fn parse_type_kinds() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "S" struct { }
    .type "E" enum { }
    .type "N" entity { }
    .type "C" component { }
}"#,
    );
    assert_eq!(m.types.len(), 4);
    assert_eq!(m.types[0].kind, AsmTypeKind::Struct);
    assert_eq!(m.types[1].kind, AsmTypeKind::Enum);
    assert_eq!(m.types[2].kind, AsmTypeKind::Entity);
    assert_eq!(m.types[3].kind, AsmTypeKind::Component);
}

// ── Contract parsing ────────────────────────────────────────

#[test]
fn parse_contract_with_methods() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .contract "IFoo" {
        .method "foo" (int) -> void slot 0
        .method "bar" (int, float) -> bool slot 1
    }
}"#,
    );
    assert_eq!(m.contracts.len(), 1);
    let c = &m.contracts[0];
    assert_eq!(c.name, "IFoo");
    assert_eq!(c.methods.len(), 2);
    assert_eq!(c.methods[0].name, "foo");
    assert_eq!(c.methods[0].slot, 0);
    assert_eq!(c.methods[0].signature.params.len(), 1);
    assert_eq!(c.methods[0].signature.params[0], AsmTypeRef::Int);
    assert_eq!(c.methods[0].signature.return_type, AsmTypeRef::Void);
    assert_eq!(c.methods[1].name, "bar");
    assert_eq!(c.methods[1].slot, 1);
    assert_eq!(c.methods[1].signature.params.len(), 2);
    assert_eq!(c.methods[1].signature.return_type, AsmTypeRef::Bool);
}

#[test]
fn parse_contract_with_generic_params() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .contract "Into" <T> {
        .method "into" () -> void slot 0
    }
}"#,
    );
    assert_eq!(m.contracts[0].generic_params, vec!["T"]);
}

// ── Impl parsing ────────────────────────────────────────────

#[test]
fn parse_impl_block() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .impl MyStruct : IFoo {
        .method "foo" (r0 int) -> void {
            .reg r0 int
            NOP
            RET_VOID
        }
    }
}"#,
    );
    assert_eq!(m.impls.len(), 1);
    let i = &m.impls[0];
    assert_eq!(i.type_name, "MyStruct");
    assert_eq!(i.contract_name, "IFoo");
    assert_eq!(i.methods.len(), 1);
    assert_eq!(i.methods[0].name, "foo");
}

// ── Method parsing ──────────────────────────────────────────

#[test]
fn parse_method_with_registers_and_instructions() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "main" () -> int {
        .reg r0 int
        LOAD_INT r0, 42
        RET r0
    }
}"#,
    );
    assert_eq!(m.methods.len(), 1);
    let method = &m.methods[0];
    assert_eq!(method.name, "main");
    assert_eq!(method.return_type, AsmTypeRef::Int);
    assert_eq!(method.registers.len(), 1);
    assert_eq!(method.registers[0].index, 0);
    assert_eq!(method.registers[0].type_ref, AsmTypeRef::Int);
    assert_eq!(method.body.len(), 2);

    // Check LOAD_INT instruction
    if let AsmStatement::Instruction(instr) = &method.body[0] {
        assert_eq!(instr.mnemonic, "LOAD_INT");
        assert_eq!(instr.operands.len(), 2);
        assert!(matches!(instr.operands[0], AsmOperand::Register(0)));
        assert!(matches!(instr.operands[1], AsmOperand::IntLit(42)));
    } else {
        panic!("expected instruction");
    }

    // Check RET instruction
    if let AsmStatement::Instruction(instr) = &method.body[1] {
        assert_eq!(instr.mnemonic, "RET");
        assert_eq!(instr.operands.len(), 1);
        assert!(matches!(instr.operands[0], AsmOperand::Register(0)));
    } else {
        panic!("expected instruction");
    }
}

#[test]
fn parse_method_with_params() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "add" (r0 int, r1 int) -> int {
        .reg r0 int
        .reg r1 int
        .reg r2 int
        ADD_I r2, r0, r1
        RET r2
    }
}"#,
    );
    let method = &m.methods[0];
    assert_eq!(method.params.len(), 2);
    assert_eq!(method.params[0].name, "r0");
    assert_eq!(method.params[0].type_ref, AsmTypeRef::Int);
    assert_eq!(method.params[1].name, "r1");
    assert_eq!(method.params[1].type_ref, AsmTypeRef::Int);
}

// ── Label parsing ───────────────────────────────────────────

#[test]
fn parse_labels_in_method() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "loop" () -> void {
        .reg r0 int
        .reg r1 bool
        .top:
        ADD_I r0, r0, r0
        BR_FALSE r1, .done
        BR .top
        .done:
        RET_VOID
    }
}"#,
    );
    let method = &m.methods[0];
    let labels: Vec<&String> = method
        .body
        .iter()
        .filter_map(|s| match s {
            AsmStatement::Label(name) => Some(name),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec!["top", "done"]);

    // Check label references in instructions
    let instrs: Vec<&AsmInstruction> = method
        .body
        .iter()
        .filter_map(|s| match s {
            AsmStatement::Instruction(i) => Some(i),
            _ => None,
        })
        .collect();
    // BR_FALSE r1, .done
    assert!(matches!(&instrs[1].operands[1], AsmOperand::LabelRef(name) if name == "done"));
    // BR .top
    assert!(matches!(&instrs[2].operands[0], AsmOperand::LabelRef(name) if name == "top"));
}

// ── Comments ────────────────────────────────────────────────

#[test]
fn parse_with_comments() {
    let m = parse_str(
        r#"
// This is a module
.module "test" "1.0.0" {
    // A simple method
    .method "main" () -> void {
        NOP // do nothing
        RET_VOID
    }
}
"#,
    );
    assert_eq!(m.methods.len(), 1);
    assert_eq!(m.methods[0].body.len(), 2);
}

// ── Type references ─────────────────────────────────────────

#[test]
fn parse_primitive_type_refs() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "Test" struct {
        .field "a" int
        .field "b" float
        .field "c" bool
        .field "d" string
    }
}"#,
    );
    let fields = &m.types[0].fields;
    assert_eq!(fields[0].type_ref, AsmTypeRef::Int);
    assert_eq!(fields[1].type_ref, AsmTypeRef::Float);
    assert_eq!(fields[2].type_ref, AsmTypeRef::Bool);
    assert_eq!(fields[3].type_ref, AsmTypeRef::String_);
}

#[test]
fn parse_array_type_ref() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "Test" struct {
        .field "arr" Array<int>
    }
}"#,
    );
    assert_eq!(
        m.types[0].fields[0].type_ref,
        AsmTypeRef::Array(Box::new(AsmTypeRef::Int))
    );
}

#[test]
fn parse_generic_type_ref() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "Test" struct {
        .field "opt" Option<int>
    }
}"#,
    );
    assert_eq!(
        m.types[0].fields[0].type_ref,
        AsmTypeRef::Generic("Option".to_string(), vec![AsmTypeRef::Int])
    );
}

#[test]
fn parse_named_type_ref() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .type "Test" struct {
        .field "other" MyType
    }
}"#,
    );
    assert_eq!(
        m.types[0].fields[0].type_ref,
        AsmTypeRef::Named("MyType".to_string())
    );
}

// ── Method references ───────────────────────────────────────

#[test]
fn parse_qualified_method_ref() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        CALL r0, MyType::my_method, r0, 1
    }
}"#,
    );
    let instr = match &m.methods[0].body[0] {
        AsmStatement::Instruction(i) => i,
        _ => panic!("expected instruction"),
    };
    assert_eq!(instr.mnemonic, "CALL");
    if let AsmOperand::MethodRef(mref) = &instr.operands[1] {
        assert_eq!(mref.type_name.as_deref(), Some("MyType"));
        assert_eq!(mref.method_name, "my_method");
        assert!(mref.module_name.is_none());
    } else {
        panic!("expected MethodRef operand");
    }
}

#[test]
fn parse_cross_module_method_ref() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        CALL r0, [OtherModule]Type::method, r0, 0
    }
}"#,
    );
    let instr = match &m.methods[0].body[0] {
        AsmStatement::Instruction(i) => i,
        _ => panic!("expected instruction"),
    };
    if let AsmOperand::MethodRef(mref) = &instr.operands[1] {
        assert_eq!(mref.module_name.as_deref(), Some("OtherModule"));
        assert_eq!(mref.type_name.as_deref(), Some("Type"));
        assert_eq!(mref.method_name, "method");
    } else {
        panic!("expected MethodRef operand");
    }
}

// ── Register notation ───────────────────────────────────────

#[test]
fn parse_register_operands() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        .reg r1 int
        MOV r0, r1
    }
}"#,
    );
    let instr = match &m.methods[0].body[0] {
        AsmStatement::Instruction(i) => i,
        _ => panic!("expected instruction"),
    };
    assert!(matches!(instr.operands[0], AsmOperand::Register(0)));
    assert!(matches!(instr.operands[1], AsmOperand::Register(1)));
}

// ── Extern and global ───────────────────────────────────────

#[test]
fn parse_extern_directive() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .extern "OtherModule" ">=1.0.0"
}"#,
    );
    assert_eq!(m.externs.len(), 1);
    assert_eq!(m.externs[0].name, "OtherModule");
    assert_eq!(m.externs[0].min_version, ">=1.0.0");
}

#[test]
fn parse_global_directive() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .global "counter" int pub
}"#,
    );
    assert_eq!(m.globals.len(), 1);
    assert_eq!(m.globals[0].name, "counter");
    assert_eq!(m.globals[0].type_ref, AsmTypeRef::Int);
    assert_eq!(m.globals[0].flags, 0x0001);
}

// ── Multi-error collection ──────────────────────────────────

#[test]
fn parse_multiple_errors_collected() {
    // Use known directives in wrong contexts and bad tokens to trigger multiple errors
    let errors = parse_str_err(
        r#".module "test" "1.0.0" {
    .type "Bad" struct {
        BADTOKEN1
    }
    .type "Bad2" struct {
        BADTOKEN2
    }
}"#,
    );
    // Should collect multiple errors, not just the first
    assert!(
        errors.len() >= 2,
        "expected at least 2 errors, got {}: {:?}",
        errors.len(),
        errors
    );
}

#[test]
fn error_messages_have_line_and_column() {
    let errors = parse_str_err(r#".module "test" "1.0.0" { BADTOKEN }"#);
    assert!(!errors.is_empty());
    for err in &errors {
        assert!(err.line > 0, "error should have line > 0");
        assert!(err.col > 0, "error should have col > 0");
        // Verify the Display format
        let msg = format!("{}", err);
        assert!(
            msg.contains("at line"),
            "error format should contain 'at line': {}",
            msg
        );
    }
}

// ── Case-insensitive instruction mnemonics ──────────────────

#[test]
fn parse_case_insensitive_mnemonics() {
    let m = parse_str(
        r#".module "test" "1.0.0" {
    .method "main" () -> void {
        nop
        RET_VOID
    }
}"#,
    );
    let method = &m.methods[0];
    assert_eq!(method.body.len(), 2);
    if let AsmStatement::Instruction(i) = &method.body[0] {
        // Mnemonic is stored as-is; case-insensitive matching happens in assembler
        assert_eq!(i.mnemonic, "nop");
    }
}

// ── Comprehensive module ────────────────────────────────────

#[test]
fn parse_comprehensive_module() {
    let m = parse_str(
        r#".module "game" "2.0.0" {
    .extern "writ_runtime" ">=1.0.0"

    .type "Player" struct {
        .field "name" string pub
        .field "health" int pub mut
    }

    .contract "IUpdatable" {
        .method "update" (float) -> void slot 0
    }

    .impl Player : IUpdatable {
        .method "update" (r0 float) -> void {
            .reg r0 float
            NOP
            RET_VOID
        }
    }

    .global "score" int pub

    .method "main" () -> int {
        .reg r0 int
        LOAD_INT r0, 0
        RET r0
    }
}"#,
    );

    assert_eq!(m.name, "game");
    assert_eq!(m.version, "2.0.0");
    assert_eq!(m.externs.len(), 1);
    assert_eq!(m.types.len(), 1);
    assert_eq!(m.contracts.len(), 1);
    assert_eq!(m.impls.len(), 1);
    assert_eq!(m.globals.len(), 1);
    assert_eq!(m.methods.len(), 1);
}
