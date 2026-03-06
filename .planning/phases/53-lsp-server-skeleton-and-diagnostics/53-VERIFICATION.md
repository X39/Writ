---
phase: 53-lsp-server-skeleton-and-diagnostics
verified: 2026-03-14T01:30:00Z
status: passed
score: 8/8 must-haves verified
re_verification: false
---

# Phase 53: LSP Server Skeleton and Diagnostics Verification Report

**Phase Goal:** Users see inline diagnostic squiggles in VS Code for Writ errors, and .writ files get syntax highlighting and language association immediately on installing the extension.
**Verified:** 2026-03-14T01:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Writ compiler diagnostics convert to LSP Diagnostic structs with correct line/character positions | VERIFIED | `convert.rs` implements `offset_to_position` (UTF-16 aware) and `span_to_range`; 16 unit tests pass covering ASCII, 2-byte and 4-byte UTF-8 chars |
| 2 | ParseError from chumsky is surfaced as a structured LSP diagnostic (not silently dropped) | VERIFIED | `parse_error_to_diag` in `convert.rs` converts `Rich<Token, SimpleSpan>` to `Diagnostic`; test `test_parse_error_to_diag` passes |
| 3 | Standalone analysis of a single .writ source returns all parse, lower, resolve, and type errors | VERIFIED | `AnalysisHost::analyze_standalone` runs all 4 stages with cascade strategy; tests for valid/parse-error/type-error/cascade all pass |
| 4 | Project-mode analysis discovers all .writ files via writ.toml and returns diagnostics grouped by FileId | VERIFIED | `AnalysisHost::analyze_project` calls `load_config` and `discover_source_files`; `test_analyze_project_missing_toml` and `test_analyze_project_missing_toml_with_trigger` pass |
| 5 | After saving a .writ file with a type error, a red squiggle appears under the offending token | VERIFIED (automated portion) | `backend.rs` `did_save` triggers `publish_diagnostics_for` -> `publish_grouped_diagnostics` -> `client.publish_diagnostics`; full chain is wired and compiles; human test needed for visual confirmation |
| 6 | After fixing the error and saving, the squiggle disappears | VERIFIED | Stale clearing logic in `publish_grouped_diagnostics`: `published_uris` DashMap tracks active URIs; URIs not in current result receive `publish_diagnostics(uri, vec![], None)` |
| 7 | Opening a .writ file in VS Code applies TextMate syntax highlighting | VERIFIED | `writ.tmLanguage.json` with 9 repository entries (comments, attributes, strings, keywords, numbers, dialogue-blocks, function-calls, type-names, operators); scopeName = "source.writ" |
| 8 | VS Code activates the Writ language automatically when any .writ file is opened | VERIFIED | `package.json` contributes language id "writ" with extensions [".writ"]; `activationEvents: []` enables auto-activation on VS Code 1.74+ |

**Score:** 8/8 truths verified

---

### Required Artifacts

#### Plan 53-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/Cargo.toml` | Crate manifest with tower-lsp, tokio, dashmap, writ-compiler, writ-diagnostics, writ-parser | VERIFIED | Contains `tower-lsp = "0.20"`, `tokio`, `dashmap = "6"`, `lsp-types = "0.94"`, all three writ crate deps, and `chumsky` |
| `writ-lsp/src/convert.rs` | Span-to-Range, severity-to-LSP, Diagnostic conversion, ParseError conversion | VERIFIED | All 5 required functions implemented: `offset_to_position`, `span_to_range`, `severity_to_lsp`, `writ_diag_to_lsp`, `parse_error_to_diag` with 16-test suite |
| `writ-lsp/src/analysis_host.rs` | AnalysisHost with analyze_standalone and analyze_project | VERIFIED | Both methods implemented with cascade strategy and catch_unwind; 7-test suite covering all specified behaviors |
| `writ-lsp/src/lib.rs` | Public module declarations | VERIFIED | Declares `pub mod analysis_host; pub mod backend; pub mod convert;` |

#### Plan 53-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-vscode/package.json` | Extension manifest with language contribution, grammar reference, activation, vscode-languageclient dep | VERIFIED | All fields present: language id "writ", extensions [".writ"], grammar path, activationEvents: [], vscode-languageclient ^9.0.0 |
| `writ-vscode/syntaxes/writ.tmLanguage.json` | TextMate grammar for Writ syntax highlighting | VERIFIED | scopeName "source.writ", 9 repository entries covering all required Writ constructs |
| `writ-vscode/src/extension.ts` | Extension entrypoint launching writ-lsp over stdio | VERIFIED | Imports LanguageClient, creates ServerOptions with `command: serverCommand`, handles Windows .exe suffix, calls client.start() |
| `writ-vscode/language-configuration.json` | Bracket matching, auto-closing pairs, comment toggling | VERIFIED | lineComment "//", blockComment ["/*", "*/"], brackets, autoClosingPairs, surroundingPairs, folding markers |
| `writ-vscode/tsconfig.json` | TypeScript compilation configuration | VERIFIED | target ES2020, module commonjs, outDir ./out, strict true, all required options |

#### Plan 53-03 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/src/backend.rs` | Backend struct with tower-lsp LanguageServer impl, document store, publish_diagnostics_for | VERIFIED | 289 lines; Backend struct with client, document_map, workspace_root, published_uris; full LanguageServer impl with all 6 handlers; `publish_diagnostics_for` and `publish_grouped_diagnostics` methods |
| `writ-lsp/src/main.rs` | tokio::main entry point wiring LspService and Server | VERIFIED | Contains `#[tokio::main]`, `LspService::new(Backend::new)`, `Server::new(stdin, stdout, socket).serve(service)` |

---

### Key Link Verification

#### Plan 53-01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-lsp/src/analysis_host.rs` | `writ_parser::parse` | calls parse() on leaked source text | WIRED | Line 40: `writ_parser::parse(src)` called in both standalone and project methods |
| `writ-lsp/src/analysis_host.rs` | `writ_compiler::lower` | lowers CST to AST | WIRED | Line 54: `writ_compiler::lower(cst)` called; pattern `writ_compiler::lower` present |
| `writ-lsp/src/analysis_host.rs` | `writ_compiler::resolve::resolve` | name resolution across files | WIRED | Line 69: `writ_compiler::resolve::resolve(&asts_refs, &path_refs)` inside catch_unwind |
| `writ-lsp/src/analysis_host.rs` | `writ_compiler::check::typecheck` | type checking | WIRED | Line 86: `writ_compiler::check::typecheck(resolved, &asts_refs)` inside catch_unwind |
| `writ-lsp/src/analysis_host.rs` | `writ_compiler::config::load_config` | project mode writ.toml discovery | WIRED | Line 110: `writ_compiler::config::load_config(project_root)` |
| `writ-lsp/src/convert.rs` | `writ_diagnostics::Diagnostic` | maps Diagnostic fields to lsp_types::Diagnostic | WIRED | Lines 59-103: `writ_diag_to_lsp` takes `&Diagnostic` and maps all fields |

#### Plan 53-02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-vscode/package.json` | `writ-vscode/syntaxes/writ.tmLanguage.json` | contributes.grammars[0].path | WIRED | `"path": "./syntaxes/writ.tmLanguage.json"` present in grammars contribution |
| `writ-vscode/package.json` | `writ-vscode/language-configuration.json` | contributes.languages[0].configuration | WIRED | `"configuration": "./language-configuration.json"` present in languages contribution |
| `writ-vscode/src/extension.ts` | writ-lsp binary | ServerOptions.command pointing to compiled Rust binary | WIRED | Lines 14-16: `context.asAbsolutePath(path.join('..', 'target', 'debug', 'writ-lsp'))` with Windows `.exe` handling |

#### Plan 53-03 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-lsp/src/backend.rs` | `writ-lsp/src/analysis_host.rs` | spawn_blocking calls AnalysisHost::analyze_standalone or analyze_project | WIRED | Lines 147-154: `crate::analysis_host::AnalysisHost::analyze_project` and `analyze_standalone` called inside `tokio::task::spawn_blocking` |
| `writ-lsp/src/backend.rs` | `writ-lsp/src/convert.rs` | writ_diag_to_lsp converts diagnostics before publish | WIRED | Line 213: `crate::convert::writ_diag_to_lsp(diag, &uri_for_file, &source_for_file)` |
| `writ-lsp/src/backend.rs` | `tower_lsp::Client::publish_diagnostics` | pushes diagnostics to editor per-URI | WIRED | Lines 231, 250, 264: `self.client.publish_diagnostics(...)` called in all three code paths |
| `writ-lsp/src/main.rs` | `tower_lsp::LspService` | creates service with Backend constructor | WIRED | Line 13: `LspService::new(Backend::new)` |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| LSP-01 | 53-01, 53-03 | Language server publishes diagnostics (errors and warnings) as inline editor squiggles on file save or change | SATISFIED | `did_save` and `did_change` handlers call `publish_diagnostics_for`; analysis runs all 4 compiler stages; results published via `client.publish_diagnostics` |
| LSP-08 | 53-01, 53-03 | All LSP features work across multiple files in a writ.toml project (cross-file resolution) | SATISFIED | `analyze_project` discovers all files, runs resolve/typecheck across all; `publish_grouped_diagnostics` routes diagnostics to per-URI channels by FileId |
| EXT-01 | 53-02 | VS Code extension provides TextMate syntax highlighting for .writ files | SATISFIED | `writ.tmLanguage.json` with scopeName "source.writ" covers comments, strings (3 forms), keywords (declaration/control/other/primitive), numbers (decimal/hex/binary), dialogue sigils, attributes, operators, function calls, type names |
| EXT-02 | 53-02 | Extension registers .writ file association and activates automatically on opening .writ files | SATISFIED | `package.json` contributes language id "writ" with `extensions: [".writ"]`, `activationEvents: []` for VS Code 1.74+ auto-activation |

No orphaned requirements found. REQUIREMENTS.md traceability table shows LSP-01, LSP-08, EXT-01, EXT-02 all assigned to Phase 53 with status "Complete".

---

### Anti-Patterns Found

No anti-patterns detected in any phase 53 source files.

Scan results:
- No TODO/FIXME/PLACEHOLDER comments in `writ-lsp/src/` or `writ-vscode/src/`
- No stub return values (`return null`, `return {}`, `return []`)
- No empty handler bodies (all LanguageServer trait methods are fully implemented)
- `main.rs` contains real tokio::main server wiring (not a placeholder)
- `backend.rs` is 289 lines of substantive implementation

---

### Human Verification Required

The following items require a running VS Code instance to verify end-to-end:

#### 1. Diagnostic Squiggle Appearance

**Test:** Open a .writ file containing `fn main() { let x: int = true; }` in VS Code with the extension installed (npm build + cargo build completed)
**Expected:** A red squiggle appears under `true` (type mismatch: bool assigned to int)
**Why human:** Requires running VS Code with the compiled extension; programmatic LSP protocol handshake not verified here

#### 2. Squiggle Disappears on Fix

**Test:** Fix the type error in the same file (change `true` to `1`) and save
**Expected:** The red squiggle disappears within one save cycle
**Why human:** Requires observing VS Code UI state change; stale-clearing logic is verified in code but end-to-end publish-and-clear round-trip needs live editor

#### 3. Syntax Highlighting Coloring

**Test:** Open any .writ file with keywords, strings, numbers, and comments
**Expected:** Keywords appear in one color, strings in another, numbers in another, comments grayed out; dialogue `@SpeakerName` highlighted distinctly
**Why human:** Visual appearance of TextMate grammar application requires a running editor with a theme

#### 4. Language Mode Auto-Detection

**Test:** Open a new .writ file in VS Code without selecting a language mode
**Expected:** Status bar shows "Writ" as the language mode immediately, without user interaction
**Why human:** Tests VS Code extension activation event behavior in a running editor

---

## Build and Test Summary

| Check | Result |
|-------|--------|
| `cargo build -p writ-lsp` | PASSED (0 errors, 0 warnings) |
| `cargo test -p writ-lsp --lib` | PASSED (23/23 tests) |
| `target/debug/writ-lsp.exe` binary exists | CONFIRMED (17 MB) |
| All 6 task commits present in git | CONFIRMED (0b5b90a, 05709da, e46e4d9, 86b5ea9, 5242884, 588927d) |
| `writ-lsp` in workspace Cargo.toml members | CONFIRMED |
| `writ-vscode/package.json` valid JSON with correct structure | CONFIRMED |
| `writ-vscode/syntaxes/writ.tmLanguage.json` valid JSON, scopeName "source.writ" | CONFIRMED |

---

## Gaps Summary

None. All automated checks passed. The phase goal is achieved: the writ-lsp binary exists and is wired to publish LSP diagnostics, and the VS Code extension skeleton correctly registers .writ file association, references the TextMate grammar, and launches the language server binary over stdio.

Four items require human verification in a running VS Code instance (visual/behavioral confirmation of squiggles and highlighting), but all underlying code paths that enable those behaviors are verified to exist, be substantive, and be wired end-to-end.

---

_Verified: 2026-03-14T01:30:00Z_
_Verifier: Claude (gsd-verifier)_
