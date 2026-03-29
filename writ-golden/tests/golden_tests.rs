/// Golden test harness for the Writ compiler.
///
/// Workflow:
///   - `compile_and_disassemble(src)` — full pipeline (parse->lower->resolve->typecheck->emit_bodies)
///     inside a 16 MB stack thread, then round-trip through Module::from_bytes, then disassemble.
///   - `run_golden_test(name)` — read `tests/golden/{name}.writ`, compile+disassemble, then either
///     bless (BLESS=1 env var) or compare against `tests/golden/{name}.writil`.
///   - On mismatch: panics with a unified diff (--- expected / +++ actual).
///   - `bless_golden(name, actual, golden_dir)` — exposed for testing; called by run_golden_test
///     when BLESS=1.
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use writ_diagnostics::{FileId, Severity};
use writ_module::Module;

// ─── Section A: compile_and_disassemble ──────────────────────────────────────

/// Compile a Writ source string and disassemble the result.
///
/// Runs the full pipeline on a 16 MB stack thread (required due to deep AST
/// recursion). After compilation, the bytes are round-tripped through
/// `Module::from_bytes` before disassembly — this ensures the golden snapshot
/// tests what is actually serialized, not what is in compiler memory.
///
/// Panics with a descriptive message if any pipeline stage fails.
pub fn compile_and_disassemble(src: &str) -> String {
    // Box::leak: promotes the source string to 'static lifetime,
    // required by writ_parser::parse's return type.
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());

    // Compile pipeline must run on a 16 MB stack thread due to deep AST recursion.
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<Vec<u8>, String> {
            let file_id = FileId(0);

            // Stage 1: Parse
            let (cst_opt, parse_errs) = writ_parser::parse(src_static);
            if !parse_errs.is_empty() {
                return Err(format!(
                    "{} parse error(s): {:?}",
                    parse_errs.len(),
                    parse_errs.first()
                ));
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
                &[],
            );
            let has_resolve_errors =
                resolve_diags.iter().any(|d| d.severity == Severity::Error);
            if has_resolve_errors {
                let msgs: Vec<_> = resolve_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("resolution error(s): {}", msgs.join("; ")));
            }

            // Stage 4: Type checking
            let (typed_ast, interner, _type_env, type_diags) = writ_compiler::check::typecheck(
                resolved,
                &[(file_id, &ast)],
                &[],
            );
            let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
            if has_type_errors {
                let msgs: Vec<_> = type_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("type error(s): {}", msgs.join("; ")));
            }

            // Stage 5: IL codegen (includes metadata + bodies + serialization)
            let active_conditions = std::collections::HashSet::new();
            writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)], true, &[], &active_conditions).map_err(
                |diags| {
                    let msgs: Vec<_> = diags.iter().map(|d| d.message.clone()).collect();
                    format!("{} codegen error(s): {}", diags.len(), msgs.join("; "))
                },
            )
        })
        .expect("thread spawn failed");

    let bytes = handle
        .join()
        .expect("compile thread panicked")
        .expect("compile_and_disassemble: compilation failed");

    // Round-trip isolation: deserialize from bytes, not from the compiler's in-memory state.
    let module = Module::from_bytes(&bytes)
        .expect("compile_and_disassemble: Module::from_bytes failed after successful compile");

    writ_assembler::disassemble(&module)
}

/// Compile a Writ source string with active conditions and disassemble the result.
///
/// Identical to `compile_and_disassemble` but passes `conditions` to `emit_bodies`
/// so that [Conditional("name")] filtering is exercised.
pub fn compile_and_disassemble_with_conditions(src: &str, conditions: &[&str]) -> String {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    let active_conditions: std::collections::HashSet<String> =
        conditions.iter().map(|s| s.to_string()).collect();

    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<Vec<u8>, String> {
            let file_id = FileId(0);

            // Stage 1: Parse
            let (cst_opt, parse_errs) = writ_parser::parse(src_static);
            if !parse_errs.is_empty() {
                return Err(format!(
                    "{} parse error(s): {:?}",
                    parse_errs.len(),
                    parse_errs.first()
                ));
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
                &[],
            );
            let has_resolve_errors =
                resolve_diags.iter().any(|d| d.severity == Severity::Error);
            if has_resolve_errors {
                let msgs: Vec<_> = resolve_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("resolution error(s): {}", msgs.join("; ")));
            }

            // Stage 4: Type checking
            let (typed_ast, interner, _type_env, type_diags) = writ_compiler::check::typecheck(
                resolved,
                &[(file_id, &ast)],
                &[],
            );
            let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
            if has_type_errors {
                let msgs: Vec<_> = type_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("type error(s): {}", msgs.join("; ")));
            }

            // Stage 5: IL codegen with active conditions
            writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)], true, &[], &active_conditions)
                .map_err(|diags| {
                    let msgs: Vec<_> = diags.iter().map(|d| d.message.clone()).collect();
                    format!("{} codegen error(s): {}", diags.len(), msgs.join("; "))
                })
        })
        .expect("thread spawn failed");

    let bytes = handle
        .join()
        .expect("compile thread panicked")
        .expect("compile_and_disassemble_with_conditions: compilation failed");

    let module = Module::from_bytes(&bytes)
        .expect("compile_and_disassemble_with_conditions: Module::from_bytes failed after successful compile");

    writ_assembler::disassemble(&module)
}

/// Run a golden test for `name` with active conditions.
///
/// Reads `tests/golden/{name}.writ`, compiles+disassembles with the given conditions, then:
/// - If `BLESS=1` env var is set: overwrites `tests/golden/{name}.writil` with actual output.
/// - Otherwise: compares against `tests/golden/{name}.writil`, panicking with a unified diff.
pub fn run_golden_test_with_conditions(name: &str, conditions: &[&str]) {
    let golden_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let src_path = golden_dir.join(format!("{name}.writ"));
    let expected_path = golden_dir.join(format!("{name}.writil"));

    let src = std::fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("run_golden_test_with_conditions: could not read {src_path:?}: {e}"));

    let actual = compile_and_disassemble_with_conditions(&src, conditions);

    if std::env::var("BLESS").as_deref() == Ok("1") {
        bless_golden(name, &actual, &golden_dir);
        return;
    }

    let expected_bytes = std::fs::read(&expected_path).unwrap_or_else(|_| {
        panic!(
            "assembly file not found — run BLESS=1 cargo test -p writ-golden -- {name} to create it\n  missing: {}",
            expected_path.display()
        )
    });
    let stripped = strip_utf16le_bom(&expected_bytes);
    let expected_raw = String::from_utf8(stripped.to_vec()).unwrap_or_else(|e| {
        panic!("run_golden_test_with_conditions: expected file {expected_path:?} is not valid UTF-8 after BOM strip: {e}")
    });
    let expected = expected_raw.replace("\r\n", "\n");

    if expected == actual {
        return;
    }

    let diff = TextDiff::from_lines(&expected, &actual);
    let mut diff_text = String::new();
    diff_text.push_str("--- expected\n+++ actual\n");
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        diff_text.push_str(&format!("{prefix}{change}"));
    }

    panic!(
        "golden file mismatch for '{name}':\n{diff_text}\nTo update: BLESS=1 cargo test -p writ-golden"
    );
}

/// Compile Writ source to a Module (round-tripped through bytes).
fn compile_to_module(src: &str) -> Module {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<Vec<u8>, String> {
            let file_id = FileId(0);
            let (cst_opt, parse_errs) = writ_parser::parse(src_static);
            if !parse_errs.is_empty() {
                return Err(format!("parse error(s): {:?}", parse_errs.first()));
            }
            let cst = cst_opt.ok_or_else(|| "no CST".to_string())?;
            let (ast, lower_errs) = writ_compiler::lower(cst);
            if !lower_errs.is_empty() {
                return Err(format!("{} lowering error(s)", lower_errs.len()));
            }
            let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
                &[(file_id, &ast)], &[(file_id, "test.writ")],
                &[],
            );
            if resolve_diags.iter().any(|d| d.severity == Severity::Error) {
                let msgs: Vec<_> = resolve_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("resolution error(s): {}", msgs.join("; ")));
            }
            let (typed_ast, interner, _type_env, type_diags) = writ_compiler::check::typecheck(
                resolved, &[(file_id, &ast)], &[],
            );
            if type_diags.iter().any(|d| d.severity == Severity::Error) {
                let msgs: Vec<_> = type_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("type error(s): {}", msgs.join("; ")));
            }
            let active_conditions = std::collections::HashSet::new();
            writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)], true, &[], &active_conditions).map_err(
                |diags| {
                    let msgs: Vec<_> = diags.iter().map(|d| d.message.clone()).collect();
                    format!("{} codegen error(s): {}", diags.len(), msgs.join("; "))
                },
            )
        })
        .expect("thread spawn failed");
    let bytes = handle.join().expect("compile thread panicked").expect("compilation failed");
    Module::from_bytes(&bytes).expect("Module::from_bytes failed")
}

// ─── Section B: bless_golden and run_golden_test ─────────────────────────────

/// Strip a UTF-16 LE BOM (`0xFF 0xFE`) from the front of `bytes` if present.
///
/// Used by `run_golden_test` when reading the expected `.writil` file so that
/// hand-edited files saved as UTF-16 LE still compare correctly. The BOM is
/// never introduced on write — `bless_golden` always writes clean UTF-8.
fn strip_utf16le_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        bytes
    }
}

/// Write `actual` IL text to the expected file for golden `name` in `golden_dir`.
///
/// Exposed as `pub(crate)` so tests can exercise the bless path with a temp dir
/// without touching env vars.
pub(crate) fn bless_golden(name: &str, actual: &str, golden_dir: &Path) {
    let expected_path = golden_dir.join(format!("{name}.writil"));
    std::fs::write(&expected_path, actual)
        .unwrap_or_else(|e| panic!("bless_golden: could not write {expected_path:?}: {e}"));
    println!("blessed: {}", expected_path.display());
}

/// Run a golden test for `name`.
///
/// Reads `tests/golden/{name}.writ`, compiles+disassembles, then:
/// - If `BLESS=1` env var is set: overwrites `tests/golden/{name}.writil` with actual output.
/// - Otherwise: compares against `tests/golden/{name}.writil`, panicking with a unified diff
///   on mismatch.
pub fn run_golden_test(name: &str) {
    let golden_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let src_path = golden_dir.join(format!("{name}.writ"));
    let expected_path = golden_dir.join(format!("{name}.writil"));

    let src = std::fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("run_golden_test: could not read {src_path:?}: {e}"));

    let actual = compile_and_disassemble(&src);

    if std::env::var("BLESS").as_deref() == Ok("1") {
        bless_golden(name, &actual, &golden_dir);
        return;
    }

    let expected_bytes = std::fs::read(&expected_path).unwrap_or_else(|_| {
        panic!(
            "assembly file not found — run BLESS=1 cargo test -p writ-golden -- {name} to create it\n  missing: {}",
            expected_path.display()
        )
    });
    let stripped = strip_utf16le_bom(&expected_bytes);
    let expected_raw = String::from_utf8(stripped.to_vec()).unwrap_or_else(|e| {
        panic!("run_golden_test: expected file {expected_path:?} is not valid UTF-8 after BOM strip: {e}")
    });
    // Normalize CRLF to LF so files edited on Windows compare correctly
    // against the disassembler's Unix line endings.
    let expected = expected_raw.replace("\r\n", "\n");

    if expected == actual {
        return;
    }

    // Build a unified diff for the failure message.
    let diff = TextDiff::from_lines(&expected, &actual);
    let mut diff_text = String::new();
    diff_text.push_str("--- expected\n+++ actual\n");
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        diff_text.push_str(&format!("{prefix}{change}"));
    }

    panic!(
        "golden file mismatch for '{name}':\n{diff_text}\nTo update: BLESS=1 cargo test -p writ-golden"
    );
}

// ─── Section C: Scaffold / harness-level tests ───────────────────────────────

/// Basic sanity: compile a trivial function, verify the disassembly contains ".module".
///
/// This validates that compile_and_disassemble runs the full round-trip pipeline
/// and that the disassembler emits at least one recognisable directive.
#[test]
fn test_harness_pass() {
    let src = "pub fn hello() {}";
    let output = compile_and_disassemble(src);
    assert!(
        output.contains(".module"),
        "disassembly should contain '.module', got:\n{output}"
    );
}

/// Verify the diff construction logic produces "--- expected" and "+++ actual" headers.
///
/// Tests the string-building path without triggering a real panic — directly
/// exercises the diff builder so the output format is confirmed.
#[test]
fn test_harness_fail_shows_diff() {
    let expected = "line A\nline B\n";
    let actual = "line A\nline C\n";

    let diff = TextDiff::from_lines(expected, actual);
    let mut diff_text = String::new();
    diff_text.push_str("--- expected\n+++ actual\n");
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        diff_text.push_str(&format!("{prefix}{change}"));
    }

    assert!(
        diff_text.contains("--- expected"),
        "diff should contain '--- expected'"
    );
    assert!(
        diff_text.contains("+++ actual"),
        "diff should contain '+++ actual'"
    );
    assert!(
        diff_text.contains("-line B"),
        "diff should show deleted line B"
    );
    assert!(
        diff_text.contains("+line C"),
        "diff should show inserted line C"
    );
}

/// Verify bless_golden writes the actual output to the expected file path.
///
/// Uses a temp dir to avoid touching the real golden directory. Does NOT
/// manipulate env vars (not thread-safe in multi-threaded test runners).
#[test]
fn test_bless_writes_file() {
    let tmp = tempfile::tempdir().expect("could not create temp dir");
    let actual = "this is the golden output\n";
    bless_golden("my_test", actual, tmp.path());

    let written_path = tmp.path().join("my_test.writil");
    assert!(
        written_path.exists(),
        "bless_golden should have created {written_path:?}"
    );

    let contents =
        std::fs::read_to_string(&written_path).expect("could not read blessed file");
    assert_eq!(
        contents, actual,
        "blessed file contents should match actual output"
    );
}

/// Verify strip_utf16le_bom strips a UTF-16 LE BOM and leaves non-BOM bytes unchanged.
#[test]
fn test_harness_bom_strip() {
    let with_bom: Vec<u8> = vec![0xFF, 0xFE, b'h', b'i'];
    assert_eq!(strip_utf16le_bom(&with_bom), b"hi");
    let without_bom: Vec<u8> = vec![b'h', b'i'];
    assert_eq!(strip_utf16le_bom(&without_bom), b"hi");
}

// ─── Section D: Function IL golden tests ─────────────────────────────────────

/// Golden test: void-return function called from main.
///
/// Locks the CALL + RET_VOID sequences for a no-op void function.
#[test]
fn test_fn_basic_call() {
    run_golden_test("fn_basic_call");
}

/// Golden test: int and bool typed parameters with typed return values.
///
/// Locks that registers carry correct type blobs for int/i64 and bool parameters
/// and return types (regression anchor for BUG-02 fix).
#[test]
fn test_fn_typed_params() {
    run_golden_test("fn_typed_params");
}

/// Golden test: self-recursive factorial-style function.
///
/// Locks that the recursive CALL instruction references the correct self-call
/// metadata token (i.e., the method token for factorial at the definition site
/// matches the token at the recursive call site).
#[test]
fn test_fn_recursion() {
    run_golden_test("fn_recursion");
}

/// Golden test: bare fn without pub visibility modifier.
///
/// Locks that `fn main() {}` (no `pub`) compiles without a parse error
/// and that the emitted IL is spec-correct for an empty void function.
/// Regression anchor for BUG-15.
#[test]
fn test_fn_empty_main() {
    run_golden_test("fn_empty_main");
}

/// Golden test: log/say/choice inbuilt function calls from a regular fn.
///
/// Locks that ::log, ::say, ::choice (root-qualified forms) resolve and emit
/// correct IL from a regular fn context. Regression anchor for BUG-01 fix.
#[test]
fn test_fn_log_say_choice() {
    run_golden_test("fn_log_say_choice");
}

/// Golden test: Option<T> usage with qualified and unqualified None/Some.
///
/// Locks that `Option::None`, `Option::Some(v)`, and nullable sugar compile
/// correctly and emit the expected LOAD_NULL / WRAP_SOME instructions.
/// Regression anchor for LANG-02.
#[test]
fn test_fn_optional() {
    run_golden_test("fn_optional");
}

/// Golden test: string .len() returns byte count.
///
/// Locks that `s.len()` on a string literal compiles to LoadString + StrLen
/// and returns the correct byte length. Regression anchor for RT-02.
#[test]
fn test_expr_str_len() {
    run_golden_test("expr_str_len");
}

/// Golden test: multi-function module with ::choice lambdas.
///
/// Locks that entity + multiple functions + choice lambdas serialize
/// correctly without orphaned-body ordering mismatch. Regression anchor for RT-03.
#[test]
fn test_fn_multi_choice() {
    run_golden_test("fn_multi_choice");
}

// ─── Section E: Variable golden tests ─────────────────────────────────────────

/// Golden test: mutable variable declaration + reassignment.
#[test]
fn test_var_let_mut() {
    run_golden_test("var_let_mut");
}

/// Golden test: variable shadowing (re-declaring same name).
#[test]
fn test_var_shadowing() {
    run_golden_test("var_shadowing");
}

/// Golden test: const declaration with constant folding.
#[test]
fn test_const_fold() {
    run_golden_test("const_fold");
}

/// Golden test: global mut variable declaration.
#[test]
fn test_global_mut_decl() {
    run_golden_test("global_mut_decl");
}

// ─── Section F: Expression golden tests ───────────────────────────────────────

/// Golden test: float literals + ADD_F, MUL_F arithmetic.
#[test]
fn test_expr_float_arith() {
    run_golden_test("expr_float_arith");
}

/// Golden test: boolean logic (&&, ||, !).
#[test]
fn test_expr_bool_logic() {
    run_golden_test("expr_bool_logic");
}

/// Golden test: all 6 integer comparison operators.
#[test]
fn test_expr_int_compare() {
    run_golden_test("expr_int_compare");
}

/// Golden test: unary negation on int and float.
#[test]
fn test_expr_unary_neg() {
    run_golden_test("expr_unary_neg");
}

/// Golden test: string concatenation.
#[test]
fn test_expr_string_concat() {
    run_golden_test("expr_string_concat");
}

/// Golden test: string escape sequences (quotes, newline, tab, backslash, null, CR).
#[test]
fn test_expr_string_escapes() {
    run_golden_test("expr_string_escapes");
}

// ─── Section G: Control flow golden tests ─────────────────────────────────────

/// Golden test: while loop.
#[test]
fn test_ctrl_while_loop() {
    run_golden_test("ctrl_while_loop");
}

/// Golden test: for-in loop over array.
#[test]
fn test_ctrl_for_array() {
    run_golden_test("ctrl_for_array");
}

/// Golden test: break + continue in while loop.
#[test]
fn test_ctrl_break_continue() {
    run_golden_test("ctrl_break_continue");
}

/// Golden test: for-in loop over exclusive range (0..5).
///
/// Locks the counter-based loop emission for range iteration: LOAD_INT start,
/// LOAD_INT end, CMP_LT_I condition, BR_FALSE exit, body, ADD_I increment, BR loop.
#[test]
fn test_ctrl_for_range() {
    run_golden_test("ctrl_for_range");
}

// ─── Section H: Type golden tests ────────────────────────────────────────────

/// Golden test: struct definition + new construction + field access.
#[test]
fn test_type_struct_new() {
    run_golden_test("type_struct_new");
}

/// Golden test: struct equality comparison (== and !=).
///
/// Locks the field-by-field GET_FIELD + CmpEqI + BitAnd emission for ==
/// and GET_FIELD + CmpEqI + Not + BitOr emission for !=.
#[test]
fn test_type_struct_eq() {
    run_golden_test("type_struct_eq");
}

/// Golden test: class declaration, construction, and field access.
///
/// Locks that class declarations emit kind=class TypeDefs,
/// new construction uses heap allocation (NEW), and GET_FIELD works.
#[test]
fn test_type_class_new() {
    run_golden_test("type_class_new");
}

/// Test: recursive struct detection produces compile error.
///
/// Verifies that a struct containing itself as a value-type field
/// is rejected with an appropriate error message (E0121).
#[test]
fn test_type_recursive_struct_error() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/type_recursive_struct.writ")
    ).expect("could not read type_recursive_struct.writ");

    let src_static: &'static str = Box::leak(src.into_boxed_str());

    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            let file_id = writ_diagnostics::FileId(0);
            let (cst_opt, parse_errs) = writ_parser::parse(src_static);
            assert!(parse_errs.is_empty(), "unexpected parse errors: {:?}", parse_errs);
            let cst = cst_opt.expect("parse failed: no CST output");
            let (ast, lower_errs) = writ_compiler::lower(cst);
            assert!(lower_errs.is_empty(), "unexpected lowering errors: {:?}", lower_errs);
            let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
                &[(file_id, &ast)], &[(file_id, "test.writ")],
                &[],
            );
            let resolve_errors: Vec<_> = resolve_diags.iter()
                .filter(|d| d.severity == writ_diagnostics::Severity::Error)
                .collect();
            assert!(resolve_errors.is_empty(), "unexpected resolve errors: {:?}", resolve_errors);
            let (_typed_ast, _interner, _type_env, type_diags) = writ_compiler::check::typecheck(resolved, &[(file_id, &ast)], &[]);
            let has_recursive_error = type_diags.iter().any(|d|
                d.message.contains("recursive") || d.code == "E0121"
            );
            if has_recursive_error {
                Ok(())
            } else {
                Err(format!(
                    "Expected recursive struct error (E0121), but got {} diagnostics: {:?}",
                    type_diags.len(),
                    type_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
                ))
            }
        })
        .expect("thread spawn failed");

    handle.join().expect("thread panicked").expect("recursive struct test failed");
}

/// Golden test: enum definition + match expression.
#[test]
fn test_type_enum_match() {
    run_golden_test("type_enum_match");
}

/// Golden test: array init, index load/store, len.
#[test]
fn test_type_array_ops() {
    run_golden_test("type_array_ops");
}

/// Golden test: array primitive methods — add, remove_at, insert, contains, slice.
///
/// Proves STR-01 (array method wiring): all five array mutation/query methods emit
/// the correct IL opcodes (ArrayAdd, ArrayRemove, ArrayInsert, ArrayContains, ArraySlice)
/// through the full compile pipeline, including string-content equality for contains.
#[test]
fn test_array_primitives() {
    run_golden_test("array_primitives");
}

// ─── Section I: Function golden tests (additional) ────────────────────────────

/// Golden test: early return + tail expression.
#[test]
fn test_fn_multi_return() {
    run_golden_test("fn_multi_return");
}

/// Golden test: string parameters + extern call (::log::info).
#[test]
fn test_fn_string_param() {
    run_golden_test("fn_string_param");
}

// ─── Section J: Advanced feature golden tests ─────────────────────────────────

/// Golden test: defer block.
#[test]
fn test_adv_defer() {
    run_golden_test("adv_defer");
}

/// Golden test: atomic block.
#[test]
fn test_adv_atomic() {
    run_golden_test("adv_atomic");
}

/// Golden test: Option match with Some/None.
#[test]
fn test_adv_option_match() {
    run_golden_test("adv_option_match");
}

/// Golden test: Option<T> intrinsic methods (is_some, is_none, unwrap).
///
/// Locks that Option intrinsic method calls type-check correctly and emit
/// the expected IS_SOME, IS_NONE, UNWRAP instructions.
#[test]
fn test_option_methods() {
    run_golden_test("option_methods");
}

/// Golden test: force-unwrap operator (n!).
///
/// Locks that the force-unwrap operator on Option compiles without E9001
/// and emits a LoadString + Crash instruction for the None arm.
/// Regression test for the TypedExpr::Crash fix (was using TypedExpr::Error
/// which caused expr_has_error to skip the entire function).
#[test]
fn test_force_unwrap() {
    run_golden_test("force_unwrap");
}

/// Golden test: Result<T, E> intrinsic methods (is_ok, is_err, unwrap, unwrap_err).
///
/// Locks that Result intrinsic method calls type-check correctly and emit
/// the expected IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR instructions.
#[test]
fn test_result_methods() {
    run_golden_test("result_methods");
}

// --- Section K: Comprehensive golden tests ---------------------------------

/// Golden test: comprehensive quest system exercising enums, functions, match,
/// control flow, arrays, Option, defer, atomic, and dialogue builtins.
///
/// Integration-level regression test — a single large file combining many
/// language features in a realistic game-scripting scenario.
#[test]
fn test_quest_system() {
    run_golden_test("quest_system");
}

// --- Section L: Dialogue golden tests ------------------------------------------

/// Golden test: dialogue/function interplay.
///
/// Exercises dlg blocks calling fn helpers, fn calling dlg,
/// Tier 1 speaker params, $ code escapes, and -> transitions.
/// First golden test using actual dlg syntax through the full pipeline.
#[test]
fn test_dlg_fn_mix() {
    run_golden_test("dlg_fn_mix");
}

/// Golden test: full quest pattern with entity + dialogue + functions + enum.
///
/// Exercises entity declaration, dlg blocks with speaker lines,
/// helper fn declarations, enum match, and $ if conditionals
/// in a realistic game scripting scenario.
#[test]
fn test_dlg_quest_pattern() {
    run_golden_test("dlg_quest_pattern");
}

/// Golden test: dialogue text interpolation with {expr} inline syntax.
///
/// Exercises lower_dlg_text -> lower_fmt_string pipeline with DlgTextSegment::Expr
/// nodes, covering both string-typed and int-typed interpolation in speaker lines.
/// Regression anchor for DAP-02: verifies {name} and {count} interpolation in dlg text.
#[test]
fn test_dlg_interp() {
    run_golden_test("dlg_interp");
}

// ─── Section L: Entity namespace golden tests ─────────────────────────────────

/// Golden test: Entity.getOrCreate<T>() emits GET_OR_CREATE instruction.
///
/// Locks the emission of the dedicated GET_OR_CREATE opcode (0x0806) for
/// singleton entity retrieval via the Entity namespace.
#[test]
fn test_entity_get_or_create() {
    run_golden_test("entity_get_or_create");
}

// ─── Section M: String escape verification ──────────────────────────────────

/// Helper: collect all string literals referenced by LoadString instructions
/// from the first method body of a compiled module.
fn collect_load_strings(module: &Module) -> Vec<String> {
    use std::io::Cursor;
    use writ_module::instruction::Instruction;
    let body = &module.method_bodies[0];
    let mut cursor = Cursor::new(&body.code);
    let mut strings = Vec::new();
    while (cursor.position() as usize) < body.code.len() {
        match Instruction::decode(&mut cursor) {
            Ok(Instruction::LoadString { string_idx, .. }) => {
                if let Ok(s) = writ_module::heap::read_string(&module.string_heap, string_idx) {
                    strings.push(s.to_string());
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    strings
}

/// Verify that basic string literals have quotes stripped and escape sequences resolved.
#[test]
fn test_string_escape_basic() {
    let module = compile_to_module(r#"
        fn main() {
            let a: string = "hello";
            let b: string = "fancy\"string";
            let c: string = "line1\nline2";
            let d: string = "col1\tcol2";
            let e: string = "back\\slash";
            let f: string = "null\0char";
            let g: string = "cr\rhere";
            let h: string = "a\"b\\c\nd";
        }
    "#);
    let strings = collect_load_strings(&module);
    assert_eq!(strings[0], "hello");
    assert_eq!(strings[1], "fancy\"string");
    assert_eq!(strings[2], "line1\nline2");
    assert_eq!(strings[3], "col1\tcol2");
    assert_eq!(strings[4], "back\\slash");
    assert_eq!(strings[5], "null\0char");
    assert_eq!(strings[6], "cr\rhere");
    assert_eq!(strings[7], "a\"b\\c\nd");
}

/// Verify that formattable string text segments have escapes resolved.
#[test]
fn test_string_escape_formattable() {
    let module = compile_to_module(r#"
        fn main() {
            let x: int = 42;
            let s: string = $"tab\there {x.into<string>()} end\"done";
        }
    "#);
    let strings = collect_load_strings(&module);
    // First LoadString is the "tab\there " text segment (escape resolved)
    assert_eq!(strings[0], "tab\there ");
    // Last LoadString is the " end\"done" text segment
    let last = strings.last().unwrap();
    assert_eq!(last, " end\"done");
}

/// Verify that string literals never contain surrounding quotes in the string heap.
#[test]
fn test_string_no_surrounding_quotes() {
    let module = compile_to_module(r#"
        fn main() {
            let s: string = "test";
        }
    "#);
    let strings = collect_load_strings(&module);
    assert_eq!(strings[0], "test");
    assert!(!strings[0].starts_with('"'));
    assert!(!strings[0].ends_with('"'));
}

/// Function overloading: two `pub fn test` with different parameter types.
#[test]
fn test_fn_overload() {
    run_golden_test("fn_overload");
}

// ─── Section N: Conditional compilation golden tests ─────────────────────────

/// Golden test: [Conditional("debug")] with active condition.
///
/// Proves COND-01: when --condition debug is active, the [Conditional("debug")] variant
/// of greet is emitted and the fallback is suppressed. Only ONE greet MethodDef appears
/// in the output, containing "Debug greeting".
#[test]
fn test_conditional_active() {
    run_golden_test_with_conditions("conditional_active", &["debug"]);
}

/// Golden test: [Conditional("debug")] with no active condition.
///
/// Proves COND-02: when no conditions are active, the fallback greet is emitted and
/// the [Conditional("debug")] variant is suppressed. Only ONE greet MethodDef appears
/// in the output, containing "Default greeting".
#[test]
fn test_conditional_inactive() {
    run_golden_test_with_conditions("conditional_inactive", &[]);
}

// ─── Section O: Reflection golden tests ──────────────────────────────────────

/// Golden test: typeof() lowers to TYPEOF instruction.
///
/// Locks REFL-01: `typeof(Point)` in a function body emits a TYPEOF instruction
/// with the correct type token for the Point struct TypeDef.
#[test]
fn golden_refl_typeof_basic() {
    run_golden_test("refl_typeof_basic");
}

/// Golden test: typeof equality comparison lowers to TYPEOF + CMPEQI.
///
/// Locks REFL-09: two `typeof(Alpha)` calls emit two separate TYPEOF instructions,
/// and the `==` comparison between them emits CMPEQI (pointer identity on interned
/// Type singletons). Also tests typeof inequality: `typeof(Alpha) == typeof(Beta)`
/// emits TYPEOF with two different type tokens.
#[test]
fn golden_refl_typeof_equality() {
    run_golden_test("refl_typeof_equality");
}

/// Golden test: static typeof(Contract) vs dynamic get_type() emit different TYPEOF tokens.
///
/// Gap closure: VERIFICATION gap 2 — static-vs-dynamic subtype typeof distinction.
/// Locks the invariant that typeof(Animal) targets the contract TypeDef token while
/// Dog's auto-generated get_type() body targets the Dog struct TypeDef token.
#[test]
fn golden_refl_typeof_subtype() {
    run_golden_test("refl_typeof_subtype");
}

/// Golden test: typeof() on an enum type emits TYPEOF instruction.
///
/// Gap closure: VERIFICATION gap 1 — enum typeof golden test.
#[test]
fn golden_refl_typeof_enum() {
    run_golden_test("refl_typeof_enum");
}

/// Golden test: typeof() on an entity type emits TYPEOF instruction.
///
/// Gap closure: VERIFICATION gap 1 — entity typeof golden test.
#[test]
fn golden_refl_typeof_entity() {
    run_golden_test("refl_typeof_entity");
}

/// Golden test: typeof() on a class type emits TYPEOF instruction.
///
/// Gap closure: VERIFICATION gap 1 — class typeof golden test.
#[test]
fn golden_refl_typeof_class() {
    run_golden_test("refl_typeof_class");
}

// ─── Section: Closure capture golden tests ────────────────────────────────────

/// Golden test: closure capturing a local variable from the enclosing scope.
///
/// Verifies the full capture pipeline end-to-end:
/// - The type checker populates the captures list on the Lambda IR node
/// - The pre-scanner registers a __closure_0 TypeDef with field `x`
/// - The call-site emitter produces NEW + SetField for the capture
/// - The closure body emitter produces GetField to load the captured variable
/// - NEW_DELEGATE wires the capture struct to the closure method
#[test]
fn test_closure_capture() {
    run_golden_test("closure_capture");
}

// ─── Section P: String utility golden tests (Phase 116) ───────────────────

/// Golden test for string utility methods: split, trim, starts_with, ends_with,
/// contains, replace, to_upper, to_lower.
///
/// Proves STR-01 through STR-06: all 8 string utility methods emit the correct
/// IL opcodes (StrSplit, StrTrim, StrStartsWith, StrEndsWith, StrContains,
/// StrReplace, StrToUpper, StrToLower) through the full compile pipeline.
#[test]
fn test_string_utilities() {
    run_golden_test("string_utilities");
}

// ─── Section Q: Phase 117 pre-work golden tests ───────────────────────────────

/// Golden test: generic class with inherent impl block.
///
/// Validates that `pub class Box<T>` with `impl Box<T> { fn get(self) -> T }` syntax
/// compiles through the full pipeline without errors. This is a mandatory pre-work gate
/// for Phase 117 (collections) — all four collection types (List, Map, Set, HashMap) use
/// the same `impl ClassName<T>` inherent-impl pattern.
///
/// Proves: compiler handles generic inherent impl blocks for user-defined generic classes.
#[test]
fn golden_generic_inherent_impl() {
    run_golden_test("generic_inherent_impl");
}

/// Golden test: simple pub class compiles as a single-file source.
///
/// Proves that a `pub class Stub { value: int }` compiles cleanly through the full
/// pipeline — the class declaration, field access, and construction all emit correct IL.
/// This is the simplest pub class that will serve as a library type in Phase 117.
#[test]
fn golden_lib_preload_stub() {
    run_golden_test("lib_preload_stub");
}

/// Cross-file resolution test: library type visible to user code.
///
/// Proves that the multi-file compiler pipeline correctly resolves types declared in one
/// "file" (the library) from another "file" (user code). This validates the cross-file
/// resolution path that `RuntimeBuilder::with_library()` relies on at the IL/runtime level.
///
/// Uses writ_compiler pipeline stages directly since writ-golden does not depend on
/// writ-runtime. The runtime-level test will come in Plan 03.
#[test]
fn lib_preload_cross_file_resolution() {
    // Library source: pub class Stub with one int field.
    let lib_src: &'static str = Box::leak("pub class Stub { value: int }".to_string().into_boxed_str());
    // User source: constructs and accesses a Stub.
    let user_src: &'static str = Box::leak(
        "fn main() {\n    let s: Stub = new Stub { value: 1 };\n    let v: int = s.value;\n}"
            .to_string()
            .into_boxed_str(),
    );

    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            use writ_diagnostics::{FileId, Severity};

            let lib_fid = FileId(0);
            let user_fid = FileId(1);

            // Parse + lower library
            let (lib_cst, lib_errs) = writ_parser::parse(lib_src);
            assert!(lib_errs.is_empty(), "lib parse errors: {:?}", lib_errs);
            let (lib_ast, lib_lower_errs) = writ_compiler::lower(lib_cst.unwrap());
            assert!(lib_lower_errs.is_empty(), "lib lower errors: {:?}", lib_lower_errs);

            // Parse + lower user
            let (user_cst, user_errs) = writ_parser::parse(user_src);
            assert!(user_errs.is_empty(), "user parse errors: {:?}", user_errs);
            let (user_ast, user_lower_errs) = writ_compiler::lower(user_cst.unwrap());
            assert!(user_lower_errs.is_empty(), "user lower errors: {:?}", user_lower_errs);

            // Resolve together: lib at FileId(0), user at FileId(1)
            let asts = vec![(lib_fid, &lib_ast), (user_fid, &user_ast)];
            let paths = vec![(lib_fid, "lib.writ"), (user_fid, "user.writ")];
            let (resolved, resolve_diags) = writ_compiler::resolve::resolve(&asts, &paths, &[]);
            let has_errors = resolve_diags.iter().any(|d| d.severity == Severity::Error);
            assert!(
                !has_errors,
                "resolution errors: {:?}",
                resolve_diags
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
                    .map(|d| &d.message)
                    .collect::<Vec<_>>()
            );

            // Typecheck
            let (_typed, _interner, _env, type_diags) =
                writ_compiler::check::typecheck(resolved, &asts, &[]);
            let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
            assert!(
                !has_type_errors,
                "type errors: {:?}",
                type_diags
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
                    .map(|d| &d.message)
                    .collect::<Vec<_>>()
            );

            Ok(())
        })
        .unwrap()
        .join()
        .unwrap();

    handle.unwrap();
}

/// Golden test: List<T> collection class compiles to valid IL.
///
/// Proves that a generic class with impl<T> List<T> { ... } backed by T[] compiles
/// cleanly through the full pipeline. Exercises add, get, set, len, remove_at, contains.
///
#[test]
fn golden_coll_list_basic() {
    run_golden_test("coll_list_basic");
}

/// Golden test: Map<K,V> collection class compiles to valid IL.
///
/// Proves that Map<K: Ord + Eq, V> with impl<K: Ord + Eq, V> Map<K, V> { ... } compiles.
/// Exercises set, get, has, remove, len with string keys and int values.
///
#[test]
fn golden_coll_map_basic() {
    run_golden_test("coll_map_basic");
}

/// Golden test: Set<T> collection class compiles to valid IL.
///
/// Proves that Set<T: Eq> with impl<T: Eq> Set<T> { ... } compiles.
/// Exercises add (with duplicate prevention), contains, len, remove.
///
#[test]
fn golden_coll_set_basic() {
    run_golden_test("coll_set_basic");
}

/// Golden test: HashMap<K,V> collection class compiles to valid IL.
///
/// Proves that HashMap<K: Hashable, V> with impl<K: Hashable, V> HashMap<K, V> { ... } compiles.
/// Exercises set, get, has, remove, len with string keys and int values.
///
#[test]
fn golden_coll_hashmap_basic() {
    run_golden_test("coll_hashmap_basic");
}

/// Golden test: List<T>.map() higher-order method compiles to valid IL.
///
/// Proves that a closure passed to List.map() dispatches via CALL_INDIRECT and that
/// the resulting doubled List<int> can be indexed and queried for length.
///
#[test]
fn golden_coll_list_map() {
    run_golden_test("coll_list_map");
}

/// Golden test: List<T>.filter() higher-order method compiles to valid IL.
///
/// Proves that a predicate closure passed to List.filter() dispatches via CALL_INDIRECT
/// and that the resulting filtered List<int> can be indexed and queried for length.
///
#[test]
fn golden_coll_list_filter() {
    run_golden_test("coll_list_filter");
}

/// Golden test: List<T>.reduce() higher-order method compiles to valid IL.
///
/// Proves that a binary accumulator closure passed to List.reduce() dispatches via
/// CALL_INDIRECT and that the final accumulated value is correctly returned.
///
#[test]
fn golden_coll_list_reduce() {
    run_golden_test("coll_list_reduce");
}

/// Golden test: for-in loop over class implementing Iterable<T> compiles to valid IL.
///
/// Proves that `for x in list` over a List<int> implementing Iterable<T> desugars to
/// CALL_VIRT iterator() + loop(CALL_VIRT next() + IS_NONE + UNWRAP). Exercises the
/// full iterator protocol: class Iterable detection, CALL_VIRT dispatch through the
/// Iterable and Iterator contracts, IS_NONE exit check, and UNWRAP element extraction.
///
#[test]
fn golden_iter_for_in_list() {
    run_golden_test("iter_for_in_list");
}

// ─── Section N: Array semantics verification (Phase 120) ─────────────────────

/// Test: removed array methods (add, remove_at, insert, contains) produce compiler errors.
///
/// Verifies that calling `arr.add(x)` on a T[] receiver is rejected with an
/// "unknown method" error — proving ARR-01 and ARR-05 enforcement (Phase 120).
/// A future change that accidentally re-adds these methods would cause this test to fail.
#[test]
fn test_array_removed_methods_produce_error() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/array_removed_methods.writ")
    ).expect("could not read array_removed_methods.writ");

    let src_static: &'static str = Box::leak(src.into_boxed_str());

    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            let file_id = writ_diagnostics::FileId(0);
            let (cst_opt, parse_errs) = writ_parser::parse(src_static);
            assert!(parse_errs.is_empty(), "unexpected parse errors: {:?}", parse_errs);
            let cst = cst_opt.expect("parse failed: no CST output");
            let (ast, lower_errs) = writ_compiler::lower(cst);
            assert!(lower_errs.is_empty(), "unexpected lowering errors: {:?}", lower_errs);
            let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
                &[(file_id, &ast)], &[(file_id, "test.writ")],
                &[],
            );
            let resolve_errors: Vec<_> = resolve_diags.iter()
                .filter(|d| d.severity == writ_diagnostics::Severity::Error)
                .collect();
            assert!(resolve_errors.is_empty(), "unexpected resolve errors: {:?}", resolve_errors);
            let (_typed_ast, _interner, _type_env, type_diags) = writ_compiler::check::typecheck(resolved, &[(file_id, &ast)], &[]);
            // Expect at least one type error — the unknown method `add` on int[]
            let has_method_error = type_diags.iter().any(|d|
                d.severity == writ_diagnostics::Severity::Error
            );
            if has_method_error {
                Ok(())
            } else {
                Err(format!(
                    "Expected type error for arr.add() on int[], but got {} diagnostics: {:?}",
                    type_diags.len(),
                    type_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
                ))
            }
        })
        .expect("thread spawn failed");

    handle.join().expect("thread panicked").expect("array_removed_methods test failed");
}
