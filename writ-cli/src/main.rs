/// writ — Writ IL toolchain CLI
///
/// Subcommands:
///   new        Create a new Writ project
///   compile    Compile a .writ source file to a binary .writc module
///   assemble   Convert .writil text to binary .writc
///   disasm     Convert binary .writc to .writil text
///   run        Execute a binary .writc module
mod cli_host;
mod bom_utils;

use std::process;
use std::io::Read;

use clap::{Parser, Subcommand};
use writ_module::{heap::read_string, Module};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TickResult};

use cli_host::CliHost;
use bom_utils::{strip_bom_and_decode, add_utf8_bom};

// ─── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "writ", about = "Writ IL toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Writ project
    New {
        /// Project name (also used as the directory name)
        name: String,
    },

    /// Compile a .writ source file to a binary .writc module
    Compile {
        /// Input .writ source file
        input: String,

        /// Output .writc binary module (default: replaces .writ with .writc)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Assemble a .writil text file to a binary .writc module
    Assemble {
        /// Input file path (or '-' to read from stdin)
        input: String,

        /// Output file path (default: replaces .writil with .writc, or appends .writc)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Disassemble a binary .writc module to .writil text
    Disasm {
        /// Input binary module file
        input: String,

        /// Include hex byte offsets and opcode comments for each instruction
        #[arg(long)]
        verbose: bool,
    },

    /// Run a binary .writc module's entry task
    Run {
        /// Input binary module file
        input: String,

        /// Name of the exported method to run (default: "main")
        #[arg(long, default_value = "main")]
        entry: String,

        /// Enable interactive choice prompts (default: auto-select 0)
        #[arg(long)]
        interactive: bool,

        /// Print execution stats and GC info after run
        #[arg(long)]
        verbose: bool,

        // NOTE: `args: Vec<String>` for passing CLI arguments to the entry method is DEFERRED
        // to a future phase. Implementing it requires decoding the method's blob-heap signature
        // to detect param count, and allocating an Array<String> on the GC heap before the
        // task starts. For Phase 21, all entry methods are called with zero args.
    },
}

// ─── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name } => cmd_new(name),
        Commands::Compile { input, output } => cmd_compile(input, output),
        Commands::Assemble { input, output } => cmd_assemble(input, output),
        Commands::Disasm { input, verbose } => cmd_disasm(input, verbose),
        Commands::Run { input, entry, interactive, verbose } => {
            cmd_run(input, entry, interactive, verbose)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

// ─── Subcommand: new ──────────────────────────────────────────────────────────

fn cmd_new(name: String) -> Result<(), String> {
    // Validate project name (alphanumeric, hyphens, underscores)
    if name.is_empty() {
        return Err("project name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "invalid project name '{}'. use alphanumeric characters, hyphens, or underscores",
            name
        ));
    }

    // Create project directory
    std::fs::create_dir(&name)
        .map_err(|e| format!("failed to create directory '{}': {}", name, e))?;

    let project_dir = std::path::Path::new(&name);

    // Create subdirectories
    let dirs = ["sources", "bin/configuration"];
    for dir in &dirs {
        std::fs::create_dir_all(project_dir.join(dir))
            .map_err(|e| format!("failed to create directory '{}': {}", dir, e))?;
    }

    // Create writ.toml
    let toml_content = format!(
        r#"# Writ Project Configuration
# For full documentation, see: https://writ-lang.dev/spec

# ============================================================================
# [project] - Required. Project metadata
# ============================================================================
[project]
# Project name (used in compiled module metadata)
name = "{}"

# Semantic version following semver (https://semver.org/)
version = "0.1.0"

# Optional: Project authors
# authors = ["Your Name"]


# ============================================================================
# [locale] - Required. Localization configuration
# ============================================================================
[locale]
# Default locale for inline dialogue text in .writ source files
# Follows BCP 47 language tags (en, de, fr, ja, ko, zh, pt-BR, en-GB, etc.)
default = "en"

# Optional: All locales your project targets for the 'writ loc export' tool
# If omitted, only the default locale is assumed
# supported = ["en", "de", "fr"]


# ============================================================================
# [compiler] - Optional. Build settings
# ============================================================================
# [compiler]
# Source directories (relative to writ.toml). If omitted, defaults to ["src/"]
# sources = ["sources/", "dialogue/"]

# Output directory for compiled .writc artifacts (relative to writ.toml)
# output = "build/"


# ============================================================================
# [locale.export] - Optional. Localization export configuration
# ============================================================================
# [locale.export]
# Output directory for localization CSV files (relative to writ.toml)
# output = "locale/"


# ============================================================================
# [libraries.<name>] - Optional. External library mappings
# ============================================================================
# Maps logical library names to architecture-specific binary names.
# Used by [Import("name")] attributes in your code.
#
# Resolution precedence (highest to lowest):
#   1. Architecture-specific override in [Import] attribute itself
#   2. Architecture-specific override in writ.toml [libraries.<name>]
#   3. 'default' key in writ.toml [libraries.<name>]
#   4. The logical name from [Import], as-is
#
# Example:
# [libraries.physics]
# default = "libphysics"
# x64 = "physics64"
# arm64 = "physics_arm"
#
# [libraries.audio]
# default = "fmod"


# ============================================================================
# [conditions] - Optional. Conditional compilation flags
# ============================================================================
# Named conditions for #[Conditional("name")] attributes in code.
# Can be overridden via CLI: writ compile --condition debug=false
#
# CLI flags take precedence over writ.toml values.
# Undefined conditions default to false.
#
# Example:
# [conditions]
# debug = true
# playstation = false
# xbox = false
# editor = true
"#,
        name
    );

    std::fs::write(project_dir.join("writ.toml"), toml_content)
        .map_err(|e| format!("failed to write writ.toml: {}", e))?;

    // Create .gitignore
    let gitignore_content = r#"# Build artifacts
/bin/configuration/*.writc
*.writc
*.writil

# Generated files
/build/
/dist/

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
"#;

    std::fs::write(project_dir.join(".gitignore"), gitignore_content)
        .map_err(|e| format!("failed to write .gitignore: {}", e))?;

    // Create a skeleton main.writ file
    let writ_content = r#"// Entry point for your Writ project

pub fn main() {
    // TODO: Add your code here
}
"#;

    std::fs::write(project_dir.join("sources/main.writ"), writ_content)
        .map_err(|e| format!("failed to write sources/main.writ: {}", e))?;

    eprintln!(
        "Created Writ project '{}':\n  {}/\n  ├─ writ.toml\n  ├─ .gitignore\n  ├─ sources/\n  │  └─ main.writ\n  └─ bin/\n     └─ configuration/",
        name, name
    );
    eprintln!("\nNext steps:");
    eprintln!("  1. cd {}", name);
    eprintln!("  2. Edit sources/main.writ with your code");
    eprintln!("  3. Run 'writ compile sources/main.writ' to compile");

    Ok(())
}

// ─── Subcommand: compile ──────────────────────────────────────────────────────

fn cmd_compile(input: String, output: Option<String>) -> Result<(), String> {
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
            let sources = [(file_id, input.as_str(), src)];

            // Stage 1: Parse
            let (cst_opt, parse_errs) = writ_parser::parse(src);
            if !parse_errs.is_empty() {
                let err_count = parse_errs.len();
                for err in &parse_errs {
                    eprintln!("parse error at {:?}: {:?}", err.span(), err);
                }
                return Err(format!("{err_count} parse error(s)"));
            }
            drop(parse_errs);
            let cst = cst_opt.ok_or_else(|| "parse failed: no output".to_string())?;

            // Stage 2: Lower CST -> AST
            let (ast, lower_errs) = writ_compiler::lower(cst);
            if !lower_errs.is_empty() {
                let diags: Vec<_> = lower_errs.iter().map(|e| e.to_diagnostic(file_id)).collect();
                eprint!("{}", writ_diagnostics::render_diagnostics(&diags, &sources));
                return Err(format!("{} lowering error(s)", lower_errs.len()));
            }

            // Stage 3: Name resolution
            let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
                &[(file_id, &ast)],
                &[(file_id, input.as_str())],
            );
            let has_resolve_errors = resolve_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
            if !resolve_diags.is_empty() {
                eprint!("{}", writ_diagnostics::render_diagnostics(&resolve_diags, &sources));
            }
            if has_resolve_errors {
                return Err("resolution failed".to_string());
            }

            // Stage 4: Type checking
            let (typed_ast, interner, type_diags) = writ_compiler::check::typecheck(
                resolved,
                &[(file_id, &ast)],
            );
            let has_type_errors = type_diags.iter().any(|d| d.severity == writ_diagnostics::Severity::Error);
            if !type_diags.is_empty() {
                eprint!("{}", writ_diagnostics::render_diagnostics(&type_diags, &sources));
            }
            if has_type_errors {
                return Err("type checking failed".to_string());
            }

            // Stage 5: IL codegen (metadata + bodies + serialization)
            let bytes = writ_compiler::emit_bodies(&typed_ast, &interner, &[(file_id, &ast)]).map_err(|diags| {
                eprint!("{}", writ_diagnostics::render_diagnostics(&diags, &sources));
                format!("{} codegen error(s)", diags.len())
            })?;

            // Determine output path
            let out_path = output.unwrap_or_else(|| {
                if input.ends_with(".writ") {
                    input[..input.len() - 5].to_string() + ".writc"
                } else {
                    input.clone() + ".writc"
                }
            });

            // Write output
            std::fs::write(&out_path, &bytes)
                .map_err(|e| format!("failed to write '{}': {}", out_path, e))?;

            eprintln!("Compiled: {out_path}");
            Ok(())
        })
        .map_err(|e| format!("failed to spawn compile thread: {e}"))?;

    handle.join().unwrap_or_else(|_| Err("compilation panicked".to_string()))
}

// ─── Subcommand: assemble ──────────────────────────────────────────────────────

fn cmd_assemble(input: String, output: Option<String>) -> Result<(), String> {
    // Read source text
    let src = if input == "-" {
        // Read from stdin
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        strip_bom_and_decode(&bytes)
            .map_err(|e| format!("failed to decode stdin: {e}"))?
    } else {
        let bytes = std::fs::read(&input)
            .map_err(|e| format!("failed to read '{}': {}", input, e))?;
        strip_bom_and_decode(&bytes)
            .map_err(|e| format!("failed to decode '{}': {}", input, e))?
    };

    // Assemble
    let module = writ_assembler::assemble(&src).map_err(|errs| {
        errs.into_iter()
            .map(|e| format!("{}:{}: {}", e.line, e.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    // Determine output path
    let out_path = output.unwrap_or_else(|| {
        if input.ends_with(".writil") {
            input[..input.len() - 7].to_string() + ".writc"
        } else if input == "-" {
            "output.writc".to_string()
        } else {
            input.clone() + ".writc"
        }
    });

    // Serialize
    let bytes = module.to_bytes().map_err(|e| format!("serialization error: {e:?}"))?;

    // Write output
    std::fs::write(&out_path, &bytes)
        .map_err(|e| format!("failed to write '{}': {}", out_path, e))?;

    eprintln!("Assembled: {out_path}");
    Ok(())
}

// ─── Subcommand: disasm ────────────────────────────────────────────────────────

fn cmd_disasm(input: String, verbose: bool) -> Result<(), String> {
    let bytes =
        std::fs::read(&input).map_err(|e| format!("failed to read '{}': {}", input, e))?;

    let module =
        Module::from_bytes(&bytes).map_err(|e| format!("failed to parse module: {e:?}"))?;

    let text = if verbose {
        writ_assembler::disassemble_verbose(&module)
    } else {
        writ_assembler::disassemble(&module)
    };

    // Add UTF-8 BOM to disasm output
    let text_with_bom = add_utf8_bom(&text);
    print!("{text_with_bom}");
    Ok(())
}

// ─── Subcommand: run ──────────────────────────────────────────────────────────

fn cmd_run(
    input: String,
    entry: String,
    interactive: bool,
    verbose: bool,
) -> Result<(), String> {
    let bytes =
        std::fs::read(&input).map_err(|e| format!("failed to read '{}': {}", input, e))?;

    let module =
        Module::from_bytes(&bytes).map_err(|e| format!("failed to parse module: {e:?}"))?;

    // Find the named export
    let main_export = module
        .export_defs
        .iter()
        .find(|e| {
            read_string(&module.string_heap, e.name).unwrap_or("") == entry.as_str()
                && e.item_kind == 0 // kind=0 is Method
        })
        .ok_or_else(|| {
            // Collect all exported names for the error message
            let available: Vec<&str> = module
                .export_defs
                .iter()
                .filter_map(|e| read_string(&module.string_heap, e.name).ok())
                .collect();

            if available.is_empty() {
                format!(
                    "no exported method '{}' found. Available exports: (none)",
                    entry
                )
            } else {
                format!(
                    "no exported method '{}' found. Available exports: [{}]",
                    entry,
                    available.join(", ")
                )
            }
        })?;

    // Convert 1-based MetadataToken to 0-based method index
    let method_idx = (main_export.item.0 & 0x00FF_FFFF) as usize - 1;

    // Create CliHost and build runtime
    let cli_host = CliHost::new(&module, interactive, verbose);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(cli_host)
        .build()
        .map_err(|e| format!("runtime build error: {e:?}"))?;

    // Spawn the entry task
    runtime
        .spawn_task(method_idx, vec![])
        .map_err(|e| format!("spawn error: {e:?}"))?;

    // Tick loop — run until all tasks complete or an unexpected state is reached
    loop {
        match runtime.tick(0.0, ExecutionLimit::None) {
            TickResult::AllCompleted | TickResult::Empty => break,
            TickResult::TasksSuspended(pending) => {
                // CliHost handles all requests synchronously in on_request.
                // Tasks should never truly suspend. If they do, warn and exit.
                eprintln!(
                    "warning: {} task(s) suspended unexpectedly",
                    pending.len()
                );
                break;
            }
            TickResult::ExecutionLimitReached => {
                // Should not occur with ExecutionLimit::None
                break;
            }
        }
    }

    // Print stats if verbose
    if verbose {
        runtime.host().print_stats();
    }

    Ok(())
}
