---
phase: quick
plan: 260319-hyo
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-lsp/tests/test_protocol.rs
  - writ-dap/tests/test_protocol.rs
autonomous: true
requirements: []

must_haves:
  truths:
    - "LSP protocol tests exercise initialize, diagnostics, hover, goto-definition, completion, and shutdown through wire-protocol messages"
    - "DAP protocol tests exercise initialize, launch, setBreakpoints, configurationDone, threads, stackTrace, continue, and disconnect through wire-protocol messages"
    - "All tests communicate via in-memory I/O with Content-Length framing, never calling internal methods directly"
  artifacts:
    - path: "writ-lsp/tests/test_protocol.rs"
      provides: "LSP wire-protocol integration tests covering major LSP features"
      min_lines: 200
    - path: "writ-dap/tests/test_protocol.rs"
      provides: "DAP wire-protocol integration tests for simple fixture debug sessions"
      min_lines: 200
  key_links:
    - from: "writ-lsp/tests/test_protocol.rs"
      to: "writ_lsp::backend::Backend"
      via: "tower_lsp::LspService and in-memory duplex streams"
      pattern: "LspService::new\\(Backend::new\\)"
    - from: "writ-dap/tests/test_protocol.rs"
      to: "writ_dap::server::DapServer"
      via: "dap::Server with Cursor/SharedWriter I/O"
      pattern: "DapServer::new\\(server\\)"
---

<objective>
Add proper LSP and DAP integration tests that communicate through the wire-protocol interface (Content-Length framed JSON-RPC over STDIN/STDOUT-equivalent streams), rather than calling internal methods directly.

Purpose: The existing test suites mix protocol-level tests with internal API tests. This plan creates focused, comprehensive protocol-level test files that exercise the full request/response cycle through in-memory I/O, verifying that the servers correctly handle real wire-protocol messages for all major features.

Output: Two test files (`writ-lsp/tests/test_protocol.rs`, `writ-dap/tests/test_protocol.rs`) that serve as the canonical integration test suites for the LSP and DAP wire protocols.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@writ-lsp/tests/test_hover_protocol.rs (existing LSP protocol test pattern with LspClient helper)
@writ-dap/tests/test_quest_system_debug.rs (existing DAP protocol test pattern with SharedWriter and parse_dap_messages)
@writ-lsp/src/backend.rs (LSP server capabilities: hover, goto-def, references, completion, signature-help, semantic-tokens, text-doc-sync)
@writ-dap/src/server/mod.rs (DAP server commands: initialize, setBreakpoints, configurationDone, launch, threads, stackTrace, scopes, variables, evaluate, next, stepIn, stepOut, continue, disconnect)
@writ-dap/src/server/handlers.rs (DAP handler implementations)
@writ-golden/tests/golden/fn_typed_params.writ (simple fixture with 3 functions: add, is_positive, main)
@writ-golden/tests/golden/fn_multi_return.writ (fixture with if-branches and multiple returns)

<interfaces>
From writ-lsp/src/backend.rs:
```rust
// Backend::new takes a Client, created by LspService::new
pub fn new(client: Client) -> Self;

// Server capabilities registered during initialize:
// - text_document_sync: FULL (open_close + change)
// - hover_provider: true
// - definition_provider: true
// - references_provider: true
// - completion_provider: trigger_characters [".","::"]
// - signature_help_provider: trigger_characters ["(",","]
// - semantic_tokens_provider: full tokens
```

From writ-dap/src/server/mod.rs:
```rust
pub struct DapServer<I: Read, O: Write> { ... }
impl<I: Read, O: Write> DapServer<I, O> {
    pub fn new(server: Server<I, O>) -> Self;
    pub fn run(&mut self);
}
```

From writ-lsp/tests/test_hover_protocol.rs (reuse this pattern):
```rust
struct LspClient {
    writer: tokio::io::DuplexStream,
    reader: tokio::io::DuplexStream,
    seq: i64,
}
// encode_lsp, read_lsp, LspClient::start, initialize, open_document, hover
```

From writ-dap/tests/test_quest_system_debug.rs (reuse this pattern):
```rust
struct SharedWriter(Arc<Mutex<Vec<u8>>>);
// framed(), make_request(), make_request_no_args(), parse_dap_messages()
// find_message(), is_response_to(), is_event()
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create LSP wire-protocol integration tests</name>
  <files>writ-lsp/tests/test_protocol.rs</files>
  <action>
Create `writ-lsp/tests/test_protocol.rs` with comprehensive LSP wire-protocol integration tests. Reuse and extend the `LspClient` helper pattern from `test_hover_protocol.rs` (do NOT import from that file -- copy the helpers into this file so it is self-contained).

The `LspClient` struct should use in-memory `tokio::io::duplex` streams connected to a `tower_lsp::Server` (same pattern as `test_hover_protocol.rs`). All communication goes through `encode_lsp` / `read_lsp` with Content-Length framing.

Add a `diagnostics` method to `LspClient` that drains notifications and collects `textDocument/publishDiagnostics` notifications. Pattern: after `open_document`, sleep briefly then drain -- the existing `open_document` helper already does this but discards notifications. Modify the helper to optionally collect diagnostics.

Fixture: Use `writ-golden/tests/golden/fn_typed_params.writ` for most tests (simple, compiles cleanly, has `add`, `is_positive`, `main`).

Tests to create:

1. `test_initialize_returns_capabilities` -- Send initialize request, verify response contains expected capabilities (hover_provider, definition_provider, references_provider, completion_provider). Send shutdown request after.

2. `test_diagnostics_clean_file` -- Open `fn_typed_params.writ` (valid program), verify `publishDiagnostics` notification has empty diagnostics array.

3. `test_diagnostics_invalid_source` -- Open a document with invalid Writ source (`fn main() { let x: int = "hello"; }`), verify `publishDiagnostics` produces at least one diagnostic with severity=Error.

4. `test_goto_definition` -- Open `fn_typed_params.writ`, send `textDocument/definition` request with position on the `add` call at line 10 col 19 (0-indexed). Verify the response location points to `add` function definition at line 0.

5. `test_completion_identifiers` -- Open `fn_typed_params.writ`, send `textDocument/completion` at a position inside `main` body. Verify at least some completions are returned (keywords like `let`, `if`, or function names like `add`).

6. `test_shutdown_graceful` -- Send initialize, then shutdown, verify shutdown response is success. Then send exit notification.

All tests use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` (required by tower-lsp).
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test --package writ-lsp --test test_protocol -- --nocapture 2>&1 | tail -30</automated>
  </verify>
  <done>All 6 LSP protocol tests pass. Each test communicates exclusively through Content-Length framed JSON-RPC messages over in-memory duplex streams, never calling internal Backend methods directly.</done>
</task>

<task type="auto">
  <name>Task 2: Create DAP wire-protocol integration tests</name>
  <files>writ-dap/tests/test_protocol.rs</files>
  <action>
Create `writ-dap/tests/test_protocol.rs` with focused DAP wire-protocol integration tests using simple fixtures. Reuse the `SharedWriter`, `framed`, `make_request`, `make_request_no_args`, `parse_dap_messages`, `find_message`, `is_response_to`, `is_event` helper pattern from `test_quest_system_debug.rs` (copy into this file so it is self-contained).

Pattern: Each test builds an input buffer of framed DAP JSON messages, creates a `DapServer` with `BufReader<Cursor<Vec<u8>>>` input and `BufWriter<SharedWriter>` output, calls `run()`, then parses the output buffer for assertions.

Fixture: Use `writ-golden/tests/golden/fn_typed_params.writ` (simple: 3 functions, `add`, `is_positive`, `main`).

Tests to create:

1. `test_initialize_capabilities` -- Send initialize + disconnect. Verify initialize response has `success: true` and body contains `supportsConfigurationDoneRequest: true`. Verify `initialized` event is sent.

2. `test_launch_and_run_to_completion` -- Send initialize, configurationDone, launch (with fn_typed_params.writ, stopOnEntry=false), then disconnect. Verify launch response succeeds. Verify a `terminated` event is emitted (program has no breakpoints, runs to completion).

3. `test_breakpoint_hit_and_inspect` -- Send initialize, setBreakpoints (line 11: `let x: int = add(3, 4);`), configurationDone, launch (stopOnEntry=false). The program should hit the breakpoint. Then send threads, stackTrace (threadId=0), scopes (frameId=0), variables, continue, disconnect. Verify: stopped event with reason "breakpoint", threads response has at least 1 thread, stackTrace response has at least 1 frame with a line number > 0, scopes response has at least 1 scope, variables response succeeds. Note: the continue/disconnect after breakpoint ensures the session terminates cleanly.

4. `test_stop_on_entry` -- Send initialize, configurationDone, launch (with stopOnEntry=true), then disconnect. Verify a `stopped` event with reason "entry" is emitted.

5. `test_launch_error_missing_program` -- Send initialize, configurationDone, launch with no "program" argument, then disconnect. Verify launch response has `success: false` and `message` contains "program".

6. `test_unknown_command_returns_error` -- Send initialize, then a request with an unsupported command (e.g., "restart"), then disconnect. Verify the response to the unsupported command has `success: false`.

All tests are synchronous `#[test]` (DAP server is synchronous).
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test --package writ-dap --test test_protocol -- --nocapture 2>&1 | tail -30</automated>
  </verify>
  <done>All 6 DAP protocol tests pass. Each test communicates exclusively through Content-Length framed DAP messages over in-memory I/O, never calling DapServer internal methods directly.</done>
</task>

</tasks>

<verification>
Run both test suites:
```
cargo test --package writ-lsp --test test_protocol
cargo test --package writ-dap --test test_protocol
```

Verify no existing tests regress:
```
cargo test --package writ-lsp
cargo test --package writ-dap
```
</verification>

<success_criteria>
- `writ-lsp/tests/test_protocol.rs` exists with 6+ protocol-level tests covering initialize, diagnostics, hover (via existing pattern), goto-definition, completion, and shutdown
- `writ-dap/tests/test_protocol.rs` exists with 6+ protocol-level tests covering initialize, launch, breakpoint hit, variable inspection, stop-on-entry, and error cases
- All tests pass on `cargo test`
- All tests communicate through wire-protocol messages (Content-Length framing), not internal method calls
- No regressions in existing test suites
</success_criteria>

<output>
After completion, create `.planning/quick/260319-hyo-add-proper-lsp-and-dap-integration-tests/260319-hyo-SUMMARY.md`
</output>
