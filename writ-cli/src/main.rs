//! writ -- Writ IL toolchain CLI.
//!
//! ## Subcommands
//!
//! - `new`      -- Create a new Writ project
//! - `build`    -- Compile all .writ sources in a project directory
//! - `compile`  -- Compile a .writ source file to a binary .writc module
//! - `assemble` -- Convert .writil text to binary .writc
//! - `disasm`   -- Convert binary .writc to .writil text
//! - `run`      -- Execute a binary .writc module
//!
//! ## Module structure
//!
//! - `cli_host`  -- RuntimeHost implementation for CLI execution
//! - `bom_utils` -- BOM detection and encoding utilities
//! - `pipeline`  -- Shared compilation pipeline (parse -> lower -> resolve -> check -> emit)
//! - `commands`  -- Subcommand implementations (new, build, compile, assemble, disasm, run)
mod cli_host;
mod bom_utils;
mod pipeline;
mod commands;

use std::process;

use clap::{Parser, Subcommand};

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

    /// Compile all .writ sources in a Writ project directory
    Build {
        /// Project directory containing writ.toml (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Compile with release profile (strips debug info)
        #[arg(long, conflicts_with = "debug")]
        release: bool,

        /// Compile with debug profile (default; includes debug info)
        #[arg(long, conflicts_with = "release")]
        debug: bool,

        /// Override the output module name (default: project.name from writ.toml)
        #[arg(long)]
        name: Option<String>,

        /// Activate a named compilation condition (may be repeated); merged with writ.toml [conditions]
        #[arg(long, action = clap::ArgAction::Append)]
        condition: Vec<String>,

        /// Treat warnings as errors (fail compilation if any warning is emitted)
        #[arg(long)]
        deny_warnings: bool,
    },

    /// Compile a .writ source file to a binary .writc module
    Compile {
        /// Input .writ source file
        input: String,

        /// Output .writc binary module (default: replaces .writ with .writc)
        #[arg(short, long)]
        output: Option<String>,

        /// Activate a named compilation condition (may be repeated)
        #[arg(long, action = clap::ArgAction::Append)]
        condition: Vec<String>,

        /// Treat warnings as errors (fail compilation if any warning is emitted)
        #[arg(long)]
        deny_warnings: bool,
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
        Commands::New { name } => commands::cmd_new(name),
        Commands::Build { path, release, debug: _, name, condition, deny_warnings } => commands::cmd_build(path, release, name, condition, deny_warnings),
        Commands::Compile { input, output, condition, deny_warnings } => commands::cmd_compile(input, output, condition, deny_warnings),
        Commands::Assemble { input, output } => commands::cmd_assemble(input, output),
        Commands::Disasm { input, verbose } => commands::cmd_disasm(input, verbose),
        Commands::Run { input, entry, interactive, verbose } => {
            commands::cmd_run(input, entry, interactive, verbose)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
