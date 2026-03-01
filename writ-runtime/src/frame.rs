use crate::value::Value;

/// A single call frame in a task's call stack.
///
/// Each executing function has one CallFrame. The frame owns the register file
/// and tracks the program counter, defer handler stack, and the caller's
/// destination register for return value delivery.
pub struct CallFrame {
    /// Index into LoadedModule.decoded_bodies (0-based).
    pub method_idx: usize,
    /// Instruction index within the decoded instruction vector (not byte offset).
    pub pc: usize,
    /// Typed register file. Sized from method_body.register_types.len() at frame creation.
    pub registers: Vec<Value>,
    /// LIFO stack of defer handler instruction indices within the same method body.
    pub defer_stack: Vec<usize>,
    /// The caller's register index where the return value should be delivered.
    pub return_register: u16,
}

impl CallFrame {
    /// Create a new call frame for the given method with `reg_count` registers.
    pub fn new(method_idx: usize, reg_count: usize, return_register: u16) -> Self {
        Self {
            method_idx,
            pc: 0,
            registers: vec![Value::Void; reg_count],
            defer_stack: Vec::new(),
            return_register,
        }
    }
}
