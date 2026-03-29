//! AnalysisHost: wraps the Writ compiler pipeline to produce LSP diagnostics.
//!
//! Supports both standalone single-file analysis and project-mode analysis
//! (when a `writ.toml` is present).
//!
//! ## SPLIT-12 review (Phase 64)
//!
//! Reviewed for split opportunities at 1,415 lines. Conclusion: no split.
//! Production code is ~390 lines (one `AnalysisHost` struct, two public methods
//! `analyze_standalone` and `analyze_project`, three private diagnostic helpers).
//! The remaining 1,025 lines are inline integration tests. Splitting the test
//! block into a separate integration test file would require test modifications
//! (violating the "tests pass without modification" success criterion). Splitting
//! the production code would fragment a tightly-coupled 5-stage sequential
//! pipeline (parse -> lower -> resolve -> typecheck -> return) that shares local
//! variables across stages.

use std::path::Path;
use writ_diagnostics::{Diagnostic, FileId};

/// Result of running the analysis pipeline on one or more Writ source files.
pub struct AnalysisResult {
    /// All diagnostics from parse/lower/resolve/typecheck stages.
    pub diagnostics: Vec<Diagnostic>,
    /// FileId -> (display_path, source_text) mapping for span conversion.
    pub file_sources: Vec<(FileId, String, String)>,
    /// Typed AST when typecheck succeeds (None if resolve or typecheck panicked/failed).
    pub typed_ast: Option<writ_compiler::check::ir::TypedAst>,
    /// Type interner when typecheck succeeds.
    pub ty_interner: Option<writ_compiler::check::ty::TyInterner>,
    /// Type environment when typecheck succeeds (fn_sigs, struct_fields, impl_index, etc.).
    pub type_env: Option<writ_compiler::check::env::TypeEnv>,
}

/// Stateless analysis host that wraps the Writ compiler pipeline.
///
/// State like caching and incremental analysis is future scope (Phase 54+).
pub struct AnalysisHost;

impl AnalysisHost {
    /// Analyze a single Writ source file, returning all diagnostics from all stages.
    ///
    /// The `source` is the full text of the file. `display_path` is a human-readable
    /// path for diagnostic messages (need not exist on disk).
    ///
    /// Cascade strategy: all four stages run even when earlier stages have errors.
    /// If resolve or typecheck panics on error AST nodes, the panic is caught and
    /// an internal diagnostic is emitted rather than crashing the server.
    pub fn analyze_standalone(source: String, display_path: String) -> AnalysisResult {
        let file_id = FileId(0);
        // Box::leak so the parser gets &'static str (consistent with run_pipeline)
        let src: &'static str = Box::leak(source.clone().into_boxed_str());

        let mut all_diags: Vec<Diagnostic> = Vec::new();
        let file_sources = vec![(file_id, display_path.clone(), source)];

        // Stage 1: Parse
        let (cst_opt, parse_errs) = writ_parser::parse(src);
        for err in &parse_errs {
            all_diags.push(crate::convert::parse_error_to_diag(err, file_id));
        }

        let cst = match cst_opt {
            Some(cst) => cst,
            None => {
                // Parse failed with no output — return parse errors only
                return AnalysisResult {
                    diagnostics: all_diags,
                    file_sources,
                    typed_ast: None,
                    ty_interner: None,
                    type_env: None,
                };
            }
        };

        // Stage 2: Lower CST -> AST
        let (ast, lower_errs) = writ_compiler::lower(cst);
        for err in &lower_errs {
            all_diags.push(err.to_diagnostic(file_id));
        }

        let per_file_asts: Vec<(FileId, writ_compiler::Ast)> = vec![(file_id, ast)];

        // Build reference slices for stages 3-4
        let asts_refs: Vec<(FileId, &writ_compiler::Ast)> =
            per_file_asts.iter().map(|(fid, ast)| (*fid, ast)).collect();
        let path_refs: Vec<(FileId, &str)> =
            vec![(file_id, display_path.as_str())];

        // Stage 3: Resolve — wrapped in catch_unwind to prevent panics from crashing the server
        let resolve_result = std::panic::catch_unwind(|| {
            writ_compiler::resolve::resolve(&asts_refs, &path_refs, &[])
        });

        let resolved = match resolve_result {
            Ok((resolved, resolve_diags)) => {
                all_diags.extend(resolve_diags);
                Some(resolved)
            }
            Err(_) => {
                all_diags.push(internal_stage_panic_diag(file_id, "resolve"));
                None
            }
        };

        // Stage 4: Typecheck — only if resolve succeeded
        let mut typed_ast_out = None;
        let mut ty_interner_out = None;
        let mut type_env_out = None;

        if let Some(resolved) = resolved {
            let typecheck_result = std::panic::catch_unwind(|| {
                writ_compiler::check::typecheck(resolved, &asts_refs, &[])
            });

            match typecheck_result {
                Ok((typed, interner, type_env, type_diags)) => {
                    all_diags.extend(type_diags);
                    typed_ast_out = Some(typed);
                    ty_interner_out = Some(interner);
                    type_env_out = Some(type_env);
                }
                Err(_) => {
                    all_diags.push(internal_stage_panic_diag(file_id, "typecheck"));
                }
            }
        }

        // Stage 5+6: Emit + Runtime — only if no compile errors
        let has_errors = all_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
        if !has_errors {
            if let (Some(typed), Some(interner)) = (&typed_ast_out, &ty_interner_out) {
                let sources_slice: Vec<(FileId, &str)> = vec![(file_id, src)];
                let runtime_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    try_runtime_diagnostic(typed, interner, &asts_refs, &sources_slice, file_id, src)
                }));
                if let Ok(Some(diag)) = runtime_result {
                    all_diags.push(diag);
                }
            }
        }

        AnalysisResult {
            diagnostics: all_diags,
            file_sources,
            typed_ast: typed_ast_out,
            ty_interner: ty_interner_out,
            type_env: type_env_out,
        }
    }

    /// Analyze all Writ source files in a project rooted at `project_root`.
    ///
    /// Discovers files via `writ.toml`. If `writ.toml` is absent and
    /// `trigger_file` is Some, falls back to standalone analysis of that file.
    ///
    /// When `trigger_source` is provided, it is used as the content for the
    /// trigger file instead of reading from disk. This ensures the LSP analyses
    /// the live editor buffer rather than stale on-disk content.
    ///
    /// If the trigger file is not inside any configured source directory, it is
    /// still included in the analysis so that single-file LSP features (hover,
    /// goto-def, diagnostics) continue to work.
    pub fn analyze_project(
        project_root: &Path,
        trigger_file: Option<&str>,
        trigger_source: Option<String>,
    ) -> AnalysisResult {
        let config = match writ_compiler::config::load_config(project_root) {
            Ok(cfg) => cfg,
            Err(writ_compiler::config::ConfigError::MissingToml(_)) => {
                // Fallback: analyze the triggering file as standalone
                if let Some(path) = trigger_file {
                    let content = trigger_source.unwrap_or_else(|| {
                        std::fs::read_to_string(path).unwrap_or_default()
                    });
                    if content.is_empty() {
                        return AnalysisResult {
                            diagnostics: Vec::new(),
                            file_sources: Vec::new(),
                            typed_ast: None,
                            ty_interner: None,
                            type_env: None,
                        };
                    }
                    return Self::analyze_standalone(content, path.to_string());
                }
                return AnalysisResult {
                    diagnostics: Vec::new(),
                    file_sources: Vec::new(),
                    typed_ast: None,
                    ty_interner: None,
                    type_env: None,
                };
            }
            Err(e) => {
                return AnalysisResult {
                    diagnostics: vec![config_error_diag(&e)],
                    file_sources: Vec::new(),
                    typed_ast: None,
                    ty_interner: None,
                    type_env: None,
                };
            }
        };

        let source_files = match writ_compiler::config::discover_source_files(project_root, &config) {
            Ok(files) => files,
            Err(e) => {
                return AnalysisResult {
                    diagnostics: vec![config_error_diag(&e)],
                    file_sources: Vec::new(),
                    typed_ast: None,
                    ty_interner: None,
                    type_env: None,
                };
            }
        };

        // Determine the canonical path of the trigger file for matching against
        // discovered source files. On Windows, canonicalize normalizes separators
        // and resolves symlinks so comparison is reliable.
        let trigger_canonical: Option<std::path::PathBuf> = trigger_file
            .map(std::path::Path::new)
            .and_then(|p| p.canonicalize().ok());

        // Check whether the trigger file is among the discovered source files.
        let trigger_in_sources = trigger_canonical.as_ref().is_some_and(|tc| {
            source_files.iter().any(|p| {
                p.canonicalize().ok().as_ref() == Some(tc)
            })
        });

        let mut all_diags: Vec<Diagnostic> = Vec::new();
        let mut file_sources: Vec<(FileId, String, String)> = Vec::new();
        let mut per_file_asts: Vec<(FileId, writ_compiler::Ast)> = Vec::new();

        // Stage 1+2: Parse and lower each file
        for (idx, path) in source_files.iter().enumerate() {
            let file_id = FileId(idx as u32);
            let display_path = path.display().to_string();

            // Use the in-memory trigger source if this is the trigger file,
            // otherwise read from disk.
            let content = match (trigger_source.as_ref(), trigger_canonical.as_ref()) {
                (Some(ts), Some(tc)) if path.canonicalize().ok().as_ref() == Some(tc) => {
                    ts.clone()
                }
                _ => {
                    match std::fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(e) => {
                            all_diags.push(io_error_diag(&display_path, &e));
                            continue;
                        }
                    }
                }
            };
            file_sources.push((file_id, display_path.clone(), content.clone()));

            let src: &'static str = Box::leak(content.into_boxed_str());

            // Stage 1: Parse
            let (cst_opt, parse_errs) = writ_parser::parse(src);
            for err in &parse_errs {
                all_diags.push(crate::convert::parse_error_to_diag(err, file_id));
            }

            let cst = match cst_opt {
                Some(cst) => cst,
                None => continue,
            };

            // Stage 2: Lower
            let (ast, lower_errs) = writ_compiler::lower(cst);
            for err in &lower_errs {
                all_diags.push(err.to_diagnostic(file_id));
            }
            per_file_asts.push((file_id, ast));
        }

        // If the trigger file was not in the discovered source directories,
        // include it as an additional file so LSP features still work.
        if !trigger_in_sources
            && let Some(path) = trigger_file {
                let file_id = FileId(file_sources.len() as u32);
                let display_path = path.to_string();

                let content = trigger_source.unwrap_or_else(|| {
                    std::fs::read_to_string(path).unwrap_or_default()
                });

                if !content.is_empty() {
                    file_sources.push((file_id, display_path, content.clone()));

                    let src: &'static str = Box::leak(content.into_boxed_str());

                    let (cst_opt, parse_errs) = writ_parser::parse(src);
                    for err in &parse_errs {
                        all_diags.push(crate::convert::parse_error_to_diag(err, file_id));
                    }

                    if let Some(cst) = cst_opt {
                        let (ast, lower_errs) = writ_compiler::lower(cst);
                        for err in &lower_errs {
                            all_diags.push(err.to_diagnostic(file_id));
                        }
                        per_file_asts.push((file_id, ast));
                    }
                }
            }

        if per_file_asts.is_empty() {
            return AnalysisResult {
                diagnostics: all_diags,
                file_sources,
                typed_ast: None,
                ty_interner: None,
                type_env: None,
            };
        }

        let asts_refs: Vec<(FileId, &writ_compiler::Ast)> =
            per_file_asts.iter().map(|(fid, ast)| (*fid, ast)).collect();
        let path_refs: Vec<(FileId, &str)> = file_sources
            .iter()
            .map(|(fid, path, _)| (*fid, path.as_str()))
            .collect();

        // Stage 3: Resolve
        let resolve_result = std::panic::catch_unwind(|| {
            writ_compiler::resolve::resolve(&asts_refs, &path_refs, &[])
        });

        let resolved = match resolve_result {
            Ok((resolved, resolve_diags)) => {
                all_diags.extend(resolve_diags);
                Some(resolved)
            }
            Err(_) => {
                all_diags.push(internal_stage_panic_diag(FileId(0), "resolve"));
                None
            }
        };

        // Stage 4: Typecheck
        let mut typed_ast_out = None;
        let mut ty_interner_out = None;
        let mut type_env_out = None;

        if let Some(resolved) = resolved {
            let typecheck_result = std::panic::catch_unwind(|| {
                writ_compiler::check::typecheck(resolved, &asts_refs, &[])
            });

            match typecheck_result {
                Ok((typed, interner, type_env, type_diags)) => {
                    all_diags.extend(type_diags);
                    typed_ast_out = Some(typed);
                    ty_interner_out = Some(interner);
                    type_env_out = Some(type_env);
                }
                Err(_) => {
                    all_diags.push(internal_stage_panic_diag(FileId(0), "typecheck"));
                }
            }
        }

        // Stage 5+6: Emit + Runtime — only if no compile errors
        let has_errors = all_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
        if !has_errors {
            if let (Some(typed), Some(interner)) = (&typed_ast_out, &ty_interner_out) {
                let sources_slice: Vec<(FileId, &str)> = file_sources
                    .iter()
                    .map(|(fid, _, src)| (*fid, src.as_str()))
                    .collect();
                let primary_source = file_sources.first()
                    .map(|(_, _, s)| s.as_str())
                    .unwrap_or("");
                let runtime_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    try_runtime_diagnostic(typed, interner, &asts_refs, &sources_slice, FileId(0), primary_source)
                }));
                if let Ok(Some(diag)) = runtime_result {
                    all_diags.push(diag);
                }
            }
        }

        AnalysisResult {
            diagnostics: all_diags,
            file_sources,
            typed_ast: typed_ast_out,
            ty_interner: ty_interner_out,
            type_env: type_env_out,
        }
    }
}

/// Create an internal diagnostic when a compiler stage panics.
fn internal_stage_panic_diag(file_id: FileId, stage: &str) -> Diagnostic {
    use chumsky::span::SimpleSpan;
    let span = SimpleSpan { start: 0, end: 0, context: () };
    Diagnostic::error(
        "E9999",
        format!("internal error: {} stage panicked (error AST node)", stage),
    )
    .with_primary(file_id, span, "analysis aborted at this stage")
    .build()
}

/// Create a diagnostic for an I/O error when reading a source file.
fn io_error_diag(path: &str, err: &std::io::Error) -> Diagnostic {
    use chumsky::span::SimpleSpan;
    let span = SimpleSpan { start: 0, end: 0, context: () };
    Diagnostic::error(
        "E9998",
        format!("failed to read {}: {}", path, err),
    )
    .with_primary(FileId(0), span, "I/O error")
    .build()
}

/// Create a diagnostic for a config error.
fn config_error_diag(err: &writ_compiler::config::ConfigError) -> Diagnostic {
    use chumsky::span::SimpleSpan;
    let span = SimpleSpan { start: 0, end: 0, context: () };
    Diagnostic::error(
        "E9997",
        format!("project configuration error: {}", err),
    )
    .with_primary(FileId(0), span, "config error")
    .build()
}

/// Attempt to compile and run the script, returning a runtime crash diagnostic if
/// the script crashes. Returns None if compilation fails, no `main` entry point
/// exists, or the script runs without crashing.
///
/// This is Stage 5 (emit) + Stage 6 (runtime execution) of the analysis pipeline.
/// Only called when no compile errors exist (guards: parse, lower, resolve, typecheck
/// must all succeed first).
fn try_runtime_diagnostic(
    typed_ast: &writ_compiler::check::ir::TypedAst,
    interner: &writ_compiler::check::ty::TyInterner,
    asts: &[(FileId, &writ_compiler::Ast)],
    sources: &[(FileId, &str)],
    file_id: FileId,
    file_source: &str,
) -> Option<Diagnostic> {
    use chumsky::span::SimpleSpan;

    // Stage 5: Emit — produce binary module bytes with debug info
    // LSP always compiles with no active conditions (conditional compilation is a CLI concern).
    let active_conditions = std::collections::HashSet::new();
    let bytes = writ_compiler::emit_bodies(typed_ast, interner, asts, true, sources, &active_conditions).ok()?;

    // Parse the binary module
    let module = writ_module::Module::from_bytes(&bytes).ok()?;

    // Find the `main` entry point
    let main_method_idx = module.method_defs.iter().enumerate().find_map(|(idx, def)| {
        let name = writ_module::heap::read_string(&module.string_heap, def.name).ok()?;
        if name == "main" { Some(idx) } else { None }
    })?;

    // Stage 6: Runtime execution
    let mut runtime = writ_runtime::RuntimeBuilder::new(module).build().ok()?;
    let task_id = runtime.spawn_task(main_method_idx, vec![]).ok()?;
    let tick_result = runtime.tick(0.0, writ_runtime::ExecutionLimit::Instructions(100_000));

    // If execution limit was reached, don't report anything (might be a game loop)
    if matches!(tick_result, writ_runtime::TickResult::ExecutionLimitReached) {
        return None;
    }

    // Check if the task crashed
    let crash = runtime.crash_info(task_id)?;

    // Build primary span from first frame's source location (top of stack = crash site)
    let primary_span = if let Some(frame) = crash.stack_trace.first() {
        if frame.line > 0 {
            let offset = line_col_to_offset(file_source, frame.line, frame.column);
            SimpleSpan { start: offset, end: offset + 1, context: () }
        } else {
            SimpleSpan { start: 0, end: 0, context: () }
        }
    } else {
        SimpleSpan { start: 0, end: 0, context: () }
    };

    Some(
        Diagnostic::error("R0001", crash.format_stacktrace())
            .with_primary(file_id, primary_span, "crash occurred here")
            .build()
    )
}

/// Convert a 1-based line/column to a byte offset in `source`.
fn line_col_to_offset(source: &str, line: u32, col: u32) -> usize {
    let target_line = line.saturating_sub(1) as usize;
    let mut current_line = 0usize;
    for (idx, ch) in source.char_indices() {
        if current_line == target_line {
            let col_offset = col.saturating_sub(1) as usize;
            let remaining = &source[idx..];
            return remaining
                .char_indices()
                .nth(col_offset)
                .map(|(i, _)| idx + i)
                .unwrap_or(idx);
        }
        if ch == '\n' {
            current_line += 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use writ_diagnostics::Severity;

    #[test]
    fn test_analyze_standalone_valid() {
        let result = AnalysisHost::analyze_standalone(
            "fn main() {}".to_string(),
            "test.writ".to_string(),
        );
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Expected no errors for valid source, got: {:?}",
            errors.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_analyze_standalone_parse_error() {
        let result = AnalysisHost::analyze_standalone(
            "fn main( {}".to_string(),
            "test.writ".to_string(),
        );
        assert!(
            !result.diagnostics.is_empty(),
            "Expected at least one diagnostic for broken syntax"
        );
    }

    #[test]
    fn test_analyze_standalone_type_error() {
        let result = AnalysisHost::analyze_standalone(
            "fn main() { let x: int = true; }".to_string(),
            "test.writ".to_string(),
        );
        let has_error = result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(has_error, "Expected at least one error diagnostic for type mismatch");
    }

    #[test]
    fn test_analyze_standalone_cascade() {
        // Source that has a resolvable structure but a type error — all stages should run
        let src = r#"fn main() { let x: int = "hello"; }"#;
        let result = AnalysisHost::analyze_standalone(src.to_string(), "cascade.writ".to_string());
        // Should have at least one error from typecheck (string != int)
        let has_error = result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(has_error, "Expected cascade diagnostics from type stage");
    }

    #[test]
    fn test_analyze_standalone_has_file_sources() {
        let result = AnalysisHost::analyze_standalone(
            "fn main() {}".to_string(),
            "my_file.writ".to_string(),
        );
        assert_eq!(result.file_sources.len(), 1);
        assert_eq!(result.file_sources[0].1, "my_file.writ");
    }

    #[test]
    fn test_analyze_project_missing_toml() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_no_toml");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Should not panic — falls back gracefully
        let result = AnalysisHost::analyze_project(&tmp, None, None);
        // No trigger file, no toml => empty result
        assert!(result.diagnostics.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    /// LSP-08: project-mode analysis happy path — real writ.toml + multiple .writ files.
    ///
    /// Verifies:
    ///   (a) file_sources contains entries for all discovered files (multi-file FileId coverage)
    ///   (b) the pipeline runs to completion (diagnostics vec is populated or empty — no panic)
    ///   (c) a type error in one file produces an Error diagnostic attributed to its FileId
    #[test]
    fn test_analyze_project_with_toml() {
        use std::collections::HashSet;
        use writ_diagnostics::Severity;

        let tmp = std::env::temp_dir().join("writ_lsp_test_project_with_toml");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Minimal valid writ.toml — [compiler] defaults to sources = ["src/"]
        fs::write(
            tmp.join("writ.toml"),
            r#"[project]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        // File 1: valid source — fn with correct types
        fs::write(src_dir.join("a.writ"), "fn greet() {}").unwrap();

        // File 2: a type error — assigning string to int
        fs::write(src_dir.join("b.writ"), r#"fn bad() { let x: int = "oops"; }"#).unwrap();

        let result = AnalysisHost::analyze_project(&tmp, None, None);

        // (a) file_sources must contain both files
        assert_eq!(
            result.file_sources.len(),
            2,
            "Expected 2 file_sources entries for 2 discovered .writ files, got {}",
            result.file_sources.len()
        );

        // Verify distinct FileIds are assigned
        let file_ids: HashSet<u32> = result.file_sources.iter().map(|(fid, _, _)| fid.0).collect();
        assert_eq!(file_ids.len(), 2, "Expected distinct FileIds for each discovered file");

        // (b) pipeline ran to completion — result is always non-panic
        // (diagnostics can be empty or non-empty; what matters is we reached here)

        // (c) the type error in b.writ must surface as at least one Error diagnostic
        let error_count = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        assert!(
            error_count >= 1,
            "Expected at least one Error diagnostic from the type error in b.writ, got {} diagnostics total: {:?}",
            result.diagnostics.len(),
            result.diagnostics.iter().map(|d| (&d.code, &d.message)).collect::<Vec<_>>()
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// Verify that project-mode analysis produces a typed_ast (needed for hover/goto-def).
    /// This is the critical test: if typed_ast is None, ALL interactive LSP features break.
    #[test]
    fn test_analyze_project_produces_typed_ast() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_project_typed_ast");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            tmp.join("writ.toml"),
            r#"[project]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        fs::write(src_dir.join("main.writ"), "fn main() { let x: int = 42; }").unwrap();

        let result = AnalysisHost::analyze_project(&tmp, None, None);

        assert!(
            result.typed_ast.is_some(),
            "Project-mode analysis must produce typed_ast for hover/goto-def to work"
        );
        assert!(
            result.ty_interner.is_some(),
            "Project-mode analysis must produce ty_interner"
        );
        assert!(
            result.type_env.is_some(),
            "Project-mode analysis must produce type_env"
        );
        assert_eq!(result.file_sources.len(), 1);

        // Verify hover would work: expr_at_offset should find an expression
        let typed_ast = result.typed_ast.as_ref().unwrap();
        assert!(
            !typed_ast.decls.is_empty(),
            "TypedAst should contain at least one declaration"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// Verify that when trigger_source is provided for a file in sources,
    /// the in-memory content is used instead of reading from disk.
    #[test]
    fn test_analyze_project_uses_trigger_source_over_disk() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_project_trigger_source");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            tmp.join("writ.toml"),
            r#"[project]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        // Disk version: valid code
        fs::write(src_dir.join("main.writ"), "fn main() {}").unwrap();

        // In-memory version: has a type error
        let trigger_path = src_dir.join("main.writ");
        let in_memory = r#"fn main() { let x: int = "wrong"; }"#.to_string();

        let result = AnalysisHost::analyze_project(
            &tmp,
            Some(trigger_path.to_str().unwrap()),
            Some(in_memory),
        );

        // Should use the in-memory content (which has a type error), not disk (which is valid)
        let has_error = result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(
            has_error,
            "Should use in-memory content with type error, not clean disk version"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    /// Verify that project-mode with trigger file NOT in source directory still
    /// produces analysis results. This was the root cause of the "writ.toml
    /// breaks everything" bug: trigger files outside configured source dirs
    /// were silently dropped.
    #[test]
    fn test_analyze_project_trigger_not_in_sources() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_project_not_in_sources");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // writ.toml with default sources = ["src/"] — no src/ directory exists
        fs::write(
            tmp.join("writ.toml"),
            r#"[project]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();

        // File in workspace ROOT, not in src/
        let trigger = tmp.join("main.writ");
        let trigger_content = "fn main() { let x: int = 42; }";
        fs::write(&trigger, trigger_content).unwrap();

        let result = AnalysisHost::analyze_project(
            &tmp,
            Some(trigger.to_str().unwrap()),
            Some(trigger_content.to_string()),
        );

        // After fix: trigger file is included even though it's outside src/
        assert!(
            result.typed_ast.is_some(),
            "Trigger file outside source dirs must still produce typed_ast"
        );
        assert_eq!(
            result.file_sources.len(),
            1,
            "file_sources must contain the trigger file"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_analyze_project_missing_toml_with_trigger() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_no_toml_trigger");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let trigger = tmp.join("trigger.writ");
        fs::write(&trigger, "fn main() {}").unwrap();

        // Should fall back to standalone analysis of the trigger file
        let result = AnalysisHost::analyze_project(&tmp, Some(trigger.to_str().unwrap()), None);
        // Valid source => no errors
        let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "Expected no errors for valid trigger file");

        let _ = fs::remove_dir_all(&tmp);
    }

    // ─── Phase 1: LSP integration tests (hover, goto-def, find-refs) ──────────

    #[test]
    fn test_hover_standalone_finds_variable() {
        // Source: fn main() { let x: int = 42; x }
        //                                       ^ offset of usage `x`
        let src = "fn main() { let x: int = 42; x }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // The usage `x` is at byte offset 29 (0-indexed: "fn main() { let x: int = 42; x }")
        let x_usage_offset = src.find("42; x").unwrap() + 4; // offset of the usage `x`
        let expr = crate::queries::expr_at_offset(typed_ast, x_usage_offset, FileId(0));
        assert!(expr.is_some(), "expr_at_offset should find the `x` usage at offset {}", x_usage_offset);

        let hover_text = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        assert!(!hover_text.is_empty(), "hover text should be non-empty");
        assert!(
            hover_text.contains("int"),
            "hover text should contain 'int', got: {}",
            hover_text
        );
    }

    #[test]
    fn test_hover_standalone_finds_binding() {
        let src = "fn main() { let x: int = 42; x }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let type_env = result.type_env.as_ref().unwrap();
        let interner = result.ty_interner.as_ref().unwrap();

        // The `x` in `let x:` is at byte offset 16
        let let_x_offset = src.find("let x").unwrap() + 4;
        let binding = crate::queries::binding_at_offset(typed_ast, let_x_offset, type_env, FileId(0));
        assert!(binding.is_some(), "binding_at_offset should find `x` at the let declaration");

        let info = binding.unwrap();
        assert_eq!(info.name, "x");
        let ty_str = interner.display_named(info.ty, &typed_ast.def_map);
        assert!(ty_str.contains("int"), "binding type should be int, got: {}", ty_str);
    }

    #[test]
    fn test_hover_project_mode_correct_file_id() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_hover_file_id");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            tmp.join("writ.toml"),
            "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
        ).unwrap();

        // Use very different lengths to avoid offset overlaps between files.
        // a.writ: a long function with a unique literal deep inside
        fs::write(src_dir.join("a.writ"), "pub fn greet_all_the_people_in_the_world() -> int { 99999 }").unwrap();
        // b.writ: short
        fs::write(src_dir.join("b.writ"), "fn main() {}").unwrap();

        let result = AnalysisHost::analyze_project(&tmp, None, None);
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Should have 2 file_sources with distinct FileIds
        assert_eq!(result.file_sources.len(), 2, "should have 2 file_sources");
        let fid0 = result.file_sources[0].0;
        let fid1 = result.file_sources[1].0;
        assert_ne!(fid0, fid1, "file ids should be distinct");

        // Determine which file is a.writ
        let (a_fid, a_src) = if result.file_sources[0].1.contains("a.writ") {
            (result.file_sources[0].0, &result.file_sources[0].2)
        } else {
            (result.file_sources[1].0, &result.file_sources[1].2)
        };
        let b_fid = if result.file_sources[0].1.contains("b.writ") {
            result.file_sources[0].0
        } else {
            result.file_sources[1].0
        };

        // The literal 99999 is deep inside a.writ — its offset should be past b.writ's length
        let a_offset = a_src.find("99999").unwrap();
        let expr_in_a = crate::queries::expr_at_offset(typed_ast, a_offset, a_fid);
        assert!(expr_in_a.is_some(), "should find expression in a.writ with a's FileId");

        // Using b.writ's FileId at a large offset (past b.writ content) should find nothing
        let cross = crate::queries::expr_at_offset(typed_ast, a_offset, b_fid);
        assert!(cross.is_none(), "should NOT find a.writ expression with b's FileId");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_resolve_trigger_file_id_standalone() {
        use url::Url;
        let file_sources = vec![
            (FileId(0), "C:\\test\\foo.writ".to_string(), String::new()),
        ];
        let uri = Url::from_file_path("C:\\test\\foo.writ").unwrap();
        let fid = crate::backend::resolve_trigger_file_id(
            &file_sources,
            &uri.to_string(),
            &uri,
        );
        assert_eq!(fid, FileId(0), "should resolve to FileId(0) for matching path");
    }

    #[test]
    fn test_resolve_trigger_file_id_case_insensitive() {
        use url::Url;
        // file_sources has uppercase drive letter
        let file_sources = vec![
            (FileId(0), "D:\\Projects\\test.writ".to_string(), String::new()),
        ];
        // URI uses lowercase drive letter
        let uri = Url::from_file_path("d:\\Projects\\test.writ").unwrap();
        let fid = crate::backend::resolve_trigger_file_id(
            &file_sources,
            &uri.to_string(),
            &uri,
        );
        assert_eq!(fid, FileId(0), "should match case-insensitively on Windows");
    }

    #[test]
    fn test_goto_def_type_annotation() {
        let src = "struct Foo {} fn main() { let x: Foo = new Foo {}; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find the `Foo` in the type annotation `let x: Foo`
        let type_foo_offset = src.find("let x: Foo").unwrap() + 7; // offset of 'F' in type ann

        let def_id = crate::queries::type_ann_def_id_at_offset(typed_ast, type_foo_offset, FileId(0));
        assert!(
            def_id.is_some(),
            "type_ann_def_id_at_offset should find struct Foo definition at offset {}",
            type_foo_offset,
        );

        let entry = typed_ast.def_map.get_entry(def_id.unwrap());
        assert_eq!(entry.name, "Foo", "definition should be named 'Foo'");
    }

    #[test]
    fn test_find_refs_from_declaration() {
        let src = "fn helper() -> int { 1 } fn main() { helper(); helper(); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find the DefId at the `helper` declaration name
        let helper_offset = src.find("helper").unwrap();
        let def_id = crate::queries::def_at_offset(&typed_ast.def_map, helper_offset, FileId(0));
        assert!(def_id.is_some(), "def_at_offset should find helper at its declaration");

        // collect_references should find the call sites
        let refs = crate::queries::collect_references(typed_ast, def_id.unwrap(), &typed_ast.def_map);
        assert!(
            refs.len() >= 2,
            "should find at least 2 references to helper(), got {}",
            refs.len()
        );
    }

    #[test]
    fn test_file_id_filtering_skips_other_files() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_fileid_filtering");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            tmp.join("writ.toml"),
            "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
        ).unwrap();

        // Make a.writ much longer so its literals are at offsets past b.writ's length
        fs::write(src_dir.join("a.writ"), "pub fn a_very_long_function_name_for_testing() -> int { 123456789 }").unwrap();
        fs::write(src_dir.join("b.writ"), "fn main() {}").unwrap();

        let result = AnalysisHost::analyze_project(&tmp, None, None);
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        assert_eq!(result.file_sources.len(), 2);

        // Find which entry is a.writ
        let (a_idx, _) = result.file_sources.iter().enumerate()
            .find(|(_, (_, path, _))| path.contains("a.writ"))
            .expect("should find a.writ");
        let b_idx = 1 - a_idx;

        let a_fid = result.file_sources[a_idx].0;
        let b_fid = result.file_sources[b_idx].0;
        let a_src = &result.file_sources[a_idx].2;

        // The literal 123456789 is at an offset well past b.writ's length
        let offset = a_src.find("123456789").unwrap();

        // Query with b.writ's FileId at a.writ's deep offset — should find nothing
        let expr = crate::queries::expr_at_offset(typed_ast, offset, b_fid);
        assert!(
            expr.is_none(),
            "expr_at_offset with wrong FileId should return None"
        );

        // Query with a.writ's FileId — should find the expression
        let expr = crate::queries::expr_at_offset(typed_ast, offset, a_fid);
        assert!(
            expr.is_some(),
            "expr_at_offset with correct FileId should find expression"
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    // ─── Extended LSP integration tests ───────────────────────────────────────

    // ── 1. Hover tests ────────────────────────────────────────────────────────

    #[test]
    fn test_hover_fn_call_shows_signature() {
        let src = "fn add(a: int, b: int) -> int { a + b } fn main() { add(1, 2); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // Find the `add` call site in main (the second occurrence of "add")
        let call_offset = src.rfind("add(1").expect("should find add(1 in source");
        let expr = crate::queries::expr_at_offset(typed_ast, call_offset, FileId(0));
        assert!(expr.is_some(), "expr_at_offset should find expression at add call, offset {}", call_offset);

        let hover = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        // The hover text shows the type of the callee expression. For a Call node
        // it shows `fn name(params) -> ret`; for a Var node it shows `name: fn(...) -> ...`.
        // Either way, it should contain "fn" and "int" (the parameter/return types).
        assert!(hover.contains("fn"), "hover should contain 'fn', got: {}", hover);
        assert!(hover.contains("int"), "hover should contain 'int' (param/return type), got: {}", hover);
    }

    #[test]
    fn test_hover_literal_shows_type() {
        let src = "fn main() { 42; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        let lit_offset = src.find("42").unwrap();
        let expr = crate::queries::expr_at_offset(typed_ast, lit_offset, FileId(0));
        assert!(expr.is_some(), "should find literal expression at offset {}", lit_offset);

        let hover = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        assert!(hover.contains("int"), "hover for integer literal should contain 'int', got: {}", hover);
    }

    #[test]
    fn test_hover_string_literal_shows_type() {
        let src = r#"fn main() { "hello"; }"#;
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        let lit_offset = src.find('"').unwrap();
        let expr = crate::queries::expr_at_offset(typed_ast, lit_offset, FileId(0));
        assert!(expr.is_some(), "should find string literal expression");

        let hover = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        assert!(hover.contains("string"), "hover for string literal should contain 'string', got: {}", hover);
    }

    #[test]
    fn test_hover_bool_literal_shows_type() {
        let src = "fn main() { true; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        let lit_offset = src.find("true").unwrap();
        let expr = crate::queries::expr_at_offset(typed_ast, lit_offset, FileId(0));
        assert!(expr.is_some(), "should find bool literal expression");

        let hover = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        assert!(hover.contains("bool"), "hover for bool literal should contain 'bool', got: {}", hover);
    }

    #[test]
    fn test_hover_field_access_shows_field_type() {
        let src = "pub struct Foo { x: int } fn main() { let f: Foo = new Foo { x: 1 }; f.x; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // Find the `.x` field access in `f.x` -- position at the dot or after to get
        // the Field expression rather than the Var `f`.
        let dot_offset = src.rfind(".x;").unwrap() + 1; // offset of 'x' in `.x`
        let expr = crate::queries::expr_at_offset(typed_ast, dot_offset, FileId(0));
        assert!(expr.is_some(), "should find expression at the field access '.x'");

        let hover = crate::queries::hover_text_for_expr(
            expr.unwrap(), &typed_ast.def_map, interner, type_env, src, typed_ast,
        );
        assert!(hover.contains("int"), "hover for field access f.x should contain 'int', got: {}", hover);
    }

    // ── 2. Goto-def tests ─────────────────────────────────────────────────────

    #[test]
    fn test_goto_def_fn_call() {
        let src = "fn target() {} fn main() { target(); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find `target()` call in main (the second occurrence of "target")
        let call_offset = src.rfind("target()").expect("should find target() call in main");
        let expr = crate::queries::expr_at_offset(typed_ast, call_offset, FileId(0));
        assert!(expr.is_some(), "should find expression at target() call site");

        let def_id = crate::queries::find_def_id_at_offset(expr.unwrap(), &typed_ast.def_map);
        assert!(def_id.is_some(), "should resolve target() call to a DefId");

        let entry = typed_ast.def_map.get_entry(def_id.unwrap());
        assert_eq!(entry.name, "target", "DefEntry name should be 'target', got '{}'", entry.name);
    }

    #[test]
    fn test_goto_def_struct_new() {
        let src = "pub struct Bar {} fn main() { let b: Bar = new Bar {}; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find `new Bar {}` -- position inside the New expression.
        // The `new` keyword starts the New expression; the `Bar` name is after it.
        // Use the offset of `Bar` in `new Bar {}` to land inside the New expr.
        let bar_in_new = src.rfind("new Bar {}").unwrap() + "new ".len();
        let expr = crate::queries::expr_at_offset(typed_ast, bar_in_new, FileId(0));
        assert!(expr.is_some(), "should find expression at 'Bar' in 'new Bar' site");

        let def_id = crate::queries::find_def_id_at_offset(expr.unwrap(), &typed_ast.def_map);
        assert!(def_id.is_some(), "should resolve 'new Bar' to a DefId");

        let entry = typed_ast.def_map.get_entry(def_id.unwrap());
        assert_eq!(entry.name, "Bar", "DefEntry name should be 'Bar', got '{}'", entry.name);
    }

    #[test]
    fn test_goto_def_cross_file() {
        let tmp = std::env::temp_dir().join("writ_lsp_test_goto_def_cross_file");
        let _ = fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            tmp.join("writ.toml"),
            "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
        ).unwrap();

        fs::write(src_dir.join("a.writ"), "pub fn helper() -> int { 1 }").unwrap();
        fs::write(src_dir.join("b.writ"), "fn main() { helper(); }").unwrap();

        let result = AnalysisHost::analyze_project(&tmp, None, None);
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast for cross-file project");

        // Look up 'helper' in the def_map -- it should exist as a public definition
        let helper_def_id = typed_ast.def_map.by_fqn.values()
            .copied()
            .find(|&id| typed_ast.def_map.get_entry(id).name == "helper")
            .or_else(|| {
                typed_ast.def_map.file_private.values()
                    .find_map(|privs| privs.get("helper").copied())
            });
        assert!(
            helper_def_id.is_some(),
            "cross-file goto-def: 'helper' should exist in def_map"
        );

        let entry = typed_ast.def_map.get_entry(helper_def_id.unwrap());
        assert_eq!(entry.name, "helper", "definition name should be 'helper'");

        let _ = fs::remove_dir_all(&tmp);
    }

    // ── 3. Find-references tests ──────────────────────────────────────────────

    #[test]
    fn test_find_refs_struct_usage() {
        let src = "pub struct Foo {} fn main() { let a: Foo = new Foo {}; let b: Foo = new Foo {}; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find DefId for `Foo` at its declaration
        let foo_offset = src.find("Foo").unwrap();
        let def_id = crate::queries::def_at_offset(&typed_ast.def_map, foo_offset, FileId(0));
        assert!(def_id.is_some(), "should find Foo definition at its declaration site");

        let refs = crate::queries::collect_references(typed_ast, def_id.unwrap(), &typed_ast.def_map);
        assert!(
            refs.len() >= 2,
            "should find at least 2 references to Foo (the two 'new Foo' usages), got {}",
            refs.len()
        );
    }

    #[test]
    fn test_find_refs_no_false_positives() {
        let src = "fn alpha() {} fn beta() {} fn main() { alpha(); beta(); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");

        // Find DefId for `alpha` at its declaration
        let alpha_offset = src.find("alpha").unwrap();
        let alpha_def_id = crate::queries::def_at_offset(&typed_ast.def_map, alpha_offset, FileId(0));
        assert!(alpha_def_id.is_some(), "should find alpha definition");

        let beta_offset = src.find("beta").unwrap();
        let beta_def_id = crate::queries::def_at_offset(&typed_ast.def_map, beta_offset, FileId(0));
        assert!(beta_def_id.is_some(), "should find beta definition");

        let alpha_refs = crate::queries::collect_references(typed_ast, alpha_def_id.unwrap(), &typed_ast.def_map);
        let beta_refs = crate::queries::collect_references(typed_ast, beta_def_id.unwrap(), &typed_ast.def_map);

        // Alpha references should exist (the Call and/or Var nodes at the call site)
        assert!(
            !alpha_refs.is_empty(),
            "should find at least 1 reference to alpha"
        );
        // Beta references should also exist at its call site
        assert!(
            !beta_refs.is_empty(),
            "should find at least 1 reference to beta"
        );
        // Key assertion: alpha and beta reference spans should not overlap
        // (no false positives between the two)
        for alpha_span in &alpha_refs {
            for beta_span in &beta_refs {
                assert_ne!(
                    alpha_span.start, beta_span.start,
                    "alpha and beta references should not overlap"
                );
            }
        }
    }

    // ── 4. Autocomplete tests (identifier completion) ─────────────────────────

    #[test]
    fn test_identifier_completions_include_user_fn() {
        let src = "pub fn my_custom_fn() {} fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let items = crate::queries::build_identifier_completions(&typed_ast.def_map, interner);
        let fn_item = items.iter().find(|i| i.label == "my_custom_fn");
        assert!(fn_item.is_some(), "completions should include 'my_custom_fn'");
        assert_eq!(
            fn_item.unwrap().kind,
            Some(lsp_types::CompletionItemKind::FUNCTION),
            "my_custom_fn should have FUNCTION kind"
        );
    }

    #[test]
    fn test_identifier_completions_include_user_struct() {
        let src = "pub struct Widget {} fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let items = crate::queries::build_identifier_completions(&typed_ast.def_map, interner);
        let struct_item = items.iter().find(|i| i.label == "Widget");
        assert!(struct_item.is_some(), "completions should include 'Widget'");
        assert_eq!(
            struct_item.unwrap().kind,
            Some(lsp_types::CompletionItemKind::STRUCT),
            "Widget should have STRUCT kind"
        );
    }

    #[test]
    fn test_identifier_completions_include_user_enum() {
        let src = "pub enum Color { Red, Green, Blue } fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let items = crate::queries::build_identifier_completions(&typed_ast.def_map, interner);
        let enum_item = items.iter().find(|i| i.label == "Color");
        assert!(enum_item.is_some(), "completions should include 'Color'");
        assert_eq!(
            enum_item.unwrap().kind,
            Some(lsp_types::CompletionItemKind::ENUM),
            "Color should have ENUM kind"
        );
    }

    // ── 5. Dot-completion tests ───────────────────────────────────────────────

    #[test]
    fn test_dot_completion_struct_method() {
        let src = "pub struct Vec2 { x: int, y: int } impl Vec2 { fn len(self) -> int { self.x } } fn main() { let v: Vec2 = new Vec2 { x: 1, y: 2 }; v.x; }";
        let mut result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_mut().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // The `v` variable has type Vec2 -- find the Vec2 DefId and build dot completions
        let vec2_def_id = typed_ast.def_map.by_fqn.values()
            .copied()
            .find(|&id| typed_ast.def_map.get_entry(id).name == "Vec2")
            .or_else(|| {
                typed_ast.def_map.file_private.values()
                    .find_map(|privs| privs.get("Vec2").copied())
            })
            .expect("should find Vec2 definition");

        let receiver_ty = interner.intern(writ_compiler::check::ty::TyKind::Struct(vec2_def_id));
        let items = crate::queries::build_dot_completions(receiver_ty, interner, &typed_ast.def_map, type_env);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"x"), "dot completions should include field 'x', got: {:?}", labels);
        assert!(labels.contains(&"y"), "dot completions should include field 'y', got: {:?}", labels);
        assert!(labels.contains(&"len"), "dot completions should include method 'len', got: {:?}", labels);
    }

    #[test]
    fn test_dot_completion_array_methods() {
        let src = "fn main() { let arr: Array<int> = [1, 2, 3]; arr.len(); }";
        let mut result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_mut().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // Build an Array<int> type for dot completion
        let int_ty = interner.intern(writ_compiler::check::ty::TyKind::Int);
        let array_ty = interner.intern(writ_compiler::check::ty::TyKind::Array(int_ty));
        let items = crate::queries::build_dot_completions(array_ty, interner, &typed_ast.def_map, type_env);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"push"), "array dot completions should include 'push', got: {:?}", labels);
        assert!(labels.contains(&"pop"), "array dot completions should include 'pop', got: {:?}", labels);
        assert!(labels.contains(&"len"), "array dot completions should include 'len', got: {:?}", labels);
        assert!(labels.contains(&"is_empty"), "array dot completions should include 'is_empty', got: {:?}", labels);
    }

    // ── 6. Signature help tests ───────────────────────────────────────────────

    #[test]
    fn test_signature_help_multi_param() {
        let src = "fn process(a: int, b: string, c: bool) {} fn main() { process(1, ); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // Find the offset just after the comma: "process(1, )"
        let comma_pos = src.rfind("1, )").unwrap() + 3; // offset of the space after the comma
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let help = crate::queries::build_signature_help(src_static, comma_pos, typed_ast, interner, type_env);
        assert!(help.is_some(), "should return signature help for multi-param fn");

        let help = help.unwrap();
        let sig = &help.signatures[0];
        assert_eq!(
            sig.parameters.as_ref().unwrap().len(), 3,
            "process has 3 parameters"
        );
        assert_eq!(
            help.active_parameter, Some(1),
            "active_parameter should be 1 (after first comma)"
        );
    }

    #[test]
    fn test_signature_help_no_args_fn() {
        let src = "fn noop() {} fn main() { noop(); }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();
        let type_env = result.type_env.as_ref().unwrap();

        // Find offset inside noop() -- between the parens
        let paren_pos = src.rfind("noop()").unwrap() + "noop(".len();
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let help = crate::queries::build_signature_help(src_static, paren_pos, typed_ast, interner, type_env);
        assert!(help.is_some(), "should return signature help for no-args fn");

        let help = help.unwrap();
        let sig = &help.signatures[0];
        assert_eq!(
            sig.parameters.as_ref().unwrap().len(), 0,
            "noop has 0 parameters"
        );
    }

    // ── 7. Semantic tokens tests ──────────────────────────────────────────────

    #[test]
    fn test_semantic_tokens_fn_declaration() {
        let src = "fn my_func() {} fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let tokens = crate::queries::collect_semantic_tokens(typed_ast, interner, src, FileId(0));

        let func_offset = src.find("my_func").unwrap();
        let func_pos = crate::convert::offset_to_position(src, func_offset);

        let func_token = tokens
            .iter()
            .find(|t| t.line == func_pos.line && t.start_char == func_pos.character);
        assert!(func_token.is_some(), "should have a semantic token at 'my_func' position");
        // TOKEN_TYPE_FUNCTION = 5
        assert_eq!(
            func_token.unwrap().token_type, 5,
            "my_func token_type should be FUNCTION (5)"
        );
    }

    #[test]
    fn test_semantic_tokens_class_declaration() {
        let src = "class MyClass {} fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let tokens = crate::queries::collect_semantic_tokens(typed_ast, interner, src, FileId(0));

        let class_offset = src.find("MyClass").unwrap();
        let class_pos = crate::convert::offset_to_position(src, class_offset);

        let class_token = tokens
            .iter()
            .find(|t| t.line == class_pos.line && t.start_char == class_pos.character);
        assert!(class_token.is_some(), "should have a semantic token at 'MyClass' position");
        // TOKEN_TYPE_TYPE = 1
        assert_eq!(
            class_token.unwrap().token_type, 1,
            "MyClass token_type should be TYPE (1)"
        );
    }

    #[test]
    fn test_semantic_tokens_enum_declaration() {
        let src = "enum Dir { North, South } fn main() {}";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let typed_ast = result.typed_ast.as_ref().expect("should produce typed_ast");
        let interner = result.ty_interner.as_ref().unwrap();

        let tokens = crate::queries::collect_semantic_tokens(typed_ast, interner, src, FileId(0));

        let dir_offset = src.find("Dir").unwrap();
        let dir_pos = crate::convert::offset_to_position(src, dir_offset);

        let dir_token = tokens
            .iter()
            .find(|t| t.line == dir_pos.line && t.start_char == dir_pos.character);
        assert!(dir_token.is_some(), "should have a semantic token at 'Dir' position");
        // TOKEN_TYPE_TYPE = 1
        assert_eq!(
            dir_token.unwrap().token_type, 1,
            "Dir token_type should be TYPE (1)"
        );
    }

    // ── 8. Position-to-offset edge cases ──────────────────────────────────────

    #[test]
    fn test_position_to_byte_offset_end_of_line() {
        let src = "fn main() {\n    42;\n}";
        // Position { line: 1, character: 6 } -- past "    42" (4 spaces + '4' + '2' = 6 chars)
        let pos = lsp_types::Position { line: 1, character: 6 };
        let offset = crate::queries::position_to_byte_offset(src, pos);
        assert!(offset.is_some(), "should return a valid byte offset for end of line content");
        // The ';' is at byte offset: "fn main() {\n" (12 bytes) + "    42" (6 bytes) = 18
        let expected = src.find("42;").unwrap() + 2; // offset of ';'
        assert_eq!(
            offset.unwrap(), expected,
            "offset should point to ';' at byte {}", expected
        );
    }

    #[test]
    fn test_position_to_byte_offset_empty_line() {
        let src = "fn main() {\n\n    42;\n}";
        // Position { line: 1, character: 0 } -- the empty line
        let pos = lsp_types::Position { line: 1, character: 0 };
        let offset = crate::queries::position_to_byte_offset(src, pos);
        assert!(offset.is_some(), "should return a valid byte offset for empty line");
        // "fn main() {\n" is 12 bytes; position line 1, char 0 = byte 12 (the '\n' of the empty line)
        assert_eq!(
            offset.unwrap(), 12,
            "empty line offset should be 12 (start of second line)"
        );
    }

    // ── 9. span_to_range conversion test ──────────────────────────────────────

    #[test]
    fn test_span_to_range_multiline() {
        let src = "fn main() {\n    let x: int = 42;\n}";
        // Create a span covering "let x: int = 42;" on line 1
        // "fn main() {\n" = 12 bytes; "    let x: int = 42;" = 20 bytes (offset 16..32)
        let let_start = src.find("let x").unwrap(); // should be 16
        let semi_end = src.find("42;").unwrap() + 3; // end after semicolon
        let span = chumsky::span::SimpleSpan { start: let_start, end: semi_end, context: () };

        let range = crate::convert::span_to_range(src, &span);
        assert_eq!(range.start.line, 1, "span start should be on line 1");
        assert_eq!(range.end.line, 1, "span end should be on line 1");
        assert_eq!(range.start.character, 4, "span start character should be 4 (after 4 spaces)");
    }

    // ── 10. Runtime crash diagnostic tests ────────────────────────────────────

    /// Force-unwrap on None should produce an R0001 diagnostic with stacktrace.
    #[test]
    fn test_runtime_crash_force_unwrap_shows_stacktrace() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "fn main() {\n    let x: Option<int> = Option::None;\n    let y: int = x!;\n}";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let runtime_diags: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.code == "R0001")
                    .collect();
                assert_eq!(runtime_diags.len(), 1, "expected one runtime crash diagnostic");
                let msg = &runtime_diags[0].message;
                assert!(msg.contains("Runtime crash"), "should contain 'Runtime crash': {}", msg);
                assert!(msg.contains("main"), "stack trace should mention 'main': {}", msg);
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// Nested call crash should list both crash_here and main in the trace.
    #[test]
    fn test_runtime_crash_nested_call_shows_full_stacktrace() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "fn crash_here() {\n    let x: Option<int> = Option::None;\n    let y: int = x!;\n}\nfn main() {\n    crash_here();\n}";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let runtime_diags: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.code == "R0001")
                    .collect();
                assert_eq!(runtime_diags.len(), 1);
                let msg = &runtime_diags[0].message;
                assert!(msg.contains("crash_here"), "should mention 'crash_here': {}", msg);
                assert!(msg.contains("main"), "should mention 'main': {}", msg);
                // crash_here should appear BEFORE main (top of stack first)
                let crash_pos = msg.find("crash_here").unwrap();
                let main_pos = msg.find("main").unwrap();
                assert!(crash_pos < main_pos, "crash_here should be listed before main in the trace");
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// Compile errors should prevent runtime execution (no R0001 when type error exists).
    #[test]
    fn test_no_runtime_diagnostic_when_compile_errors() {
        let src = "fn main() { let x: int = true; }";
        let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
        let runtime_diags: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.code == "R0001")
            .collect();
        assert!(runtime_diags.is_empty(), "should not attempt runtime when compile errors exist");
    }

    /// A clean script should produce no R0001 diagnostic.
    #[test]
    fn test_no_runtime_diagnostic_clean_script() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "fn main() { let x: int = 42; }";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let runtime_diags: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.code == "R0001")
                    .collect();
                assert!(runtime_diags.is_empty(), "clean script should produce no runtime diagnostics");
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// A file without `main` should not trigger runtime execution.
    #[test]
    fn test_no_runtime_diagnostic_no_main() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "fn helper() { let x: int = 42; }";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let runtime_diags: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.code == "R0001")
                    .collect();
                assert!(runtime_diags.is_empty(), "no main = no runtime execution");
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// For-range loops should compile without runtime crash or error diagnostics.
    ///
    /// Regression test: previously, `for i in 0..5` fell through to Range struct
    /// construction with type_idx=0 (unregistered), causing a VM crash surfaced
    /// as a runtime error diagnostic in the LSP.
    #[test]
    fn test_for_range_no_runtime_crash() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "pub fn main() {\n    let mut sum = 0;\n    for i in 0..5 {\n        sum = sum + i;\n    }\n}";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let errors: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.severity == Severity::Error)
                    .collect();
                assert!(errors.is_empty(), "for-range should compile and run cleanly, got: {:?}",
                    errors.iter().map(|d| &d.message).collect::<Vec<_>>());
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// The primary span of a runtime crash diagnostic should point to the crash site.
    #[test]
    fn test_runtime_crash_primary_span_points_to_crash_site() {
        let handle = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let src = "fn main() {\n    let x: Option<int> = Option::None;\n    let y: int = x!;\n}";
                let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
                let runtime_diags: Vec<_> = result.diagnostics.iter()
                    .filter(|d| d.code == "R0001")
                    .collect();
                assert_eq!(runtime_diags.len(), 1);
                let span = &runtime_diags[0].primary_span;
                assert!(
                    span.start != 0 || span.end != 0,
                    "primary span should point to crash site, not 0..0"
                );
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_incomplete_impl_class_receiver_shows_type_name() {
        // Bug fix: E0123 should show "MyClass", not "impl#0"
        let src = r#"
pub contract MyContract {
    fn implementedFunc(self);
    fn notImplementedFunc(self);
}
pub class MyClass {}

impl MyContract for MyClass {
    fn implementedFunc(self){}
}

pub fn main() {
    let c = new MyClass{};
    c.implementedFunc();
}
"#.to_string();
        let result = AnalysisHost::analyze_standalone(src, "test.writ".to_string());
        let e0123 = result.diagnostics.iter()
            .find(|d| d.code == "E0123" && d.severity == Severity::Error);
        assert!(e0123.is_some(), "expected E0123 for incomplete impl");
        let msg = &e0123.unwrap().message;
        assert!(msg.contains("MyClass"), "E0123 should mention 'MyClass', got: {}", msg);
        assert!(!msg.contains("impl#"), "E0123 should not mention 'impl#', got: {}", msg);
    }

    #[test]
    fn test_complete_impl_class_receiver_no_crash() {
        // Bug fix: complete impl with self methods should not crash the runtime
        let src = r#"
pub contract MyContract {
    fn implementedFunc(self);
    fn notImplementedFunc(self);
}
pub class MyClass {}

impl MyContract for MyClass {
    fn implementedFunc(self){}
    fn notImplementedFunc(self){}
}

pub fn main() {
    let c = new MyClass{};
    c.implementedFunc();
    c.notImplementedFunc();
}
"#.to_string();
        let result = AnalysisHost::analyze_standalone(src, "test.writ".to_string());
        let errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "complete impl should have no errors, got: {:?}",
            errors.iter().map(|d| format!("{}: {}", d.code, d.message)).collect::<Vec<_>>());
    }

    /// DIAG-04: analyze_standalone must not panic when the source has a syntax error
    /// (e.g. unterminated string literal). The lowerer handles Cst::Expr::Error nodes
    /// by producing AstExpr::Error, so the pipeline continues and diagnostics are returned.
    #[test]
    fn test_analyze_standalone_partial_parse_no_panic() {
        // Source with unterminated string — parser should recover partially
        let source = "pub fn main() {\n    let x: int = 42;\n    let y: string = \"unterminated\n}\n".to_string();
        let result = AnalysisHost::analyze_standalone(source, "test.writ".to_string());
        // Must not panic — diagnostics should contain parse errors
        assert!(!result.diagnostics.is_empty(), "should have parse errors");
        // typed_ast may be Some (partial recovery) or None (total failure) — both acceptable
    }

    /// DIAG-04: analyze_standalone with valid items alongside one bad item must not panic.
    /// The parser performs item-level error recovery, so good items continue lowering.
    #[test]
    fn test_analyze_standalone_valid_portion_has_typed_ast() {
        // Source where most items are valid but one has a syntax error
        let source = "pub fn good() -> int { 42 }\npub fn bad( { }\npub fn also_good() -> bool { true }\n".to_string();
        let result = AnalysisHost::analyze_standalone(source, "test.writ".to_string());
        // Should have parse errors but may still produce typed_ast from recovered items
        assert!(result.diagnostics.iter().any(|d| d.severity == Severity::Error),
            "should have at least one parse error");
        // The key assertion: no panic occurred, server stays alive
    }
}
