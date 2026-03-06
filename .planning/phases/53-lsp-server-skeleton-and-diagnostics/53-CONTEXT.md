# Phase 53: LSP Server Skeleton and Diagnostics - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

New `writ-lsp` crate with an AnalysisHost that wraps the existing compiler pipeline, a TextMate grammar for syntax highlighting, .writ file association, and push diagnostics to the editor. Users see inline squiggles for parse, resolve, and type errors. Cross-file diagnostics work across writ.toml projects. No semantic completions, hover, or navigation — those are Phase 54.

</domain>

<decisions>
## Implementation Decisions

### Single-file vs project mode
- Standalone mode: opening a lone .writ file (no writ.toml) gets full diagnostics (parse, resolve, typecheck) as a one-file project
- Cross-file imports in standalone mode show as unresolved (Claude decides whether error or warning severity)
- Project mode: when writ.toml exists, auto-discover all .writ files recursively under the project root
- writ.toml discovery: workspace root only — do not walk up parent directories (unlike Cargo.toml)

### Diagnostic display
- No cap on diagnostics per file — show all errors the compiler produces
- Show all severities: errors (red), warnings (yellow), notes (blue) as squiggles — VS Code's built-in severity filter is sufficient
- Related information (secondary labels) and diagnostic source name: Claude's discretion

### Diagnostic cascade
- Whether to run downstream stages when earlier stages have errors: Claude's discretion
- Error grouping and cross-file error placement: Claude's discretion
- Re-analysis scope on file change: Claude's discretion (REQUIREMENTS.md says incremental compilation/salsa is out of scope for v5.0)

### Claude's Discretion
- TextMate grammar granularity — dialogue markers, entity declarations, lifecycle hooks, formattable string interpolation, attributes: Claude picks based on what's practical in TextMate grammar, knowing that semantic highlighting (Phase 54 DIFF-01) will refine further
- Diagnostic cascade strategy (per-function isolation, stop-at-first-stage, or hybrid)
- Re-analysis scope (full project recompile vs changed-file-only)
- Related information mapping to LSP DiagnosticRelatedInformation
- Diagnostic source name ("writ" vs "writ-compiler")
- AnalysisHost architecture and internal caching
- Debounce timing for on-change diagnostics (REQUIREMENTS.md baseline: 300ms idle)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-cli/src/main.rs:321` `run_pipeline()`: Full 5-stage compilation pipeline (parse → lower → resolve → typecheck → emit). This is the exact logic the AnalysisHost needs to wrap, minus the emit stage (LSP needs diagnostics, not .writil bytes)
- `writ-diagnostics::Diagnostic`: Rich diagnostic type with severity, code, message, FileId, SimpleSpan (byte offsets), primary_label, secondary_labels, help, notes — maps directly to LSP Diagnostic
- `writ-diagnostics::render_diagnostics()`: Ariadne-based rendering — not needed for LSP (LSP uses structured diagnostics), but useful reference for what info is available
- `writ-compiler::config::load_config()`: Loads writ.toml project configuration — reusable for project discovery
- `writ-parser::parse()`: Returns `(Option<Cst>, Vec<ParseError>)` — parser already has error recovery via chumsky

### Established Patterns
- Pipeline stages accumulate `Vec<Diagnostic>` independently — each stage returns its own diagnostics
- `emit_all_bodies` has per-function error tolerance (Phase 52): skips broken functions, reports E9001 per function
- FileId(u32) identifies files across the pipeline — LSP needs to maintain a FileId registry mapping URIs to FileIds
- `writ_parser::parse` requires `&'static str` — LSP will need to Box::leak source text (same pattern as `run_pipeline`)
- Compilation runs on a separate thread in `cmd_compile` — LSP should similarly offload analysis

### Integration Points
- Workspace Cargo.toml needs `writ-lsp` added to members
- AnalysisHost consumes the same types as `run_pipeline`: `Vec<(FileId, String, &'static str)>`
- Diagnostics need byte-offset SimpleSpan converted to LSP line/column Position — source text needed for offset→line mapping
- VS Code extension package.json needs TextMate grammar reference and language contribution point

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 53-lsp-server-skeleton-and-diagnostics*
*Context gathered: 2026-03-14*
