//! Handler methods for each DAP Command variant.
//!
//! Each method corresponds to one arm of the handle_request() match in mod.rs.

use std::io::{Read, Write};

use dap::prelude::*;
use writ_diagnostics;
use writ_module::heap::read_string;
use writ_runtime::runtime::RuntimeBuilder;

use crate::breakpoints::BreakpointTable;
use crate::debug_host::DebugHost;
use crate::launch::{compile_and_load, compile_and_load_project};
use crate::variables::{make_variables_ref, unpack_variables_ref};

use super::DapServer;
use super::helpers::{decode_frame_id, build_thread_list};

impl<I: Read, O: Write> DapServer<I, O> {
    pub(super) fn handle_initialize(&mut self, req: Request) {
        let caps = types::Capabilities {
            supports_configuration_done_request: Some(true),
            supports_step_back: Some(false),
            ..Default::default()
        };
        let rsp = req.success(ResponseBody::Initialize(caps));
        let _ = self.server.respond(rsp);
        let _ = self.server.send_event(Event::Initialized);
    }

    pub(super) fn handle_set_breakpoints(&mut self, req: Request, args: requests::SetBreakpointsArguments) {
        let source_path = args.source.path.clone()
            .or_else(|| args.source.name.clone())
            .unwrap_or_default();

        let requested_lines: Vec<u32> = args
            .breakpoints
            .as_ref()
            .map(|bps| bps.iter().map(|bp| bp.line as u32).collect())
            .unwrap_or_default();

        let dap_source = args.source.clone();

        if let Some(rt) = self.runtime.as_mut() {
            // Post-launch: resolve breakpoints against the loaded module.
            let resolved = rt.host_mut().breakpoints.set_breakpoints(&requested_lines);

            let dap_bps: Vec<types::Breakpoint> = resolved
                .iter()
                .map(|bp| types::Breakpoint {
                    id: Some(bp.id as i64),
                    verified: true,
                    line: Some(bp.line as i64),
                    source: Some(dap_source.clone()),
                    ..Default::default()
                })
                .collect();

            // Pad unresolved breakpoints with unverified entries.
            let mut result = dap_bps;
            while result.len() < requested_lines.len() {
                result.push(types::Breakpoint {
                    verified: false,
                    ..Default::default()
                });
            }

            let rsp = req.success(ResponseBody::SetBreakpoints(
                responses::SetBreakpointsResponse { breakpoints: result },
            ));
            let _ = self.server.respond(rsp);
        } else {
            // Pre-launch: store pending breakpoints, respond unverified.
            self.pending_breakpoints
                .push((source_path, requested_lines.clone()));

            let dap_bps: Vec<types::Breakpoint> = requested_lines
                .iter()
                .map(|&line| types::Breakpoint {
                    verified: false,
                    line: Some(line as i64),
                    source: Some(dap_source.clone()),
                    ..Default::default()
                })
                .collect();

            let rsp = req.success(ResponseBody::SetBreakpoints(
                responses::SetBreakpointsResponse { breakpoints: dap_bps },
            ));
            let _ = self.server.respond(rsp);
        }
    }

    pub(super) fn handle_configuration_done(&mut self, req: Request) {
        let rsp = req.success(ResponseBody::ConfigurationDone);
        let _ = self.server.respond(rsp);
        self.configuration_done = true;
        // If launch already completed, start execution now.
        if self.launch_done {
            self.start_execution();
        }
    }

    pub(super) fn handle_launch(&mut self, req: Request, args: requests::LaunchRequestArguments) {
        // Extract the "program" field from additional_data.
        let additional = args.additional_data.as_ref()
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let program_path = match additional.get("program").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => {
                let err = req.error("launch requires 'program' argument");
                let _ = self.server.respond(err);
                return;
            }
        };

        let stop_on_entry = additional
            .get("stopOnEntry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.stop_on_entry = stop_on_entry;

        // Detect launch mode: .writ file = single-file, directory or writ.toml = project mode.
        let path = std::path::Path::new(&program_path);
        let is_project = path.is_dir()
            || program_path.ends_with("writ.toml");

        let (module, source_paths, method_file_ids) = if is_project {
            // Project mode: compile all .writ files discovered via writ.toml.
            let project_root = if program_path.ends_with("writ.toml") {
                path.parent().unwrap_or(path).to_path_buf()
            } else {
                path.to_path_buf()
            };
            match compile_and_load_project(&project_root) {
                Ok((module, file_id_paths, method_file_ids)) => (module, file_id_paths, method_file_ids),
                Err(e) => {
                    let err = req.error(&format!("compile error: {}", e));
                    let _ = self.server.respond(err);
                    return;
                }
            }
        } else {
            // Single-file mode.
            match compile_and_load(&program_path) {
                Ok((module, _src, method_file_ids)) => {
                    let file_id_paths = vec![(writ_diagnostics::FileId(0), program_path.clone())];
                    (module, file_id_paths, method_file_ids)
                }
                Err(e) => {
                    let err = req.error(&format!("compile error: {}", e));
                    let _ = self.server.respond(err);
                    return;
                }
            }
        };

        // Build breakpoint table and debug host.
        let breakpoint_table = BreakpointTable::new(&module);
        let debug_host = DebugHost::new(breakpoint_table, &module);

        // Build the runtime.
        let mut runtime = match RuntimeBuilder::new(module.clone())
            .with_host(debug_host)
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let err = req.error(&format!("runtime error: {:?}", e));
                let _ = self.server.respond(err);
                return;
            }
        };

        // Find the "main" export (item_kind == 0 = method).
        // If not found in exports, fall back to searching method_defs
        // by name. This allows `fn main()` (without `pub`) to work.
        let main_method_idx = module.export_defs.iter().find_map(|export| {
            if export.item_kind == 0 {
                let name = read_string(&module.string_heap, export.name)
                    .unwrap_or("");
                if name == "main" {
                    export.item.row_index().map(|idx| (idx - 1) as usize)
                } else {
                    None
                }
            } else {
                None
            }
        }).or_else(|| {
            // Fallback: search method_defs by name (non-pub entry points)
            module.method_defs.iter().enumerate().find_map(|(idx, md)| {
                let name = read_string(&module.string_heap, md.name)
                    .unwrap_or("");
                if name == "main" { Some(idx) } else { None }
            })
        });

        let main_idx = match main_method_idx {
            Some(idx) => idx,
            None => {
                let err = req.error("no 'main' export found in program");
                let _ = self.server.respond(err);
                return;
            }
        };

        let task_id = match runtime.spawn_task(main_idx, vec![]) {
            Ok(id) => id,
            Err(e) => {
                let err = req.error(&format!("spawn error: {:?}", e));
                let _ = self.server.respond(err);
                return;
            }
        };

        // Set source_paths and method_file_ids before resolving breakpoints so events
        // include the correct source reference.
        self.source_paths = source_paths;
        self.method_file_ids = method_file_ids;

        // Resolve pending breakpoints now that the module is loaded.
        for (_, lines) in &self.pending_breakpoints.clone() {
            let resolved = runtime.host_mut().breakpoints.set_breakpoints(lines);
            // Send breakpoint update events for each resolved breakpoint.
            for bp in &resolved {
                let source = self.source_paths.first().map(|(_, p)| {
                    types::Source {
                        path: Some(p.clone()),
                        name: std::path::Path::new(p)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string()),
                        ..Default::default()
                    }
                });
                let _ = self.server.send_event(Event::Breakpoint(
                    events::BreakpointEventBody {
                        reason: types::BreakpointEventReason::Changed,
                        breakpoint: types::Breakpoint {
                            id: Some(bp.id as i64),
                            verified: true,
                            line: Some(bp.line as i64),
                            source,
                            ..Default::default()
                        },
                    },
                ));
            }
        }
        self.pending_breakpoints.clear();

        self.runtime = Some(runtime);
        self.module = Some(module);
        self.task_id = Some(task_id);

        // Respond to launch before any execution.
        let rsp = req.success(ResponseBody::Launch);
        let _ = self.server.respond(rsp);

        self.launch_done = true;
        // Defer execution until configurationDone arrives (VS Code may send
        // launch before setBreakpoints/configurationDone).
        if self.configuration_done {
            self.start_execution();
        }
    }

    /// Begin VM execution after both launch and configurationDone have been received.
    pub(super) fn start_execution(&mut self) {
        if self.stop_on_entry {
            let thread_id = self.task_id.map(|t| t.index as i64).unwrap_or(0);
            let _ = self.server.send_event(Event::Stopped(events::StoppedEventBody {
                reason: types::StoppedEventReason::Entry,
                description: None,
                thread_id: Some(thread_id),
                preserve_focus_hint: None,
                text: None,
                all_threads_stopped: Some(true),
                hit_breakpoint_ids: None,
            }));
        } else {
            self.run_until_stop();
        }
    }

    pub(super) fn handle_threads(&mut self, req: Request) {
        let threads = if let (Some(rt), Some(module)) = (self.runtime.as_ref(), self.module.as_ref()) {
            let task_ids = rt.all_task_ids();
            if task_ids.is_empty() {
                // Check if the main task crashed -- if so, report it as a stopped thread
                // so VSCode can inspect the crash state.
                if let Some(task_id) = self.task_id {
                    if rt.crash_info(task_id).is_some() {
                        vec![types::Thread {
                            id: task_id.index as i64,
                            name: "main (crashed)".to_string(),
                        }]
                    } else {
                        vec![types::Thread { id: 0, name: "terminated".to_string() }]
                    }
                } else {
                    vec![types::Thread { id: 0, name: "terminated".to_string() }]
                }
            } else {
                build_thread_list(&task_ids, |tid| rt.call_stack_frames(tid), module)
            }
        } else {
            vec![types::Thread { id: 0, name: "terminated".to_string() }]
        };
        let rsp = req.success(ResponseBody::Threads(responses::ThreadsResponse { threads }));
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_stack_trace(&mut self, req: Request, args: requests::StackTraceArguments) {
        let thread_id = args.thread_id;
        let frames = if let Some(task_id) = self.resolve_task_id(thread_id) {
            self.build_stack_frames(task_id)
        } else if let Some(task_id) = self.task_id {
            // Fallback to main task if thread_id not found
            self.build_stack_frames(task_id)
        } else {
            vec![]
        };
        let total = frames.len() as i64;
        let rsp = req.success(ResponseBody::StackTrace(responses::StackTraceResponse {
            stack_frames: frames,
            total_frames: Some(total),
        }));
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_scopes(&mut self, req: Request, args: requests::ScopesArguments) {
        let frame_id = args.frame_id;
        let (task_idx, frame_idx) = decode_frame_id(frame_id);

        let local_count = self.count_active_locals(task_idx, frame_idx);

        let vars_ref = make_variables_ref(task_idx, frame_idx);

        let scope = types::Scope {
            name: "Locals".to_string(),
            variables_reference: vars_ref,
            expensive: false,
            presentation_hint: Some(types::ScopePresentationhint::String("locals".to_string())),
            named_variables: Some(local_count as i64),
            ..Default::default()
        };

        let rsp = req.success(ResponseBody::Scopes(responses::ScopesResponse {
            scopes: vec![scope],
        }));
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_variables(&mut self, req: Request, args: requests::VariablesArguments) {
        let (task_idx, frame_idx) = unpack_variables_ref(args.variables_reference);
        let variables = self.get_variables(task_idx, frame_idx);

        let rsp = req.success(ResponseBody::Variables(responses::VariablesResponse {
            variables,
        }));
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_evaluate(&mut self, req: Request, args: requests::EvaluateArguments) {
        let expr = args.expression.clone();
        let frame_id = args.frame_id.unwrap_or(0);
        let (task_idx, display_frame_idx) = decode_frame_id(frame_id);

        let result = self.do_evaluate(&expr, task_idx, display_frame_idx);

        let rsp = req.success(ResponseBody::Evaluate(responses::EvaluateResponse {
            result: result.0,
            type_field: result.1,
            variables_reference: 0,
            ..Default::default()
        }));
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_disconnect(&mut self, req: Request) {
        let rsp = req.success(ResponseBody::Disconnect);
        let _ = self.server.respond(rsp);
    }

    pub(super) fn handle_next(&mut self, req: Request) {
        // Step Over: stop at next line at same or lower call depth.
        let (current_line, current_method) = self.current_position();
        if let (Some(rt), Some(task_id)) = (self.runtime.as_mut(), self.task_id) {
            rt.host_mut().set_step_over(task_id, current_line, current_method);
        }
        let rsp = req.success(ResponseBody::Next);
        let _ = self.server.respond(rsp);
        self.run_until_stop();
    }

    pub(super) fn handle_step_in(&mut self, req: Request) {
        // Step Into: stop at next line in any method (including callees).
        let (current_line, current_method) = self.current_position();
        if let Some(rt) = self.runtime.as_mut() {
            rt.host_mut().set_step_into(current_line, current_method);
        }
        let rsp = req.success(ResponseBody::StepIn);
        let _ = self.server.respond(rsp);
        self.run_until_stop();
    }

    pub(super) fn handle_step_out(&mut self, req: Request) {
        // Step Out: stop after returning from the current frame.
        if let (Some(rt), Some(task_id)) = (self.runtime.as_mut(), self.task_id) {
            rt.host_mut().set_step_out(task_id);
        }
        let rsp = req.success(ResponseBody::StepOut);
        let _ = self.server.respond(rsp);
        self.run_until_stop();
    }

    pub(super) fn handle_continue(&mut self, req: Request) {
        // Continue: resume execution until next breakpoint or completion.
        if let Some(rt) = self.runtime.as_mut() {
            rt.host_mut().clear_step();
        }
        let rsp = req.success(ResponseBody::Continue(responses::ContinueResponse {
            all_threads_continued: Some(true),
        }));
        let _ = self.server.respond(rsp);
        self.run_until_stop();
    }
}
