# Phase 21: Disassembler and Runner CLI - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement a disassembler that converts binary IL modules to human-readable text, and a runner CLI that executes IL modules from the command line with say() output visible on stdout. The CLI binary (`writ`) provides `run`, `assemble`, and `disasm` subcommands. End-to-end stack validation is confirmed by executing a real dialogue module.

</domain>

<decisions>
## Implementation Decisions

### Disassembly Output Format
- Round-trippable: disasm produces valid `.writil` text format that can be fed back to `writ assemble`
- Covers all 21 metadata tables for full round-trip fidelity
- Default output is clean (no extra annotations)
- `--verbose` flag adds hex offsets and opcode byte comments for debugging

### Runner say() and Host Output
- Always annotated: all host interactions prefixed with `[say]`, `[choice]`, `[entity:spawn]`, `[entity:destroy]`, `[extern]`
- `choice()` prompts the user interactively when `--interactive` flag is passed
- Default: auto-pick choice 0 (non-interactive, safe for CI/piped usage)
- `--verbose` flag adds execution stats at the end (instructions executed, tasks spawned, etc.)

### Entry Point Convention
- Export-based: runner looks for an exported method named `"main"` (lowercase)
- CLI override: `--entry <name>` to run a different exported method
- If no exported `"main"` found and no `--entry` override: error with helpful message listing available exports
- Entry method signature flexibility: if method accepts a parameter, pass `Array<String>` of CLI args (from `-- arg1 arg2`); if zero-parameter, call with no args

### CLI Binary Structure
- Binary lives in existing `writ-cli` crate, binary name is `writ`
- Long-term vision: unified CLI (`writ build`, `writ run`, `writ disasm`, `writ assemble`, etc.)
- For now: `run`, `assemble`, `disasm` subcommands
- CLI argument parsing: clap with derive macros
- `assemble` accepts file path or stdin (`-` for stdin)
- Binary output extension: `.writc` (text = `.writil`, compiled binary = `.writc`)

### Code Organization
- Disassembler logic in `writ-assembler` crate (text<->binary bridge, symmetric with assemble)
- `CliHost` (RuntimeHost impl for say/choice/entity logging) in `writ-cli` (CLI-specific behavior)
- `NullHost` in `writ-runtime` stays untouched (testing-only host)

### Claude's Discretion
- Whether disassembler resolves string heap offsets to inline literals or keeps raw indices
- Dialogue output to stdout vs entity/debug output to stderr split
- Exact execution stats format and content
- Interactive choice prompt UX details (prompt formatting, timeout behavior)

</decisions>

<specifics>
## Specific Ideas

- User wants the CLI to feel like a unified tool (`writ run`, `writ assemble`, `writ disasm`) not a collection of separate binaries
- Choice prompts should be interactive when `--interactive` is passed, auto-picking first choice by default
- The disassembler should produce output clean enough to edit and reassemble — this is a key developer workflow

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module::Module::from_bytes()` / `to_bytes()`: Binary module I/O for disasm and run
- `writ-module::Instruction` enum: 91 opcodes with `opcode()` method, encode/decode — disassembler needs the inverse
- `writ-module::tables::ExportDefRow`: name + item_kind + item token — entry point discovery
- `writ-assembler::assemble(src)`: Text-to-binary pipeline — assemble subcommand wraps this
- `writ-runtime::RuntimeBuilder::new(module).build()`: Runtime construction for run subcommand
- `writ-runtime::Runtime::tick()` / `spawn_task()`: Task execution loop
- `writ-runtime::host::RuntimeHost` trait: CliHost will implement this
- `writ-runtime::NullHost`: Reference for host interface contract
- `writ-module::ModuleHeader`: 200-byte header with string/blob heap offsets, table directory

### Established Patterns
- Workspace crates: each concern in its own crate (writ-parser, writ-module, writ-runtime, writ-assembler)
- `RuntimeBuilder` pattern for constructing runtime with custom host
- Assembler pipeline: tokenize -> parse -> assemble (can mirror for disassemble)
- Module format: 21 metadata tables, string heap, blob heap, method bodies with decoded instructions

### Integration Points
- `writ-cli/src/main.rs`: Entry point for the `writ` binary — currently a stub
- `writ-cli/Cargo.toml`: Needs dependencies on writ-module, writ-runtime, writ-assembler, clap
- `writ-assembler/src/lib.rs`: Add `pub mod disassembler` and `pub fn disassemble(module) -> String`
- `HostRequest::ExternCall`: How say() reaches the host — CliHost intercepts this
- `ExportDefRow` in loaded module: Where entry point "main" is discovered

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 21-disassembler-and-runner-cli*
*Context gathered: 2026-03-02*
