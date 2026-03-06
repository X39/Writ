//! Compile-on-launch pipeline for the DAP server.
//!
//! Duplicates the 5-stage pipeline from writ-cli (which is private there),
//! adapting it for single-file DAP launch use.

use writ_module::Module;

/// Compile a `.writ` source file and return the decoded module plus its source text.
///
/// The source text is retained (as a `Box::leak`-ed `&'static str`) so that the
/// caller can later perform breakpoint line-mapping against the raw source.
///
/// Only single-file launch is supported in Phase 55. Multi-file (writ.toml) support
/// is deferred to a future phase.
///
/// # Errors
/// Returns `Err(String)` with a human-readable message if any pipeline stage fails.
pub fn compile_and_load(program_path: &str) -> Result<(Module, &'static str), String> {
    let bytes = std::fs::read(program_path)
        .map_err(|e| format!("failed to read '{}': {}", program_path, e))?;

    // Decode UTF-8 (strip BOM if present).
    let src_owned = decode_utf8_strip_bom(&bytes)
        .map_err(|e| format!("failed to decode '{}': {}", program_path, e))?;

    // Leak the source to get a 'static reference required by the parser.
    let src: &'static str = Box::leak(src_owned.into_boxed_str());

    let file_id = writ_diagnostics::FileId(0);
    let display_path = program_path.to_string();

    let compiled_bytes =
        run_pipeline(vec![(file_id, display_path, src)], true)?;

    let module = Module::from_bytes(&compiled_bytes)
        .map_err(|e| format!("failed to decode compiled module: {:?}", e))?;

    Ok((module, src))
}

/// Compile a writ.toml project directory and return the decoded module plus
/// a list of (FileId, path) for all discovered source files.
///
/// Uses `writ_compiler::config::load_config` and `discover_source_files`
/// to find all .writ files, then compiles them through the same 5-stage
/// pipeline used by single-file launch.
///
/// # Errors
/// Returns `Err(String)` if writ.toml is missing, no source files found,
/// or any pipeline stage fails.
pub fn compile_and_load_project(
    project_root: &std::path::Path,
) -> Result<(Module, Vec<(writ_diagnostics::FileId, String)>), String> {
    let config = writ_compiler::config::load_config(project_root)
        .map_err(|e| format!("failed to load writ.toml: {}", e))?;

    let discovered = writ_compiler::config::discover_source_files(project_root, &config)
        .map_err(|e| format!("failed to discover source files: {}", e))?;

    if discovered.is_empty() {
        return Err("no .writ source files found in project".to_string());
    }

    let mut file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)> = Vec::new();
    let mut file_id_paths: Vec<(writ_diagnostics::FileId, String)> = Vec::new();

    for (n, file_path) in discovered.iter().enumerate() {
        let file_id = writ_diagnostics::FileId(n as u32);
        let bytes = std::fs::read(file_path)
            .map_err(|e| format!("failed to read '{}': {}", file_path.display(), e))?;
        let src_owned = decode_utf8_strip_bom(&bytes)
            .map_err(|e| format!("failed to decode '{}': {}", file_path.display(), e))?;
        let src: &'static str = Box::leak(src_owned.into_boxed_str());
        let display_path = file_path.display().to_string();
        file_sources.push((file_id, display_path.clone(), src));
        file_id_paths.push((file_id, display_path));
    }

    let compiled_bytes = run_pipeline(file_sources, true)?;
    let module = Module::from_bytes(&compiled_bytes)
        .map_err(|e| format!("failed to decode compiled module: {:?}", e))?;

    Ok((module, file_id_paths))
}

/// Run the 5-stage Writ compilation pipeline.
///
/// `file_sources`: Vec of (FileId, display_path, source_str).
/// `emit_debug_info`: always `true` for DAP (we need SourceSpan data).
fn run_pipeline(
    file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)>,
    emit_debug_info: bool,
) -> Result<Vec<u8>, String> {
    let sources_for_render: Vec<(writ_diagnostics::FileId, &str, &str)> = file_sources
        .iter()
        .map(|(fid, path, src)| (*fid, path.as_str(), *src))
        .collect();

    // Stages 1+2: Parse and lower each file
    let mut per_file_asts: Vec<(writ_diagnostics::FileId, writ_compiler::Ast)> = Vec::new();

    for (file_id, display_path, src) in &file_sources {
        // Stage 1: Parse
        let (cst_opt, parse_errs) = writ_parser::parse(src);
        if !parse_errs.is_empty() {
            let err_count = parse_errs.len();
            for err in &parse_errs {
                eprintln!("[DAP] parse error in {}: {:?}", display_path, err);
            }
            return Err(format!("{} parse error(s)", err_count));
        }
        let cst = cst_opt
            .ok_or_else(|| format!("parse failed: no output for {}", display_path))?;

        // Stage 2: Lower CST -> AST
        let (ast, lower_errs) = writ_compiler::lower(cst);
        if !lower_errs.is_empty() {
            let diags: Vec<_> = lower_errs
                .iter()
                .map(|e| e.to_diagnostic(*file_id))
                .collect();
            eprint!(
                "{}",
                writ_diagnostics::render_diagnostics(&diags, &sources_for_render)
            );
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
    let (resolved, resolve_diags) =
        writ_compiler::resolve::resolve(&asts_refs, &path_refs);
    let has_resolve_errors = resolve_diags
        .iter()
        .any(|d| d.severity == writ_diagnostics::Severity::Error);
    if !resolve_diags.is_empty() {
        eprint!(
            "{}",
            writ_diagnostics::render_diagnostics(&resolve_diags, &sources_for_render)
        );
    }
    if has_resolve_errors {
        return Err("resolution failed".to_string());
    }

    // Stage 4: Type checking
    let (typed_ast, interner, _type_env, type_diags) =
        writ_compiler::check::typecheck(resolved, &asts_refs);
    let has_type_errors = type_diags
        .iter()
        .any(|d| d.severity == writ_diagnostics::Severity::Error);
    if !type_diags.is_empty() {
        eprint!(
            "{}",
            writ_diagnostics::render_diagnostics(&type_diags, &sources_for_render)
        );
    }
    if has_type_errors {
        return Err("type checking failed".to_string());
    }

    // Stage 5: IL codegen (always emit_debug_info=true for DAP)
    let sources: Vec<(writ_diagnostics::FileId, &str)> = file_sources
        .iter()
        .map(|(fid, _, src)| (*fid, *src))
        .collect();

    let bytes =
        writ_compiler::emit_bodies(&typed_ast, &interner, &asts_refs, emit_debug_info, &sources)
            .map_err(|diags| {
                eprint!(
                    "{}",
                    writ_diagnostics::render_diagnostics(&diags, &sources_for_render)
                );
                format!("{} codegen error(s)", diags.len())
            })?;

    Ok(bytes)
}

/// Strip a UTF-8 BOM (0xEF 0xBB 0xBF) from bytes if present, then decode to String.
fn decode_utf8_strip_bom(bytes: &[u8]) -> Result<String, String> {
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|e| format!("UTF-8 decode error: {}", e))
}
