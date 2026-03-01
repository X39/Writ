/// Disassembler unit tests: verify output format of disassemble() and disassemble_verbose().

#[test]
fn disassemble_empty_module() {
    let src = r#"
.module "test" "1.0.0" {
}
"#;
    let module = writ_assembler::assemble(src).expect("should assemble");
    let text = writ_assembler::disassemble(&module);
    assert!(text.contains(".module"), "output should contain .module directive");
    assert!(text.contains("\"test\""), "output should contain module name");
    assert!(text.contains("\"1.0.0\""), "output should contain module version");
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
    assert!(text.contains(".type"), "output should contain .type directive");
    assert!(text.contains("\"MyStruct\""), "output should contain type name");
    assert!(text.contains("struct"), "output should contain kind");
    assert!(text.contains(".field"), "output should contain .field directive");
    assert!(text.contains("\"x\""), "output should contain field name x");
    assert!(text.contains("\"y\""), "output should contain field name y");
    assert!(text.contains("int"), "output should contain int type");
    assert!(text.contains("float"), "output should contain float type");
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
    assert!(text.contains(".contract"), "output should contain .contract directive");
    assert!(text.contains("\"IFoo\""), "output should contain contract name");
    assert!(text.contains("\"do_thing\""), "output should contain method name");
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
    assert!(text.contains(".method"), "output should contain .method directive");
    assert!(text.contains("\"main\""), "output should contain method name");
    assert!(text.contains("LOAD_INT"), "output should contain LOAD_INT mnemonic");
    assert!(text.contains("42"), "output should contain integer value");
    assert!(text.contains("RET"), "output should contain RET mnemonic");
    assert!(text.contains("r0"), "output should contain register r0");
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
    assert!(text.contains(".impl"), "output should contain .impl directive");
    assert!(text.contains("MyStruct"), "output should contain type name in impl");
    assert!(text.contains("IFoo"), "output should contain contract name in impl");
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
    assert!(text.contains("// +0x"), "verbose output should contain hex offset comments");
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
    assert!(text.contains(".extern"), "output should contain .extern directive");
    assert!(text.contains("\"MyLib\""), "output should contain module ref name");
    assert!(text.contains("\"2.0.0\""), "output should contain min version");
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
    assert!(text.contains("BR_FALSE"), "should contain BR_FALSE mnemonic");
    assert!(text.contains("LOAD_TRUE"), "should contain LOAD_TRUE mnemonic");
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
