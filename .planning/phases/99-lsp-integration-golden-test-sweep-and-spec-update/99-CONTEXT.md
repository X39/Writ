# Phase 99: LSP Integration, Golden Test Sweep, and Spec Update - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure/testing phase)

<domain>
## Phase Boundary

Regression-safe E2E LSP tests for the attribute system, golden snapshot review/bless pass, and language spec documentation for attribute argument encoding, user-defined attribute declarations, and the runtime query API.

</domain>

<decisions>
## Implementation Decisions

### LSP E2E Tests
- Use the existing `test_protocol.rs` LspClient infrastructure (in-memory duplex streams, JSON-RPC framing)
- Deprecated test: open a `.writ` source with `[Deprecated("msg")]` usage, assert publishDiagnostics contains Warning severity with the message string
- Speaker validation test: open a `.writ` source with `@speaker` targeting a non-Singleton entity, assert publishDiagnostics contains E0007
- Test fixtures can be inline strings (no separate fixture files needed)

### Golden Test Sweep
- Run `cargo insta test --review` to identify any pending snapshot changes from phases 93-98
- Bless all correct snapshots and commit
- Verify `cargo test` passes clean with no pending review items

### Language Spec
- Add a new spec section covering: attribute argument blob encoding format, `attribute Name(params);` declaration syntax, builtin attribute semantic effects, and the three runtime query method signatures
- Place in the existing `language-spec/spec/` splatted files, numbered appropriately after existing sections

### Claude's Discretion
All implementation details at Claude's discretion — infrastructure/testing phase with clear success criteria from ROADMAP.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-lsp/tests/test_protocol.rs` — full LSP E2E test client with `open_document_and_collect_diagnostics()`, `drain_notifications()`, `recv_response()`
- `writ-golden/tests/golden/` — 44 `.writ` files with `.writil` golden outputs
- `writ-golden/tests/golden_tests.rs` — test registration for golden file compilation
- `language-spec/spec/` — 28+ splatted spec files, numbered `00_` through `28_`

### Established Patterns
- LSP tests use `fixture_source()` for file fixtures or inline `&str` for short sources
- Golden tests: `.writ` → compile → compare `.writil` via insta snapshots
- Spec files: markdown with `writ` code fences, numbered for ordering

### Integration Points
- LSP backend reads from `writ-compiler` pipeline — diagnostics are emitted as part of `textDocument/didOpen` → `publishDiagnostics`
- Golden tests exercise the full compile pipeline (`writ-compiler::compile()`)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure/testing phase. Refer to ROADMAP success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
