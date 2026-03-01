use thiserror::Error;

/// Runtime errors that can occur during module loading or execution.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("module load error: {0}")]
    LoadError(String),

    #[error("invalid instruction at method {method_idx} offset {offset}: {detail}")]
    DecodeError {
        method_idx: usize,
        offset: usize,
        detail: String,
    },

    #[error("execution error: {0}")]
    ExecutionError(String),
}

/// Information about a task crash, including stack trace.
#[derive(Debug, Clone)]
pub struct CrashInfo {
    pub message: String,
    pub stack_trace: Vec<StackFrame>,
}

/// A single frame in a crash stack trace.
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub method_idx: usize,
    pub method_name: String,
    pub pc: usize,
}

/// Error returned by host when a request fails.
#[derive(Debug, Clone)]
pub enum HostError {
    NotSupported(String),
    Failed(String),
}
