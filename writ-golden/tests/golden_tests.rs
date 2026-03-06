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
            );
            let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
            if has_type_errors {
                let msgs: Vec<_> = type_diags.iter().map(|d| d.message.clone()).collect();
                return Err(format!("type error(s): {}", msgs.join("; ")));
            }

            // Stage 5: IL codegen (includes metadata + bodies + serialization)
            writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)], true, &[]).map_err(
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
                &[(file_id, &ast)], &[(file_id, "test.writ")]
            );
            let resolve_errors: Vec<_> = resolve_diags.iter()
                .filter(|d| d.severity == writ_diagnostics::Severity::Error)
                .collect();
            assert!(resolve_errors.is_empty(), "unexpected resolve errors: {:?}", resolve_errors);
            let (_typed_ast, _interner, _type_env, type_diags) = writ_compiler::check::typecheck(
                resolved, &[(file_id, &ast)]
            );
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
