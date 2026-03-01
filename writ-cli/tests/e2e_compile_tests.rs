/// End-to-end integration tests for the Writ compile pipeline.
///
/// These tests exercise the full source -> compile -> load -> run pipeline
/// using the compiler API directly (no subprocess). This validates that all
/// phases (parse, lower, resolve, typecheck, codegen, serialize, deserialize,
/// execute) integrate correctly.
use writ_compiler;
use writ_diagnostics::{FileId, Severity};
use writ_module::{heap::read_string, Module};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TickResult};

// ─── Pipeline helper ──────────────────────────────────────────────────────────

/// Compile a Writ source string through the full pipeline.
///
/// Returns Ok(bytes) on success, Err(String) on any pipeline error.
/// Uses Box::leak for the source string to satisfy 'static lifetime
/// required by writ_parser::parse.
fn compile_source(src: &str) -> Result<Vec<u8>, String> {
    // Box::leak: promotes the source string to 'static lifetime,
    // required by writ_parser::parse's return type.
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    let file_id = FileId(0);

    // Stage 1: Parse
    let (cst_opt, parse_errs) = writ_parser::parse(src_static);
    if !parse_errs.is_empty() {
        return Err(format!("{} parse error(s): {:?}", parse_errs.len(), parse_errs.first()));
    }
    let cst = cst_opt.ok_or_else(|| "parse failed: no CST output".to_string())?;

    // Stage 2: Lower CST -> AST
    let (ast, lower_errs) = writ_compiler::lower(cst);
    if !lower_errs.is_empty() {
        return Err(format!("{} lowering error(s)", lower_errs.len()));
    }

    // Stage 3: Name resolution
    let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
        &[(file_id, &ast)],
        &[(file_id, "test.writ")],
    );
    let has_resolve_errors = resolve_diags.iter().any(|d| d.severity == Severity::Error);
    if has_resolve_errors {
        let msgs: Vec<_> = resolve_diags.iter().map(|d| d.message.clone()).collect();
        return Err(format!("resolution error(s): {}", msgs.join("; ")));
    }

    // Stage 4: Type checking
    let (typed_ast, interner, type_diags) = writ_compiler::check::typecheck(
        resolved,
        &[(file_id, &ast)],
    );
    let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
    if has_type_errors {
        let msgs: Vec<_> = type_diags.iter().map(|d| d.message.clone()).collect();
        return Err(format!("type error(s): {}", msgs.join("; ")));
    }

    // Stage 5: IL codegen (includes metadata + bodies + serialization)
    writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)])
        .map_err(|diags| {
            let msgs: Vec<_> = diags.iter().map(|d| d.message.clone()).collect();
            format!("{} codegen error(s): {}", diags.len(), msgs.join("; "))
        })
}

/// Try to compile; return (has_any_error, error_messages_from_all_stages).
/// Used for negative tests where we expect any pipeline error (resolution or type).
fn compile_expect_error(src: &str) -> (bool, Vec<String>) {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    let file_id = FileId(0);

    let (cst_opt, parse_errs) = writ_parser::parse(src_static);
    if !parse_errs.is_empty() {
        return (false, vec![format!("parse errors: {:?}", parse_errs)]);
    }
    let cst = match cst_opt {
        Some(c) => c,
        None => return (false, vec!["parse returned no output".to_string()]),
    };

    let (ast, lower_errs) = writ_compiler::lower(cst);
    if !lower_errs.is_empty() {
        return (false, vec![format!("lower errors: {:?}", lower_errs)]);
    }

    let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
        &[(file_id, &ast)],
        &[(file_id, "test.writ")],
    );
    let resolve_errors: Vec<String> = resolve_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    if !resolve_errors.is_empty() {
        return (true, resolve_errors);
    }

    // Also check type errors — undefined variable references (E0102) are type errors,
    // not resolution errors, since the resolver only validates type-level names.
    let (_typed_ast, _interner, type_diags) = writ_compiler::check::typecheck(
        resolved,
        &[(file_id, &ast)],
    );
    let type_errors: Vec<String> = type_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();

    let has_errors = !type_errors.is_empty();
    (has_errors, type_errors)
}

// ─── Test 1: Compile minimal program ─────────────────────────────────────────

/// Compile a minimal valid Writ program and verify the output is non-empty bytes.
#[test]
fn test_compile_minimal_program() {
    let src = r#"pub fn main() {
    let x: int = 42;
}"#;

    let result = compile_source(src);
    assert!(result.is_ok(), "minimal program should compile: {:?}", result.err());

    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "compiled bytes should not be empty");
    assert!(
        bytes.len() > 100,
        "compiled module should be at least 100 bytes, got {}",
        bytes.len()
    );
}

// ─── Test 2: Compiled module has valid WRIT magic header ─────────────────────

/// Compile a minimal program and verify the first 4 bytes are the WRIT magic bytes.
#[test]
fn test_compile_produces_valid_module_header() {
    let src = r#"pub fn main() {
    let x: int = 42;
}"#;

    let bytes = compile_source(src).expect("should compile successfully");

    // WRIT magic bytes
    assert!(
        bytes.len() >= 4,
        "module must have at least 4 bytes for magic"
    );
    assert_eq!(
        &bytes[0..4],
        b"WRIT",
        "first 4 bytes must be WRIT magic, got {:?}",
        &bytes[0..4]
    );

    // Module::from_bytes must succeed
    let loaded = Module::from_bytes(&bytes);
    assert!(
        loaded.is_ok(),
        "Module::from_bytes should succeed on compiled output: {:?}",
        loaded.err()
    );
}

// ─── Test 3: Compile and run minimal program ──────────────────────────────────

/// Compile a minimal Writ program, load the module, find the 'main' export,
/// and run it to completion. Validates the full source -> execute pipeline.
#[test]
fn test_compile_and_run_minimal() {
    let src = r#"pub fn main() {
    let x: int = 42;
}"#;

    let bytes = compile_source(src).expect("should compile successfully");
    let module = Module::from_bytes(&bytes).expect("module should deserialize");

    // The compiled module must export 'main'
    let main_export = module
        .export_defs
        .iter()
        .find(|e| read_string(&module.string_heap, e.name).unwrap_or("") == "main")
        .expect("compiled pub fn main() must appear in export_defs");

    assert_eq!(
        main_export.item_kind, 0,
        "main export should be kind=0 (Method)"
    );

    // Convert 1-based MetadataToken to 0-based method index
    let method_idx = (main_export.item.0 & 0x00FF_FFFF) as usize - 1;

    // Build runtime and spawn main
    let mut runtime = RuntimeBuilder::new(module)
        .build()
        .expect("runtime should build");
    runtime
        .spawn_task(method_idx, vec![])
        .expect("spawn_task should succeed");

    // Tick until completion
    let mut completed = false;
    loop {
        match runtime.tick(0.0, ExecutionLimit::None) {
            TickResult::AllCompleted | TickResult::Empty => {
                completed = true;
                break;
            }
            TickResult::TasksSuspended(_) => break,
            TickResult::ExecutionLimitReached => break,
        }
    }

    assert!(completed, "main should run to completion without suspension");
}

// ─── Test 4: Compile error on undefined name ─────────────────────────────────

/// Compile a program with an undefined name reference.
/// The pipeline should fail at the type checking stage (E0102 UndefinedVariable)
/// since the Writ resolver only validates type-level names; expression-level
/// undefined variable references are caught by the typechecker.
#[test]
fn test_compile_error_on_invalid_name() {
    let src = r#"pub fn main() {
    let x = undefined_name;
}"#;

    // compile_source should fail (at the type checking stage)
    let result = compile_source(src);
    assert!(result.is_err(), "program with undefined name should fail to compile");

    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("type") || err_msg.contains("codegen"),
        "error should occur at type checking or codegen stage, got: {:?}",
        err_msg
    );

    // Also test via compile_expect_error for diagnostic content
    let (has_errors, msgs) = compile_expect_error(src);
    assert!(has_errors, "pipeline should produce errors for undefined name");
    assert!(!msgs.is_empty(), "should have at least one error message");

    // The error message should mention 'undefined_name' or an undefined variable
    let combined = msgs.join(" ");
    assert!(
        combined.contains("undefined_name")
            || combined.contains("not found")
            || combined.contains("unresolved")
            || combined.contains("undefined"),
        "error message should reference the undefined name, got: {:?}",
        combined
    );
}

// ─── Test: LocaleDef rows in compiled module ──────────────────────────────────

/// Compiling a source with a [Locale("ja")] dlg override produces a .writil
/// module that contains at least one LocaleDef row when deserialized.
#[test]
fn test_locale_override_produces_locale_def_rows() {
    // Use empty dlg bodies to avoid Entity/say resolution requirements.
    let src = r#"
dlg greet() {}

[Locale("ja")]
dlg greet() {}
"#;
    let bytes = compile_source(src).expect("should compile without errors");
    let module = Module::from_bytes(&bytes).expect("should deserialize successfully");
    assert!(
        module.locale_defs.len() > 0,
        "compiled module should have at least one LocaleDef row for [Locale(\"ja\")] override"
    );
}
