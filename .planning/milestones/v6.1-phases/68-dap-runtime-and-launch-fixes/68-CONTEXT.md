# Phase 68: DAP Runtime and Launch Fixes - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix two issues in the DAP server: (1) resolve the "Switch target byte offset not found in offset map" decode error when running programs with enum match expressions, and (2) add support for launching writ.toml multi-file projects through DAP (not just single .writ files). No new DAP features (conditional breakpoints, logpoints, etc.) — strictly bug fix + multi-file launch.

</domain>

<decisions>
## Implementation Decisions

### Decode error fix (DAP-01)
- Fix at compile time in the emitter, not in the runtime loader
- Root cause: `emit_enum_match` in `writ-compiler/src/emit/body/patterns.rs` patches SWITCH instruction offsets as instruction-index distances (target_pos - switch_idx), but the binary format expects byte-position-relative offsets
- Br/BrTrue/BrFalse go through the `add_fixup()` → `apply_fixups()` pipeline in `serialize.rs` which converts instruction indices to byte offsets — SWITCH bypasses this because it uses inline Vec patching
- Fix: extend `encode_instructions()` in `serialize.rs` to post-process SWITCH instructions, converting their instruction-index offsets to byte-position offsets using `instr_byte_starts[]`
- Alternatively, SWITCH could use the fixup system, but since it has variable-length offsets (one per variant), the current per-instruction fixup model doesn't fit — post-process is simpler

### Multi-file launch (DAP-02)
- Auto-detect mode from the `program` launch argument: if path ends in `.writ` → single-file mode (existing behavior); if path is a directory or `writ.toml` → project mode
- Project mode reuses `writ_compiler::config::load_config()` and `discover_source_files()` from the `writ build` pipeline
- DAP's `compile_and_load` gets a second entry point (or refactored to accept both modes) that mirrors `cmd_build` in `writ-cli/src/commands/build.rs`
- All discovered source files get unique FileId values, just like `writ build`

### Multi-file source tracking
- Extend `DapServer.source_path: Option<String>` to `source_paths: Vec<(FileId, String)>` to track all source files
- Stack frame source references use FileId from SourceSpan to look up the correct file path
- Breakpoints are resolved per-file using the source path to FileId mapping

### Claude's Discretion
- Whether to refactor `compile_and_load` into one function with an enum parameter or two separate functions
- Exact error message wording for project-mode failures (missing writ.toml, no source files found)
- Whether to share the `run_pipeline` function between writ-cli and writ-dap (currently duplicated)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Compiler emission (DAP-01 fix)
- `writ-compiler/src/emit/body/patterns.rs` — SWITCH offset patching in `emit_enum_match()` (the bug site)
- `writ-compiler/src/emit/serialize.rs` — `encode_instructions()` fixup pipeline (where byte-offset conversion happens)
- `writ-compiler/src/emit/body/labels.rs` — `LabelAllocator` and `apply_fixups()` (existing fixup model)

### Runtime loader (verification)
- `writ-runtime/src/loader.rs` — `decode_and_reindex()` that builds offset_map and reports the error

### DAP server (DAP-02 multi-file launch)
- `writ-dap/src/launch.rs` — `compile_and_load()` single-file pipeline (extend for multi-file)
- `writ-dap/src/server/handlers.rs` — `handle_launch()` that calls compile_and_load
- `writ-dap/src/server/mod.rs` — DapServer struct with source_path field

### Reference implementation (multi-file pattern)
- `writ-cli/src/commands/build.rs` — `cmd_build()` showing writ.toml project compilation pattern
- `writ-compiler/src/config.rs` — `load_config()` and `discover_source_files()` APIs

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ_compiler::config::load_config()` + `discover_source_files()`: Already handles writ.toml parsing and .writ file discovery — reuse directly in DAP
- `run_pipeline()` in `writ-dap/src/launch.rs`: Already accepts `Vec<(FileId, String, &'static str)>` for multi-file input — just needs multi-file callers
- `compute_instr_byte_starts()` in `serialize.rs`: Already computes instruction-to-byte-offset mapping — reuse for SWITCH fixup

### Established Patterns
- Branch fixup pipeline: instruction-index labels → byte-position labels → apply_fixups() — SWITCH needs analogous treatment
- Single-file vs project detection: `writ build` uses writ.toml presence; DAP should mirror this
- FileId assignment: sequential u32 starting at 0, one per source file

### Integration Points
- `writ-dap/src/launch.rs` `compile_and_load()` is the entry point for both fixes
- `writ-dap/src/server/handlers.rs` `handle_launch()` passes program_path to compile_and_load
- `writ-dap/src/server/mod.rs` DapServer struct holds source_path (needs extension to multi-file)
- `writ-compiler/src/emit/serialize.rs` `encode_instructions()` is where SWITCH byte-offset conversion must be added

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 68-dap-runtime-and-launch-fixes*
*Context gathered: 2026-03-18*
