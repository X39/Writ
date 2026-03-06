# Phase 55: DAP Server Core - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

New `writ-dap` crate implementing the Debug Adapter Protocol over stdio. Users can launch a Writ program via F5 in VS Code, set source-level breakpoints on .writ lines, step through execution (over/into/out), and see a call stack with real function names and source file locations. No variable inspection, watch expressions, or task-as-thread display — those are Phase 56.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All areas delegated to Claude's judgment. Key decisions to make during research and planning:

**DAP protocol crate:**
- Whether to use the `dap` 0.4.1-alpha1 crate or hand-roll DAP message types with serde_json
- STATE.md flags `dap` crate as pre-release risk — validate it compiles against workspace toolchain as first task; have serde_json fallback ready
- Follow the LSP pattern: stdio transport, tokio runtime, similar binary structure to writ-lsp

**Launch & attach flow:**
- How F5 triggers compilation and execution — whether DAP server compiles .writ → .writil on launch or expects pre-compiled input
- Single-file vs writ.toml project handling (follow LSP's standalone/project mode pattern from Phase 53)
- How the DAP server creates and manages the RuntimeHost with debug_enabled=true

**Breakpoint UX:**
- How source-line breakpoints map to IL instruction addresses (SourceSpan table lookup)
- Verified vs pending breakpoint model
- How to handle breakpoints on lines with no instructions (snap to nearest valid line)

**Call stack display:**
- Frame naming format (e.g., "module::function" vs just "function")
- Source path resolution for multi-file projects
- How to handle frames in extern/runtime-internal code (hide or show as "[extern]")

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-runtime/src/host.rs`: `RuntimeHost` trait with `debug_enabled()`, `before_instruction()`, `on_function_enter()`, `on_function_exit()` hooks — DAP server implements this trait
- `writ-runtime/src/host.rs`: `DebugAction` enum (Continue/Break/StepOver/StepInto/StepOut/Disconnect) — DAP maps user actions to these
- `writ-runtime/src/task.rs`: `SuspendReason` enum (HostRequest/Breakpoint/DebugStep) with location context — DAP reads this to report stop events
- `writ-runtime/src/dispatch/mod.rs`: `ExecutionResult::DebugSuspend` variant — scheduler stops task and yields control to DAP
- `writ-runtime/src/scheduler.rs`: Scheduler already handles DebugSuspend by stopping task execution and returning to caller
- `writ-module/src/module.rs`: `SourceSpan` (method_idx, pc, line, col) and `DebugLocal` (register, name, type_ref, start_pc, end_pc) — breakpoint mapping source
- `writ-cli/src/cli_host.rs`: `CliHost` as reference RuntimeHost implementation — DAP server's DebugHost follows same pattern
- `writ-lsp/src/main.rs`: tower-lsp over stdio with tokio — DAP server mirrors this binary structure

### Established Patterns
- RuntimeHost trait uses default method implementations (NullHost/CliHost override selectively)
- LSP uses `Backend::new` constructor pattern with tower-lsp
- Compilation pipeline: `run_pipeline()` in writ-cli handles parse → lower → resolve → typecheck → emit
- Module loading: `Domain` loads .writil modules and resolves cross-module references
- Task scheduling: Scheduler round-robins ready tasks, yields on DebugSuspend

### Integration Points
- Workspace Cargo.toml needs `writ-dap` added to members
- DAP server needs writ-compiler (for compilation), writ-runtime (for execution with debug hooks), writ-module (for module loading and source span lookup)
- VS Code extension (Phase 57) will launch writ-dap binary over stdio
- writ-cli's `run_pipeline()` logic can be reused for compile-on-launch

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 55-dap-server-core*
*Context gathered: 2026-03-14*
