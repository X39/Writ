//! `writ compile` subcommand — compile a single .writ source file.

use crate::bom_utils::strip_bom_and_decode;
use crate::pipeline::run_pipeline;

pub fn cmd_compile(input: String, output: Option<String>, condition: Vec<String>, deny_warnings: bool) -> Result<(), String> {
    // Detect directory input and give helpful error
    if std::path::Path::new(&input).is_dir() {
        return Err(format!(
            "'{}' is a directory. Use `writ build` to compile a project.",
            input
        ));
    }

    // Build the active conditions set from CLI flags.
    let active_conditions: std::collections::HashSet<String> = condition.into_iter().collect();

    // The compiler pipeline performs deep recursive AST walks (emit_expr,
    // scan_expr_for_lambdas, has_error_nodes, collect_lambda_bodies_from_expr)
    // that overflow the default 1 MB thread stack on even simple programs.
    // Spawn the entire pipeline on a thread with a 16 MB stack — the standard
    // Rust pattern used by rustc, swc, and other AST-heavy compilers.
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            let bytes = std::fs::read(&input)
                .map_err(|e| format!("failed to read '{}': {}", input, e))?;
            let src_owned = strip_bom_and_decode(&bytes)
                .map_err(|e| format!("failed to decode '{}': {}", input, e))?;
            // Leak the source string to obtain a 'static reference required by the
            // writ_parser::parse signature (Rich<'static, Token<'src>, Span> needs
            // 'src = 'static).
            let src: &'static str = Box::leak(src_owned.into_boxed_str());
            let file_id = writ_diagnostics::FileId(0);

            let compiled_bytes = run_pipeline(
                vec![(file_id, input.clone(), src)],
                None,       // no module_name override
                true,       // always emit debug info in single-file mode
                &active_conditions,
                deny_warnings,
                &[],        // no library modules in single-file mode
            )?;

            // Determine output path
            let out_path = output.unwrap_or_else(|| {
                if input.ends_with(".writ") {
                    input[..input.len() - 5].to_string() + ".writc"
                } else {
                    input.clone() + ".writc"
                }
            });

            std::fs::write(&out_path, &compiled_bytes)
                .map_err(|e| format!("failed to write '{}': {}", out_path, e))?;

            eprintln!("Compiled: {out_path}");
            Ok(())
        })
        .map_err(|e| format!("failed to spawn compile thread: {e}"))?;

    handle.join().unwrap_or_else(|_| Err("compilation panicked".to_string()))
}
