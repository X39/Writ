/// Assembler error with line:column diagnostics.
#[derive(Debug, Clone, thiserror::Error)]
#[error("error: {message} at line {line}, column {col}")]
pub struct AssembleError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl AssembleError {
    /// Create a new assembler error with a message and source location.
    pub fn new(message: impl Into<String>, line: u32, col: u32) -> Self {
        AssembleError {
            message: message.into(),
            line,
            col,
        }
    }
}
