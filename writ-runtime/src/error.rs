use thiserror::Error;

use crate::value::Value;

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
    /// 1-based source line number (0 = unknown).
    pub line: u32,
    /// 1-based source column number (0 = unknown).
    pub column: u32,
    /// Register values preserved at crash time (before unwind). Enables variable
    /// inspection in the DAP debugger during crash halt.
    pub registers: Vec<Value>,
}

impl CrashInfo {
    /// Format the crash as a human-readable string with stack trace.
    ///
    /// Output format:
    /// ```text
    /// Runtime crash: unwrap called on None
    ///
    /// Stack trace:
    ///   at crash_here (line 3, col 5)
    ///   at main (line 7, col 3)
    /// ```
    ///
    /// If the stack trace is empty, returns just the crash message line.
    pub fn format_stacktrace(&self) -> String {
        let mut out = format!("Runtime crash: {}", self.message);
        if !self.stack_trace.is_empty() {
            out.push_str("\n\nStack trace:");
            for frame in &self.stack_trace {
                let name = if frame.method_name.is_empty() {
                    format!("method_{}", frame.method_idx)
                } else {
                    frame.method_name.clone()
                };
                if frame.line > 0 {
                    out.push_str(&format!(
                        "\n  at {} (line {}, col {})",
                        name, frame.line, frame.column,
                    ));
                } else {
                    out.push_str(&format!("\n  at {}", name));
                }
            }
        }
        out
    }
}

/// Error returned by host when a request fails.
#[derive(Debug, Clone)]
pub enum HostError {
    NotSupported(String),
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stacktrace_with_locations() {
        let crash = CrashInfo {
            message: "unwrap called on None".to_string(),
            stack_trace: vec![
                StackFrame {
                    method_idx: 0,
                    method_name: "crash_here".to_string(),
                    pc: 5,
                    line: 3,
                    column: 5,
                    registers: vec![],
                },
                StackFrame {
                    method_idx: 1,
                    method_name: "main".to_string(),
                    pc: 2,
                    line: 7,
                    column: 3,
                    registers: vec![],
                },
            ],
        };
        let output = crash.format_stacktrace();
        assert!(output.starts_with("Runtime crash: unwrap called on None"));
        assert!(output.contains("at crash_here (line 3, col 5)"));
        assert!(output.contains("at main (line 7, col 3)"));
    }

    #[test]
    fn test_format_stacktrace_empty_trace() {
        let crash = CrashInfo {
            message: "something broke".to_string(),
            stack_trace: vec![],
        };
        let output = crash.format_stacktrace();
        assert_eq!(output, "Runtime crash: something broke");
        assert!(!output.contains("Stack trace:"));
    }

    #[test]
    fn test_format_stacktrace_unknown_location() {
        let crash = CrashInfo {
            message: "error".to_string(),
            stack_trace: vec![
                StackFrame {
                    method_idx: 0,
                    method_name: "foo".to_string(),
                    pc: 0,
                    line: 0,
                    column: 0,
                    registers: vec![],
                },
            ],
        };
        let output = crash.format_stacktrace();
        assert!(output.contains("at foo"));
        assert!(!output.contains("line"));
    }

    #[test]
    fn test_format_stacktrace_empty_method_name_uses_fallback() {
        let crash = CrashInfo {
            message: "error".to_string(),
            stack_trace: vec![
                StackFrame {
                    method_idx: 3,
                    method_name: String::new(),
                    pc: 0,
                    line: 1,
                    column: 1,
                    registers: vec![],
                },
            ],
        };
        let output = crash.format_stacktrace();
        assert!(output.contains("at method_3"));
    }
}
