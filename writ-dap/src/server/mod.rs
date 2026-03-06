//! DapServer: DAP protocol command dispatch loop for the writ debugger.
//!
//! Handles the full DAP lifecycle:
//!   initialize -> initialized event -> setBreakpoints -> configurationDone ->
//!   launch -> (execution) -> stopped event -> stackTrace/threads ->
//!   next/stepIn/stepOut -> stopped event -> continue -> terminated -> disconnect

use std::io::{Read, Write};
use std::io::{BufReader, BufWriter};

use dap::prelude::*;
use writ_module::module::Module;
use writ_runtime::{Runtime, TaskId};
use writ_diagnostics;

use crate::debug_host::DebugHost;

mod handlers;
mod helpers;
mod inspection;

/// The main DAP server struct. Manages protocol dispatch and VM lifecycle.
///
/// Type parameters:
/// - `I`: Input stream (implements `Read`, wrapped in `BufReader` internally by `Server`).
/// - `O`: Output stream (implements `Write`).
pub struct DapServer<I: Read, O: Write> {
    pub(super) server: Server<I, O>,
    /// Breakpoints stored before launch (source_path, requested_lines).
    pub(super) pending_breakpoints: Vec<(String, Vec<u32>)>,
    /// The running runtime (set after successful launch).
    pub(super) runtime: Option<Runtime<DebugHost>>,
    /// Maps FileId -> source file path for all files in the launched program.
    /// Single-file mode: one entry. Project mode: one entry per discovered .writ file.
    pub(super) source_paths: Vec<(writ_diagnostics::FileId, String)>,
    /// The compiled module (kept for method name / span resolution).
    pub(super) module: Option<Module>,
    /// The main task ID spawned on launch.
    pub(super) task_id: Option<TaskId>,
    /// Whether to stop on entry (from launch args).
    pub(super) stop_on_entry: bool,
    /// Whether configurationDone has been received.
    /// Execution is deferred until both launch and configurationDone have been processed,
    /// since VS Code may send them in either order.
    pub(super) configuration_done: bool,
    /// Whether launch has been processed (runtime is ready but execution not yet started).
    pub(super) launch_done: bool,
}

impl<I: Read, O: Write> DapServer<I, O> {
    /// Create a new DapServer wrapping a `dap::Server`.
    pub fn new(server: Server<I, O>) -> Self {
        DapServer {
            server,
            pending_breakpoints: Vec::new(),
            runtime: None,
            source_paths: Vec::new(),
            module: None,
            task_id: None,
            stop_on_entry: false,
            configuration_done: false,
            launch_done: false,
        }
    }

    /// Run the DAP command dispatch loop until `Disconnect` is received or input ends.
    pub fn run(&mut self) {
        loop {
            let req = match self.server.poll_request() {
                Ok(Some(r)) => r,
                Ok(None) => break, // input stream closed
                Err(e) => {
                    eprintln!("[writ-dap] poll_request error: {:?}", e);
                    break;
                }
            };

            let should_break = matches!(req.command, Command::Disconnect(_));

            self.handle_request(req);

            if should_break {
                break;
            }
        }
    }

    /// Dispatch a single request and send the response.
    fn handle_request(&mut self, req: Request) {
        match req.command.clone() {
            Command::Initialize(_args) => {
                self.handle_initialize(req);
            }

            Command::SetBreakpoints(args) => {
                self.handle_set_breakpoints(req, args);
            }

            Command::ConfigurationDone => {
                self.handle_configuration_done(req);
            }

            Command::Launch(args) => {
                self.handle_launch(req, args);
            }

            Command::Threads => {
                self.handle_threads(req);
            }

            Command::StackTrace(args) => {
                self.handle_stack_trace(req, args);
            }

            Command::Scopes(args) => {
                self.handle_scopes(req, args);
            }

            Command::Variables(args) => {
                self.handle_variables(req, args);
            }

            Command::Evaluate(args) => {
                self.handle_evaluate(req, args);
            }

            Command::Disconnect(_) => {
                self.handle_disconnect(req);
            }

            Command::Next(_args) => {
                self.handle_next(req);
            }

            Command::StepIn(_args) => {
                self.handle_step_in(req);
            }

            Command::StepOut(_args) => {
                self.handle_step_out(req);
            }

            Command::Continue(_args) => {
                self.handle_continue(req);
            }

            Command::Pause(_) => {
                let rsp = req.error("pause not supported");
                let _ = self.server.respond(rsp);
            }

            _ => {
                let rsp = req.error("not supported");
                let _ = self.server.respond(rsp);
            }
        }
    }
}

// ─── Convenience constructor for stdio DAP server ─────────────────────────────

/// Create a DapServer that reads from stdin and writes to stdout.
pub fn stdio_server() -> DapServer<std::io::Stdin, std::io::Stdout> {
    let input = BufReader::new(std::io::stdin());
    let output = BufWriter::new(std::io::stdout());
    let server = Server::new(input, output);
    DapServer::new(server)
}
