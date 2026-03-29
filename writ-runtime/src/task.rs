use crate::error::CrashInfo;
use crate::frame::CallFrame;
use crate::host::{HostRequest, RequestId};
use crate::value::{TaskId, Value};

/// Task execution states per spec section 2.17.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Suspended,
    Completed,
    Cancelled,
}

/// Why a task was suspended. Enables the DAP server to distinguish
/// breakpoint pauses from host-request suspensions.
#[derive(Debug, Clone)]
pub enum SuspendReason {
    /// Suspended waiting for a host response (e.g., extern call, entity spawn).
    HostRequest(crate::host::RequestId),
    /// Suspended at a debug breakpoint.
    Breakpoint { method_idx: u32, pc: u32, line: u32, col: u16 },
    /// Suspended after a debug step completed.
    DebugStep { mode: crate::host::DebugAction, method_idx: u32, pc: u32, line: u32, col: u16 },
    /// Suspended before crash unwind so the debugger can inspect the live stack.
    /// On resume, the runtime will execute the full crash unwind (defers, cancellation).
    CrashPending { message: String },
}

/// A cooperative task in the runtime scheduler.
///
/// Each task owns a call stack of frames, tracks its lifecycle state,
/// parent-child relationships for scoped cancellation, and execution metrics.
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub call_stack: Vec<CallFrame>,
    pub parent_id: Option<TaskId>,
    pub scoped_children: Vec<TaskId>,
    pub pending_request: Option<(RequestId, HostRequest)>,
    /// Destination register for the pending request's return value.
    /// Set when a task suspends on a `HostResponse::Suspend`; used by `confirm()`
    /// to write the result into the correct register.
    pub pending_r_dst: u16,
    /// Why this task is currently suspended. Set when entering Suspended state,
    /// cleared when the task is resumed. Used by the DAP server to distinguish
    /// breakpoint pauses from host-request suspensions.
    pub suspend_reason: Option<SuspendReason>,
    pub return_value: Option<Value>,
    pub crash_info: Option<CrashInfo>,
    pub atomic_depth: u32,
    pub instructions_executed: u64,
    pub suspend_count: u32,
    pub atomic_locks: Vec<u32>,
}

impl Task {
    /// Create a new task with the given ID and an initial call frame.
    pub fn new(id: TaskId, initial_frame: CallFrame) -> Self {
        Self {
            id,
            state: TaskState::Ready,
            call_stack: vec![initial_frame],
            parent_id: None,
            scoped_children: Vec::new(),
            pending_request: None,
            pending_r_dst: 0,
            return_value: None,
            crash_info: None,
            atomic_depth: 0,
            instructions_executed: 0,
            suspend_count: 0,
            atomic_locks: Vec::new(),
            suspend_reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::CallFrame;
    use crate::host::{DebugAction, RequestId};

    fn make_task() -> Task {
        let frame = CallFrame::new(0, 4, 0);
        Task::new(TaskId::new(0, 0), frame)
    }

    #[test]
    fn task_new_has_no_suspend_reason() {
        let task = make_task();
        assert!(task.suspend_reason.is_none());
    }

    #[test]
    fn suspend_reason_host_request_can_be_constructed() {
        let reason = SuspendReason::HostRequest(RequestId(5));
        match reason {
            SuspendReason::HostRequest(RequestId(5)) => {}
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn suspend_reason_breakpoint_can_be_constructed() {
        let reason = SuspendReason::Breakpoint { method_idx: 1, pc: 10, line: 5, col: 3 };
        match reason {
            SuspendReason::Breakpoint { method_idx: 1, pc: 10, line: 5, col: 3 } => {}
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn suspend_reason_debug_step_can_be_constructed() {
        let reason = SuspendReason::DebugStep {
            mode: DebugAction::StepOver,
            method_idx: 2,
            pc: 20,
            line: 8,
            col: 0,
        };
        match reason {
            SuspendReason::DebugStep { mode: DebugAction::StepOver, method_idx: 2, .. } => {}
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn suspend_reason_crash_pending_can_be_constructed() {
        let reason = SuspendReason::CrashPending { message: "unwrap on None".into() };
        match reason {
            SuspendReason::CrashPending { ref message } if message == "unwrap on None" => {}
            _ => panic!("unexpected variant"),
        }
    }
}
