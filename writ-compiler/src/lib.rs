//! Writ compiler: source-to-IL compilation pipeline.
//!
//! ## Module structure
//!
//! - `ast`     -- Simplified AST produced by lowering (CST -> AST)
//! - `lower`   -- CST-to-AST lowering: desugars and normalises syntax
//! - `resolve` -- Name resolution: builds DefMap, resolves names to DefIds
//! - `check`   -- Type checking: produces TypedAst from resolved AST
//! - `emit`    -- IL emission: TypedAst -> binary .writc module bytes
//! - `config`  -- writ.toml parsing and project configuration

pub mod ast;
pub mod check;
pub mod config;
pub mod emit;
pub mod lower;
pub mod resolve;

// Public API re-exports
pub use ast::Ast;
pub use lower::lower;
pub use lower::error::LoweringError;
pub use lower::context::LoweringContext;

// Emit API
pub use emit::emit_bodies;

/// Compile Writ source code to a binary module.
///
/// This is the all-in-one convenience entry point: parse -> lower -> resolve ->
/// typecheck -> emit. Returns compiled `.writc` bytes on success.
///
/// For multi-file compilation, library dependencies, or finer control over
/// diagnostics, use `compile_with_libraries` or the individual pipeline stages.
///
/// **Stack note:** The compiler performs deep recursive AST walks. If you are
/// calling this from the main thread, consider spawning on a thread with a
/// larger stack (16 MB is typical):
///
/// ```ignore
/// std::thread::Builder::new()
///     .stack_size(16 * 1024 * 1024)
///     .spawn(|| writ_compiler::compile_source(src))
///     .unwrap().join().unwrap()
/// ```
pub fn compile_source(src: &'static str) -> Result<Vec<u8>, String> {
    let file_id = writ_diagnostics::FileId(0);

    // Stage 1: Parse
    let (cst_opt, parse_errs) = writ_parser::parse(src);
    if !parse_errs.is_empty() {
        return Err(format!("{} parse error(s)", parse_errs.len()));
    }
    let cst = cst_opt.ok_or("parse failed: no output")?;

    // Stage 2: Lower CST -> AST
    let (ast, lower_errs) = lower(cst);
    if !lower_errs.is_empty() {
        let msgs: Vec<String> = lower_errs.iter().map(|e| format!("{:?}", e)).collect();
        return Err(format!("lowering error(s): {}", msgs.join("; ")));
    }

    // Stage 3: Name resolution
    let asts_refs: Vec<(writ_diagnostics::FileId, &Ast)> = vec![(file_id, &ast)];
    let path_refs: Vec<(writ_diagnostics::FileId, &str)> = vec![(file_id, "<source>")];
    let (resolved, resolve_diags) = resolve::resolve(&asts_refs, &path_refs, &[]);
    if resolve_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error) {
        let msgs: Vec<String> = resolve_diags.iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(format!("resolution error(s): {}", msgs.join("; ")));
    }

    // Stage 4: Type checking
    let (typed_ast, interner, _type_env, type_diags) = check::typecheck(resolved, &asts_refs, &[]);
    if type_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error) {
        let msgs: Vec<String> = type_diags.iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(format!("type error(s): {}", msgs.join("; ")));
    }

    // Stage 5: IL codegen
    let sources: Vec<(writ_diagnostics::FileId, &str)> = vec![(file_id, src)];
    let active_conditions = std::collections::HashSet::new();
    emit_bodies(&typed_ast, &interner, &asts_refs, false, &sources, &active_conditions)
        .map_err(|diags| {
            let msgs: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
            format!("codegen error(s): {}", msgs.join("; "))
        })
}

/// Compile Writ source code against pre-compiled library modules.
///
/// This is like `compile_source` but exposes the cross-module compilation path:
/// library types are injected into the DefMap before name resolution and their
/// method signatures are injected into the TypeEnv before type checking.
///
/// `library_modules` is a slice of decoded `Module` objects (from `Module::from_bytes`)
/// that provide type definitions callable from the user source.
///
/// **Stack note:** Same as `compile_source` — consider a 16 MB stack thread for
/// deeply recursive programs.
pub fn compile_with_libraries(
    src: &'static str,
    library_modules: &[&writ_module::Module],
) -> Result<Vec<u8>, String> {
    let file_id = writ_diagnostics::FileId(0);

    // Stage 1: Parse
    let (cst_opt, parse_errs) = writ_parser::parse(src);
    if !parse_errs.is_empty() {
        return Err(format!("{} parse error(s)", parse_errs.len()));
    }
    let cst = cst_opt.ok_or("parse failed: no output")?;

    // Stage 2: Lower CST -> AST
    let (ast, lower_errs) = lower(cst);
    if !lower_errs.is_empty() {
        let msgs: Vec<String> = lower_errs.iter().map(|e| format!("{:?}", e)).collect();
        return Err(format!("lowering error(s): {}", msgs.join("; ")));
    }

    // Stage 3: Name resolution (with library module types injected into DefMap)
    let asts_refs: Vec<(writ_diagnostics::FileId, &Ast)> = vec![(file_id, &ast)];
    let path_refs: Vec<(writ_diagnostics::FileId, &str)> = vec![(file_id, "<source>")];
    let (resolved, resolve_diags) = resolve::resolve(&asts_refs, &path_refs, library_modules);
    if resolve_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error) {
        let msgs: Vec<String> = resolve_diags.iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(format!("resolution error(s): {}", msgs.join("; ")));
    }

    // Stage 4: Type checking (with library method signatures injected into TypeEnv)
    let (typed_ast, interner, _type_env, type_diags) =
        check::typecheck(resolved, &asts_refs, library_modules);
    if type_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error) {
        let msgs: Vec<String> = type_diags.iter()
            .filter(|d| d.severity == writ_diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(format!("type error(s): {}", msgs.join("; ")));
    }

    // Stage 5: IL codegen
    let sources: Vec<(writ_diagnostics::FileId, &str)> = vec![(file_id, src)];
    let active_conditions = std::collections::HashSet::new();
    emit_bodies(&typed_ast, &interner, &asts_refs, false, &sources, &active_conditions)
        .map_err(|diags| {
            let msgs: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
            format!("codegen error(s): {}", msgs.join("; "))
        })
}
