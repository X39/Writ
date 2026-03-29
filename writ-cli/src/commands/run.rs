//! `writ run` subcommand — execute a binary .writc module.

use writ_module::{heap::read_string, Module};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TickResult};

use crate::cli_host::CliHost;

/// Pre-compiled writ-std library, embedded at build time.
///
/// The `writ-cli/build.rs` script compiles `writ-std/src/collections.writ` and
/// writes the resulting bytes to `$OUT_DIR/writ-std.writc`. This constant
/// embeds those bytes so the CLI can load the standard library without any
/// file-system access at runtime.
const WRIT_STD_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/writ-std.writc"));

pub fn cmd_run(
    input: String,
    entry: String,
    interactive: bool,
    verbose: bool,
) -> Result<(), String> {
    let bytes =
        std::fs::read(&input).map_err(|e| format!("failed to read '{}': {}", input, e))?;

    let module =
        Module::from_bytes(&bytes).map_err(|e| format!("failed to parse module: {e:?}"))?;

    // Find the named export (pub functions appear in export_defs).
    // If not found, fall back to searching method_defs directly by name.
    // This allows `fn main()` (without `pub`) to work as an entry point.
    let method_idx = if let Some(export) = module
        .export_defs
        .iter()
        .find(|e| {
            read_string(&module.string_heap, e.name).unwrap_or("") == entry.as_str()
                && e.item_kind == 0 // kind=0 is Method
        })
    {
        // Convert 1-based MetadataToken to 0-based method index
        (export.item.0 & 0x00FF_FFFF) as usize - 1
    } else {
        // Fallback: search method_defs by name (handles non-pub entry points like `fn main()`)
        module
            .method_defs
            .iter()
            .enumerate()
            .find(|(_, md)| {
                read_string(&module.string_heap, md.name).unwrap_or("") == entry.as_str()
            })
            .map(|(idx, _)| idx)
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
            })?
    };

    // Load writ-std library (pre-compiled and embedded at build time)
    let std_module = Module::from_bytes(WRIT_STD_BYTES)
        .map_err(|e| format!("failed to load writ-std: {e:?}"))?;

    // Create CliHost and build runtime
    let cli_host = CliHost::new(&module, interactive, verbose);
    let mut runtime = RuntimeBuilder::new(module)
        .with_library(std_module)
        .with_host(cli_host)
        .build()
        .map_err(|e| format!("runtime build error: {e:?}"))?;

    // Spawn the entry task
    runtime
        .spawn_task(method_idx, vec![])
        .map_err(|e| format!("spawn error: {e:?}"))?;

    // Tick the runtime once — CliHost handles all requests synchronously in
    // on_request, so a single tick drains all tasks without iteration.
    match runtime.tick(0.0, ExecutionLimit::None) {
        TickResult::AllCompleted | TickResult::Empty => {}
        TickResult::TasksSuspended(pending) => {
            // CliHost handles all requests synchronously in on_request.
            // Tasks should never truly suspend. If they do, warn and exit.
            eprintln!(
                "warning: {} task(s) suspended unexpectedly",
                pending.len()
            );
        }
        TickResult::ExecutionLimitReached => {
            // Should not occur with ExecutionLimit::None
        }
    }

    // Print stats if verbose
    if verbose {
        runtime.host().print_stats();
    }

    Ok(())
}
