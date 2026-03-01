use std::collections::HashMap;

use writ_module::Instruction;

use crate::entity::EntityRegistry;
use crate::gc::GcHeap;
use crate::host::{LogLevel, RuntimeHost};
use crate::loader::LoadedModule;
use crate::task::{Task, TaskState};
use crate::value::{TaskId, Value};

mod arith;
mod calls;
mod concurrency;
mod entities;
mod helpers;
mod intrinsics;
mod objects;

// ──── Dispatch Table Types ────────────────────────────────────────────

/// Globally unique key for dispatch table lookup.
///
/// Encoded as `(type_key, contract_key, slot, type_args_hash)` where:
/// - `type_key = (module_idx << 16) | typedef_row_idx`
/// - `contract_key = (module_idx << 16) | contractdef_row_idx`
/// - `slot` is the method slot within the contract
/// - `type_args_hash` is the raw ImplDef contract token value, used to
///   distinguish generic specializations (e.g. `Into<Float>` vs `Into<String>`)
///   that share the same base ContractDef but have different compiler-generated
///   TypeRef tokens. Set to 0 for non-generic (non-specialized) lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchKey {
    pub type_key: u32,
    pub contract_key: u32,
    pub slot: u16,
    pub type_args_hash: u32,
}

/// Target of a virtual dispatch resolution.
#[derive(Debug, Clone, Copy)]
pub enum DispatchTarget {
    /// IL method body in a specific module.
    Method { module_idx: usize, method_idx: usize },
    /// Runtime-provided native implementation.
    Intrinsic(IntrinsicId),
}

/// All intrinsic method implementations from spec section 2.18.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicId {
    // Int (13)
    IntAdd, IntSub, IntMul, IntDiv, IntMod, IntNeg, IntNot,
    IntEq, IntOrd, IntBitAnd, IntBitOr, IntIntoFloat, IntIntoString,
    // Float (10)
    FloatAdd, FloatSub, FloatMul, FloatDiv, FloatMod, FloatNeg,
    FloatEq, FloatOrd, FloatIntoInt, FloatIntoString,
    // Bool (3)
    BoolEq, BoolNot, BoolIntoString,
    // String (6)
    StringAdd, StringEq, StringOrd, StringIndexChar, StringIndexRange, StringIntoString,
    // Array (4)
    ArrayIndex, ArrayIndexSet, ArraySlice, ArrayIterable,
}

/// The dispatch table for O(1) contract method resolution.
pub struct DispatchTable {
    table: HashMap<DispatchKey, DispatchTarget>,
}

impl DispatchTable {
    pub fn new() -> Self {
        DispatchTable { table: HashMap::new() }
    }

    pub fn insert(&mut self, key: DispatchKey, target: DispatchTarget) {
        self.table.insert(key, target);
    }

    /// Look up an entry by exact key (including type_args_hash).
    ///
    /// For CALL_VIRT dispatch, use the full key with type_args_hash from the instruction.
    pub fn get(&self, key: &DispatchKey) -> Option<&DispatchTarget> {
        self.table.get(key)
    }

    /// Look up an entry by (type_key, contract_key, slot), ignoring type_args_hash.
    ///
    /// Used for tests and legacy lookups where type_args_hash is not available.
    /// Returns the first matching entry (arbitrary if multiple specializations exist).
    pub fn get_any(&self, type_key: u32, contract_key: u32, slot: u16) -> Option<&DispatchTarget> {
        self.table.iter()
            .find(|(k, _)| k.type_key == type_key && k.contract_key == contract_key && k.slot == slot)
            .map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

impl Default for DispatchTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of executing a single instruction.
pub(crate) enum ExecutionResult {
    /// Instruction executed successfully, continue to next.
    Continue,
    /// Task finished (RET from bottom frame).
    Completed(Value),
    /// Task suspended waiting for host response.
    #[allow(dead_code)]
    Suspended(crate::host::RequestId),
    /// Task crashed with the given message.
    Crash(String),
    /// Execution budget exhausted.
    LimitReached,
    /// DeferEnd instruction encountered (used during defer handler execution).
    DeferComplete,
    /// Task wants to spawn a scoped child task.
    SpawnChild {
        r_dst: u16,
        method_idx: usize,
        args: Vec<Value>,
    },
    /// Task wants to spawn a detached task.
    SpawnDetachedTask {
        r_dst: u16,
        method_idx: usize,
        args: Vec<Value>,
    },
    /// Task wants to join another task.
    JoinTask { r_dst: u16, target: TaskId },
    /// Task wants to cancel another task.
    CancelTask { target: TaskId },
}

/// Execution context bundling all parameters for a single instruction dispatch.
///
/// Created in `execute_one()` after bounds checking, then passed by `&mut` to
/// each handler function in the sub-modules, avoiding repetitive 9-parameter signatures.
pub(super) struct ExecContext<'a> {
    pub task: &'a mut Task,
    pub modules: &'a [LoadedModule],
    pub current_module_idx: usize,
    pub dispatch_table: &'a DispatchTable,
    pub heap: &'a mut dyn GcHeap,
    pub host: &'a mut dyn RuntimeHost,
    pub globals: &'a mut Vec<Value>,
    pub next_request_id: &'a mut u32,
    pub entity_registry: &'a mut EntityRegistry,
}

/// Decode a MethodDef metadata token to a 0-based method body index.
///
/// The compiler emits MethodDef tokens (table_id=7, 1-based row_index) in CALL,
/// TailCall, SpawnTask, SpawnDetached, and NewDelegate instructions. The runtime
/// must strip the table_id byte and convert the 1-based row_index to a 0-based
/// index for decoded_bodies/method_bodies array access.
///
/// Token layout: bits 31-24 = table_id, bits 23-0 = row_index (1-based).
///
/// Example: 0x07000001 → table_id=7, row_index=1 → array_index=0
///           0x07000002 → table_id=7, row_index=2 → array_index=1
#[inline]
pub(super) fn decode_method_token(token: u32) -> Option<usize> {
    let row_index = token & 0x00FF_FFFF;
    if row_index == 0 {
        None // null token
    } else {
        Some(row_index as usize - 1) // convert 1-based to 0-based
    }
}

/// Execute a single instruction for the given task.
///
/// Fetches the instruction at the current frame's PC, increments PC,
/// then dispatches via exhaustive match.
pub(crate) fn execute_one(
    task: &mut Task,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
) -> ExecutionResult {
    let module = &modules[current_module_idx];

    let frame = match task.call_stack.last_mut() {
        Some(f) => f,
        None => return ExecutionResult::Crash("empty call stack".into()),
    };

    let method_idx = frame.method_idx;
    let pc = frame.pc;

    // Bounds check
    if method_idx >= module.decoded_bodies.len() {
        return ExecutionResult::Crash(format!("method index {} out of range", method_idx));
    }
    let body = &module.decoded_bodies[method_idx];
    if pc >= body.len() {
        return ExecutionResult::Crash(format!(
            "PC {} out of range for method {} ({} instructions)",
            pc,
            method_idx,
            body.len()
        ));
    }

    let instr = body[pc].clone();
    frame.pc += 1;
    task.instructions_executed += 1;

    let mut ctx = ExecContext {
        task,
        modules,
        current_module_idx,
        dispatch_table,
        heap,
        host,
        globals,
        next_request_id,
        entity_registry,
    };

    match instr {
        // ── Meta ──────────────────────────────────────────────
        Instruction::Nop => ExecutionResult::Continue,
        Instruction::Crash { r_msg } => arith::exec_crash(&mut ctx, r_msg),

        // ── Data Movement ─────────────────────────────────────
        Instruction::Mov { r_dst, r_src } => arith::exec_mov(&mut ctx, r_dst, r_src),
        Instruction::LoadInt { r_dst, value } => arith::exec_load_int(&mut ctx, r_dst, value),
        Instruction::LoadFloat { r_dst, value } => arith::exec_load_float(&mut ctx, r_dst, value),
        Instruction::LoadTrue { r_dst } => arith::exec_load_true(&mut ctx, r_dst),
        Instruction::LoadFalse { r_dst } => arith::exec_load_false(&mut ctx, r_dst),
        Instruction::LoadString { r_dst, string_idx } => arith::exec_load_string(&mut ctx, r_dst, string_idx),
        Instruction::LoadNull { r_dst } => arith::exec_load_null(&mut ctx, r_dst),

        // ── Integer Arithmetic ────────────────────────────────
        Instruction::AddI { r_dst, r_a, r_b } => arith::exec_add_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::SubI { r_dst, r_a, r_b } => arith::exec_sub_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::MulI { r_dst, r_a, r_b } => arith::exec_mul_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::DivI { r_dst, r_a, r_b } => arith::exec_div_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::ModI { r_dst, r_a, r_b } => arith::exec_mod_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::NegI { r_dst, r_src } => arith::exec_neg_i(&mut ctx, r_dst, r_src),

        // ── Float Arithmetic ──────────────────────────────────
        Instruction::AddF { r_dst, r_a, r_b } => arith::exec_add_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::SubF { r_dst, r_a, r_b } => arith::exec_sub_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::MulF { r_dst, r_a, r_b } => arith::exec_mul_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::DivF { r_dst, r_a, r_b } => arith::exec_div_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::ModF { r_dst, r_a, r_b } => arith::exec_mod_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::NegF { r_dst, r_src } => arith::exec_neg_f(&mut ctx, r_dst, r_src),

        // ── Bitwise & Logical ─────────────────────────────────
        Instruction::BitAnd { r_dst, r_a, r_b } => arith::exec_bit_and(&mut ctx, r_dst, r_a, r_b),
        Instruction::BitOr { r_dst, r_a, r_b } => arith::exec_bit_or(&mut ctx, r_dst, r_a, r_b),
        Instruction::Shl { r_dst, r_a, r_b } => arith::exec_shl(&mut ctx, r_dst, r_a, r_b),
        Instruction::Shr { r_dst, r_a, r_b } => arith::exec_shr(&mut ctx, r_dst, r_a, r_b),
        Instruction::Not { r_dst, r_src } => arith::exec_not(&mut ctx, r_dst, r_src),

        // ── Comparison ────────────────────────────────────────
        Instruction::CmpEqI { r_dst, r_a, r_b } => arith::exec_cmp_eq_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::CmpEqF { r_dst, r_a, r_b } => arith::exec_cmp_eq_f(&mut ctx, r_dst, r_a, r_b),
        Instruction::CmpEqB { r_dst, r_a, r_b } => arith::exec_cmp_eq_b(&mut ctx, r_dst, r_a, r_b),
        Instruction::CmpEqS { r_dst, r_a, r_b } => arith::exec_cmp_eq_s(&mut ctx, r_dst, r_a, r_b),
        Instruction::CmpLtI { r_dst, r_a, r_b } => arith::exec_cmp_lt_i(&mut ctx, r_dst, r_a, r_b),
        Instruction::CmpLtF { r_dst, r_a, r_b } => arith::exec_cmp_lt_f(&mut ctx, r_dst, r_a, r_b),

        // ── Control Flow ──────────────────────────────────────
        Instruction::Br { offset } => arith::exec_br(&mut ctx, offset),
        Instruction::BrTrue { r_cond, offset } => arith::exec_br_true(&mut ctx, r_cond, offset),
        Instruction::BrFalse { r_cond, offset } => arith::exec_br_false(&mut ctx, r_cond, offset),
        Instruction::Switch { r_tag, offsets } => arith::exec_switch(&mut ctx, r_tag, offsets),

        Instruction::Ret { r_src } => {
            let ret_val = ctx.task.call_stack.last().unwrap().registers[r_src as usize];
            execute_ret(ctx.task, ret_val, ctx.modules, ctx.current_module_idx,
                        ctx.dispatch_table, ctx.heap, ctx.host, ctx.globals,
                        ctx.next_request_id, ctx.entity_registry)
        }
        Instruction::RetVoid => {
            execute_ret(ctx.task, Value::Void, ctx.modules, ctx.current_module_idx,
                        ctx.dispatch_table, ctx.heap, ctx.host, ctx.globals,
                        ctx.next_request_id, ctx.entity_registry)
        }

        // ── Calls & Delegates ─────────────────────────────────
        Instruction::Call { r_dst, method_idx, r_base, argc } =>
            calls::exec_call(&mut ctx, r_dst, method_idx, r_base, argc),
        Instruction::CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc } =>
            calls::exec_call_virt(&mut ctx, r_dst, r_obj, contract_idx, slot, r_base, argc),
        Instruction::CallExtern { r_dst, extern_idx, r_base, argc } =>
            calls::exec_call_extern(&mut ctx, r_dst, extern_idx, r_base, argc),
        Instruction::NewDelegate { r_dst, method_idx, r_target } =>
            calls::exec_new_delegate(&mut ctx, r_dst, method_idx, r_target),
        Instruction::CallIndirect { r_dst, r_delegate, r_base, argc } =>
            calls::exec_call_indirect(&mut ctx, r_dst, r_delegate, r_base, argc),
        Instruction::TailCall { method_idx, r_base, argc } =>
            calls::exec_tail_call(&mut ctx, method_idx, r_base, argc),

        // ── Object Model ──────────────────────────────────────
        Instruction::New { r_dst, type_idx } => objects::exec_new(&mut ctx, r_dst, type_idx),
        Instruction::GetField { r_dst, r_obj, field_idx } =>
            objects::exec_get_field(&mut ctx, r_dst, r_obj, field_idx),
        Instruction::SetField { r_obj, field_idx, r_val } =>
            objects::exec_set_field(&mut ctx, r_obj, field_idx, r_val),

        // ── Entity Instructions ───────────────────────────────
        Instruction::SpawnEntity { r_dst, type_idx } =>
            entities::exec_spawn_entity(&mut ctx, r_dst, type_idx),
        Instruction::InitEntity { r_entity } =>
            entities::exec_init_entity(&mut ctx, r_entity),
        Instruction::DestroyEntity { r_entity } =>
            entities::exec_destroy_entity(&mut ctx, r_entity),
        Instruction::GetComponent { r_dst, r_entity, comp_type_idx } =>
            entities::exec_get_component(&mut ctx, r_dst, r_entity, comp_type_idx),
        Instruction::GetOrCreate { r_dst, type_idx } =>
            entities::exec_get_or_create(&mut ctx, r_dst, type_idx),
        Instruction::FindAll { r_dst, type_idx } =>
            entities::exec_find_all(&mut ctx, r_dst, type_idx),
        Instruction::EntityIsAlive { r_dst, r_entity } =>
            entities::exec_entity_is_alive(&mut ctx, r_dst, r_entity),

        // ── Arrays ────────────────────────────────────────────
        Instruction::NewArray { r_dst, elem_type } =>
            objects::exec_new_array(&mut ctx, r_dst, elem_type),
        Instruction::ArrayInit { r_dst, elem_type, count, r_base } =>
            objects::exec_array_init(&mut ctx, r_dst, elem_type, count, r_base),
        Instruction::ArrayLoad { r_dst, r_arr, r_idx } =>
            objects::exec_array_load(&mut ctx, r_dst, r_arr, r_idx),
        Instruction::ArrayStore { r_arr, r_idx, r_val } =>
            objects::exec_array_store(&mut ctx, r_arr, r_idx, r_val),
        Instruction::ArrayLen { r_dst, r_arr } =>
            objects::exec_array_len(&mut ctx, r_dst, r_arr),
        Instruction::ArrayAdd { r_arr, r_val } =>
            objects::exec_array_add(&mut ctx, r_arr, r_val),
        Instruction::ArrayRemove { r_arr, r_idx } =>
            objects::exec_array_remove(&mut ctx, r_arr, r_idx),
        Instruction::ArrayInsert { r_arr, r_idx, r_val } =>
            objects::exec_array_insert(&mut ctx, r_arr, r_idx, r_val),
        Instruction::ArraySlice { r_dst, r_arr, r_start, r_end } =>
            objects::exec_array_slice(&mut ctx, r_dst, r_arr, r_start, r_end),

        // ── Type Operations — Option ──────────────────────────
        Instruction::WrapSome { r_dst, r_val } => objects::exec_wrap_some(&mut ctx, r_dst, r_val),
        Instruction::Unwrap { r_dst, r_opt } => objects::exec_unwrap(&mut ctx, r_dst, r_opt),
        Instruction::IsSome { r_dst, r_opt } => objects::exec_is_some(&mut ctx, r_dst, r_opt),
        Instruction::IsNone { r_dst, r_opt } => objects::exec_is_none(&mut ctx, r_dst, r_opt),

        // ── Type Operations — Result ──────────────────────────
        Instruction::WrapOk { r_dst, r_val } => objects::exec_wrap_ok(&mut ctx, r_dst, r_val),
        Instruction::WrapErr { r_dst, r_err } => objects::exec_wrap_err(&mut ctx, r_dst, r_err),
        Instruction::UnwrapOk { r_dst, r_result } => objects::exec_unwrap_ok(&mut ctx, r_dst, r_result),
        Instruction::IsOk { r_dst, r_result } => objects::exec_is_ok(&mut ctx, r_dst, r_result),
        Instruction::IsErr { r_dst, r_result } => objects::exec_is_err(&mut ctx, r_dst, r_result),
        Instruction::ExtractErr { r_dst, r_result } => objects::exec_extract_err(&mut ctx, r_dst, r_result),

        // ── Type Operations — Enum ────────────────────────────
        Instruction::NewEnum { r_dst, type_idx, tag, field_count, r_base } =>
            objects::exec_new_enum(&mut ctx, r_dst, type_idx, tag, field_count, r_base),
        Instruction::GetTag { r_dst, r_enum } => objects::exec_get_tag(&mut ctx, r_dst, r_enum),
        Instruction::ExtractField { r_dst, r_enum, field_idx } =>
            objects::exec_extract_field(&mut ctx, r_dst, r_enum, field_idx),

        // ── Concurrency ───────────────────────────────────────
        Instruction::SpawnTask { r_dst, method_idx, r_base, argc } =>
            concurrency::exec_spawn_task(&mut ctx, r_dst, method_idx, r_base, argc),
        Instruction::SpawnDetached { r_dst, method_idx, r_base, argc } =>
            concurrency::exec_spawn_detached(&mut ctx, r_dst, method_idx, r_base, argc),
        Instruction::Join { r_dst, r_task } => concurrency::exec_join(&mut ctx, r_dst, r_task),
        Instruction::Cancel { r_task } => concurrency::exec_cancel(&mut ctx, r_task),
        Instruction::DeferPush { r_dst: _, method_idx } =>
            concurrency::exec_defer_push(&mut ctx, method_idx as usize),
        Instruction::DeferPop => concurrency::exec_defer_pop(&mut ctx),
        Instruction::DeferEnd => ExecutionResult::DeferComplete,

        // ── Globals & Atomics ─────────────────────────────────
        Instruction::LoadGlobal { r_dst, global_idx } =>
            concurrency::exec_load_global(&mut ctx, r_dst, global_idx),
        Instruction::StoreGlobal { global_idx, r_src } =>
            concurrency::exec_store_global(&mut ctx, global_idx, r_src),
        Instruction::AtomicBegin => {
            ctx.task.atomic_depth += 1;
            ExecutionResult::Continue
        }
        Instruction::AtomicEnd => concurrency::exec_atomic_end(&mut ctx),

        // ── Conversion ────────────────────────────────────────
        Instruction::I2f { r_dst, r_src } => arith::exec_i2f(&mut ctx, r_dst, r_src),
        Instruction::F2i { r_dst, r_src } => arith::exec_f2i(&mut ctx, r_dst, r_src),
        Instruction::I2s { r_dst, r_src } => arith::exec_i2s(&mut ctx, r_dst, r_src),
        Instruction::F2s { r_dst, r_src } => arith::exec_f2s(&mut ctx, r_dst, r_src),
        Instruction::B2s { r_dst, r_src } => arith::exec_b2s(&mut ctx, r_dst, r_src),
        Instruction::Convert { r_dst, r_src, .. } => arith::exec_convert(&mut ctx, r_dst, r_src),

        // ── Strings ───────────────────────────────────────────
        Instruction::StrConcat { r_dst, r_a, r_b } => arith::exec_str_concat(&mut ctx, r_dst, r_a, r_b),
        Instruction::StrBuild { r_dst, count, r_base } => arith::exec_str_build(&mut ctx, r_dst, count, r_base),
        Instruction::StrLen { r_dst, r_str } => arith::exec_str_len(&mut ctx, r_dst, r_str),

        // ── Boxing ────────────────────────────────────────────
        Instruction::Box { r_dst, r_val } => arith::exec_box(&mut ctx, r_dst, r_val),
        Instruction::Unbox { r_dst, r_boxed } => arith::exec_unbox(&mut ctx, r_dst, r_boxed),
    }
}

// ── Helper functions ──────────────────────────────────────────────

/// Execute RET: run defer handlers in LIFO order, pop frame, deliver return value.
fn execute_ret(
    task: &mut Task,
    ret_val: Value,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
) -> ExecutionResult {
    // Step 1: Run defers in LIFO order
    while let Some(handler_pc) = task.call_stack.last_mut().unwrap().defer_stack.pop() {
        if let Err(secondary) = execute_defer_handler(
            task, handler_pc, modules, current_module_idx, dispatch_table, heap, host, globals, next_request_id, entity_registry,
        ) {
            host.on_log(
                LogLevel::Error,
                &format!("secondary crash in defer: {}", secondary),
            );
        }
    }

    // Step 2: Pop frame
    let popped = task.call_stack.pop().unwrap();

    // Step 3: Deliver result
    if task.call_stack.is_empty() {
        task.state = TaskState::Completed;
        task.return_value = Some(ret_val);
        ExecutionResult::Completed(ret_val)
    } else {
        let caller = task.call_stack.last_mut().unwrap();
        // HOOK_RETURN_SINK (u16::MAX) is used by lifecycle hook frames to discard
        // the return value without writing to any caller register.
        if popped.return_register != u16::MAX {
            caller.registers[popped.return_register as usize] = ret_val;
        }
        ExecutionResult::Continue
    }
}

/// Execute a defer handler starting at `handler_pc` within the current method.
/// Runs instructions until DeferEnd is encountered.
/// Returns Ok(()) on success, Err(message) on crash (secondary crash).
pub(crate) fn execute_defer_handler(
    task: &mut Task,
    handler_pc: usize,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
) -> Result<(), String> {
    // Save current PC
    let saved_pc = task.call_stack.last().unwrap().pc;

    // Set PC to defer handler
    task.call_stack.last_mut().unwrap().pc = handler_pc;

    loop {
        let result = execute_one(task, modules, current_module_idx, dispatch_table, heap, host, globals, next_request_id, entity_registry);
        match result {
            ExecutionResult::Continue => continue,
            ExecutionResult::DeferComplete => {
                // Restore PC
                if let Some(frame) = task.call_stack.last_mut() {
                    frame.pc = saved_pc;
                }
                return Ok(());
            }
            ExecutionResult::Crash(msg) => {
                // Restore PC
                if let Some(frame) = task.call_stack.last_mut() {
                    frame.pc = saved_pc;
                }
                return Err(msg);
            }
            ExecutionResult::Completed(_) => {
                return Ok(());
            }
            _ => continue,
        }
    }
}

/// Execute crash propagation: unwind the entire call stack, running defer handlers
/// at each frame level in LIFO order. Secondary crashes are logged and swallowed.
pub(crate) fn execute_crash(
    task: &mut Task,
    msg: String,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
) {
    // Build crash info BEFORE unwinding
    let crash_info = crate::error::CrashInfo {
        message: msg.clone(),
        stack_trace: task
            .call_stack
            .iter()
            .rev()
            .map(|f| crate::error::StackFrame {
                method_idx: f.method_idx,
                method_name: String::new(),
                pc: f.pc,
            })
            .collect(),
    };

    // Unwind all frames, executing defers at each level
    while !task.call_stack.is_empty() {
        while let Some(handler_pc) = task.call_stack.last_mut().unwrap().defer_stack.pop() {
            if let Err(secondary) = execute_defer_handler(
                task, handler_pc, modules, current_module_idx, dispatch_table, heap, host, globals, next_request_id, entity_registry,
            ) {
                host.on_log(
                    LogLevel::Error,
                    &format!(
                        "secondary crash in defer during crash unwind: {}",
                        secondary
                    ),
                );
            }
        }
        task.call_stack.pop();
    }

    task.state = TaskState::Cancelled;
    task.crash_info = Some(crash_info);
    host.on_log(
        LogLevel::Error,
        &format!("task {} crashed: {}", task.id.index, msg),
    );
}
