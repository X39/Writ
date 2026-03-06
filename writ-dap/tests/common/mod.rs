//! Shared bidirectional DAP test client.
//!
//! Uses `std::io::pipe()` to create two pipe pairs for full-duplex communication
//! with a `DapServer` running on a background thread. Each request/response is
//! exchanged synchronously, enabling dynamic value extraction (threadId, frameId,
//! variablesReference) that was impossible with the old pre-built-input approach.

#![allow(dead_code)] // Different test crates use different subsets of methods.

use std::io::{BufReader, BufWriter, PipeReader, PipeWriter, Read, Write};
use std::thread::{self, JoinHandle};

use dap::prelude::*;
use serde_json::{json, Value};
use writ_dap::server::DapServer;

/// Bidirectional DAP test client that communicates with a `DapServer` over pipes.
pub struct DapClient {
    writer: Option<PipeWriter>,
    reader: BufReader<PipeReader>,
    server_thread: Option<JoinHandle<()>>,
    seq: i64,
}

impl DapClient {
    /// Spawn a DapServer on a background thread and return a connected client.
    pub fn start() -> Self {
        // Pipe 1: client writes → server reads
        let (server_input, client_writer) = std::io::pipe().unwrap();
        // Pipe 2: server writes → client reads
        let (client_reader, server_output) = std::io::pipe().unwrap();

        let server_thread = thread::spawn(move || {
            let reader = BufReader::new(server_input);
            let writer = BufWriter::new(server_output);
            let server = Server::new(reader, writer);
            let mut dap_server = DapServer::new(server);
            dap_server.run();
        });

        DapClient {
            writer: Some(client_writer),
            reader: BufReader::new(client_reader),
            server_thread: Some(server_thread),
            seq: 0,
        }
    }

    // ─── Low-level protocol ─────────────────────────────────────────────

    /// Send a framed DAP request with the given command and arguments.
    pub fn send(&mut self, command: &str, args: Value) -> i64 {
        self.seq += 1;
        let seq = self.seq;
        let body = serde_json::to_string(&json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": args
        }))
        .unwrap();
        let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let w = self.writer.as_mut().expect("client writer already closed");
        w.write_all(framed.as_bytes()).unwrap();
        w.flush().unwrap();
        seq
    }

    /// Send a framed DAP request with no arguments.
    pub fn send_no_args(&mut self, command: &str) -> i64 {
        self.seq += 1;
        let seq = self.seq;
        let body = serde_json::to_string(&json!({
            "seq": seq,
            "type": "request",
            "command": command
        }))
        .unwrap();
        let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let w = self.writer.as_mut().expect("client writer already closed");
        w.write_all(framed.as_bytes()).unwrap();
        w.flush().unwrap();
        seq
    }

    /// Read one Content-Length framed message from the server.
    pub fn read_message(&mut self) -> Value {
        // Read headers until blank line (\r\n\r\n).
        // The header section may contain multiple headers separated by \r\n.
        let mut header = String::new();
        loop {
            let mut byte = [0u8; 1];
            self.reader.read_exact(&mut byte).expect("server closed unexpectedly");
            header.push(byte[0] as char);
            if header.ends_with("\r\n\r\n") {
                break;
            }
        }

        // Parse Content-Length from header section
        let length: usize = header
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap_or_else(|| panic!("expected Content-Length header in: {:?}", header))
            .trim()
            .parse()
            .expect("invalid Content-Length");

        let mut body = vec![0u8; length];
        self.reader.read_exact(&mut body).expect("short read on body");
        serde_json::from_slice(&body).expect("invalid JSON in response body")
    }

    /// Read messages until a response matching `req_seq` arrives.
    /// Returns (response, collected_events).
    pub fn recv_response(&mut self, req_seq: i64) -> (Value, Vec<Value>) {
        let mut events = Vec::new();
        loop {
            let msg = self.read_message();
            if msg.get("type").and_then(|v| v.as_str()) == Some("event") {
                events.push(msg);
                continue;
            }
            if msg.get("type").and_then(|v| v.as_str()) == Some("response")
                && msg.get("request_seq").and_then(|v| v.as_i64()) == Some(req_seq)
            {
                return (msg, events);
            }
            // Skip unexpected messages (e.g., responses to other seqs)
            events.push(msg);
        }
    }

    /// Read messages until an event of the given type arrives.
    pub fn recv_event(&mut self, event_type: &str) -> Value {
        loop {
            let msg = self.read_message();
            if msg.get("type").and_then(|v| v.as_str()) == Some("event")
                && msg.get("event").and_then(|v| v.as_str()) == Some(event_type)
            {
                return msg;
            }
        }
    }

    /// Read messages until a "stopped" or "terminated" event arrives.
    /// Collects all events (including breakpoint change events) along the way.
    pub fn recv_execution_event(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        loop {
            let msg = self.read_message();
            let is_terminal = msg.get("type").and_then(|v| v.as_str()) == Some("event")
                && matches!(
                    msg.get("event").and_then(|v| v.as_str()),
                    Some("stopped") | Some("terminated")
                );
            events.push(msg);
            if is_terminal {
                return events;
            }
        }
    }

    // ─── Typed convenience methods ──────────────────────────────────────

    /// Send initialize + initialized notification, return response body.
    pub fn initialize(&mut self) -> Value {
        let seq = self.send("initialize", json!({
            "adapterID": "writ-dap-test",
            "clientName": "protocol-test"
        }));
        let (resp, events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "initialize should succeed, got: {}",
            resp
        );

        // The server sends an initialized event automatically
        let has_initialized = events
            .iter()
            .any(|e| e.get("event").and_then(|v| v.as_str()) == Some("initialized"));
        if !has_initialized {
            // It might arrive as the next message
            let event = self.recv_event("initialized");
            assert_eq!(
                event.get("event").and_then(|v| v.as_str()),
                Some("initialized")
            );
        }

        resp
    }

    /// Send configurationDone, return response.
    pub fn configuration_done(&mut self) -> Value {
        let seq = self.send_no_args("configurationDone");
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "configurationDone should succeed, got: {}",
            resp
        );
        resp
    }

    /// Send setBreakpoints, return response body (contains breakpoints array).
    pub fn set_breakpoints(&mut self, path: &str, lines: &[i64]) -> Value {
        let bp_list: Vec<Value> = lines.iter().map(|&l| json!({ "line": l })).collect();
        let seq = self.send("setBreakpoints", json!({
            "source": { "path": path },
            "breakpoints": bp_list
        }));
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "setBreakpoints should succeed, got: {}",
            resp
        );
        resp.get("body").cloned().unwrap_or(json!({}))
    }

    /// Send launch, return (response, events).
    /// After a successful launch, reads additional events until stopped/terminated.
    pub fn launch(&mut self, program: &str, stop_on_entry: bool) -> (Value, Vec<Value>) {
        let seq = self.send("launch", json!({
            "program": program,
            "stopOnEntry": stop_on_entry
        }));
        let (resp, mut events) = self.recv_response(seq);
        // Server sends response first, then runs the VM which emits events.
        if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
            let post_events = self.recv_execution_event();
            events.extend(post_events);
        }
        (resp, events)
    }

    /// Send launch with custom args (e.g., missing program), return (response, events).
    pub fn launch_raw(&mut self, args: Value) -> (Value, Vec<Value>) {
        let seq = self.send("launch", args);
        self.recv_response(seq)
    }

    /// Send threads request, return response body (contains threads array).
    pub fn threads(&mut self) -> Value {
        let seq = self.send_no_args("threads");
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "threads should succeed, got: {}",
            resp
        );
        resp.get("body").cloned().unwrap_or(json!({}))
    }

    /// Send stackTrace request, return response body (contains stackFrames array).
    pub fn stack_trace(&mut self, thread_id: i64) -> Value {
        let seq = self.send("stackTrace", json!({ "threadId": thread_id }));
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "stackTrace should succeed, got: {}",
            resp
        );
        resp.get("body").cloned().unwrap_or(json!({}))
    }

    /// Send scopes request, return response body (contains scopes array).
    pub fn scopes(&mut self, frame_id: i64) -> Value {
        let seq = self.send("scopes", json!({ "frameId": frame_id }));
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "scopes should succeed, got: {}",
            resp
        );
        resp.get("body").cloned().unwrap_or(json!({}))
    }

    /// Send variables request, return response body (contains variables array).
    pub fn variables(&mut self, vars_ref: i64) -> Value {
        let seq = self.send("variables", json!({ "variablesReference": vars_ref }));
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "variables should succeed, got: {}",
            resp
        );
        resp.get("body").cloned().unwrap_or(json!({}))
    }

    /// Send continue request, return (response, events).
    /// After a successful continue, reads additional events until stopped/terminated.
    pub fn continue_(&mut self, thread_id: i64) -> (Value, Vec<Value>) {
        let seq = self.send("continue", json!({ "threadId": thread_id }));
        let (resp, mut events) = self.recv_response(seq);
        if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
            let post_events = self.recv_execution_event();
            events.extend(post_events);
        }
        (resp, events)
    }

    /// Send evaluate request, return (response, events).
    pub fn evaluate(&mut self, expression: &str, frame_id: i64) -> (Value, Vec<Value>) {
        let seq = self.send("evaluate", json!({
            "expression": expression,
            "frameId": frame_id
        }));
        self.recv_response(seq)
    }

    /// Send disconnect and close the connection.
    pub fn disconnect(&mut self) {
        let seq = self.send("disconnect", json!({}));
        let (resp, _events) = self.recv_response(seq);
        assert_eq!(
            resp.get("success").and_then(|v| v.as_bool()),
            Some(true),
            "disconnect should succeed, got: {}",
            resp
        );
    }

    /// Send disconnect, drop writer, join server thread.
    pub fn shutdown(&mut self) {
        self.disconnect();
        // Drop the writer so the server's read loop sees EOF and exits.
        self.writer.take();
        if let Some(handle) = self.server_thread.take() {
            handle.join().expect("server thread panicked");
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Resolve a path relative to the workspace root.
pub fn workspace_file(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .expect("workspace root should exist");
    workspace_root.join(relative).to_string_lossy().into_owned()
}
