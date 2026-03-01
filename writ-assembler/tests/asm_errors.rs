/// Error diagnostic tests: line:col, multi-error, descriptive messages.

#[test]
fn undefined_label_error() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        BR .nonexistent
        RET_VOID
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err(), "should fail on undefined label");
    let errors = result.unwrap_err();
    let has_label_error = errors.iter().any(|e| e.message.contains("nonexistent"));
    assert!(has_label_error, "error should mention the undefined label name; errors: {:?}", errors);
}

#[test]
fn unknown_mnemonic_error() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        FAKE_OP r0
        RET_VOID
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err(), "should fail on unknown mnemonic");
    let errors = result.unwrap_err();
    let has_mnemonic_error = errors.iter().any(|e| e.message.contains("FAKE_OP"));
    assert!(has_mnemonic_error, "error should mention unknown mnemonic; errors: {:?}", errors);
}

#[test]
fn wrong_operand_count() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        ADD_I r0, r1
        RET_VOID
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err(), "should fail on wrong operand count");
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "should have at least one error");
}

#[test]
fn multiple_errors_collected() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        FAKE_OP_1 r0
        FAKE_OP_2 r1
        FAKE_OP_3 r2
        RET_VOID
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err(), "should fail with multiple errors");
    let errors = result.unwrap_err();
    assert!(errors.len() >= 3, "should collect at least 3 errors, got {}", errors.len());
}

#[test]
fn error_format_matches_spec() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        FAKE_OP r0
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    let formatted = format!("{}", errors[0]);
    // Format should be: error: <message> at line <N>, column <M>
    assert!(formatted.contains("error:"), "should start with 'error:'; got: {}", formatted);
    assert!(formatted.contains("at line"), "should contain 'at line'; got: {}", formatted);
    assert!(formatted.contains("column"), "should contain 'column'; got: {}", formatted);
}

#[test]
fn undefined_type_in_impl_error() {
    let src = r#"
.module "test" "1.0.0" {
    .contract "IFoo" {
        .method "foo" () -> void slot 0
    }
    .impl NonExistentType : IFoo {
        .method "foo" () -> void {
            RET_VOID
        }
    }
}
"#;
    let result = writ_assembler::assemble(src);
    assert!(result.is_err(), "should fail on undefined type in impl");
    let errors = result.unwrap_err();
    let has_type_error = errors.iter().any(|e| e.message.contains("NonExistentType"));
    assert!(has_type_error, "error should mention undefined type; errors: {:?}", errors);
}
