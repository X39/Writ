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
            return_value: None,
            crash_info: None,
            atomic_depth: 0,
            instructions_executed: 0,
            suspend_count: 0,
            atomic_locks: Vec::new(),
        }
    }
}
