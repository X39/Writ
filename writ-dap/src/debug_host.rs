//! DebugHost: RuntimeHost implementation for the DAP server.
//!
//! Bridges the VM's instruction-level hooks to the DAP step/breakpoint model.
//! The `DebugHost` is used by the DAP server to intercept execution and
//! notify the debug client when a breakpoint or step completes.

use std::collections::HashMap;
use writ_module::{heap::read_string, module::Module};
use writ_runtime::{
    DebugAction, GcStats, HostRequest, HostResponse, LogLevel, RequestId, RuntimeHost, TaskId,
    Value,
};
use crate::breakpoints::BreakpointTable;

/// Why the debugger stopped execution.
#[derive(Debug, Clone)]
pub enum StopReason {
    /// A breakpoint was hit. Contains the breakpoint id.
    Breakpoint(u32),
    /// A step (over/into/out) completed.
    Step,
    /// Initial launch stop (stop on entry).
    Entry,
    /// User-requested pause.
    Pause,
    /// Runtime crash (e.g., unwrap on None, division by zero).
    /// Contains the crash message for display in the debug console.
    Exception(String),
}

/// Stepping mode state machine.
///
/// Controls how `before_instruction` decides whether to stop at the current
/// instruction.
#[derive(Debug, Clone)]
pub enum StepMode {
    /// No stepping active — only breakpoints can stop execution.
    None,
    /// Stop at the next line at the same or lower call depth, different from origin.
    StepOver {
        origin_depth: usize,
        origin_line: u32,
        origin_method: u32,
    },
    /// Stop at the next line in any method (including callees), different from origin.
    StepInto {
        origin_line: u32,
        origin_method: u32,
    },
    /// Stop when the call depth decreases below the origin depth (i.e., after return).
    StepOut {
        origin_depth: usize,
    },
}

/// RuntimeHost implementation for the DAP debug server.
///
/// Intercepts `before_instruction` calls from the VM to implement stepping
/// and breakpoint logic. When a stop condition is met, `pending_stop` is set
/// to the stop reason and `DebugAction::Break` is returned.
pub struct DebugHost {
    /// Active breakpoint table (built from the compiled module).
    pub breakpoints: BreakpointTable,
    /// Current stepping mode.
    step_mode: StepMode,
    /// Per-task call depth tracker (updated by on_function_enter/exit).
    call_depths: HashMap<TaskId, usize>,
    /// Pending stop reason — set when execution should pause.
    /// The DAP server takes this via `take_pending_stop()` to send an event.
    pub pending_stop: Option<StopReason>,
    /// Whether debug hooks are active.
    debug_active: bool,
    /// Extern function name table (from Module's ExternDef rows), for log routing.
    extern_names: Vec<String>,
    /// Buffered log messages to drain via DAP Event::Output.
    pub log_messages: Vec<(LogLevel, String)>,
    /// Suppress breakpoint re-hit at this (method_idx, byte_pc) on the next
    /// `before_instruction` call. Set when a breakpoint fires so that after
    /// resume the same breakpoint doesn't immediately re-trigger (the runtime
    /// returns DebugSuspend before advancing the PC).
    suppress_breakpoint_at: Option<(u32, u32)>,
}

impl DebugHost {
    /// Create a new DebugHost with the given breakpoint table and module for extern name resolution.
    pub fn new(breakpoints: BreakpointTable, module: &Module) -> Self {
        let extern_names = module
            .extern_defs
            .iter()
            .map(|ed| {
                read_string(&module.string_heap, ed.name)
                    .unwrap_or("?")
                    .to_string()
            })
            .collect();
        DebugHost {
            breakpoints,
            step_mode: StepMode::None,
            call_depths: HashMap::new(),
            pending_stop: None,
            debug_active: true,
            extern_names,
            log_messages: Vec::new(),
            suppress_breakpoint_at: None,
        }
    }

    /// Resolve an extern_idx metadata token to a function name.
    ///
    /// Token layout: bits 31-24 = table_id (16 for ExternDef), bits 23-0 = 1-based row.
    fn resolve_extern_name(&self, extern_idx: u32) -> &str {
        let row_1based = (extern_idx & 0x00FF_FFFF) as usize;
        if row_1based == 0 {
            return "?";
        }
        let idx = row_1based - 1; // convert to 0-based
        self.extern_names.get(idx).map(|s| s.as_str()).unwrap_or("?")
    }

    /// Drain buffered log messages (to be sent as DAP Event::Output).
    pub fn drain_log_messages(&mut self) -> Vec<(LogLevel, String)> {
        std::mem::take(&mut self.log_messages)
    }

    /// Set the stepping mode directly (for advanced callers).
    pub fn set_step_mode(&mut self, mode: StepMode) {
        self.step_mode = mode;
    }

    /// Set StepOver mode using the current position.
    ///
    /// The DAP server calls this when the user issues a "next" (step over) command.
    pub fn set_step_over(&mut self, task_id: TaskId, current_line: u32, current_method: u32) {
        let depth = self.current_depth(task_id);
        self.step_mode = StepMode::StepOver {
            origin_depth: depth,
            origin_line: current_line,
            origin_method: current_method,
        };
    }

    /// Set StepInto mode.
    ///
    /// The DAP server calls this when the user issues a "stepIn" command.
    pub fn set_step_into(&mut self, current_line: u32, current_method: u32) {
        self.step_mode = StepMode::StepInto {
            origin_line: current_line,
            origin_method: current_method,
        };
    }

    /// Set StepOut mode using the current call depth.
    ///
    /// The DAP server calls this when the user issues a "stepOut" command.
    pub fn set_step_out(&mut self, task_id: TaskId) {
        let depth = self.current_depth(task_id);
        self.step_mode = StepMode::StepOut {
            origin_depth: depth,
        };
    }

    /// Reset to no stepping (Continue mode).
    pub fn clear_step(&mut self) {
        self.step_mode = StepMode::None;
    }

    /// Take and return the pending stop reason.
    ///
    /// Returns `None` if no stop is pending. Clears the pending stop.
    pub fn take_pending_stop(&mut self) -> Option<StopReason> {
        self.pending_stop.take()
    }

    /// Get the current call depth for a task.
    pub fn current_depth(&self, task_id: TaskId) -> usize {
        self.call_depths.get(&task_id).copied().unwrap_or(0)
    }
}

impl RuntimeHost for DebugHost {
    fn debug_enabled(&self) -> bool {
        self.debug_active
    }

    fn before_instruction(
        &mut self,
        task_id: TaskId,
        method_idx: u32,
        pc: u32,
        source_line: u32,
        _source_col: u16,
    ) -> DebugAction {
        // 1. Check breakpoints first (they take priority over step mode).
        //    Skip the check if we just resumed from this exact breakpoint position
        //    (the runtime returns DebugSuspend before advancing the PC, so without
        //    suppression the same breakpoint would re-fire immediately).
        if self.suppress_breakpoint_at == Some((method_idx, pc)) {
            self.suppress_breakpoint_at = None;
        } else if let Some(bp_id) = self.breakpoints.lookup(method_idx as usize, pc) {
            self.suppress_breakpoint_at = Some((method_idx, pc));
            self.pending_stop = Some(StopReason::Breakpoint(bp_id));
            return DebugAction::Break;
        }

        // 2. Check stepping mode.
        let depth = self.current_depth(task_id);

        match &self.step_mode {
            StepMode::None => {}

            StepMode::StepOver {
                origin_depth,
                origin_line,
                origin_method,
            } => {
                // Stop when:
                // - we are at the same or lower call depth (not inside a callee), AND
                // - the current line or method differs from where we started.
                if depth <= *origin_depth
                    && (source_line != *origin_line || method_idx != *origin_method)
                {
                    self.pending_stop = Some(StopReason::Step);
                    return DebugAction::Break;
                }
            }

            StepMode::StepInto {
                origin_line,
                origin_method,
            } => {
                // Stop at any line change (including in callees).
                if source_line != *origin_line || method_idx != *origin_method {
                    self.pending_stop = Some(StopReason::Step);
                    return DebugAction::Break;
                }
            }

            StepMode::StepOut { origin_depth } => {
                // Stop when we have returned from the origin frame.
                if depth < *origin_depth {
                    self.pending_stop = Some(StopReason::Step);
                    return DebugAction::Break;
                }
            }
        }

        DebugAction::Continue
    }

    fn on_function_enter(&mut self, task_id: TaskId, _method_idx: u32) {
        let depth = self.call_depths.entry(task_id).or_insert(0);
        *depth += 1;
    }

    fn on_function_exit(&mut self, task_id: TaskId, _method_idx: u32) {
        let depth = self.call_depths.entry(task_id).or_insert(0);
        *depth = depth.saturating_sub(1);
    }

    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        // Auto-confirm all game-host requests with default values.
        // The DAP server is not a real game host — it just needs execution to proceed.
        match req {
            HostRequest::ExternCall { extern_idx, display_args, .. } => {
                let name = self.resolve_extern_name(*extern_idx);
                match name {
                    "say" | "say_localized" => HostResponse::Value(Value::Void),
                    "choice" => HostResponse::Value(Value::Int(0)),
                    "log::trace" => {
                        let msg = display_args.first().cloned().unwrap_or_default();
                        self.on_log(LogLevel::Trace, &msg);
                        HostResponse::Value(Value::Void)
                    }
                    "log::debug" => {
                        let msg = display_args.first().cloned().unwrap_or_default();
                        self.on_log(LogLevel::Debug, &msg);
                        HostResponse::Value(Value::Void)
                    }
                    "log::info" => {
                        let msg = display_args.first().cloned().unwrap_or_default();
                        self.on_log(LogLevel::Info, &msg);
                        HostResponse::Value(Value::Void)
                    }
                    "log::warn" => {
                        let msg = display_args.first().cloned().unwrap_or_default();
                        self.on_log(LogLevel::Warn, &msg);
                        HostResponse::Value(Value::Void)
                    }
                    "log::error" => {
                        let msg = display_args.first().cloned().unwrap_or_default();
                        self.on_log(LogLevel::Error, &msg);
                        HostResponse::Value(Value::Void)
                    }
                    _ => HostResponse::Value(Value::Void),
                }
            }
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, level: LogLevel, message: &str) {
        self.log_messages.push((level, message.to_string()));
    }

    fn on_gc_complete(&mut self, _stats: &GcStats) {}
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::module::{MethodBody, Module, SourceSpan};

    fn task(index: u32) -> TaskId {
        TaskId::new(index, 0)
    }

    fn make_module_with_spans(spans: &[(usize, u32, u32)]) -> Module {
        let max_method = spans.iter().map(|(m, _, _)| *m).max().map(|m| m + 1).unwrap_or(0);
        let mut method_bodies: Vec<MethodBody> = (0..max_method)
            .map(|_| MethodBody {
                register_types: vec![],
                code: vec![],
                debug_locals: vec![],
                source_spans: vec![],
            })
            .collect();
        for &(method_idx, line, pc) in spans {
            method_bodies[method_idx].source_spans.push(SourceSpan { pc, line, column: 0 });
        }
        let mut module = Module::new();
        module.method_bodies = method_bodies;
        module
    }

    fn make_host(spans: &[(usize, u32, u32)]) -> DebugHost {
        let module = make_module_with_spans(spans);
        let table = BreakpointTable::new(&module);
        DebugHost::new(table, &module)
    }

    // ─── debug_enabled ────────────────────────────────────────────────────────

    #[test]
    fn test_debug_enabled_returns_true() {
        let host = make_host(&[]);
        assert!(host.debug_enabled(), "DebugHost.debug_enabled() must return true");
    }

    // ─── Breakpoint tests ─────────────────────────────────────────────────────

    #[test]
    fn test_breakpoint_hit() {
        let mut host = make_host(&[(0, 10, 5)]);
        host.breakpoints.set_breakpoints(&[10]);

        let action = host.before_instruction(task(0), 0, 5, 10, 0);
        assert_eq!(action, DebugAction::Break, "should break on breakpoint");
        assert!(host.pending_stop.is_some(), "pending_stop should be set");
        match host.take_pending_stop() {
            Some(StopReason::Breakpoint(_)) => {}
            other => panic!("expected Breakpoint stop reason, got {:?}", other),
        }
    }

    #[test]
    fn test_no_breakpoint_miss() {
        let mut host = make_host(&[(0, 10, 5)]);
        host.breakpoints.set_breakpoints(&[10]);

        // Different pc — should not hit
        let action = host.before_instruction(task(0), 0, 6, 10, 0);
        assert_eq!(action, DebugAction::Continue, "should not break at wrong pc");
        assert!(host.pending_stop.is_none(), "no pending stop expected");
    }

    // ─── StepOver tests ───────────────────────────────────────────────────────

    #[test]
    fn test_step_over_same_depth() {
        // StepOver from line 10 at depth 0 should stop at line 20 at depth 0.
        let mut host = make_host(&[]);
        host.set_step_over(task(0), 10, 0);

        // Same line — should NOT stop.
        let a = host.before_instruction(task(0), 0, 0, 10, 0);
        assert_eq!(a, DebugAction::Continue, "should not stop at origin line");

        // Different line at same depth — should stop.
        let a = host.before_instruction(task(0), 0, 0, 20, 0);
        assert_eq!(a, DebugAction::Break, "should stop at different line, same depth");
    }

    #[test]
    fn test_step_over_same_line_skip() {
        // Multiple instructions on the same line should NOT cause a stop.
        let mut host = make_host(&[]);
        host.set_step_over(task(0), 10, 0);

        // Multiple instructions on origin line.
        let a = host.before_instruction(task(0), 0, 1, 10, 0);
        assert_eq!(a, DebugAction::Continue);
        let a = host.before_instruction(task(0), 0, 2, 10, 0);
        assert_eq!(a, DebugAction::Continue);

        // Next line — should stop.
        let a = host.before_instruction(task(0), 0, 3, 11, 0);
        assert_eq!(a, DebugAction::Break);
    }

    #[test]
    fn test_step_over_skips_deeper() {
        // StepOver should NOT stop at lines inside a called function (deeper depth).
        let mut host = make_host(&[]);
        host.set_step_over(task(0), 10, 0);

        // Simulate entering a callee.
        host.on_function_enter(task(0), 1);

        // At deeper depth — should NOT stop, even on a different line.
        let a = host.before_instruction(task(0), 1, 0, 20, 0);
        assert_eq!(a, DebugAction::Continue, "should skip lines in callee");

        // Return from callee.
        host.on_function_exit(task(0), 1);

        // Back at origin depth, different line — should stop.
        let a = host.before_instruction(task(0), 0, 5, 15, 0);
        assert_eq!(a, DebugAction::Break, "should stop after returning from callee");
    }

    // ─── StepInto tests ───────────────────────────────────────────────────────

    #[test]
    fn test_step_into_stops_at_callee() {
        // StepInto should stop at the first instruction in the callee (method 1, line 20).
        let mut host = make_host(&[]);
        host.set_step_into(10, 0); // origin: line 10, method 0

        // Different method/line — stop immediately.
        let a = host.before_instruction(task(0), 1, 0, 20, 0);
        assert_eq!(a, DebugAction::Break, "should stop at callee entry");
    }

    #[test]
    fn test_step_into_skips_origin_line() {
        // Should not stop on the origin line/method.
        let mut host = make_host(&[]);
        host.set_step_into(10, 0);

        let a = host.before_instruction(task(0), 0, 1, 10, 0);
        assert_eq!(a, DebugAction::Continue, "should not stop at origin line");
    }

    // ─── StepOut tests ────────────────────────────────────────────────────────

    #[test]
    fn test_step_out_stops_after_return() {
        // StepOut should stop when call depth decreases below the origin depth.
        let mut host = make_host(&[]);

        // Simulate being inside a function (depth 1).
        host.on_function_enter(task(0), 0);
        host.set_step_out(task(0));

        // Still inside — should not stop.
        let a = host.before_instruction(task(0), 0, 0, 10, 0);
        assert_eq!(a, DebugAction::Continue, "should not stop while still in callee");

        // Return from function.
        host.on_function_exit(task(0), 0);

        // Depth is now 0, origin was 1 — should stop.
        let a = host.before_instruction(task(0), 0, 0, 5, 0);
        assert_eq!(a, DebugAction::Break, "should stop after returning from frame");
    }

    // ─── Call depth tracking ──────────────────────────────────────────────────

    #[test]
    fn test_call_depth_tracking() {
        let mut host = make_host(&[]);
        let t = task(0);

        assert_eq!(host.current_depth(t), 0, "initial depth should be 0");

        host.on_function_enter(t, 0);
        assert_eq!(host.current_depth(t), 1);

        host.on_function_enter(t, 1);
        assert_eq!(host.current_depth(t), 2);

        host.on_function_exit(t, 1);
        assert_eq!(host.current_depth(t), 1);

        host.on_function_exit(t, 0);
        assert_eq!(host.current_depth(t), 0);
    }

    #[test]
    fn test_call_depth_saturating_underflow() {
        let mut host = make_host(&[]);
        let t = task(0);
        // Exiting without entering should not panic.
        host.on_function_exit(t, 0);
        assert_eq!(host.current_depth(t), 0, "depth should saturate at 0");
    }

    #[test]
    fn test_call_depth_independent_per_task() {
        let mut host = make_host(&[]);
        let t0 = task(0);
        let t1 = task(1);

        host.on_function_enter(t0, 0);
        host.on_function_enter(t0, 0);

        assert_eq!(host.current_depth(t0), 2);
        assert_eq!(host.current_depth(t1), 0, "tasks should have independent depths");
    }

    // ─── Exception variant ────────────────────────────────────────────────────

    #[test]
    fn test_stop_reason_exception_variant() {
        // Verify the Exception variant can be constructed and matched.
        let reason = StopReason::Exception("unwrap called on None".to_string());
        match reason {
            StopReason::Exception(msg) => assert_eq!(msg, "unwrap called on None"),
            other => panic!("expected Exception, got {:?}", other),
        }
    }

    // ─── take_pending_stop ────────────────────────────────────────────────────

    #[test]
    fn test_take_pending_stop_clears() {
        let mut host = make_host(&[(0, 10, 5)]);
        host.breakpoints.set_breakpoints(&[10]);
        host.before_instruction(task(0), 0, 5, 10, 0);

        let first = host.take_pending_stop();
        assert!(first.is_some());
        let second = host.take_pending_stop();
        assert!(second.is_none(), "take_pending_stop should clear the value");
    }
}
