# Phase 57: VS Code Extension Integration - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Bundle writ-lsp and writ-dap server binaries inside the VS Code extension so installing the VSIX gives a complete out-of-the-box experience — no PATH setup, no manual binary installation. Provide a default launch.json snippet for Writ debugging. End-to-end smoke test validates the integration. The LSP and DAP servers themselves are already implemented (Phases 53-56) — this phase is purely about packaging, extension wiring, and the debugging launch configuration.

</domain>

<decisions>
## Implementation Decisions

### Binary bundling strategy
- Binaries placed in `writ-vscode/bin/` directory (writ-lsp and writ-dap executables)
- Extension resolves bundled binaries via `context.asAbsolutePath(path.join('bin', 'writ-lsp'))` replacing the current dev path (`../target/debug/writ-lsp`)
- Platform-specific: build script compiles for the target platform and copies binaries to `bin/` before `vsce package`
- `.vscodeignore` ensures only `bin/`, `out/`, `syntaxes/`, and config files are included in the VSIX (no `src/`, `node_modules/`, etc.)
- `bin/` directory added to `.gitignore` — binaries are build artifacts, not checked in

### DAP client integration
- Register a `DebugAdapterDescriptorFactory` for debug type `"writ"` — standard VS Code pattern for stdio-based debug adapters
- Factory returns `DebugAdapterExecutable` pointing to the bundled `writ-dap` binary path
- Extension contributes `debuggers` in package.json with type `"writ"`, label `"Writ Debug"`, and `configurationAttributes` for launch requests
- DAP server started from the same bundled `bin/` directory as the LSP server

### launch.json snippet design
- Debug type: `"writ"`, request: `"launch"`
- Required field: `"program"` — path to the .writ entry file
- Snippet uses `"${workspaceFolder}/${file}"` as the default program value so the current file is pre-filled
- Snippet name: `"Writ: Launch Current File"`
- Configuration attributes define `program` as required string with description
- No other required fields — cwd defaults to workspace folder, no args needed for basic use

### Claude's Discretion
- Build/package script implementation details (shell script, npm script, or Makefile)
- `.vscodeignore` exact contents beyond the essentials listed above
- Whether to add a second snippet for "Launch with Arguments"
- End-to-end smoke test strategy and framework choice
- Error messages when bundled binary is not found (fallback behavior)
- Whether to support a `writ.serverPath` setting for dev override (replaces current hardcoded dev path)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Extension code
- `writ-vscode/package.json` — Current extension manifest (needs debuggers contribution, configurationSnippets)
- `writ-vscode/src/extension.ts` — Current activation code (needs DAP factory registration, bundled binary paths)
- `writ-vscode/tsconfig.json` — TypeScript configuration
- `writ-vscode/language-configuration.json` — Language configuration

### Server binaries
- `writ-lsp/src/main.rs` — LSP server binary entry point (stdio transport)
- `writ-dap/src/main.rs` — DAP server binary entry point (stdio transport)

### Prior decisions
- `53-CONTEXT.md` — LSP extension decisions: `activationEvents: []`, TextMate grammar, dev path pattern
- `55-CONTEXT.md` — DAP server architecture: stdio transport, DebugHost pattern

### VS Code API reference
- VS Code `DebugAdapterDescriptorFactory` API — for DAP integration pattern
- VS Code `configurationSnippets` in `contributes.debuggers` — for launch.json snippet

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-vscode/src/extension.ts`: Existing LSP client setup — needs modification to resolve bundled binary and add DAP factory
- `writ-vscode/package.json`: Existing language contribution — needs `debuggers` section added
- `writ-vscode/syntaxes/writ.tmLanguage.json`: TextMate grammar — no changes needed
- `vscode-languageclient` dependency already declared — no new LSP dependencies needed

### Established Patterns
- LSP client uses `LanguageClient` from `vscode-languageclient/node` with stdio `ServerOptions`
- Platform detection: `process.platform === 'win32'` for `.exe` suffix — reuse for DAP binary
- Extension activation is implicit via language contributions (`activationEvents: []`)

### Integration Points
- `package.json` `contributes.debuggers` — new section for Writ debug adapter registration
- `extension.ts` `activate()` — add DAP factory registration alongside existing LSP client start
- `extension.ts` `deactivate()` — ensure DAP cleanup alongside LSP client stop
- Workspace `Cargo.toml` — no changes needed (writ-lsp and writ-dap already in members)

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

*Phase: 57-vs-code-extension-integration*
*Context gathered: 2026-03-16*
