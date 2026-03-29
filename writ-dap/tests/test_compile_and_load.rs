/// Integration test for compile_and_load (DAP-01) and compile_and_load_project (DAP-02).
///
/// Verifies that `compile_and_load` can compile a real .writ source file
/// through the full 5-stage pipeline and return a decoded Module with
/// populated method_defs, method_bodies, and source_spans (debug info).
///
/// Also verifies that `compile_and_load_project` discovers and compiles all
/// .writ files in a writ.toml project directory.
use writ_dap::launch::compile_and_load;
use writ_dap::launch::compile_and_load_project;
use writ_module::heap::read_string;
use std::fs;

/// Resolve a path relative to the workspace root from this crate's manifest dir.
/// CARGO_MANIFEST_DIR = writ-dap/
fn workspace_file(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Walk up one level from writ-dap/ to the workspace root.
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .expect("workspace root should exist");
    workspace_root.join(relative).to_string_lossy().into_owned()
}

#[test]
fn test_compile_and_load_produces_module_with_methods() {
    // Arrange: use fn_multi_return.writ — it has actual if-branch statements that
    // produce source span entries when emit_debug_info=true.
    // fn_basic_call.writ has empty bodies that may produce no spans.
    let fixture_path = workspace_file("writ-golden/tests/golden/fn_multi_return.writ");

    // Act: compile the .writ file through the full DAP pipeline.
    let result = compile_and_load(&fixture_path);

    // Assert: compilation succeeds and the module is populated.
    assert!(
        result.is_ok(),
        "compile_and_load should succeed for fn_multi_return.writ, got: {:?}",
        result.err()
    );

    let (module, src, _method_file_ids) = result.unwrap();

    // method_defs must be non-empty (fn_multi_return.writ defines abs and main).
    assert!(
        !module.method_defs.is_empty(),
        "module should have at least one method_def after compilation"
    );

    // method_bodies must match method_defs in count.
    assert_eq!(
        module.method_defs.len(),
        module.method_bodies.len(),
        "method_defs and method_bodies should have the same length"
    );

    // source_spans must be present (emit_debug_info=true is always set in compile_and_load).
    // fn_multi_return.writ has if-branch statements that produce source spans.
    let has_spans = module
        .method_bodies
        .iter()
        .any(|body| !body.source_spans.is_empty());
    assert!(
        has_spans,
        "at least one method body should have source_spans (emit_debug_info=true)"
    );

    // The leaked source text must be non-empty and contain the expected function.
    assert!(
        !src.is_empty(),
        "returned source text should not be empty"
    );
    assert!(
        src.contains("fn main"),
        "returned source text should contain the source program"
    );
}

#[test]
fn test_compile_and_load_returns_error_for_nonexistent_file() {
    // Arrange: a path that does not exist.
    let bad_path = "/nonexistent/path/to/program.writ";

    // Act + Assert: should return an Err with a human-readable message.
    let result = compile_and_load(bad_path);
    assert!(
        result.is_err(),
        "compile_and_load should return Err for a missing file"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("nonexistent") || msg.contains("failed to read"),
        "error message should mention the file path or describe the I/O failure, got: {}",
        msg
    );
}

/// Create a temporary writ.toml project with two source files and verify
/// that compile_and_load_project discovers and compiles both.
#[test]
fn test_compile_and_load_project_multi_file() {
    // Create a temp project directory
    let tmp = std::env::temp_dir().join("writ_dap_test_multi_file");
    let _ = fs::remove_dir_all(&tmp);
    let src_dir = tmp.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Write writ.toml
    fs::write(
        tmp.join("writ.toml"),
        r#"[project]
name = "test-multi"
version = "0.1.0"
"#,
    )
    .unwrap();

    // Write two .writ source files
    // File 1: defines a helper function
    fs::write(
        src_dir.join("helpers.writ"),
        "fn add(a: int, b: int) -> int { a + b }\n",
    )
    .unwrap();

    // File 2: defines main that calls the helper
    fs::write(
        src_dir.join("main.writ"),
        "fn main() { let result: int = add(1, 2); }\n",
    )
    .unwrap();

    // Compile via project mode
    let result = compile_and_load_project(&tmp);
    let _ = fs::remove_dir_all(&tmp);

    assert!(
        result.is_ok(),
        "compile_and_load_project should succeed for a valid multi-file project, got: {:?}",
        result.err()
    );

    let (module, file_id_paths, _method_file_ids) = result.unwrap();

    // Both source files should be tracked
    assert_eq!(
        file_id_paths.len(),
        2,
        "should have 2 file_id_path entries (one per .writ file)"
    );

    // Module should contain methods from both files (add + main = at least 2)
    assert!(
        module.method_defs.len() >= 2,
        "module should have at least 2 method_defs (add + main), got: {}",
        module.method_defs.len()
    );

    // Verify main exists
    let has_main = module.method_defs.iter().any(|md| {
        read_string(&module.string_heap, md.name)
            .map(|n| n == "main")
            .unwrap_or(false)
    });
    assert!(has_main, "module should contain a 'main' method");

    // Verify add exists
    let has_add = module.method_defs.iter().any(|md| {
        read_string(&module.string_heap, md.name)
            .map(|n| n == "add")
            .unwrap_or(false)
    });
    assert!(has_add, "module should contain an 'add' method from helpers.writ");
}

/// Verify that compile_and_load_project returns an error for a directory without writ.toml.
#[test]
fn test_compile_and_load_project_missing_toml() {
    let tmp = std::env::temp_dir().join("writ_dap_test_no_toml");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let result = compile_and_load_project(&tmp);
    let _ = fs::remove_dir_all(&tmp);

    assert!(
        result.is_err(),
        "compile_and_load_project should fail when writ.toml is missing"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("writ.toml"),
        "error message should mention writ.toml, got: {}",
        msg
    );
}

/// Verify that compile_and_load_project returns an error when no .writ files exist.
#[test]
fn test_compile_and_load_project_no_source_files() {
    let tmp = std::env::temp_dir().join("writ_dap_test_no_sources");
    let _ = fs::remove_dir_all(&tmp);
    let src_dir = tmp.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Write writ.toml but no .writ files
    fs::write(
        tmp.join("writ.toml"),
        r#"[project]
name = "empty"
version = "0.1.0"
"#,
    )
    .unwrap();

    let result = compile_and_load_project(&tmp);
    let _ = fs::remove_dir_all(&tmp);

    assert!(
        result.is_err(),
        "compile_and_load_project should fail when no .writ files found"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("no .writ source files"),
        "error message should mention missing source files, got: {}",
        msg
    );
}
