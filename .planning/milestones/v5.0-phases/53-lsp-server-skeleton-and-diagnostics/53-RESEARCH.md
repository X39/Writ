# Phase 53: LSP Server Skeleton and Diagnostics - Research

**Researched:** 2026-03-14
**Domain:** Language Server Protocol (tower-lsp), VS Code extension (TypeScript/vscode-languageclient), TextMate grammar
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Single-file vs project mode:**
- Standalone mode: opening a lone .writ file (no writ.toml) gets full diagnostics (parse, resolve, typecheck) as a one-file project
- Cross-file imports in standalone mode show as unresolved (Claude decides whether error or warning severity)
- Project mode: when writ.toml exists, auto-discover all .writ files recursively under the project root
- writ.toml discovery: workspace root only — do not walk up parent directories (unlike Cargo.toml)

**Diagnostic display:**
- No cap on diagnostics per file — show all errors the compiler produces
- Show all severities: errors (red), warnings (yellow), notes (blue) as squiggles — VS Code's built-in severity filter is sufficient
- Related information (secondary labels) and diagnostic source name: Claude's discretion

**Diagnostic cascade:**
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

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LSP-01 | Language server publishes diagnostics (errors and warnings) as inline editor squiggles on file save or change | tower-lsp `Client::publish_diagnostics` + `did_save`/`did_change` handlers |
| LSP-08 | All LSP features work across multiple files in a writ.toml project (cross-file resolution) | AnalysisHost loads all files via `discover_source_files`, feeds multi-file slice to `resolve::resolve` |
| EXT-01 | VS Code extension provides TextMate syntax highlighting for .writ files | `.tmLanguage.json` grammar file + `contributes.grammars` in package.json |
| EXT-02 | Extension registers .writ file association and activates automatically on opening .writ files | `contributes.languages` in package.json with `extensions: [".writ"]` |
</phase_requirements>

---

## Summary

Phase 53 requires two deliverables: a new `writ-lsp` Rust binary that speaks LSP over stdio using `tower-lsp`, and a VS Code extension that bundles the grammar and launches the server. The Rust crate wraps the existing 5-stage compiler pipeline to produce structured diagnostics rather than compiled bytes. The extension ships a TextMate grammar for immediate syntax highlighting and a TypeScript entrypoint that launches `writ-lsp` as a child process over stdio.

The `tower-lsp` crate (v0.20.0) is the de facto standard Rust LSP framework. It provides the `LanguageServer` async trait, the `Client` handle for push-notifications, and a tokio-based `Server` that reads/writes JSON-RPC over stdin/stdout. The compiler pipeline is synchronous and CPU-bound, so analysis must be offloaded via `tokio::task::spawn_blocking` to avoid blocking the async reactor. The document store uses `DashMap<String, String>` for lock-free concurrent access by URI.

The VS Code side uses `vscode-languageclient` (Node.js npm package) with a simple TypeScript extension that launches the Rust binary as a stdio server. The TextMate grammar needs to cover: keywords, control flow, dialogue sigils (`@`, `$`, `#`), entity/dlg declarations, string literals (including formattable `$"..."` prefix), comments, and attributes. Semantic highlighting (Phase 54) will refine further.

**Primary recommendation:** Add `writ-lsp` as a new Cargo workspace member with `tower-lsp`, `tokio`, and `dashmap` dependencies. Create a `writ-vscode` directory at the workspace root with a minimal TypeScript extension. Wire `did_open`, `did_change`, and `did_save` all to trigger `spawn_blocking` analysis and `publish_diagnostics`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tower-lsp | 0.20.0 | Rust LSP framework — `LanguageServer` trait, `Client::publish_diagnostics`, stdio `Server` | De facto standard for Rust LSP servers; 2100+ dependents |
| tokio | 1.x | Async runtime required by tower-lsp | tower-lsp default feature depends on tokio |
| dashmap | 6.x | Lock-free concurrent `HashMap` for document store (URI → content) | Industry pattern for LSP document stores (tower-lsp-boilerplate uses it) |
| lsp-types | 0.94.1 | LSP types: `Diagnostic`, `Range`, `Position`, `DiagnosticSeverity` | Re-exported from tower-lsp; pinned by tower-lsp 0.20.0 |
| async-trait | 0.1 | Required for `#[tower_lsp::async_trait]` macro on `impl LanguageServer` | Tower-lsp's trait uses it |
| vscode-languageclient | ^9.0 | Node.js LSP client for the VS Code extension | Official Microsoft library for VS Code LSP client side |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde_json | 1.0 | JSON serialization for any custom LSP payloads | Already in tower-lsp dependency graph |
| url | 2.x | `Url` type for document URIs | Used by `Client::publish_diagnostics(uri: Url, ...)` |
| tokio (features) | 1.x | `rt-multi-thread`, `macros`, `io-std` | `io-std` needed for `tokio::io::stdin()/stdout()` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tower-lsp | tower-lsp-server (community fork) | Fork addresses lsp-types maintenance, but adds churn for a first LSP phase; use tower-lsp 0.20 for now |
| tower-lsp | lsp-server (rust-analyzer's crate) | Lower-level, more control; but requires building async dispatch manually — not worth it here |
| DashMap | `Arc<RwLock<HashMap>>` | DashMap has better concurrent write throughput; RwLock is fine but more boilerplate |
| TypeScript extension | Standalone .vsix with embedded binary | Embedding binary is Phase 57 (EXT-03); Phase 53 uses dev path to binary |

**Installation:**
```bash
# In writ-lsp/Cargo.toml
[dependencies]
tower-lsp = "0.20"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std"] }
dashmap = "6"
async-trait = "0.1"
url = "2"
writ-compiler = { path = "../writ-compiler" }
writ-diagnostics = { path = "../writ-diagnostics" }
writ-parser = { path = "../writ-parser" }

# In writ-vscode/ (extension)
npm install vscode-languageclient
```

---

## Architecture Patterns

### Recommended Project Structure

```
writ-lsp/
├── Cargo.toml              # [[bin]] name = "writ-lsp"
└── src/
    ├── main.rs             # tokio::main, LspService::new, Server::new(stdin, stdout).serve
    ├── backend.rs          # Backend struct, impl LanguageServer
    ├── analysis_host.rs    # AnalysisHost: wraps compiler pipeline, returns Vec<Diagnostic>
    └── convert.rs          # span_to_range(), severity_to_lsp(), diag_to_lsp()

writ-vscode/
├── package.json            # contributes: languages, grammars; activationEvents; vscode-languageclient dep
├── tsconfig.json
├── src/
│   └── extension.ts        # activate(): ServerOptions (command: writ-lsp path), LanguageClient
└── syntaxes/
    └── writ.tmLanguage.json
```

### Pattern 1: tower-lsp Backend with Document Store

**What:** Backend struct holds `Client` + `DashMap<String, String>` for document contents. All `LanguageServer` notification handlers update the map and trigger analysis.

**When to use:** Always — this is the standard tower-lsp pattern.

**Example:**
```rust
// Source: tower-lsp 0.20.0 docs + tower-lsp-boilerplate pattern
use dashmap::DashMap;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    document_map: DashMap<String, String>,  // URI string → source text
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(
                            SaveOptions { include_text: Some(true) }
                        )),
                        ..Default::default()
                    }
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> { Ok(()) }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        self.document_map.insert(uri.clone(), text);
        self.publish_diagnostics_for(params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // FULL sync — last content_change has full text
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri.to_string();
            self.document_map.insert(uri, change.text);
        }
        // No re-publish on change — wait for did_save (saves are the trigger per CONTEXT.md)
        // OR re-publish with debounce here for live diagnostics
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // did_save may include text if include_text = true in SaveOptions
        self.publish_diagnostics_for(params.text_document.uri).await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        document_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

### Pattern 2: spawn_blocking for Compiler Analysis

**What:** Compiler pipeline is synchronous and CPU-bound. Call it inside `tokio::task::spawn_blocking` so it runs on a dedicated thread and does not block the async reactor.

**When to use:** Every time analysis is triggered (did_open, did_save, did_change).

**Example:**
```rust
// Source: tokio docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
impl Backend {
    async fn publish_diagnostics_for(&self, uri: Url) {
        let uri_str = uri.to_string();
        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return,
        };
        let client = self.client.clone();

        // Clone any Arc-wrapped project state here before moving into closure
        let result = tokio::task::spawn_blocking(move || {
            // AnalysisHost::analyze() runs the compiler pipeline synchronously
            AnalysisHost::analyze_standalone(source)
        })
        .await;

        match result {
            Ok(lsp_diags) => {
                client.publish_diagnostics(uri, lsp_diags, None).await;
            }
            Err(e) => {
                eprintln!("analysis panicked: {e:?}");
            }
        }
    }
}
```

### Pattern 3: Byte Offset to LSP Position Conversion

**What:** The Writ compiler uses `SimpleSpan` (byte offsets). LSP requires `Position { line: u32, character: u32 }` where character is the UTF-16 code unit offset within the line. For ASCII-only source (99% of Writ code), byte offset == UTF-16 offset.

**When to use:** In `convert.rs` whenever mapping a `SimpleSpan` to an LSP `Range`.

**Example:**
```rust
// Source: lsp-types docs, Position struct; standard offset-to-line algorithm
use lsp_types::{Position, Range};
use chumsky::span::Span as _;

/// Convert a byte offset into the source text to an LSP Position.
/// Returns Position { line: 0, character: 0 } for out-of-bounds offsets.
fn offset_to_position(source: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut last_line_start = 0usize;
    for (i, ch) in source.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }
    // character = UTF-16 code units from line start to byte_offset
    let line_slice = &source[last_line_start..byte_offset.min(source.len())];
    let character = line_slice.chars().map(|c| c.len_utf16() as u32).sum();
    Position { line, character }
}

fn span_to_range(source: &str, span: &chumsky::span::SimpleSpan) -> Range {
    Range {
        start: offset_to_position(source, span.start()),
        end: offset_to_position(source, span.end()),
    }
}
```

### Pattern 4: AnalysisHost — Compiler Pipeline Wrapper

**What:** `AnalysisHost` wraps `run_pipeline` logic from `writ-cli/src/main.rs` but stops before the emit stage and collects `Vec<Diagnostic>` from all stages instead of rendering them.

**Cascade strategy (Claude's discretion): continue through all stages up to typecheck regardless of earlier errors.** This matches what users expect — seeing resolve AND type errors simultaneously rather than stopping at the first stage with errors. The emit stage is skipped entirely (LSP needs diagnostics, not .writil bytes).

**Exception:** If parse produces zero CST output (total parse failure), stop — there is nothing to lower/resolve/typecheck.

```rust
// Source: writ-cli/src/main.rs:321 run_pipeline() adapted
pub struct AnalysisResult {
    pub diagnostics: Vec<writ_diagnostics::Diagnostic>,
    pub source_texts: Vec<(writ_diagnostics::FileId, String)>,  // for span conversion
}

impl AnalysisHost {
    pub fn analyze_standalone(source: String) -> AnalysisResult {
        let src: &'static str = Box::leak(source.clone().into_boxed_str());
        let file_id = writ_diagnostics::FileId(0);
        let mut all_diags: Vec<writ_diagnostics::Diagnostic> = Vec::new();

        // Stage 1: Parse (error recovery via chumsky — PREP-02 done)
        let (cst_opt, _parse_errs) = writ_parser::parse(src);
        // parse errors — add to all_diags (convert ParseError to Diagnostic)
        let cst = match cst_opt {
            Some(c) => c,
            None => return AnalysisResult { diagnostics: all_diags,
                                            source_texts: vec![(file_id, source)] },
        };

        // Stage 2: Lower
        let (ast, lower_errs) = writ_compiler::lower(cst);
        all_diags.extend(lower_errs.iter().map(|e| e.to_diagnostic(file_id)));

        // Stage 3: Resolve (run even if lower had errors)
        let asts_refs = vec![(file_id, &ast)];
        let path_refs = vec![(file_id, "<standalone>")];
        let (resolved, resolve_diags) = writ_compiler::resolve::resolve(&asts_refs, &path_refs);
        all_diags.extend(resolve_diags);

        // Stage 4: Typecheck (run even if resolve had errors)
        let (_typed, _interner, type_diags) = writ_compiler::check::typecheck(resolved, &asts_refs);
        all_diags.extend(type_diags);

        // Stage 5: SKIP emit — LSP only needs diagnostics

        AnalysisResult { diagnostics: all_diags, source_texts: vec![(file_id, source)] }
    }
}
```

### Pattern 5: VS Code Extension — Launch Rust Binary over stdio

**What:** TypeScript extension.ts uses `vscode-languageclient` with a command-based `ServerOptions` to spawn the `writ-lsp` binary over stdio.

**When to use:** Phase 53 uses a dev path. Phase 57 (EXT-03) will bundle the binary.

**Example:**
```typescript
// Source: vscode-languageclient Node.js docs + github.com/microsoft/vscode-discussions/discussions/1099
import * as path from 'path';
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    // Phase 53: dev path — points to compiled Cargo target
    // Phase 57 will use context.asAbsolutePath() to resolve bundled binary
    const serverCommand = context.asAbsolutePath(
        path.join('..', 'target', 'debug', 'writ-lsp')
    );

    const serverOptions = {
        command: serverCommand,
        args: [],
        options: { shell: false }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'writ' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.writ')
        }
    };

    client = new LanguageClient('writ', 'Writ Language Server', serverOptions, clientOptions);
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
```

### Pattern 6: TextMate Grammar for Writ

**What:** JSON grammar file covering Writ's lexical structure. Semantic highlighting (Phase 54) will layer on top.

**Practical scope coverage for Phase 53:**

```json
{
    "scopeName": "source.writ",
    "patterns": [
        { "include": "#comments" },
        { "include": "#attributes" },
        { "include": "#keywords" },
        { "include": "#dialogue-blocks" },
        { "include": "#strings" },
        { "include": "#numbers" },
        { "include": "#operators" }
    ],
    "repository": {
        "comments": {
            "patterns": [
                { "name": "comment.line.double-slash.writ",
                  "match": "//.*$" },
                { "name": "comment.block.writ",
                  "begin": "/\\*", "end": "\\*/",
                  "beginCaptures": {"0": {"name": "comment.block.writ"}},
                  "endCaptures": {"0": {"name": "comment.block.writ"}} }
            ]
        },
        "attributes": {
            "name": "meta.attribute.writ",
            "begin": "\\[", "end": "\\]",
            "patterns": [{ "include": "#strings" }]
        },
        "keywords": {
            "patterns": [
                { "name": "keyword.declaration.writ",
                  "match": "\\b(fn|dlg|struct|enum|contract|impl|entity|component|namespace|extern|using)\\b" },
                { "name": "keyword.control.writ",
                  "match": "\\b(if|else|match|for|while|in|return|break|continue|try)\\b" },
                { "name": "keyword.other.writ",
                  "match": "\\b(let|mut|const|global|pub|priv|void|self|true|false|null|new|on|use|spawn|detached|join|cancel|defer|atomic)\\b" },
                { "name": "storage.type.primitive.writ",
                  "match": "\\b(int|float|bool|string|char)\\b" }
            ]
        },
        "dialogue-blocks": {
            "comment": "@ speaker attribution sigil inside dlg blocks",
            "patterns": [
                { "name": "markup.bold.speaker.writ",
                  "match": "^\\s*(@[A-Za-z_][A-Za-z0-9_]*)\\b",
                  "captures": {"1": {"name": "entity.name.tag.speaker.writ"}} },
                { "name": "keyword.operator.dialogue-escape.writ",
                  "match": "\\$(?=\\s|\\{|if|match|choice)" }
            ]
        },
        "strings": {
            "patterns": [
                { "name": "string.quoted.double.formattable.writ",
                  "begin": "\\$\"", "end": "\"",
                  "beginCaptures": {"0": {"name": "punctuation.definition.string.begin.writ"}},
                  "endCaptures": {"0": {"name": "punctuation.definition.string.end.writ"}},
                  "patterns": [
                      { "name": "constant.character.escape.writ", "match": "\\\\." },
                      { "name": "meta.interpolation.writ",
                        "begin": "\\{", "end": "\\}",
                        "beginCaptures": {"0": {"name": "punctuation.section.interpolation.begin.writ"}},
                        "endCaptures": {"0": {"name": "punctuation.section.interpolation.end.writ"}} }
                  ]
                },
                { "name": "string.quoted.double.writ",
                  "begin": "\"", "end": "\"",
                  "beginCaptures": {"0": {"name": "punctuation.definition.string.begin.writ"}},
                  "endCaptures": {"0": {"name": "punctuation.definition.string.end.writ"}},
                  "patterns": [
                      { "name": "constant.character.escape.writ", "match": "\\\\." }
                  ]
                },
                { "name": "string.quoted.triple.writ",
                  "begin": "\"\"\"", "end": "\"\"\"" }
            ]
        },
        "numbers": {
            "name": "constant.numeric.writ",
            "match": "\\b[0-9][0-9_]*(\\.[0-9][0-9_]*)?\\b"
        },
        "operators": {
            "name": "keyword.operator.writ",
            "match": "(::|->|\\.\\.=|\\.\\.|[+\\-*/%=<>!&|^?!])"
        }
    }
}
```

### Anti-Patterns to Avoid

- **Blocking tokio reactor:** Never call the compiler pipeline directly in an `async fn` body without `spawn_blocking`. The writ compiler walks deep ASTs recursively and is slow + stack-hungry.
- **Leaking memory without bound:** `Box::leak` source text is the existing pattern. For an LSP that processes many files repeatedly, consider a `SourceArena` or interning scheme in a future phase. Phase 53 should continue the Box::leak pattern consistent with `run_pipeline`.
- **Using `tokio::sync::Mutex` to guard the document store:** Use `DashMap` or `std::sync::Mutex` instead. Holding `tokio::sync::Mutex` across `.await` points causes subtle deadlocks.
- **Publishing diagnostics for wrong file:** Always map diagnostics by `primary_file: FileId` back to their source URI before calling `publish_diagnostics`. Cross-file diagnostics must be published per-URI, not all under the triggering file.
- **Forgetting to clear stale diagnostics:** When a file is fixed, publish an empty `Vec<Diagnostic>` for that URI to clear old squiggles. `did_close` should also clear.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON-RPC LSP protocol | Custom stdio framing | tower-lsp `Server` | Content-length headers, batching, error codes — all specified by LSP spec 3.17 |
| LSP type definitions | Custom structs for Diagnostic, Range, Position | `lsp_types` crate (re-exported from tower-lsp) | 60+ types; Position UTF-16 encoding rules are subtle |
| Async trait dispatch | Manual boxed futures | `#[tower_lsp::async_trait]` macro | Required by tower-lsp trait; don't remove the macro |
| TextMate grammar tooling | Custom syntax highlighter | VS Code built-in tmLanguage engine | TextMate regex engine is built into VS Code's editor core |
| Document change tracking | Implement INCREMENTAL sync | Use FULL sync | FULL is simpler; compiler pipeline re-processes whole file anyway; INCREMENTAL only matters with salsa (out of scope) |

**Key insight:** The LSP protocol is vastly more complex than just "send JSON." Content-length framing, initialization handshake, capability negotiation, notification vs request distinction — tower-lsp handles all of this. The 200 lines of AnalysisHost wrapping are the actual engineering work.

---

## Common Pitfalls

### Pitfall 1: Box::leak Accumulation
**What goes wrong:** Each analysis run leaks the source text. Over a long session, memory grows unboundedly.
**Why it happens:** `writ_parser::parse` requires `&'static str`. The existing pattern uses `Box::leak` to satisfy this.
**How to avoid:** Phase 53 should stay consistent with existing pattern. Document the leak in a TODO comment. A proper solution (source interner / typed-arena) is v6+ scope.
**Warning signs:** VS Code process memory grows linearly with edits over hours.

### Pitfall 2: Blocking the Tokio Reactor
**What goes wrong:** LSP stops responding; VS Code shows "Language server not responding".
**Why it happens:** Calling `writ_compiler::lower()` etc. directly inside `async fn did_save()` without `spawn_blocking`.
**How to avoid:** Always wrap analysis in `tokio::task::spawn_blocking(move || { ... })`.await`.
**Warning signs:** LSP works for small files but hangs on larger ones.

### Pitfall 3: UTF-16 Position Encoding
**What goes wrong:** Squiggles appear at wrong positions for files with non-ASCII characters (e.g., dialogue text with accented characters).
**Why it happens:** LSP Position.character is in UTF-16 code units, not bytes or Unicode scalars.
**How to avoid:** Use the `offset_to_position()` helper that sums `char.len_utf16()` per character.
**Warning signs:** Squiggles are offset by 1 character for every non-BMP character in the file.

### Pitfall 4: Stale Diagnostics on Fix
**What goes wrong:** Red squiggle remains after fixing the error.
**Why it happens:** `publish_diagnostics` was never called with an empty vec after the error was removed.
**How to avoid:** Always call `publish_diagnostics(uri, vec![], None)` when analysis produces zero diagnostics. Also handle `did_close` by clearing.
**Warning signs:** Fixing a file doesn't remove squiggles until VS Code is restarted.

### Pitfall 5: Cross-File Diagnostic URI Mismatch
**What goes wrong:** A diagnostic from file B appears under file A's squiggles; file B shows nothing.
**Why it happens:** All diagnostics are published to the triggering file's URI rather than the URI corresponding to `primary_file: FileId`.
**How to avoid:** The `AnalysisHost` must return diagnostics grouped by `FileId`. The backend must map each `FileId` back to its `Url` via a registry (URI map) and call `publish_diagnostics` once per affected file.
**Warning signs:** Opening file A shows errors that belong in file B; file B is clean.

### Pitfall 6: ParseError Conversion Gap
**What goes wrong:** Parse errors don't show as squiggles because `writ_parser::parse` returns `Vec<ParseError>` (not `Vec<writ_diagnostics::Diagnostic>`).
**Why it happens:** `run_pipeline` in writ-cli prints parse errors directly without converting to `Diagnostic` structs. LSP needs structured diagnostics.
**How to avoid:** The `AnalysisHost` must convert `ParseError` to `writ_diagnostics::Diagnostic` manually (or the parser must expose a conversion method).
**Warning signs:** Syntax errors (misspelled keywords, unclosed braces) produce no squiggles even though semantic errors do.

### Pitfall 7: writ.toml Discovery Race
**What goes wrong:** When the user opens a file outside a writ.toml project, the server tries to load writ.toml and panics or emits a confusing error.
**Why it happens:** Not testing for `MissingToml` variant of `ConfigError`.
**How to avoid:** Check `ConfigError::MissingToml` — on this branch, treat the file as standalone (one-file project).
**Warning signs:** Server crashes or logs errors on opening a single .writ file with no project.

---

## Code Examples

Verified patterns from official sources:

### Client::publish_diagnostics Signature
```rust
// Source: docs.rs/tower-lsp/latest/tower_lsp/struct.Client.html
pub async fn publish_diagnostics(
    &self,
    uri: Url,
    diags: Vec<lsp_types::Diagnostic>,
    version: Option<i32>,
)
```

### lsp_types::Diagnostic Structure
```rust
// Source: docs.rs/lsp-types/latest/lsp_types/struct.Diagnostic.html
pub struct Diagnostic {
    pub range: Range,                                          // required
    pub severity: Option<DiagnosticSeverity>,                  // None = client decides
    pub code: Option<NumberOrString>,                          // e.g. NumberOrString::String("E0042".into())
    pub code_description: Option<CodeDescription>,             // LSP 3.16+
    pub source: Option<String>,                               // e.g. Some("writ".into())
    pub message: String,                                       // required
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
    pub tags: Option<Vec<DiagnosticTag>>,
    pub data: Option<serde_json::Value>,
}
```

### Severity Mapping
```rust
// Source: lsp_types::DiagnosticSeverity
use lsp_types::DiagnosticSeverity;
use writ_diagnostics::Severity;

fn severity_to_lsp(s: Severity) -> DiagnosticSeverity {
    match s {
        Severity::Error   => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note    => DiagnosticSeverity::INFORMATION,  // LSP has no "Note"; INFORMATION renders blue
    }
}
```

### DiagnosticRelatedInformation
```rust
// Source: lsp-types; writ-diagnostics SecondaryLabel → LSP RelatedInformation
use lsp_types::{DiagnosticRelatedInformation, Location};

fn secondary_to_related(label: &writ_diagnostics::SecondaryLabel, uri: Url, source: &str)
    -> DiagnosticRelatedInformation
{
    DiagnosticRelatedInformation {
        location: Location {
            uri,
            range: span_to_range(source, &label.span),
        },
        message: label.message.clone(),
    }
}
```

### Full Diagnostic Conversion
```rust
fn writ_diag_to_lsp(
    diag: &writ_diagnostics::Diagnostic,
    uri_for_file: impl Fn(writ_diagnostics::FileId) -> Url,
    source_for_file: impl Fn(writ_diagnostics::FileId) -> &str,
) -> lsp_types::Diagnostic {
    let source = source_for_file(diag.primary_file);
    let range = span_to_range(source, &diag.primary_span);
    let related = if diag.secondary_labels.is_empty() {
        None
    } else {
        Some(diag.secondary_labels.iter().map(|label| {
            let label_source = source_for_file(label.file_id);
            let label_uri = uri_for_file(label.file_id);
            secondary_to_related(label, label_uri, label_source)
        }).collect())
    };

    lsp_types::Diagnostic {
        range,
        severity: Some(severity_to_lsp(diag.severity)),
        code: if diag.code.is_empty() { None } else { Some(NumberOrString::String(diag.code.clone())) },
        source: Some("writ".into()),
        message: diag.message.clone(),
        related_information: related,
        ..Default::default()
    }
}
```

### package.json Contributes (VS Code Extension)
```json
{
  "name": "writ",
  "displayName": "Writ Language Support",
  "version": "0.1.0",
  "engines": { "vscode": "^1.74.0" },
  "activationEvents": [],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "writ",
      "aliases": ["Writ"],
      "extensions": [".writ"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "writ",
      "scopeName": "source.writ",
      "path": "./syntaxes/writ.tmLanguage.json"
    }]
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  },
  "devDependencies": {
    "@types/vscode": "^1.74.0",
    "typescript": "^5.0.0"
  }
}
```

**Note:** `activationEvents: []` is sufficient as of VS Code 1.74.0. Languages contributed by the extension activate it automatically when a `.writ` file is opened.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `lspower` (separate crate) | `tower-lsp` is the standard | 2021 | lspower is abandoned; use tower-lsp 0.20 |
| `gluon-lang/lsp-types` (unmaintained) | `lsp-types` 0.94+ (new maintainer) | 2023 | tower-lsp 0.20 uses 0.94.1; type definitions are stable |
| `activationEvents: ["onLanguage:X"]` required | `activationEvents: []` sufficient | VS Code 1.74.0 | Extensions contributing languages activate automatically; no manual event needed |
| `TransportKind.ipc` for native binaries | Plain `{ command: "...", args: [] }` ServerOptions | Always valid | `Executable` shape in ServerOptions speaks stdio by default for native binaries |
| Blocking reactor for compiler work | `tokio::task::spawn_blocking` | Established pattern | Required for any synchronous CPU-bound work in tower-lsp async handlers |

**Deprecated/outdated:**
- `lspower`: abandoned; do not use
- `languageserver-types` crate (old name for `lsp-types`): use `lsp-types` directly

---

## Open Questions

1. **ParseError → Diagnostic conversion**
   - What we know: `writ_parser::parse` returns `(Option<Cst>, Vec<ParseError>)`; `ParseError` is chumsky's `Rich<'static, Token, Span>`
   - What's unclear: Whether `ParseError` has a `.to_diagnostic(FileId)` method or requires manual conversion
   - Recommendation: Inspect `writ-parser/src/lib.rs` during planning; if no conversion exists, add one or do inline conversion in `AnalysisHost`

2. **Resolve/typecheck with partial AST after lower errors**
   - What we know: Lower errors produce an `Ast` with error nodes; Phase 52 added per-function skip at emit stage
   - What's unclear: Whether `resolve::resolve` and `check::typecheck` handle error nodes gracefully without panicking
   - Recommendation: Cascade strategy "continue through typecheck" may need a guard: if lower had errors, still call resolve/typecheck but wrap in `std::panic::catch_unwind`

3. **FileId→URI registry for project mode**
   - What we know: FileId(u32) is assigned sequentially during analysis; URIs come from LSP notifications
   - What's unclear: Where to store the FileId↔URI mapping during project-mode analysis (it's built anew each analysis run)
   - Recommendation: Build `HashMap<FileId, Url>` inside `AnalysisHost::analyze_project()` and return it alongside diagnostics

4. **Workspace root detection**
   - What we know: `InitializeParams` carries `workspace_folders` and `root_uri`; writ.toml discovery is workspace-root-only
   - What's unclear: When multiple workspace folders are open, which root to use for writ.toml search
   - Recommendation: Use `workspace_folders[0]` if present, fall back to `root_uri`; document limitation in code comment

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` |
| Config file | None required (Cargo workspace) |
| Quick run command | `cargo test -p writ-lsp` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-01 | `Client::publish_diagnostics` called with correct severity for error/warning/note | unit | `cargo test -p writ-lsp test_severity_mapping` | ❌ Wave 0 |
| LSP-01 | Byte offset → LSP Position correct for ASCII and multi-byte chars | unit | `cargo test -p writ-lsp test_offset_to_position` | ❌ Wave 0 |
| LSP-01 | `writ_diag_to_lsp()` maps `Diagnostic` fields correctly | unit | `cargo test -p writ-lsp test_diag_conversion` | ❌ Wave 0 |
| LSP-01 | ParseError is surfaced as Diagnostic (not silently dropped) | unit | `cargo test -p writ-lsp test_parse_error_surfaced` | ❌ Wave 0 |
| LSP-08 | Project mode collects diagnostics grouped by FileId | unit | `cargo test -p writ-lsp test_project_mode_diagnostics` | ❌ Wave 0 |
| EXT-01 | Grammar file is valid JSON and has required scopeName | smoke | manual in VS Code OR json schema validate | ❌ Wave 0 |
| EXT-02 | package.json contributes .writ extension and language id | unit | JSON parse test or manual | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-lsp/src/` — entire crate does not exist yet; create in Wave 0
- [ ] `writ-lsp/tests/convert_tests.rs` — covers LSP-01 span/severity/diagnostic conversion
- [ ] `writ-lsp/tests/analysis_host_tests.rs` — covers LSP-01 pipeline integration, LSP-08 multi-file grouping
- [ ] `writ-vscode/syntaxes/writ.tmLanguage.json` — covers EXT-01 (create grammar in Wave 0)
- [ ] `writ-vscode/package.json` — covers EXT-02 (create extension skeleton in Wave 0)

---

## Sources

### Primary (HIGH confidence)
- docs.rs/tower-lsp/latest/tower_lsp/ — LanguageServer trait, Client::publish_diagnostics signature, LspService/Server setup
- docs.rs/lsp-types/latest/lsp_types/struct.Diagnostic.html — Diagnostic struct fields, DiagnosticSeverity enum
- docs.rs/lsp-types/latest/lsp_types/struct.Position.html — Position { line, character } semantics
- code.visualstudio.com/api/language-extensions/syntax-highlight-guide — TextMate grammar structure, package.json grammars contribution
- code.visualstudio.com/api/language-extensions/language-server-extension-guide — extension.ts LanguageClient setup, ServerOptions
- docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html — spawn_blocking for blocking compiler work
- D:/dev/git/Writ/writ-cli/src/main.rs (run_pipeline) — authoritative pipeline stages
- D:/dev/git/Writ/writ-diagnostics/src/diagnostic.rs — Diagnostic/SecondaryLabel types
- D:/dev/git/Writ/language-spec/spec/05_4_lexical_structure.md — Writ keywords, sigils, string forms

### Secondary (MEDIUM confidence)
- github.com/ebkalderon/tower-lsp/blob/master/Cargo.toml — tower-lsp 0.20.0 dependencies (tokio 1.17, lsp-types 0.94.1, tower 0.4)
- thunderseethe.dev/posts/lsp-base/ — did_open/did_change/publish_diagnostics pattern verified against tower-lsp trait
- github.com/IWANABETHATGUY/tower-lsp-boilerplate — DashMap document store, on_change pattern
- github.com/microsoft/vscode-discussions/discussions/1099 — stdio Executable ServerOptions for native binary
- macromates.com/manual/en/language_grammars — TextMate scope naming conventions

### Tertiary (LOW confidence)
- WebSearch results re: tower-lsp-server community fork vs original tower-lsp — decision: use original 0.20 for Phase 53

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — tower-lsp 0.20.0 verified via Cargo.toml on GitHub; lsp-types version pinned; tokio pattern from official docs
- Architecture: HIGH — patterns verified from tower-lsp docs and existing `run_pipeline` in writ-cli
- Pitfalls: HIGH — Box::leak and UTF-16 encoding verified from compiler source and LSP spec; other pitfalls from established LSP development patterns
- TextMate grammar: MEDIUM — grammar structure verified from VS Code official docs; Writ-specific patterns from language-spec; scope names follow standard conventions

**Research date:** 2026-03-14
**Valid until:** 2026-06-14 (tower-lsp 0.20 is stable; VS Code extension API is stable)
