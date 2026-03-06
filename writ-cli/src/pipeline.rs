//! Shared 5-stage compile pipeline: parse -> lower -> resolve -> typecheck -> emit.

/// Shared 5-stage pipeline: parse -> lower -> resolve -> typecheck -> emit.
///
/// `file_sources`: Vec of (FileId, display_path, leaked_source_str) for each source file.
/// `module_name`: Optional override for the module name in ModuleDef. If None, uses the
///   namespace heuristic from find_module_name in collect.rs. (Reserved for future use.)
/// `emit_debug_info`: Whether to include DebugLocal rows in the output.
///
/// Returns the compiled binary bytes on success, or an error message string.
///
/// NOTE: This function runs synchronously. Thread spawning (with 16MB stack) is done
/// by the callers (cmd_compile, cmd_build) before invoking this helper.
pub fn run_pipeline(
    file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)>,
    _module_name: Option<&str>,
    emit_debug_info: bool,
) -> Result<Vec<u8>, String> {
    // Build sources slice for diagnostic rendering
    let sources_for_render: Vec<(writ_diagnostics::FileId, &str, &str)> = file_sources
        .iter()
        .map(|(fid, path, src)| (*fid, path.as_str(), *src))
        .collect();

    // Stage 1+2: Parse and lower each file
    let mut per_file_asts: Vec<(writ_diagnostics::FileId, writ_compiler::Ast)> = Vec::new();

    for (file_id, display_path, src) in &file_sources {
        // Stage 1: Parse
        let (cst_opt, parse_errs) = writ_parser::parse(src);
        if !parse_errs.is_empty() {
            let err_count = parse_errs.len();
            for err in &parse_errs {
                eprintln!("parse error in {}: {:?}", display_path, err);
            }
            return Err(format!("{err_count} parse error(s)"));
        }
        drop(parse_errs);
        let cst = cst_opt.ok_or_else(|| format!("parse failed: no output for {}", display_path))?;

        // Stage 2: Lower CST -> AST
        let (ast, lower_errs) = writ_compiler::lower(cst);
        if !lower_errs.is_empty() {
            let diags: Vec<_> = lower_errs.iter().map(|e| e.to_diagnostic(*file_id)).collect();
            eprint!("{}", writ_diagnostics::render_diagnostics(&diags, &sources_for_render));
            return Err(format!("{} lowering error(s)", lower_errs.len()));
        }

        per_file_asts.push((*file_id, ast));
    }

    // Build reference slices for stages 3-5
    let asts_refs: Vec<(writ_diagnostics::FileId, &writ_compiler::Ast)> = per_file_asts
        .iter()
        .map(|(fid, ast)| (*fid, ast))
        .collect();
    let path_refs: Vec<(writ_diagnostics::FileId, &str)> = file_sources
        .iter()
        .map(|(fid, path, _)| (*fid, path.as_str()))
        .collect();

    // Stage 3: Name resolution
    let (resolved, resolve_diags) = writ_compiler::resolve::resolve(&asts_refs, &path_refs);
    let has_resolve_errors = resolve_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
    if !resolve_diags.is_empty() {
        eprint!("{}", writ_diagnostics::render_diagnostics(&resolve_diags, &sources_for_render));
    }
    if has_resolve_errors {
        return Err("resolution failed".to_string());
    }

    // Stage 4: Type checking
    let (typed_ast, interner, _type_env, type_diags) = writ_compiler::check::typecheck(resolved, &asts_refs);
    let has_type_errors = type_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
    if !type_diags.is_empty() {
        eprint!("{}", writ_diagnostics::render_diagnostics(&type_diags, &sources_for_render));
    }
    if has_type_errors {
        return Err("type checking failed".to_string());
    }

    // Stage 5: IL codegen
    // Build sources slice for SourceSpan line/col computation (PREP-01)
    let sources: Vec<(writ_diagnostics::FileId, &str)> = file_sources
        .iter()
        .map(|(fid, _, src)| (*fid, *src))
        .collect();

    let bytes = writ_compiler::emit_bodies(&typed_ast, &interner, &asts_refs, emit_debug_info, &sources)
        .map_err(|diags| {
            eprint!("{}", writ_diagnostics::render_diagnostics(&diags, &sources_for_render));
            format!("{} codegen error(s)", diags.len())
        })?;

    Ok(bytes)
}
