//! `writ build` subcommand — compile a Writ project directory.

use crate::bom_utils::strip_bom_and_decode;
use crate::pipeline::run_pipeline;

pub fn cmd_build(path: String, release: bool, name_override: Option<String>) -> Result<(), String> {
    let project_root = std::path::Path::new(&path).to_path_buf();

    // Load writ.toml
    let config = writ_compiler::config::load_config(&project_root).map_err(|e| {
        match e {
            writ_compiler::config::ConfigError::MissingToml(_) => {
                format!("writ.toml not found in '{}'. Run `writ new <name>` to create a project.", path)
            }
            other => format!("{}", other),
        }
    })?;

    // Determine profile
    let profile_name = if release { "release" } else { "debug" };
    let profile_cfg = if release { &config.profile.release } else { &config.profile.debug };
    let emit_debug_info = profile_cfg.debug_info;

    // Determine module name: --name flag > project.name from writ.toml
    let module_name = name_override.unwrap_or_else(|| config.project.name.clone());

    // Discover source files
    let discovered = writ_compiler::config::discover_source_files(&project_root, &config)
        .map_err(|e| format!("failed to discover source files: {}", e))?;

    if discovered.is_empty() {
        return Err(format!(
            "no .writ source files found in {:?}",
            config.compiler.sources
        ));
    }

    // Print discovered files
    eprintln!("Compiling {} file(s) [{}]:", discovered.len(), profile_name);
    for f in &discovered {
        eprintln!("  {}", f.display());
    }

    // Spawn compilation on a 16MB-stack thread
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> Result<Vec<u8>, String> {
            // Read and leak all source files
            let mut file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)> = Vec::new();

            for (n, file_path) in discovered.iter().enumerate() {
                let file_id = writ_diagnostics::FileId(n as u32);
                let bytes = std::fs::read(file_path)
                    .map_err(|e| format!("failed to read '{}': {}", file_path.display(), e))?;
                let src_owned = strip_bom_and_decode(&bytes)
                    .map_err(|e| format!("failed to decode '{}': {}", file_path.display(), e))?;
                let src: &'static str = Box::leak(src_owned.into_boxed_str());
                let display_path = file_path.display().to_string();
                file_sources.push((file_id, display_path, src));
            }

            run_pipeline(file_sources, None, emit_debug_info)
        })
        .map_err(|e| format!("failed to spawn compile thread: {e}"))?;

    let compiled_bytes = handle.join().unwrap_or_else(|_| Err("compilation panicked".to_string()))?;

    // Determine output path: {output_base}/{profile}/{module_name}.writc
    let output_base = config.compiler.output.as_deref().unwrap_or("build");
    let out_dir = project_root.join(output_base).join(profile_name);
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create output directory '{}': {}", out_dir.display(), e))?;
    let out_path = out_dir.join(format!("{}.writc", module_name));

    std::fs::write(&out_path, &compiled_bytes)
        .map_err(|e| format!("failed to write '{}': {}", out_path.display(), e))?;

    eprintln!("Compiled: {}", out_path.display());
    Ok(())
}
