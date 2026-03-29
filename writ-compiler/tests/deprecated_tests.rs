//! Tests for the [Deprecated] attribute — W0006 warning emission.
//!
//! Task 1 tests: TypeEnv.deprecated_items population
//! Task 2 tests: W0006 warning emission at call/ident/construction sites

use writ_compiler::ast::Ast;
use writ_compiler::check::typecheck;
use writ_compiler::lower::lower;
use writ_compiler::resolve;
use writ_diagnostics::{Diagnostic, FileId, Severity};

// =========================================================
// Test helpers
// =========================================================

/// Parse, lower, resolve, and typecheck a single source string.
/// Returns (typed_ast, type_env, diagnostics).
fn typecheck_src_with_env(
    src: &'static str,
) -> (writ_compiler::check::ir::TypedAst, writ_compiler::check::env::TypeEnv, Vec<Diagnostic>) {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);

    let file_id = FileId(0);
    let asts: Vec<(FileId, &Ast)> = vec![(file_id, &ast)];
    let file_paths: Vec<(FileId, &str)> = vec![(file_id, "src/test.writ")];
    let (resolved, resolve_diags) = resolve::resolve(&asts, &file_paths, &[]);

    let resolve_errors: Vec<&Diagnostic> = resolve_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        resolve_errors.is_empty(),
        "resolve errors: {:?}",
        resolve_errors
    );

    let (typed_ast, _interner, type_env, type_diags) = typecheck(resolved, &asts, &[]);
    (typed_ast, type_env, type_diags)
}

/// Parse, lower, resolve, and typecheck multiple source files.
/// Returns (typed_ast, type_env, diagnostics).
fn typecheck_multi(
    files: &[(&str, &'static str)],
) -> (writ_compiler::check::ir::TypedAst, writ_compiler::check::env::TypeEnv, Vec<Diagnostic>) {
    let mut asts_owned = Vec::new();
    for (_, src) in files.iter() {
        let (items, parse_errors) = writ_parser::parse(src);
        let items = items.expect("parse returned None");
        let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
        assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
        let (ast, lower_errors) = lower(items);
        assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);
        asts_owned.push(ast);
    }

    let asts_with_ids: Vec<(FileId, &Ast)> = asts_owned
        .iter()
        .enumerate()
        .map(|(i, ast)| (FileId(i as u32), ast))
        .collect();
    let file_paths: Vec<(FileId, &str)> = files
        .iter()
        .enumerate()
        .map(|(i, (path, _))| (FileId(i as u32), *path))
        .collect();

    let (resolved, resolve_diags) = resolve::resolve(&asts_with_ids, &file_paths, &[]);

    let resolve_errors: Vec<&Diagnostic> = resolve_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        resolve_errors.is_empty(),
        "resolve errors: {:?}",
        resolve_errors
    );

    let (typed_ast, _interner, type_env, type_diags) = typecheck(resolved, &asts_with_ids, &[]);
    (typed_ast, type_env, type_diags)
}

fn has_warning(diags: &[Diagnostic], code: &str) -> bool {
    diags
        .iter()
        .any(|d| d.code == code && d.severity == Severity::Warning)
}

fn count_warnings(diags: &[Diagnostic], code: &str) -> usize {
    diags
        .iter()
        .filter(|d| d.code == code && d.severity == Severity::Warning)
        .count()
}

fn warning_message(diags: &[Diagnostic], code: &str) -> Option<String> {
    diags
        .iter()
        .find(|d| d.code == code && d.severity == Severity::Warning)
        .map(|d| d.message.clone())
}

fn has_no_warnings(diags: &[Diagnostic], code: &str) -> bool {
    !diags
        .iter()
        .any(|d| d.code == code && d.severity == Severity::Warning)
}

// =========================================================
// Task 1: TypeEnv.deprecated_items population tests
// =========================================================

/// Test 1: TypeEnv built from source with `[Deprecated("use bar")] fn foo() {}`
/// has deprecated_items containing the DefId for `foo` mapped to `"use bar"`.
#[test]
fn deprecated_items_populated_with_message() {
    let (_ast, type_env, _diags) = typecheck_src_with_env(
        r#"
[Deprecated("use bar")]
pub fn foo() {}
"#,
    );

    let found = type_env
        .deprecated_items
        .values()
        .find(|msg| msg.as_str() == "use bar");
    assert!(
        found.is_some(),
        "Expected deprecated_items to contain 'use bar', got: {:?}",
        type_env.deprecated_items
    );
}

/// Test 2: TypeEnv built from bare `[Deprecated] fn foo() {}` has
/// deprecated_items containing DefId for `foo` mapped to empty string `""`.
#[test]
fn deprecated_items_bare_maps_to_empty_string() {
    let (_ast, type_env, _diags) = typecheck_src_with_env(
        r#"
[Deprecated]
pub fn foo() {}
"#,
    );

    let found = type_env
        .deprecated_items
        .values()
        .find(|msg| msg.is_empty());
    assert!(
        found.is_some(),
        "Expected deprecated_items to contain empty-string entry for bare [Deprecated], got: {:?}",
        type_env.deprecated_items
    );
}

/// Test 3: TypeEnv built from source with NO deprecated items has empty deprecated_items.
#[test]
fn deprecated_items_empty_when_no_attribute() {
    let (_ast, type_env, _diags) = typecheck_src_with_env(
        r#"
pub fn foo() {}
pub fn bar() {}
"#,
    );

    assert!(
        type_env.deprecated_items.is_empty(),
        "Expected no deprecated_items, got: {:?}",
        type_env.deprecated_items
    );
}

// =========================================================
// Task 2: W0006 emission tests
// =========================================================

/// Test 4: Calling `[Deprecated("use bar")] fn foo()` from a different file
/// produces exactly one W0006 warning containing "use bar".
#[test]
fn deprecated_call_cross_file_emits_w0006() {
    let (_ast, _type_env, diags) = typecheck_multi(&[
        (
            "src/lib.writ",
            r#"
[Deprecated("use bar")]
pub fn foo() {}

pub fn bar() {}
"#,
        ),
        (
            "src/main.writ",
            r#"
pub fn main() {
    foo();
}
"#,
        ),
    ]);

    assert!(
        has_warning(&diags, writ_diagnostics::code::W0006),
        "Expected W0006 warning when calling deprecated fn from different file. Diags: {:?}",
        diags
    );
    let msg = warning_message(&diags, writ_diagnostics::code::W0006).unwrap();
    assert!(
        msg.contains("use bar"),
        "Expected warning to contain 'use bar', got: '{}'",
        msg
    );
    assert_eq!(
        count_warnings(&diags, writ_diagnostics::code::W0006),
        1,
        "Expected exactly one W0006 warning"
    );
}

/// Test 5: Calling `[Deprecated("msg")] fn foo()` from the SAME file produces
/// NO W0006 warning (self-deprecation suppression).
#[test]
fn deprecated_call_same_file_no_warning() {
    let (_ast, _type_env, diags) = typecheck_src_with_env(
        r#"
[Deprecated("use bar")]
pub fn foo() {}

pub fn main() {
    foo();
}
"#,
    );

    assert!(
        has_no_warnings(&diags, writ_diagnostics::code::W0006),
        "Expected NO W0006 warning when calling deprecated fn from same file. Diags: {:?}",
        diags
    );
}

/// Test 6: Bare `[Deprecated] fn foo()` called from different file produces
/// W0006 with a default message containing the function name.
#[test]
fn deprecated_bare_call_cross_file_default_message() {
    let (_ast, _type_env, diags) = typecheck_multi(&[
        (
            "src/lib.writ",
            r#"
[Deprecated]
pub fn foo() {}
"#,
        ),
        (
            "src/main.writ",
            r#"
pub fn main() {
    foo();
}
"#,
        ),
    ]);

    assert!(
        has_warning(&diags, writ_diagnostics::code::W0006),
        "Expected W0006 warning for bare [Deprecated] fn call from different file. Diags: {:?}",
        diags
    );
    let msg = warning_message(&diags, writ_diagnostics::code::W0006).unwrap();
    assert!(
        msg.contains("foo"),
        "Expected default deprecation message to contain function name 'foo', got: '{}'",
        msg
    );
}

/// Test 7: Using `new DeprecatedStruct { ... }` from different file produces W0006.
#[test]
fn deprecated_construction_cross_file_emits_w0006() {
    let (_ast, _type_env, diags) = typecheck_multi(&[
        (
            "src/types.writ",
            r#"
[Deprecated("use NewFoo")]
pub struct Foo {
    pub x: int,
}
"#,
        ),
        (
            "src/main.writ",
            r#"
pub fn make_foo() {
    let _ = new Foo { x: 1 };
}
"#,
        ),
    ]);

    assert!(
        has_warning(&diags, writ_diagnostics::code::W0006),
        "Expected W0006 warning for `new DeprecatedStruct` from different file. Diags: {:?}",
        diags
    );
    let msg = warning_message(&diags, writ_diagnostics::code::W0006).unwrap();
    assert!(
        msg.contains("use NewFoo"),
        "Expected warning to contain 'use NewFoo', got: '{}'",
        msg
    );
}
