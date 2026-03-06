# Phase 59: VSIX Release Build - Context

**Gathered:** 2026-03-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Execute the full VSIX packaging pipeline to produce a distributable .vsix artifact with bundled release binaries. All build infrastructure (npm scripts, copy-bins.js, smoke-test.sh, .vscodeignore) was created in Phase 57 — this phase runs the pipeline and verifies the output.

</domain>

<decisions>
## Implementation Decisions

### Build target
- Build for current platform only (Windows x86_64) — this is v0.1.0, single-platform distribution
- Cross-compilation for other platforms is a future concern

### Build pipeline
- Use existing `npm run package` in writ-vscode/ which chains: compile → build-binaries → copy-binaries → vsce package
- `cargo build --release -p writ-lsp -p writ-dap` produces optimized binaries in target/release/
- `copy-bins.js` copies release binaries to writ-vscode/bin/
- `vsce package` bundles everything into a .vsix

### Artifact handling
- .vsix file lands in writ-vscode/ directory (standard vsce behavior)
- No need to copy it elsewhere — this is the canonical location

### Post-build verification
- Run smoke-test.sh as a separate step after packaging (not chained into npm run package)
- Smoke test validates: extension.js, writ-lsp binary, writ-dap binary, package.json, TextMate grammar, debuggers contribution
- Clearer diagnostics on failure when run separately

### Claude's Discretion
- Whether to fix any build warnings or errors encountered during cargo build --release
- Exact npm/node version requirements (use whatever is currently installed)
- Whether to add the .vsix path to .gitignore if not already excluded

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Build scripts
- `writ-vscode/package.json` — npm scripts defining the build pipeline (compile, build-binaries, copy-binaries, package)
- `writ-vscode/scripts/copy-bins.js` — Binary copy script (target/release/ → bin/)
- `writ-vscode/scripts/smoke-test.sh` — Post-build VSIX validation script

### Extension packaging
- `writ-vscode/.vscodeignore` — Controls what goes into the .vsix
- `writ-vscode/tsconfig.json` — TypeScript compilation config

### Prior phase decisions
- `.planning/phases/57-vs-code-extension-integration/57-CONTEXT.md` — Binary bundling strategy, .vscodeignore setup, build script design

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `npm run package`: Complete pipeline — compile TS, cargo build --release, copy bins, vsce package
- `scripts/copy-bins.js`: Platform-aware binary copy (adds .exe on Windows), fails fast if binary missing
- `scripts/smoke-test.sh`: Validates VSIX contents including debuggers contribution via Node.js extraction

### Established Patterns
- Release binaries go to `target/release/` (standard Cargo convention)
- `bin/` directory is in `.gitignore` (build artifacts not committed)
- `.vscodeignore` excludes `*.vsix` from VSIX but not from working directory

### Integration Points
- `Cargo.toml` workspace members include `writ-lsp` and `writ-dap` — cargo build --release targets them
- `writ-vscode/bin/` is the binary staging directory between cargo build and vsce package
- `.vscodeignore` already configured — no changes needed

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Run the existing pipeline and verify it works.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 59-vsix-release-build*
*Context gathered: 2026-03-16*
