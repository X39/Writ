//! Comprehensive LSP wire-protocol integration tests.
//!
//! These tests start a real tower-lsp server using in-memory duplex streams,
//! communicate via the LSP JSON-RPC protocol (Content-Length framing), and
//! verify the full request/response cycle for all major LSP features.
//!
//! All tests are self-contained and never call internal Backend methods directly.

use std::time::Duration;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp::{LspService, Server};
use url::Url;
use writ_lsp::backend::Backend;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Read a fixture source file relative to the workspace root.
fn fixture_source(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .expect("workspace root should exist");
    std::fs::read_to_string(workspace_root.join(relative))
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", relative, e))
}

/// Encode a JSON value as an LSP wire-protocol message (Content-Length header).
fn encode_lsp(msg: &Value) -> Vec<u8> {
    let body = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
}

/// Read a single LSP message from a stream.
/// Returns None if the stream is closed or times out.
async fn read_lsp(reader: &mut (impl AsyncReadExt + Unpin)) -> Option<Value> {
    let mut header = Vec::new();
    loop {
        let mut b = [0u8; 1];
        match reader.read(&mut b).await {
            Ok(0) => return None,
            Ok(_) => {
                header.push(b[0]);
                if header.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    let s = String::from_utf8(header).ok()?;
    let len: usize = s
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .and_then(|s| s.trim().parse().ok())?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await.ok()?;
    serde_json::from_slice(&body).ok()
}

// ─── LSP Test Client ──────────────────────────────────────────────────────────

struct LspClient {
    writer: tokio::io::DuplexStream,
    reader: tokio::io::DuplexStream,
    seq: i64,
}

impl LspClient {
    /// Start an LSP server with in-memory duplex streams.
    /// Does NOT send initialize — call `initialize()` manually or use `start_initialized()`.
    async fn start_raw() -> Self {
        let (client_to_server, server_input) = tokio::io::duplex(65536);
        let (server_output, server_to_client) = tokio::io::duplex(65536);

        let (service, socket) = LspService::new(Backend::new);
        let server = Server::new(server_input, server_output, socket);
        tokio::spawn(async move { server.serve(service).await });

        LspClient {
            writer: client_to_server,
            reader: server_to_client,
            seq: 0,
        }
    }

    /// Start and initialize the LSP server. Ready for document operations.
    async fn start() -> Self {
        let mut client = Self::start_raw().await;
        client.initialize().await;
        client
    }

    fn next_id(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    async fn send(&mut self, msg: Value) {
        let bytes = encode_lsp(&msg);
        self.writer.write_all(&bytes).await.unwrap();
    }

    /// Read messages until we find a response with the given id.
    /// Notifications (no id) are skipped.
    async fn recv_response(&mut self, id: i64) -> Value {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let msg = tokio::time::timeout_at(deadline, read_lsp(&mut self.reader))
                .await
                .expect("timeout waiting for LSP response")
                .expect("LSP stream closed");
            if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                return msg;
            }
            // Skip notifications (publishDiagnostics, etc.)
        }
    }

    /// Drain all pending notifications within a short timeout window.
    /// Returns all messages collected (including publishDiagnostics notifications).
    async fn drain_notifications(&mut self, timeout_ms: u64) -> Vec<Value> {
        let mut collected = Vec::new();
        loop {
            match tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                read_lsp(&mut self.reader),
            )
            .await
            {
                Ok(Some(msg)) => collected.push(msg),
                _ => break,
            }
        }
        collected
    }

    /// Send `initialize` + `initialized` and wait for the initialize response.
    /// Returns the raw initialize response.
    async fn initialize(&mut self) -> Value {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "capabilities": {},
                "processId": null
            }
        }))
        .await;

        let resp = self.recv_response(id).await;

        // Send initialized notification (no response expected)
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }))
        .await;

        resp
    }

    /// Send `initialize` with a custom rootUri + `initialized`.
    /// Used for project-mode tests that need a workspace root (e.g. writ.toml discovery).
    async fn initialize_with_root(&mut self, root_uri: &str) -> Value {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "capabilities": {},
                "processId": null,
                "rootUri": root_uri
            }
        }))
        .await;

        let resp = self.recv_response(id).await;

        self.send(json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }))
        .await;

        resp
    }

    /// Open a document, wait for analysis, and collect any publishDiagnostics notifications.
    async fn open_document_and_collect_diagnostics(
        &mut self,
        uri: &str,
        text: &str,
    ) -> Vec<Value> {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "writ",
                    "version": 1,
                    "text": text
                }
            }
        }))
        .await;

        // Give the server time to analyze the document (analysis is async)
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Drain pending notifications and collect publishDiagnostics
        let all = self.drain_notifications(200).await;
        all.into_iter()
            .filter(|msg| {
                msg.get("method")
                    .and_then(|v| v.as_str())
                    == Some("textDocument/publishDiagnostics")
            })
            .collect()
    }

    /// Open a document and wait for analysis to complete (discard notifications).
    async fn open_document(&mut self, uri: &str, text: &str) {
        self.open_document_and_collect_diagnostics(uri, text).await;
    }

    /// Send a hover request and return the result value (or null).
    async fn hover(&mut self, uri: &str, line: u32, character: u32) -> Option<Value> {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;

        let resp = self.recv_response(id).await;
        resp.get("result").cloned().filter(|v| !v.is_null())
    }

    /// Send a goto definition request and return the result.
    async fn goto_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<Value> {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;

        let resp = self.recv_response(id).await;
        resp.get("result").cloned().filter(|v| !v.is_null())
    }

    /// Send a completion request and return the result.
    async fn completion(&mut self, uri: &str, line: u32, character: u32) -> Option<Value> {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        }))
        .await;

        let resp = self.recv_response(id).await;
        resp.get("result").cloned().filter(|v| !v.is_null())
    }

    /// Send a dot-triggered completion request (includes triggerCharacter: ".").
    async fn dot_completion(&mut self, uri: &str, line: u32, character: u32) -> Option<Value> {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": {
                    "triggerKind": 2,
                    "triggerCharacter": "."
                }
            }
        }))
        .await;

        let resp = self.recv_response(id).await;
        resp.get("result").cloned().filter(|v| !v.is_null())
    }

    /// Send a shutdown request and return the response.
    async fn shutdown(&mut self) -> Value {
        let id = self.next_id();
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "shutdown"
        }))
        .await;
        self.recv_response(id).await
    }

    /// Send exit notification (no response expected).
    async fn exit(&mut self) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }))
        .await;
    }
}

// ─── Constants ────────────────────────────────────────────────────────────────

const FIXTURE_URI: &str = "file:///test/fn_typed_params.writ";
const FIXTURE_PATH: &str = "writ-golden/tests/golden/fn_typed_params.writ";

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Test 1: Initialize returns expected server capabilities.
///
/// Sends initialize request and verifies the response includes capability
/// declarations for hover, goto-definition, references, and completion.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_initialize_returns_capabilities() {
    let mut client = LspClient::start_raw().await;
    let resp = client.initialize().await;

    let result = resp
        .get("result")
        .expect("initialize should return a result");

    assert!(
        resp.get("error").is_none(),
        "initialize should not return an error"
    );

    let capabilities = result
        .get("capabilities")
        .expect("result should have capabilities");

    // Verify hover provider is declared
    assert!(
        capabilities.get("hoverProvider").is_some(),
        "server should declare hoverProvider capability, got: {}",
        capabilities
    );

    // Verify goto-definition provider
    assert!(
        capabilities.get("definitionProvider").is_some(),
        "server should declare definitionProvider capability, got: {}",
        capabilities
    );

    // Verify references provider
    assert!(
        capabilities.get("referencesProvider").is_some(),
        "server should declare referencesProvider capability, got: {}",
        capabilities
    );

    // Verify completion provider with trigger characters
    let completion_provider = capabilities
        .get("completionProvider")
        .expect("server should declare completionProvider capability");
    assert!(
        completion_provider.get("triggerCharacters").is_some(),
        "completionProvider should declare triggerCharacters, got: {}",
        completion_provider
    );

    // Send shutdown to clean up
    let shutdown_resp = client.shutdown().await;
    assert!(
        shutdown_resp.get("error").is_none(),
        "shutdown after initialize should succeed"
    );
}

/// Test 2: Opening a valid Writ file produces zero diagnostics.
///
/// Opens fn_typed_params.writ (syntactically and semantically correct), then
/// verifies the publishDiagnostics notification carries an empty diagnostics array.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_clean_file() {
    let mut client = LspClient::start().await;
    let source = fixture_source(FIXTURE_PATH);

    let diag_notifications = client
        .open_document_and_collect_diagnostics(FIXTURE_URI, &source)
        .await;

    // The server may or may not send publishDiagnostics for a clean file.
    // If it does, the diagnostics array must be empty (or the notification absent).
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        // Only check notifications that target our document
        let notif_uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if notif_uri == FIXTURE_URI {
            let diagnostics = params
                .get("diagnostics")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            assert!(
                diagnostics.is_empty(),
                "clean file should produce zero diagnostics, got: {:?}",
                diagnostics
            );
        }
    }
}

/// Test 3: Opening invalid Writ source produces at least one Error-severity diagnostic.
///
/// Opens a document with a type mismatch (assigning string literal to int), and
/// verifies the publishDiagnostics notification contains an error-severity diagnostic.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_invalid_source() {
    let mut client = LspClient::start().await;

    // Type mismatch: assigning a string to an int variable
    let invalid_source = r#"pub fn main() {
    let x: int = "hello";
}
"#;
    let error_uri = "file:///test/invalid.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(error_uri, invalid_source)
        .await;

    // Collect all diagnostics from notifications targeting our document
    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if notif_uri == error_uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                all_diagnostics.extend(diags.iter().cloned());
            }
        }
    }

    assert!(
        !all_diagnostics.is_empty(),
        "invalid source should produce at least one diagnostic, \
         got {} publishDiagnostics notifications",
        diag_notifications.len()
    );

    // Verify at least one diagnostic has severity Error (1 in LSP)
    let has_error = all_diagnostics.iter().any(|d| {
        d.get("severity")
            .and_then(|v| v.as_i64())
            .map(|s| s == 1)
            .unwrap_or(false)
    });
    assert!(
        has_error,
        "at least one diagnostic should have severity=Error (1), got: {:?}",
        all_diagnostics
    );
}

/// Test 4: Goto-definition navigates to the function definition.
///
/// Opens fn_typed_params.writ and sends textDocument/definition at the position
/// of the `add` call site in `main` (line 10, 0-indexed). Verifies the response
/// location points to line 0 (the `add` function definition).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_goto_definition() {
    let mut client = LspClient::start().await;
    let source = fixture_source(FIXTURE_PATH);
    client.open_document(FIXTURE_URI, &source).await;

    // fn_typed_params.writ line 10 (0-indexed), col 18:
    //   `    let x: int = add(3, 4);`
    //                     ^~~ col 18 (start of `add`)
    let result = client.goto_definition(FIXTURE_URI, 10, 18).await;

    assert!(
        result.is_some(),
        "goto definition on `add` call should return a location"
    );
    let result = result.unwrap();

    // The result can be a Location object or an array of locations.
    // Extract the range to verify it points to line 0 (where `add` is defined).
    let location = if result.is_array() {
        result
            .as_array()
            .and_then(|a| a.first())
            .cloned()
            .expect("should have at least one location")
    } else {
        result
    };

    let range = location
        .get("range")
        .expect("location should have a range");
    let start_line = range
        .get("start")
        .and_then(|s| s.get("line"))
        .and_then(|l| l.as_i64())
        .expect("range.start.line should be present");

    assert_eq!(
        start_line, 0,
        "goto definition of `add` should point to line 0 (pub fn add), got line {}",
        start_line
    );
}

/// Test 5: Completion returns identifiers visible in the document.
///
/// Opens fn_typed_params.writ and requests completions at a position inside
/// `main`. Verifies that at least some completions are returned (keywords or
/// function names from the document).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_identifiers() {
    let mut client = LspClient::start().await;
    let source = fixture_source(FIXTURE_PATH);
    client.open_document(FIXTURE_URI, &source).await;

    // Request completions at line 11 (inside main body):
    //   `    let flag: bool = is_positive(x);`
    //                ^~~ col 8 (after `let `)
    // This triggers identifier completion (no dot or :: trigger).
    let result = client.completion(FIXTURE_URI, 11, 8).await;

    assert!(
        result.is_some(),
        "completion inside main body should return results"
    );
    let result = result.unwrap();

    // Completions can be an array or a CompletionList with `items`
    let items: Vec<Value> = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    assert!(
        !items.is_empty(),
        "completion should return at least some items (keywords / identifiers)"
    );

    // Verify items have a `label` field (required by LSP spec)
    for item in &items {
        assert!(
            item.get("label").is_some(),
            "each completion item should have a label, got: {}",
            item
        );
    }

    // Check that expected Writ keywords or function names appear
    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    let has_expected = labels.iter().any(|&l| {
        l == "let"
            || l == "if"
            || l == "fn"
            || l == "add"
            || l == "is_positive"
            || l == "main"
            || l == "return"
    });

    assert!(
        has_expected,
        "completions should include known keywords or function names, got: {:?}",
        labels
    );

    // Verify sort_text is present on all items
    for item in &items {
        assert!(
            item.get("sortText").is_some(),
            "each completion item should have sortText, got: {}",
            item
        );
    }

    // Verify keyword sort_text starts with "6_"
    let kw_item = items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()) == Some("let")
    }).expect("should have 'let' keyword");
    let kw_sort = kw_item.get("sortText").and_then(|v| v.as_str()).unwrap();
    assert!(
        kw_sort.starts_with("6_"),
        "keyword sort_text should start with '6_', got: {}",
        kw_sort
    );

    // Verify user-defined function sort_text starts with "1_"
    let fn_item = items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()) == Some("add")
    }).expect("should have 'add' function");
    let fn_sort = fn_item.get("sortText").and_then(|v| v.as_str()).unwrap();
    assert!(
        fn_sort.starts_with("1_"),
        "user fn sort_text should start with '1_', got: {}",
        fn_sort
    );
    // Verify function kind is FUNCTION (3)
    let fn_kind = fn_item.get("kind").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(fn_kind, 3, "function kind should be FUNCTION (3), got: {}", fn_kind);
}

/// Test 6: Shutdown and exit lifecycle completes gracefully.
///
/// Sends initialize, then shutdown, verifies shutdown response is success,
/// then sends exit notification.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_shutdown_graceful() {
    let mut client = LspClient::start().await;

    let shutdown_resp = client.shutdown().await;

    // Shutdown should succeed (no error field, or null result)
    assert!(
        shutdown_resp.get("error").is_none(),
        "shutdown should not return an error, got: {}",
        shutdown_resp
    );

    // Result should be null for shutdown
    let result = shutdown_resp.get("result");
    assert!(
        result.is_some(),
        "shutdown response should have a result field"
    );

    // Send exit notification (server may close the stream after this)
    client.exit().await;
}

// ─── Hover Tests ─────────────────────────────────────────────────────────────
//
// Source: writ-golden/tests/golden/quest_system.writ
// These tests verify hover behavior over various language constructs.

const QUEST_URI: &str = "file:///test/quest_system.writ";
const QUEST_PATH: &str = "writ-golden/tests/golden/quest_system.writ";

/// Extract the hover markdown text from a hover result.
fn hover_text(hover_result: &Option<Value>) -> String {
    hover_result
        .as_ref()
        .and_then(|v| v.get("contents"))
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Hover on `available` (line 61) should show type `bool`.
/// Source: `let available: bool = is_quest_available(status);`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_available_shows_bool() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 61 (0-indexed: 60), col 8: `available`
    let hover = client.hover(QUEST_URI, 60, 8).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("bool"),
        "hovering `available` on line 61 should show bool, got: {}",
        text
    );
}

/// Hover on `types` (line 133) should show `QuestType[]`.
/// Source: `for t in types {`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_types_shows_quest_type_array() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 133 (0-indexed: 132), col 13: `types`
    let hover = client.hover(QUEST_URI, 132, 13).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("QuestType"),
        "hovering `types` on line 133 should show QuestType[], got: {}",
        text
    );
}

/// Hover on `t` (line 133) should show `QuestType`.
/// Source: `for t in types {`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_for_binding_shows_quest_type() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 133 (0-indexed: 132), col 8: `t`
    let hover = client.hover(QUEST_URI, 132, 8).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("QuestType"),
        "hovering `t` on line 133 should show QuestType, got: {}",
        text
    );
}

/// Hover on `main` (line 112) should show function signature.
/// Source: `fn main() {`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_main_shows_function_sig() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 112 (0-indexed: 111), col 3: `main`
    let hover = client.hover(QUEST_URI, 111, 3).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("fn") && text.contains("main"),
        "hovering `main` on line 112 should show function info, got: {}",
        text
    );
}

// ─── Deprecated Hover/Diagnostic Tests ───────────────────────────────────────
//
// These tests verify that [Deprecated("msg")] attributes surface correctly in
// LSP hover tooltips.
//
// Note: W0006 diagnostic squiggles are integration-tested at the compiler level in
// writ-compiler/tests/deprecated_tests.rs. The LSP diagnostic pipeline routes
// W0006 through Severity::Warning → DiagnosticSeverity::WARNING automatically
// (see writ-lsp/src/convert.rs). Single-file LSP tests do not trigger W0006
// because same-file deprecation references are suppressed by design.

const DEPRECATED_URI: &str = "file:///test/deprecated_test.writ";

/// Hovering over a deprecated function's declaration shows **Deprecated** notice.
///
/// Source line 0: `[Deprecated("use bar instead")] fn foo() -> void { }`
/// Hover at col 36 (the `foo` name on the declaration site).
/// Verifies that hover_text_for_def prepends the deprecation notice.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_deprecated_hover_on_declaration() {
    let mut client = LspClient::start().await;

    // Line 0: [Deprecated("use bar instead")] fn foo() -> void { }
    // `foo` name starts at col 36 (after "[Deprecated("use bar instead")] fn ")
    // Line 1: fn main() -> void { }
    let source = "[Deprecated(\"use bar instead\")] fn foo() -> void { }\nfn main() -> void { }\n";

    client.open_document(DEPRECATED_URI, source).await;

    // Hover on line 0 (0-indexed), col 36: the `foo` declaration name
    let hover = client.hover(DEPRECATED_URI, 0, 36).await;
    let text = hover_text(&hover);

    assert!(
        text.contains("Deprecated"),
        "hover on deprecated fn declaration should contain 'Deprecated', got: {:?}",
        text
    );
    assert!(
        text.contains("use bar instead"),
        "hover on deprecated fn declaration should contain the deprecation message, got: {:?}",
        text
    );
    assert!(
        text.contains("foo"),
        "hover on deprecated fn declaration should contain the fn name, got: {:?}",
        text
    );
}

/// Hovering over a call to a deprecated function shows **Deprecated** notice.
///
/// Source line 1: `fn main() -> void { foo(); }`
/// Hover at col 20 (the `foo` call site).
/// Verifies that hover_text_for_expr prepends the deprecation notice for Call expressions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_deprecated_hover_on_call_site() {
    let mut client = LspClient::start().await;

    // Line 0: [Deprecated("use bar instead")] fn foo() -> void { }
    // Line 1: fn main() -> void { foo(); }
    //         `foo` starts at col 20
    let source = "[Deprecated(\"use bar instead\")] fn foo() -> void { }\nfn main() -> void { foo(); }\n";

    client.open_document(DEPRECATED_URI, source).await;

    // Hover on line 1 (0-indexed), col 20: the `foo()` call expression
    let hover = client.hover(DEPRECATED_URI, 1, 20).await;
    let text = hover_text(&hover);

    assert!(
        text.contains("Deprecated"),
        "hover on deprecated fn call site should contain 'Deprecated', got: {:?}",
        text
    );
    assert!(
        text.contains("use bar instead"),
        "hover on deprecated fn call site should contain the deprecation message, got: {:?}",
        text
    );
}

/// Hover on `QuestStatus` (line 121) should show enum info.
/// Source: `QuestStatus::NotStarted,`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_quest_status_shows_enum_info() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 121 (0-indexed: 120), col 8: `QuestStatus`
    let hover = client.hover(QUEST_URI, 120, 8).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("QuestStatus"),
        "hovering `QuestStatus` on line 121 should show enum info, got: {}",
        text
    );
}

/// Hover on `MAX_QUESTS` (line 165) should show const type.
/// Source: `if active_quest_count < MAX_QUESTS {`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_max_quests_shows_const() {
    let mut client = LspClient::start().await;
    let source = fixture_source(QUEST_PATH);
    client.open_document(QUEST_URI, &source).await;

    // Line 165 (0-indexed: 164), col 30: `MAX_QUESTS`
    let hover = client.hover(QUEST_URI, 164, 30).await;
    let text = hover_text(&hover);
    assert!(
        !text.is_empty(),
        "hovering `MAX_QUESTS` on line 165 should produce hover"
    );
    assert!(
        text.contains("int"),
        "hovering `MAX_QUESTS` should show int type, got: {}",
        text
    );
}

// ─── Doc Comment Hover Tests ─────────────────────────────────────────────────
//
// Source: writ-golden/tests/golden/documented_functions.writ
// Tests that doc comments appear in hover text.

const DOC_URI: &str = "file:///test/documented_functions.writ";
const DOC_PATH: &str = "writ-golden/tests/golden/documented_functions.writ";

/// Hover on `add(3, 4)` call (line 36) should show doc comment.
/// Source: `let sum: int = add(3, 4);`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_add_call_shows_doc_comment() {
    let mut client = LspClient::start().await;
    let source = fixture_source(DOC_PATH);
    client.open_document(DOC_URI, &source).await;

    // Line 36 (0-indexed: 35), col 20: `add`
    let hover = client.hover(DOC_URI, 35, 20).await;
    let text = hover_text(&hover);

    assert!(
        text.contains("fn") && text.contains("add"),
        "hovering `add(3, 4)` should show function signature, got: {}",
        text
    );
    assert!(
        text.contains("Adds two integers"),
        "hovering `add(3, 4)` should show doc comment, got: {}",
        text
    );
}

// ─── New-keyword Completion Tests ────────────────────────────────────────────

/// Test: Completion after `new ` returns only constructable types.
///
/// Source defines a struct `Point`, an enum `Color`, and a function `helper`.
/// After typing `new ` only `Point` (constructable with `new`) should appear.
/// `Color` (enum) and `helper` (fn) must be excluded.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_after_new_keyword() {
    let mut client = LspClient::start().await;

    // Source: struct (constructable), enum (not), fn (not), main with `new ` at end of line 4.
    let source = "pub struct Point { x: int, y: int }\nenum Color { Red, Green, Blue }\nfn helper() -> int { 0 }\nfn main() {\n    let p = new \n}";
    let uri = "file:///test/new_completion.writ";

    client.open_document(uri, source).await;

    // Line 4 (0-indexed), character 16: right after "    let p = new "
    // "    let p = new " = 4 + 12 = 16 characters
    let result = client.completion(uri, 4, 16).await;

    assert!(
        result.is_some(),
        "completion after 'new ' should return results"
    );
    let result = result.unwrap();

    let items: Vec<Value> = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    assert!(
        labels.contains(&"Point"),
        "completion after 'new ' should include 'Point' (struct), got: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"Color"),
        "completion after 'new ' must NOT include 'Color' (enum), got: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"helper"),
        "completion after 'new ' must NOT include 'helper' (fn), got: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"fn"),
        "completion after 'new ' must NOT include keyword 'fn', got: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"let"),
        "completion after 'new ' must NOT include keyword 'let', got: {:?}",
        labels
    );
    assert!(
        !labels.contains(&"if"),
        "completion after 'new ' must NOT include keyword 'if', got: {:?}",
        labels
    );

    // Verify sort_text on new-keyword completions starts with "0_"
    let point_item = items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()) == Some("Point")
    }).expect("should have 'Point'");
    let point_sort = point_item.get("sortText").and_then(|v| v.as_str()).unwrap();
    assert!(
        point_sort.starts_with("0_"),
        "new-keyword sort_text should start with '0_', got: {}",
        point_sort
    );
    // Point is a struct — kind should be STRUCT (22)
    let point_kind = point_item.get("kind").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(point_kind, 22, "struct kind should be STRUCT (22), got: {}", point_kind);
}

/// Test: Completion NOT after `new` returns the full identifier/keyword list.
///
/// Verifies there is no regression — regular identifier completions still
/// include keywords when the cursor is not after `new `.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_not_after_new_still_returns_all() {
    let mut client = LspClient::start().await;

    // Source: struct and main, with `let x = ` (no `new`).
    let source = "pub struct Point { x: int, y: int }\nfn main() {\n    let x = \n}";
    let uri = "file:///test/no_new_completion.writ";

    client.open_document(uri, source).await;

    // Line 2 (0-indexed), character 12: right after "    let x = "
    // "    let x = " = 4 + 8 = 12 characters
    let result = client.completion(uri, 2, 12).await;

    assert!(
        result.is_some(),
        "regular identifier completion should return results"
    );
    let result = result.unwrap();

    let items: Vec<Value> = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    // Regular completions should still include keywords
    assert!(
        labels.contains(&"new"),
        "regular completions should include keyword 'new', got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"if"),
        "regular completions should include keyword 'if', got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"true"),
        "regular completions should include keyword 'true', got: {:?}",
        labels
    );
}

/// Hover on `square(5)` call (line 37) should show doc comment.
/// Source: `let sq: int = square(5);`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_square_call_shows_doc_comment() {
    let mut client = LspClient::start().await;
    let source = fixture_source(DOC_PATH);
    client.open_document(DOC_URI, &source).await;

    // Line 37 (0-indexed: 36), col 19: `square`
    let hover = client.hover(DOC_URI, 36, 19).await;
    let text = hover_text(&hover);

    assert!(
        text.contains("fn") && text.contains("square"),
        "hovering `square(5)` should show function signature, got: {}",
        text
    );
    assert!(
        text.contains("Computes the square"),
        "hovering `square(5)` should show doc comment, got: {}",
        text
    );
}

// ─── Private-def Completion Tests ────────────────────────────────────────────

/// Test: Identifier completions include non-pub (file-private) structs.
///
/// Source defines a non-pub `SomeStruct` and a `pub fn main`. At a cursor
/// position inside main after `let s = `, `SomeStruct` must appear in
/// identifier completions with kind STRUCT (22) and sortText starting with "0_".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_shows_private_struct() {
    let mut client = LspClient::start().await;

    // Non-pub struct + pub fn main with a complete body.
    // We request completion inside main (after the opening brace on line 4).
    let source = "struct SomeStruct {\n    x: int,\n    y: bool\n}\n\npub fn main() {\n    \n}\n";
    let uri = "file:///test/private_struct_completion.writ";

    client.open_document(uri, source).await;

    // Line 6 (0-indexed), character 4: inside the main body (after "    ")
    let result = client.completion(uri, 6, 4).await;

    assert!(
        result.is_some(),
        "completion inside main should return results"
    );
    let result = result.unwrap();

    let items: Vec<Value> = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    assert!(
        labels.contains(&"SomeStruct"),
        "completion should include private struct 'SomeStruct', got: {:?}",
        labels
    );

    // Verify kind is STRUCT (22)
    let struct_item = items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()) == Some("SomeStruct")
    }).expect("should have 'SomeStruct'");

    let struct_kind = struct_item.get("kind").and_then(|v| v.as_i64()).unwrap_or(-1);
    assert_eq!(
        struct_kind, 22,
        "SomeStruct kind should be STRUCT (22), got: {}",
        struct_kind
    );

    // Verify sortText starts with "0_"
    let struct_sort = struct_item.get("sortText").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        struct_sort.starts_with("0_"),
        "SomeStruct sortText should start with '0_', got: {}",
        struct_sort
    );
}

/// Test: new-keyword completions include non-pub (file-private) structs with field detail.
///
/// Source defines a non-pub `Point` struct. After typing `new ` inside main,
/// `Point` must appear with a detail string that contains its fields.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_after_new_shows_private_struct_with_detail() {
    let mut client = LspClient::start().await;

    let source = "struct Point { x: int, y: int }\npub fn main() {\n    let p = new \n}\n";
    let uri = "file:///test/private_new_completion.writ";

    client.open_document(uri, source).await;

    // Line 2 (0-indexed), character 16: right after "    let p = new "
    let result = client.completion(uri, 2, 16).await;

    assert!(
        result.is_some(),
        "completion after 'new ' should return results"
    );
    let result = result.unwrap();

    let items: Vec<Value> = if result.is_array() {
        result.as_array().cloned().unwrap_or_default()
    } else {
        result
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    assert!(
        labels.contains(&"Point"),
        "new-keyword completion should include private struct 'Point', got: {:?}",
        labels
    );

    // Verify detail contains field info
    let point_item = items.iter().find(|item| {
        item.get("label").and_then(|l| l.as_str()) == Some("Point")
    }).expect("should have 'Point'");

    let detail = point_item.get("detail").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        !detail.is_empty(),
        "Point should have non-empty detail text, got empty string"
    );
    assert!(
        detail.contains("x: int"),
        "Point detail should contain 'x: int', got: {}",
        detail
    );
    assert!(
        detail.contains("y: int"),
        "Point detail should contain 'y: int', got: {}",
        detail
    );
}

// ─── Entity namespace hover + completion tests ──────────────────────────────

/// Test: hovering `getOrCreate` on `Entity.getOrCreate<Guard>()` shows the method signature.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_entity_get_or_create() {
    let mut client = LspClient::start().await;
    let source = r#"[Singleton]
entity Guard {
    health: int = 100,
}
fn main() {
    let g = Entity.getOrCreate<Guard>();
}
"#;
    let uri = "file:///test/entity_get_or_create.writ";
    client.open_document(uri, source).await;

    // Line 5 (0-indexed), `Entity.getOrCreate<Guard>()` — hover on `Entity` at col 12
    let hover = client.hover(uri, 5, 12).await;
    let text = hover_text(&hover);
    assert!(
        text.contains("Entity"),
        "hovering Entity should show Entity namespace info, got: {}",
        text
    );

    client.shutdown().await;
}

/// Test: completion on `Entity.` returns the namespace methods.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_entity_dot_namespace() {
    let mut client = LspClient::start().await;
    let source = r#"[Singleton]
entity Guard {
    health: int = 100,
}
fn main() {
    Entity.
}
"#;
    let uri = "file:///test/entity_dot_completion.writ";
    client.open_document(uri, source).await;

    // Line 5 (0-indexed), col 11: right after `Entity.`
    let result = client.dot_completion(uri, 5, 11).await;

    let items: Vec<Value> = match result {
        Some(v) if v.is_array() => v.as_array().cloned().unwrap_or_default(),
        Some(v) => v.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
        None => vec![],
    };

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
        .collect();

    assert!(
        labels.contains(&"getOrCreate"),
        "Entity. completions should include getOrCreate, got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"destroy"),
        "Entity. completions should include destroy, got: {:?}",
        labels
    );

    client.shutdown().await;
}

/// Test: Valid contract-typed code produces zero LSP diagnostics.
///
/// A contract with a complete implementation and a properly-typed variable
/// must produce no errors. This is a regression test confirming E0122
/// (ContractAsType error, removed in Phase 84) is truly gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_contract_valid_no_errors() {
    let mut client = LspClient::start().await;
    let source = r#"pub contract Vocalize {
    fn speak(self, msg: string) -> void;
}
pub class NPC {}
impl Vocalize for NPC {
    fn speak(self, msg: string) -> void {}
}
pub fn main() {
    let s: Vocalize = new NPC {};
    s.speak("hello");
}
"#;
    let uri = "file:///test/contract_valid.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(uri, source)
        .await;

    // Collect all diagnostics from notifications targeting our document
    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if notif_uri == uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                all_diagnostics.extend(diags.iter().cloned());
            }
        }
    }

    assert!(
        all_diagnostics.is_empty(),
        "valid contract code should produce zero diagnostics, got: {:?}",
        all_diagnostics
    );

    client.shutdown().await;
}

/// Test: Assigning a non-implementing type to a contract-typed variable produces an error diagnostic.
///
/// BadClass does NOT implement Vocalize — the compiler should produce E0112.
/// This is a regression test confirming E0123 (incomplete impl) still fires.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_contract_invalid_produces_error() {
    let mut client = LspClient::start().await;
    let source = r#"pub contract Vocalize {
    fn speak(self, msg: string) -> void;
}
pub class BadClass {}
pub fn main() {
    let s: Vocalize = new BadClass {};
}
"#;
    let uri = "file:///test/contract_invalid.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(uri, source)
        .await;

    // Collect all diagnostics from notifications targeting our document
    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if notif_uri == uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                all_diagnostics.extend(diags.iter().cloned());
            }
        }
    }

    assert!(
        !all_diagnostics.is_empty(),
        "assigning non-implementing type to contract should produce at least one diagnostic, \
         got {} publishDiagnostics notifications",
        diag_notifications.len()
    );

    // Verify at least one diagnostic has severity Error (1 in LSP)
    let has_error = all_diagnostics.iter().any(|d| {
        d.get("severity")
            .and_then(|v| v.as_i64())
            .map(|s| s == 1)
            .unwrap_or(false)
    });
    assert!(
        has_error,
        "at least one diagnostic should have severity=Error (1), got: {:?}",
        all_diagnostics
    );

    client.shutdown().await;
}

/// Test: Code action for incomplete contract impl generates method stubs.
///
/// An impl block missing required methods (E0123) should produce a quick-fix
/// code action that inserts stub method bodies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_code_action_implement_missing_methods() {
    let mut client = LspClient::start().await;
    let source = r#"pub contract Greetable {
    fn greet(self) -> string;
    fn farewell(self, name: string) -> void;
}
pub class Person {}
impl Greetable for Person {
}
pub fn main() {}
"#;
    let uri = "file:///test/code_action.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(uri, source)
        .await;

    // Collect E0123 diagnostics
    let mut e0123_diags: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if notif_uri == uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                for d in diags {
                    if d.get("code").and_then(|v| v.as_str()) == Some("E0123") {
                        e0123_diags.push(d.clone());
                    }
                }
            }
        }
    }

    assert!(
        !e0123_diags.is_empty(),
        "incomplete impl should produce E0123 diagnostic"
    );

    // Send code action request with the E0123 diagnostic
    let diag = &e0123_diags[0];
    let range = diag.get("range").cloned().unwrap();

    let id = client.next_id();
    client
        .send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": range,
                "context": {
                    "diagnostics": e0123_diags
                }
            }
        }))
        .await;

    let resp = client.recv_response(id).await;
    let actions = resp
        .get("result")
        .and_then(|v| v.as_array())
        .expect("code action should return an array");

    assert!(
        !actions.is_empty(),
        "should produce at least one code action for E0123"
    );

    // Verify the action is a quickfix with workspace edit
    let action = &actions[0];
    assert_eq!(
        action.get("kind").and_then(|v| v.as_str()),
        Some("quickfix"),
        "action should be a quickfix"
    );

    let edit = action
        .get("edit")
        .expect("action should have an edit");
    let changes = edit
        .get("changes")
        .expect("edit should have changes");
    let file_edits = changes
        .get(uri)
        .and_then(|v| v.as_array())
        .expect("changes should contain edits for the document URI");

    assert!(
        !file_edits.is_empty(),
        "should have at least one text edit"
    );

    // Verify the inserted text contains both missing method stubs
    let new_text = file_edits[0]
        .get("newText")
        .and_then(|v| v.as_str())
        .expect("edit should have newText");

    assert!(
        new_text.contains("fn greet(self)"),
        "stub should contain greet method, got: {}",
        new_text
    );
    assert!(
        new_text.contains("fn farewell(self,"),
        "stub should contain farewell method, got: {}",
        new_text
    );
    assert!(
        new_text.contains("-> string"),
        "greet stub should have return type, got: {}",
        new_text
    );

    client.shutdown().await;
}

// ─── Attribute Diagnostic E2E Tests ──────────────────────────────────────────
//
// These tests verify the end-to-end LSP diagnostic pipeline for attribute-based
// diagnostics added in phases 93-98.

/// Test: [Deprecated] attribute on a function in a cross-file project produces W0006
/// Warning diagnostic with the deprecation message at the call site.
///
/// Uses project-mode (writ.toml in temp dir) because W0006 is only produced for
/// cross-file deprecation references (same-file references are suppressed by design).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_deprecated_warning_published() {
    let tmp = std::env::temp_dir().join("writ_lsp_test_deprecated_e2e");
    let _ = std::fs::remove_dir_all(&tmp);
    let src_dir = tmp.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Minimal project manifest — sources default to src/
    std::fs::write(
        tmp.join("writ.toml"),
        "[project]\nname = \"test-deprecated\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // lib.writ: defines a deprecated function
    std::fs::write(
        src_dir.join("lib.writ"),
        "[Deprecated(\"use bar instead\")]\npub fn foo() {}\n",
    )
    .unwrap();

    // main.writ: calls the deprecated function (triggers W0006 cross-file)
    std::fs::write(
        src_dir.join("main.writ"),
        "pub fn main() { foo(); }\n",
    )
    .unwrap();

    let mut client = LspClient::start_raw().await;

    let root_uri = Url::from_directory_path(&tmp).unwrap().to_string();
    client.initialize_with_root(&root_uri).await;

    let main_path = src_dir.join("main.writ");
    let main_uri = Url::from_file_path(&main_path).unwrap().to_string();
    let main_content = std::fs::read_to_string(&main_path).unwrap();

    let diag_notifications = client
        .open_document_and_collect_diagnostics(&main_uri, &main_content)
        .await;

    // Collect all diagnostics across all published URIs (project mode publishes per file)
    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
            all_diagnostics.extend(diags.iter().cloned());
        }
    }

    // W0006 should be emitted as a Warning (severity 2) with the deprecation message
    let has_w0006 = all_diagnostics.iter().any(|d| {
        d.get("code").and_then(|v| v.as_str()) == Some("W0006")
    });
    assert!(
        has_w0006,
        "Expected W0006 diagnostic for deprecated function call across files. Got diagnostics: {:?}",
        all_diagnostics
    );

    let has_warning_severity = all_diagnostics.iter().any(|d| {
        d.get("code").and_then(|v| v.as_str()) == Some("W0006")
            && d.get("severity").and_then(|v| v.as_i64()) == Some(2)
    });
    assert!(
        has_warning_severity,
        "W0006 should be severity 2 (Warning). Got diagnostics: {:?}",
        all_diagnostics
    );

    let has_message = all_diagnostics.iter().any(|d| {
        d.get("code").and_then(|v| v.as_str()) == Some("W0006")
            && d.get("message")
                .and_then(|v| v.as_str())
                .map(|msg| msg.contains("use bar instead"))
                .unwrap_or(false)
    });
    assert!(
        has_message,
        "W0006 diagnostic message should contain 'use bar instead'. Got diagnostics: {:?}",
        all_diagnostics
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Test: @speaker targeting a non-Singleton entity produces an E0007 diagnostic.
///
/// Uses standalone (single-file) mode — E0007 fires at resolve stage regardless
/// of file boundary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_speaker_validation_e0007() {
    let mut client = LspClient::start().await;

    let source = r#"entity Npc {}

dlg greet() {
    @Npc say("hello");
}
"#;
    let uri = "file:///test/speaker_e0007.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(uri, source)
        .await;

    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if notif_uri == uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                all_diagnostics.extend(diags.iter().cloned());
            }
        }
    }

    // E0007 = invalid speaker (entity is not [Singleton])
    let has_e0007 = all_diagnostics.iter().any(|d| {
        d.get("code").and_then(|v| v.as_str()) == Some("E0007")
    });
    assert!(
        has_e0007,
        "Expected E0007 diagnostic for non-Singleton @speaker. Got diagnostics: {:?}",
        all_diagnostics
    );

    // E0007 should be an Error (severity 1)
    let has_error_severity = all_diagnostics.iter().any(|d| {
        d.get("code").and_then(|v| v.as_str()) == Some("E0007")
            && d.get("severity").and_then(|v| v.as_i64()) == Some(1)
    });
    assert!(
        has_error_severity,
        "E0007 should be severity 1 (Error). Got diagnostics: {:?}",
        all_diagnostics
    );
}

/// DIAG-04: Hover on incomplete source must not crash the LSP server.
///
/// Opens a document with a syntax error (unterminated string literal) and sends
/// a hover request. The response must be a valid JSON-RPC response — not an
/// error code — even though the source does not parse cleanly. The server is
/// expected to return null (no hover info) gracefully rather than crash.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hover_on_incomplete_source_no_crash() {
    let mut client = LspClient::start().await;

    let incomplete_source = "pub fn main() {\n    let x: int = 42;\n    let y: string = \"unterminated\n}\n";
    let uri = "file:///test/incomplete_hover.writ";

    // Open document with syntax error — gives server time to analyze
    client.open_document(uri, incomplete_source).await;

    // Send hover request at line 1, character 8 (over `x`)
    let id = client.next_id();
    client.send(serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 8 }
        }
    })).await;

    let resp = client.recv_response(id).await;

    // The response must be a valid JSON-RPC response — no error field
    assert!(
        resp.get("error").is_none(),
        "hover on incomplete source must not return a JSON-RPC error, got: {:?}",
        resp
    );
    // result may be null (graceful degradation) or Some value — both acceptable
    assert!(
        resp.get("result").is_some(),
        "hover response must have a 'result' field (even if null), got: {:?}",
        resp
    );
}

/// DIAG-04: Completion on incomplete source must not crash the LSP server.
///
/// Opens a document with a syntax error and sends a completion request.
/// The response must be a valid JSON-RPC response, not a crash. The server
/// is expected to return null or an empty list gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_completion_on_incomplete_source_no_crash() {
    let mut client = LspClient::start().await;

    let incomplete_source = "pub fn main() {\n    let x: int = 42;\n    let y: string = \"unterminated\n}\n";
    let uri = "file:///test/incomplete_completion.writ";

    // Open document with syntax error — gives server time to analyze
    client.open_document(uri, incomplete_source).await;

    // Send completion request at line 1, character 12 (after `x: i`)
    let id = client.next_id();
    client.send(serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 12 }
        }
    })).await;

    let resp = client.recv_response(id).await;

    // The response must be a valid JSON-RPC response — no error field
    assert!(
        resp.get("error").is_none(),
        "completion on incomplete source must not return a JSON-RPC error, got: {:?}",
        resp
    );
    // result may be null (graceful degradation) or a list — both acceptable
    assert!(
        resp.get("result").is_some(),
        "completion response must have a 'result' field (even if null), got: {:?}",
        resp
    );
}
