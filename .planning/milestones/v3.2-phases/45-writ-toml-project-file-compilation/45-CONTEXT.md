# Phase 45: writ.toml Project File Compilation - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a `writ build` subcommand that compiles all `.writ` source files in a project directory (containing `writ.toml`) into a single `.writc` module. Support `--release` and `--debug` profile flags and a `--name` override. Single-file `writ compile foo.writ` remains unchanged.

</domain>

<decisions>
## Implementation Decisions

### Build subcommand design
- `writ build` is the dedicated project-mode subcommand — `writ compile .` does NOT auto-detect directories
- `writ compile foo.writ` continues to work as before — single-file mode is unaffected
- `writ build` accepts an optional path argument: `writ build [path]` — defaults to `.` (cwd), can point at any directory with a writ.toml
- When no writ.toml is found: error with hint — "writ.toml not found. Run `writ new <name>` to create a project."
- The `writ new` scaffold "Next steps" message updated to say `writ build` instead of `writ compile sources/main.writ`

### Profile flags (--release/--debug)
- Full `[profile.debug]` and `[profile.release]` sections in writ.toml — spec amendment needed
- Debug profile is the default when neither --release nor --debug is passed
- Release strips debug info (DebugLocal entries); debug includes them — this is the only concrete difference for now
- Profile sections exist in toml for future extensibility (optimization, strip, etc.) but only `debug_info` has effect in this phase
- The `debug` condition flag is NOT automatically set by profile — conditions and profiles are separate systems

### Output location
- Output path: `{output_base}/{profile}/{name}.writc` — e.g., `build/debug/my-game.writc`
- Default `output_base` is `build/`; `compiler.output` in writ.toml overrides the base path (e.g., `output = "dist/"` → `dist/debug/my-game.writc`)
- Output directories auto-created silently if they don't exist
- Scaffold .gitignore updated to include `/build/` (in addition to existing `*.writc` glob)

### Multi-file module naming
- `--name` flag on `writ build` overrides the module name
- Fallback chain: `--name` flag → `project.name` from writ.toml
- All discovered `.writ` files are compiled into ONE module — all top-level declarations share the same namespace
- Output verbosity: list each discovered file path, then the output path (not just a summary count)

### Claude's Discretion
- How to merge multiple ASTs through the pipeline (concatenation strategy)
- FileId assignment for multi-file compilation (how to map errors back to source files)
- Profile toml section field names and defaults
- Exact error message formatting and ariadne integration for multi-file diagnostics

</decisions>

<specifics>
## Specific Ideas

- Output should list all discovered files: each `.writ` file printed, then final "Compiled: build/debug/my-game.writc"
- The profile system should be designed for future extensibility even though only `debug_info` matters now
- Release "just strips debug artifacts for now" — optimization is a different concern for the future

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/config.rs`: `WritConfig`, `load_config()`, `discover_source_files()` — config loading and file discovery fully implemented with walkdir
- `writ-compiler/src/config.rs`: `CompilerConfig` has `sources: Vec<String>` and `output: Option<String>` — both ready to use
- `writ-cli/src/main.rs`: `cmd_compile()` has the full 5-stage pipeline (parse → lower → resolve → typecheck → emit) — can be refactored into a shared pipeline function
- `writ-cli/src/main.rs`: `cmd_new()` has the scaffold generation — needs "Next steps" message update
- `writ-cli/src/bom_utils.rs`: `strip_bom_and_decode()` — handles BOM stripping for source files

### Established Patterns
- Pipeline runs on a separate thread with 16MB stack (deep AST recursion) — multi-file must also run on this thread
- `resolve()` already accepts `&[(FileId, &Ast)]` — multi-file resolution is API-ready
- `typecheck()` already accepts `&[(FileId, &Ast)]` — multi-file typechecking is API-ready
- `emit_bodies()` already accepts `&[(FileId, &Ast)]` — multi-file codegen is API-ready
- `lower()` takes `Vec<Spanned<Item>>` (single file's CST output) — needs per-file lowering, then merge ASTs
- Clap derive pattern for CLI (Parser + Subcommand derives) — add `Build` variant to `Commands` enum

### Integration Points
- `Commands` enum in `main.rs` — add `Build` variant with optional path, --release, --debug, --name flags
- `WritConfig` in `config.rs` — add profile deserialization (new `ProfileConfig` struct)
- `writ-compiler/src/emit/mod.rs` `emit_bodies()` — may need a flag to skip DebugLocal emission for release profile
- `writ new` scaffold in `cmd_new()` — update "Next steps" text and .gitignore content

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 45-writ-toml-project-file-compilation*
*Context gathered: 2026-03-06*
