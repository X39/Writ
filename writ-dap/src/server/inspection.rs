//! Runtime inspection methods on DapServer.
//!
//! These methods run the VM, resolve task/frame positions, and build
//! DAP-formatted stack frames, variables, and evaluation results.

use std::io::{Read, Write};

use dap::prelude::*;
use writ_module::heap::read_string;
use writ_runtime::{LogLevel, SuspendReason, TaskState, TaskId};
use writ_runtime::runtime::{ExecutionLimit, TickResult};

use crate::debug_host::StopReason;

use super::DapServer;
use super::helpers::{instr_to_byte_pc, collect_frame_variables, evaluate_local};

impl<I: Read, O: Write> DapServer<I, O> {
    /// Run the VM until a debug suspension (breakpoint/step) or program completion.
    ///
    /// Sends the appropriate DAP event and returns.
    pub(super) fn run_until_stop(&mut self) {
        let task_id = match self.task_id {
            Some(id) => id,
            None => return,
        };

        // If the task is already in a terminal state (e.g., user clicked Continue after
        // crash inspection), emit terminated+exited and return without attempting to resume.
        // A Cancelled task with crash_info means the user was viewing the crash state;
        // exit with code 1 to indicate the crash.
        if let Some(rt) = self.runtime.as_ref() {
            let state = rt.task_state(task_id);
            if matches!(state, Some(TaskState::Completed) | Some(TaskState::Cancelled)) {
                let exit_code = if rt.crash_info(task_id).is_some() { 1 } else { 0 };
                let _ = self.server.send_event(Event::Terminated(Some(
                    events::TerminatedEventBody { restart: None },
                )));
                let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                    exit_code,
                }));
                return;
            }
        }

        let runtime = match self.runtime.as_mut() {
            Some(rt) => rt,
            None => return,
        };

        // If the task is suspended, check whether it's a CrashPending resume
        // (user clicked Continue after crash inspection) or a normal debug resume.
        if runtime.task_state(task_id) == Some(TaskState::Suspended) {
            let is_crash_resume = matches!(
                runtime.suspend_reason(task_id),
                Some(SuspendReason::CrashPending { .. })
            );
            if let Err(e) = runtime.resume_debug(task_id) {
                eprintln!("[writ-dap] resume_debug error: {:?}", e);
                return;
            }
            // If we just resumed a CrashPending, the task is now Cancelled (unwound).
            // Emit terminated + exited with code 1 and return without entering the tick loop.
            if is_crash_resume {
                let _ = self.server.send_event(Event::Terminated(Some(
                    events::TerminatedEventBody { restart: None },
                )));
                let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                    exit_code: 1,
                }));
                return;
            }
        }

        loop {
            let tick_result = runtime.tick(0.0, ExecutionLimit::Instructions(1000));

            // Drain log messages buffered by DebugHost and send as DAP Output events.
            let log_messages = runtime.host_mut().drain_log_messages();
            for (level, msg) in log_messages {
                let category = match level {
                    LogLevel::Error => types::OutputEventCategory::Stderr,
                    _ => types::OutputEventCategory::Stdout,
                };
                let prefix = match level {
                    LogLevel::Trace => "TRACE",
                    LogLevel::Debug => "DEBUG",
                    LogLevel::Info => "INFO",
                    LogLevel::Warn => "WARN",
                    LogLevel::Error => "ERROR",
                };
                let _ = self.server.send_event(Event::Output(events::OutputEventBody {
                    output: format!("[{prefix}] {msg}\n"),
                    category: Some(category),
                    ..Default::default()
                }));
            }

            // Check if the task is now suspended after this tick.
            match runtime.task_state(task_id) {
                Some(TaskState::Suspended) => {
                    // Check if it's a debug suspension (breakpoint, step, or crash-before-unwind).
                    let is_debug_suspend = matches!(
                        runtime.suspend_reason(task_id),
                        Some(SuspendReason::Breakpoint { .. })
                        | Some(SuspendReason::DebugStep { .. })
                        | Some(SuspendReason::CrashPending { .. })
                    );

                    if is_debug_suspend {
                        // CrashPending takes priority: emit stopped(exception) with the crash
                        // message directly, bypassing DebugHost's stop reason mechanism.
                        if let Some(SuspendReason::CrashPending { message }) =
                            runtime.suspend_reason(task_id)
                        {
                            let message = message.clone();
                            let _ = self.server.send_event(Event::Output(events::OutputEventBody {
                                output: format!("Runtime crash: {}\n", message),
                                category: Some(types::OutputEventCategory::Stderr),
                                ..Default::default()
                            }));
                            let thread_id = self.task_id.map(|t| t.index as i64).unwrap_or(0);
                            let _ = self.server.send_event(Event::Stopped(events::StoppedEventBody {
                                reason: types::StoppedEventReason::Exception,
                                description: Some(message.clone()),
                                thread_id: Some(thread_id),
                                preserve_focus_hint: None,
                                text: Some(message),
                                all_threads_stopped: Some(true),
                                hit_breakpoint_ids: None,
                            }));
                            return;
                        }

                        // Breakpoint or DebugStep: take the stop reason from DebugHost.
                        let stop_reason = runtime.host_mut().take_pending_stop();
                        let (dap_reason, hit_ids) = match stop_reason {
                            Some(StopReason::Breakpoint(id)) => {
                                (types::StoppedEventReason::Breakpoint, Some(vec![id as i64]))
                            }
                            Some(StopReason::Step) => {
                                (types::StoppedEventReason::Step, None)
                            }
                            Some(StopReason::Entry) => {
                                (types::StoppedEventReason::Entry, None)
                            }
                            Some(StopReason::Pause) => {
                                (types::StoppedEventReason::Pause, None)
                            }
                            Some(StopReason::Exception(_)) => {
                                // Exception stops are handled by CrashPending above,
                                // but include a fallback here for completeness.
                                (types::StoppedEventReason::Exception, None)
                            }
                            None => {
                                // DebugHost didn't set a stop reason (shouldn't happen).
                                (types::StoppedEventReason::Step, None)
                            }
                        };

                        let thread_id = self.task_id.map(|t| t.index as i64).unwrap_or(0);
                        let _ = self.server.send_event(Event::Stopped(events::StoppedEventBody {
                            reason: dap_reason,
                            description: None,
                            thread_id: Some(thread_id),
                            preserve_focus_hint: None,
                            text: None,
                            all_threads_stopped: Some(true),
                            hit_breakpoint_ids: hit_ids,
                        }));
                        return;
                    }
                    // Otherwise it's a HostRequest suspension — auto-confirmed by DebugHost.
                    // The tick loop will process the auto-confirm on next iteration.
                }

                Some(TaskState::Cancelled) => {
                    // Check if this was a runtime crash (unwrap on None, etc.).
                    // In debug mode, halt so the user can inspect the crash message
                    // instead of the debug session abruptly disappearing.
                    if let Some(crash) = runtime.crash_info(task_id) {
                        let crash_msg = crash.message.clone();
                        // Send crash message as output event first.
                        let _ = self.server.send_event(Event::Output(events::OutputEventBody {
                            output: format!("Runtime crash: {}\n", crash_msg),
                            category: Some(types::OutputEventCategory::Stderr),
                            ..Default::default()
                        }));
                        // Send stopped event with exception reason so the user can inspect state.
                        let thread_id = self.task_id.map(|t| t.index as i64).unwrap_or(0);
                        let _ = self.server.send_event(Event::Stopped(events::StoppedEventBody {
                            reason: types::StoppedEventReason::Exception,
                            description: Some(crash_msg.clone()),
                            thread_id: Some(thread_id),
                            preserve_focus_hint: None,
                            text: Some(crash_msg),
                            all_threads_stopped: Some(true),
                            hit_breakpoint_ids: None,
                        }));
                        return;
                    }
                    // Non-crash cancellation: terminate normally.
                    let _ = self.server.send_event(Event::Terminated(Some(
                        events::TerminatedEventBody { restart: None },
                    )));
                    let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                        exit_code: 0,
                    }));
                    return;
                }

                Some(TaskState::Completed) => {
                    let _ = self.server.send_event(Event::Terminated(Some(
                        events::TerminatedEventBody { restart: None },
                    )));
                    let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                        exit_code: 0,
                    }));
                    return;
                }

                _ => {}
            }

            // Check overall tick result.
            match tick_result {
                TickResult::AllCompleted | TickResult::Empty => {
                    let _ = self.server.send_event(Event::Terminated(Some(
                        events::TerminatedEventBody { restart: None },
                    )));
                    let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                        exit_code: 0,
                    }));
                    return;
                }
                TickResult::TasksSuspended(_) => {
                    // All tasks are suspended on HostRequests. DebugHost auto-confirms them
                    // via on_request, so confirm them now and continue.
                    // In practice, DebugHost returns the response synchronously, so tasks
                    // should not end up in TasksSuspended for long.
                    // Give the scheduler one more tick to process them.
                    continue;
                }
                TickResult::ExecutionLimitReached => {
                    // More work to do — continue ticking.
                    continue;
                }
            }
        }
    }

    /// Resolve a DAP thread_id to a TaskId by matching against active tasks.
    pub(super) fn resolve_task_id(&self, thread_id: i64) -> Option<TaskId> {
        let runtime = self.runtime.as_ref()?;
        runtime.all_task_ids().into_iter().find(|t| t.index as i64 == thread_id)
    }

    /// Resolve a thread_id to a TaskId, including crashed (Cancelled) tasks.
    ///
    /// Falls back to `self.task_id` when the task is Cancelled with crash info —
    /// `all_task_ids()` excludes Cancelled tasks, so scopes/variables handlers would
    /// silently return empty without this fallback.
    fn resolve_task_id_or_crashed(&self, thread_id: i64) -> Option<TaskId> {
        // First try active tasks via the normal path.
        if let Some(id) = self.resolve_task_id(thread_id) {
            return Some(id);
        }
        // Fallback: check if self.task_id matches the requested thread and has crash info.
        if let Some(task_id) = self.task_id {
            if task_id.index as i64 == thread_id {
                if let Some(rt) = self.runtime.as_ref() {
                    if rt.crash_info(task_id).is_some() {
                        return Some(task_id);
                    }
                }
            }
        }
        None
    }

    /// Count active (in-scope) locals for a given display frame.
    pub(super) fn count_active_locals(&self, task_idx: u32, display_frame_idx: u32) -> usize {
        let task_id = match self.resolve_task_id_or_crashed(task_idx as i64) {
            Some(id) => id,
            None => return 0,
        };
        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => return 0,
        };
        let module = match self.module.as_ref() {
            Some(m) => m,
            None => return 0,
        };
        let display_frame_idx = display_frame_idx as usize;

        // Primary path: active call stack.
        if let Some(frames) = runtime.call_stack_frames(task_id)
            && !frames.is_empty()
        {
            if display_frame_idx >= frames.len() {
                return 0;
            }
            let actual_idx = frames.len() - 1 - display_frame_idx;
            let (method_idx, pc) = match frames.get(actual_idx) {
                Some(&f) => f,
                None => return 0,
            };
            let byte_pc = instr_to_byte_pc(runtime, method_idx, pc);
            let body = match module.method_bodies.get(method_idx) {
                Some(b) => b,
                None => return 0,
            };
            return body.debug_locals.iter()
                .filter(|dl| dl.start_pc <= byte_pc && byte_pc < dl.end_pc)
                .count();
        }

        // Crash fallback: use preserved registers from CrashInfo.stack_trace.
        // Crash frames are already in top-to-bottom order so display_frame_idx
        // indexes directly (no reversal needed).
        if let Some(crash) = runtime.crash_info(task_id) {
            let crash_frame = match crash.stack_trace.get(display_frame_idx) {
                Some(f) => f,
                None => return 0,
            };
            let byte_pc = instr_to_byte_pc(runtime, crash_frame.method_idx, crash_frame.pc);
            let body = match module.method_bodies.get(crash_frame.method_idx) {
                Some(b) => b,
                None => return 0,
            };
            return body.debug_locals.iter()
                .filter(|dl| dl.start_pc <= byte_pc && byte_pc < dl.end_pc)
                .count();
        }

        0
    }

    /// Get variable list for a given (task_idx, display_frame_idx).
    pub(super) fn get_variables(&self, task_idx: u32, display_frame_idx: u32) -> Vec<types::Variable> {
        let task_id = match self.resolve_task_id_or_crashed(task_idx as i64) {
            Some(id) => id,
            None => return vec![],
        };
        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => return vec![],
        };
        let module = match self.module.as_ref() {
            Some(m) => m,
            None => return vec![],
        };
        let display_frame_idx = display_frame_idx as usize;

        // Primary path: active call stack.
        if let Some(frames) = runtime.call_stack_frames(task_id)
            && !frames.is_empty()
        {
            if display_frame_idx >= frames.len() {
                return vec![];
            }
            let actual_idx = frames.len() - 1 - display_frame_idx;
            let (method_idx, pc) = match frames.get(actual_idx) {
                Some(&f) => f,
                None => return vec![],
            };
            let byte_pc = instr_to_byte_pc(runtime, method_idx, pc);
            let regs = match runtime.frame_registers(task_id, actual_idx) {
                Some(r) => r,
                None => return vec![],
            };
            return collect_frame_variables(module, method_idx, byte_pc as usize, &regs, runtime.heap());
        }

        // Crash fallback: use preserved registers from CrashInfo.stack_trace.
        // Crash frames are already in top-to-bottom order so display_frame_idx
        // indexes directly (no reversal needed).
        if let Some(crash) = runtime.crash_info(task_id) {
            let crash_frame = match crash.stack_trace.get(display_frame_idx) {
                Some(f) => f,
                None => return vec![],
            };
            let byte_pc = instr_to_byte_pc(runtime, crash_frame.method_idx, crash_frame.pc);
            return collect_frame_variables(
                module,
                crash_frame.method_idx,
                byte_pc as usize,
                &crash_frame.registers,
                runtime.heap(),
            );
        }

        vec![]
    }

    /// Evaluate an expression (local variable name lookup) in the given frame.
    pub(super) fn do_evaluate(&self, expr: &str, task_idx: u32, display_frame_idx: u32) -> (String, Option<String>) {
        let task_id = match self.resolve_task_id_or_crashed(task_idx as i64) {
            Some(id) => id,
            None => return ("unavailable".into(), None),
        };
        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => return ("unavailable".into(), None),
        };
        let module = match self.module.as_ref() {
            Some(m) => m,
            None => return ("unavailable".into(), None),
        };
        let display_frame_idx = display_frame_idx as usize;

        // Primary path: active call stack.
        if let Some(frames) = runtime.call_stack_frames(task_id)
            && !frames.is_empty()
        {
            if display_frame_idx >= frames.len() {
                return ("unavailable".into(), None);
            }
            let actual_idx = frames.len() - 1 - display_frame_idx;
            let (method_idx, pc) = match frames.get(actual_idx) {
                Some(&f) => f,
                None => return ("unavailable".into(), None),
            };
            let byte_pc = instr_to_byte_pc(runtime, method_idx, pc);
            let regs = match runtime.frame_registers(task_id, actual_idx) {
                Some(r) => r,
                None => return ("unavailable".into(), None),
            };
            return evaluate_local(module, method_idx, byte_pc as usize, &regs, runtime.heap(), expr);
        }

        // Crash fallback: use preserved registers from CrashInfo.stack_trace.
        if let Some(crash) = runtime.crash_info(task_id) {
            let crash_frame = match crash.stack_trace.get(display_frame_idx) {
                Some(f) => f,
                None => return ("unavailable".into(), None),
            };
            let byte_pc = instr_to_byte_pc(runtime, crash_frame.method_idx, crash_frame.pc);
            return evaluate_local(
                module,
                crash_frame.method_idx,
                byte_pc as usize,
                &crash_frame.registers,
                runtime.heap(),
                expr,
            );
        }

        ("unavailable".into(), None)
    }

    /// Build DAP stack frames from a specific task's call stack.
    ///
    /// Frames are ordered top-to-bottom (top of stack = index 0).
    /// Frame IDs are globally unique: task_idx * 10000 + display_frame_index.
    pub(super) fn build_stack_frames(&self, for_task_id: TaskId) -> Vec<types::StackFrame> {
        let task_id = for_task_id;
        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => return vec![],
        };
        let module = match self.module.as_ref() {
            Some(m) => m,
            None => return vec![],
        };
        // Use the first source file path as fallback for all frames.
        // True per-frame source file attribution requires FileId in SourceSpan
        // (not available in current module format -- deferred to future phase).
        let source_path = self.source_paths.first()
            .map(|(_, p)| p.as_str())
            .unwrap_or("");
        let filename = std::path::Path::new(source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(source_path);

        // Determine which frames to display and whether they are already in
        // top-to-bottom order (crash frames) or bottom-to-top (call stack frames).
        let (raw_frames, already_reversed) = match runtime.call_stack_frames(task_id) {
            Some(f) if !f.is_empty() => (f, false),
            _ => {
                // Call stack is empty (crashed task has unwound frames).
                // Fall back to CrashInfo.stack_trace if available.
                if let Some(crash) = runtime.crash_info(task_id) {
                    let crash_frames: Vec<(usize, usize)> = crash
                        .stack_trace
                        .iter()
                        .map(|sf| (sf.method_idx, sf.pc))
                        .collect();
                    (crash_frames, true)
                } else {
                    return vec![];
                }
            }
        };

        // Build an iterator in top-to-bottom order.
        // call_stack_frames is bottom-to-top so we .rev(); crash frames are already top-to-bottom.
        let frames_iter: Box<dyn Iterator<Item = (usize, (usize, usize))>> = if already_reversed {
            Box::new(raw_frames.into_iter().enumerate())
        } else {
            Box::new(raw_frames.into_iter().rev().enumerate())
        };

        frames_iter
            .map(|(frame_index, (method_idx, pc))| {
                // Resolve method name from string heap.
                let method_name = module
                    .method_defs
                    .get(method_idx)
                    .and_then(|def| read_string(&module.string_heap, def.name).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("method_{}", method_idx));

                // Translate instruction-index PC to byte-offset PC for span lookup.
                let byte_pc = instr_to_byte_pc(runtime, method_idx, pc);

                // Resolve source location: find largest span.pc <= byte_pc.
                let (line, col) = module
                    .method_bodies
                    .get(method_idx)
                    .and_then(|body| {
                        body.source_spans
                            .iter()
                            .filter(|span| span.pc <= byte_pc)
                            .max_by_key(|span| span.pc)
                            .map(|span| (span.line as i64, span.column as i64))
                    })
                    .unwrap_or((0, 0));

                let source = types::Source {
                    path: Some(source_path.to_string()),
                    name: Some(filename.to_string()),
                    ..Default::default()
                };

                // Globally unique frame ID: task_idx * 10000 + display_frame_index
                let task_idx = task_id.index;
                let frame_id = (task_idx as i64) * 10000 + frame_index as i64;

                types::StackFrame {
                    id: frame_id,
                    name: method_name,
                    source: Some(source),
                    line,
                    column: col,
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Get the current (line, method_idx) from the task's suspend reason.
    ///
    /// Used by step commands to establish the origin position before stepping.
    /// Returns (0, 0) if unavailable.
    pub(super) fn current_position(&self) -> (u32, u32) {
        let task_id = match self.task_id {
            Some(id) => id,
            None => return (0, 0),
        };
        let runtime = match self.runtime.as_ref() {
            Some(rt) => rt,
            None => return (0, 0),
        };
        match runtime.suspend_reason(task_id) {
            Some(SuspendReason::Breakpoint { line, method_idx, .. }) => (*line, *method_idx),
            Some(SuspendReason::DebugStep { line, method_idx, .. }) => (*line, *method_idx),
            _ => {
                // No suspend reason — use top of call stack if available.
                if let Some(frames) = runtime.call_stack_frames(task_id)
                    && let Some(&(method_idx, pc)) = frames.last() {
                        let byte_pc = instr_to_byte_pc(runtime, method_idx, pc);
                        let line = self.module.as_ref()
                            .and_then(|m| m.method_bodies.get(method_idx))
                            .and_then(|body| {
                                body.source_spans.iter()
                                    .filter(|s| s.pc <= byte_pc)
                                    .max_by_key(|s| s.pc)
                                    .map(|s| s.line)
                            })
                            .unwrap_or(0);
                        return (line, method_idx as u32);
                    }
                (0, 0)
            }
        }
    }
}
