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

    /// Create a new call frame acquiring the register Vec from a pool.
    ///
    /// Prefer this over `new` in hot-path call dispatch to eliminate per-call
    /// heap allocation for recursive functions.
    pub fn with_pool(
        pool: &mut RegisterPool,
        method_idx: usize,
        reg_count: usize,
        return_register: u16,
    ) -> Self {
        Self {
            method_idx,
            pc: 0,
            registers: pool.acquire(reg_count),
            defer_stack: Vec::new(),
            return_register,
        }
    }
}

/// Maximum number of `Vec<Value>` entries retained in the free-list.
const POOL_CAP: usize = 64;

/// Free-list pool of `Vec<Value>` allocations for reuse across call frames.
///
/// On each function return the caller releases the register Vec back into the
/// pool. The next call of equal or smaller register count reuses the allocation,
/// eliminating per-call heap allocation for recursive workloads (FRAME-01 through
/// FRAME-04, FRAME-06).
///
/// All registers are guaranteed to contain `Value::Void` after `acquire` returns,
/// whether the Vec was freshly allocated or pulled from the free-list (FRAME-03,
/// FRAME-06).
pub struct RegisterPool {
    free_list: Vec<Vec<Value>>,
}

impl RegisterPool {
    /// Create a new, empty pool.
    pub fn new() -> Self {
        Self {
            free_list: Vec::new(),
        }
    }

    /// Acquire a register Vec of `reg_count` slots, all initialised to `Value::Void`.
    ///
    /// Scans the free-list from the back for the first entry whose capacity is
    /// sufficient. If found, that entry is removed via `swap_remove`, resized to
    /// `reg_count`, and returned. Otherwise a fresh `Vec` is allocated.
    #[inline]
    pub fn acquire(&mut self, reg_count: usize) -> Vec<Value> {
        // Scan free-list from back for any entry with capacity >= reg_count.
        let found = self
            .free_list
            .iter()
            .rposition(|v| v.capacity() >= reg_count);

        if let Some(idx) = found {
            let mut v = self.free_list.swap_remove(idx);
            // Resize to exactly reg_count, filling new slots with Value::Void.
            // Existing slots are already Void (enforced in release).
            v.resize(reg_count, Value::Void);
            v
        } else {
            vec![Value::Void; reg_count]
        }
    }

    /// Release a register Vec back into the pool for future reuse.
    ///
    /// Steps:
    /// 1. If the free-list is at capacity (`POOL_CAP`), drop the Vec immediately.
    /// 2. Fill all slots with `Value::Void` to drop any heap-allocated values and
    ///    prevent stale data from leaking to the next caller (FRAME-03).
    /// 3. Clear the Vec (sets `len = 0`, preserving `capacity`) so that `acquire`
    ///    can safely call `resize` from an empty base.
    /// 4. Push to the free-list.
    #[inline]
    pub fn release(&mut self, mut v: Vec<Value>) {
        if self.free_list.len() >= POOL_CAP {
            // Free-list is full; drop v to enforce the cap (FRAME-04).
            return;
        }
        // Drop values held in registers (FRAME-03 safety).
        v.fill(Value::Void);
        // Reset length while preserving capacity for the next acquire.
        v.clear();
        self.free_list.push(v);
    }
}

impl Default for RegisterPool {
    fn default() -> Self {
        Self::new()
    }
}
