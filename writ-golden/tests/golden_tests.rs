/// Golden test harness for the Writ compiler.
///
/// Workflow:
///   - `compile_and_disassemble(src)` — full pipeline (parse->lower->resolve->typecheck->emit_bodies)
///     inside a 16 MB stack thread, then round-trip through Module::from_bytes, then disassemble.
///   - `run_golden_test(name)` — read `tests/golden/{name}.writ`, compile+disassemble, then either
///     bless (BLESS=1 env var) or compare against `tests/golden/{name}.expected`.
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
            writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)]).map_err(
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

/// Write `actual` IL text to the expected file for golden `name` in `golden_dir`.
///
/// Exposed as `pub(crate)` so tests can exercise the bless path with a temp dir
/// without touching env vars.
pub(crate) fn bless_golden(name: &str, actual: &str, golden_dir: &Path) {
    let expected_path = golden_dir.join(format!("{name}.expected"));
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

    let expected = std::fs::read_to_string(&expected_path).unwrap_or_else(|_| {
        panic!(
            "assembly file not found — run BLESS=1 cargo test -p writ-golden -- {name} to create it\n  missing: {}",
            expected_path.display()
        )
    });

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

    let written_path = tmp.path().join("my_test.expected");
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
