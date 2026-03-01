use crate::heap::HeapObject;
use crate::host::{LogLevel, RequestId};
use crate::value::Value;

use super::{helpers, intrinsics, DispatchKey, DispatchTarget, ExecContext, ExecutionResult};

pub(super) fn exec_call(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let method_idx = match super::decode_method_token(method_idx) {
        Some(idx) => idx,
        None => return ExecutionResult::Crash("call to null method token".into()),
    };
    if method_idx >= module.decoded_bodies.len() {
        return ExecutionResult::Crash(format!("call to invalid method index {}", method_idx));
    }
    let reg_count = module.module.method_bodies[method_idx].register_types.len();

    // Collect args from caller frame
    let mut args = Vec::with_capacity(argc as usize);
    {
        let caller = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(caller.registers[r_base as usize + i]);
        }
    }

    let mut new_frame = crate::frame::CallFrame::new(method_idx, reg_count, r_dst);
    for (i, arg) in args.into_iter().enumerate() {
        if i < new_frame.registers.len() {
            new_frame.registers[i] = arg;
        }
    }

    ctx.task.call_stack.push(new_frame);
    ExecutionResult::Continue
}

pub(super) fn exec_call_virt(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_obj: u16,
    contract_idx: u32,
    slot: u16,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let obj_val = ctx.task.call_stack.last().unwrap().registers[r_obj as usize];

    // Determine type_key from the object value's runtime type
    let type_key = resolve_runtime_type_key(obj_val, ctx.heap, ctx.modules);

    // Resolve contract_key from the contract_idx in the current module
    let contract_key = resolve_contract_key_from_idx(contract_idx, ctx.modules, ctx.current_module_idx);

    // Derive type_args_hash from the resolved ContractDef token
    let type_args_hash = resolve_type_args_hash(contract_idx, ctx.modules, ctx.current_module_idx);
    let key = DispatchKey { type_key, contract_key, slot, type_args_hash };

    // Primary lookup: exact match including type_args_hash
    let resolved_target = ctx.dispatch_table.get(&key).or_else(|| {
        if type_args_hash == 0 {
            ctx.dispatch_table.get_any(type_key, contract_key, slot)
        } else {
            None
        }
    });

    match resolved_target {
        Some(DispatchTarget::Method { module_idx, method_idx }) => {
            let module_idx = *module_idx;
            let method_idx = *method_idx;
            let target_module = &ctx.modules[module_idx];
            if method_idx >= target_module.decoded_bodies.len() {
                return ExecutionResult::Crash(format!(
                    "CALL_VIRT: method index {} out of range in module {}",
                    method_idx, module_idx
                ));
            }
            let reg_count = target_module.module.method_bodies[method_idx].register_types.len();

            let mut args = Vec::with_capacity(argc as usize);
            {
                let caller = ctx.task.call_stack.last().unwrap();
                for i in 0..argc as usize {
                    args.push(caller.registers[r_base as usize + i]);
                }
            }

            let mut new_frame = crate::frame::CallFrame::new(method_idx, reg_count, r_dst);
            for (i, arg) in args.into_iter().enumerate() {
                if i < new_frame.registers.len() {
                    new_frame.registers[i] = arg;
                }
            }

            ctx.task.call_stack.push(new_frame);
            ExecutionResult::Continue
        }
        Some(DispatchTarget::Intrinsic(id)) => {
            let id = *id;
            intrinsics::execute_intrinsic(ctx, id, r_dst, r_obj, r_base, argc)
        }
        None => {
            ExecutionResult::Crash(format!(
                "CALL_VIRT: no implementation for type_key=0x{:08x}, contract_key=0x{:08x}, slot={}",
                type_key, contract_key, slot
            ))
        }
    }
}

pub(super) fn exec_call_extern(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    extern_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }

    // Pre-resolve args to human-readable strings before issuing HostRequest.
    let display_args: Vec<String> = args.iter().map(|v| match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Ref(href) => ctx.heap.read_string(*href)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<ref>".to_string()),
        Value::Void => "void".to_string(),
        Value::Entity(e) => format!("<entity@{}>", e.index),
    }).collect();

    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;

    let req = crate::host::HostRequest::ExternCall {
        task_id: ctx.task.id,
        extern_idx,
        args,
        display_args,
    };

    let response = ctx.host.on_request(req_id, &req);
    match response {
        crate::host::HostResponse::Value(val) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }
        crate::host::HostResponse::Confirmed => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Void;
            ExecutionResult::Continue
        }
        crate::host::HostResponse::EntityHandle(eid) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Entity(eid);
            ExecutionResult::Continue
        }
        crate::host::HostResponse::Error(e) => {
            ExecutionResult::Crash(format!("extern call failed: {:?}", e))
        }
    }
}

pub(super) fn exec_new_delegate(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_target: u16,
) -> ExecutionResult {
    let target = {
        let frame = ctx.task.call_stack.last().unwrap();
        if matches!(frame.registers[r_target as usize], Value::Void) {
            None
        } else {
            Some(frame.registers[r_target as usize])
        }
    };
    let decoded_idx = match super::decode_method_token(method_idx) {
        Some(idx) => idx,
        None => return ExecutionResult::Crash("NewDelegate: null method token".into()),
    };
    let href = ctx.heap.alloc_delegate(decoded_idx, target);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_call_indirect(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_delegate: u16,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let delegate_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_delegate as usize]);
    let (method_idx, _target) = match ctx.heap.get_object(delegate_ref) {
        Ok(HeapObject::Delegate { method_idx, target }) => (*method_idx, *target),
        _ => return ExecutionResult::Crash("CallIndirect: not a delegate".into()),
    };

    if method_idx >= module.decoded_bodies.len() {
        return ExecutionResult::Crash(format!("CallIndirect: invalid method index {}", method_idx));
    }
    let reg_count = module.module.method_bodies[method_idx].register_types.len();

    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }

    let mut new_frame = crate::frame::CallFrame::new(method_idx, reg_count, r_dst);
    for (i, arg) in args.into_iter().enumerate() {
        if i < new_frame.registers.len() {
            new_frame.registers[i] = arg;
        }
    }

    ctx.task.call_stack.push(new_frame);
    ExecutionResult::Continue
}

pub(super) fn exec_tail_call(
    ctx: &mut ExecContext<'_>,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let method_idx = match super::decode_method_token(method_idx) {
        Some(idx) => idx,
        None => return ExecutionResult::Crash("TailCall: null method token".into()),
    };
    if method_idx >= module.decoded_bodies.len() {
        return ExecutionResult::Crash(format!("TailCall: invalid method index {}", method_idx));
    }
    let reg_count = module.module.method_bodies[method_idx].register_types.len();

    // Collect args from current frame
    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }

    // Execute defers before replacing frame (LIFO order)
    while let Some(handler_pc) = ctx.task.call_stack.last_mut().unwrap().defer_stack.pop() {
        if let Err(secondary) = super::execute_defer_handler(
            ctx.task, handler_pc, ctx.modules, ctx.current_module_idx,
            ctx.dispatch_table, ctx.heap, ctx.host, ctx.globals,
            ctx.next_request_id, ctx.entity_registry,
        ) {
            ctx.host.on_log(
                LogLevel::Error,
                &format!("secondary crash in defer during tail call: {}", secondary),
            );
        }
    }

    // Replace current frame
    let current = ctx.task.call_stack.last_mut().unwrap();
    current.method_idx = method_idx;
    current.pc = 0;
    current.registers = vec![Value::Void; reg_count];
    for (i, arg) in args.into_iter().enumerate() {
        if i < current.registers.len() {
            current.registers[i] = arg;
        }
    }

    ExecutionResult::Continue
}

// ──── CALL_VIRT Helpers ───────────────────────────────────────────────

/// Resolve a runtime value to its type_key for dispatch table lookup.
pub(super) fn resolve_runtime_type_key(
    val: Value,
    heap: &dyn crate::gc::GcHeap,
    modules: &[crate::loader::LoadedModule],
) -> u32 {
    match val {
        Value::Int(_) => find_type_key_by_name(modules, 0, "Int"),
        Value::Float(_) => find_type_key_by_name(modules, 0, "Float"),
        Value::Bool(_) => find_type_key_by_name(modules, 0, "Bool"),
        Value::Ref(href) => {
            match heap.get_object(href) {
                Ok(HeapObject::String(_)) => find_type_key_by_name(modules, 0, "String"),
                Ok(HeapObject::Array { .. }) => find_type_key_by_name(modules, 0, "Array"),
                Ok(HeapObject::Boxed(inner)) => {
                    resolve_runtime_type_key(*inner, heap, modules)
                }
                _ => u32::MAX,
            }
        }
        Value::Entity(_) => find_type_key_by_name(modules, 0, "Entity"),
        Value::Void => u32::MAX,
    }
}

/// Find a type_key by name in a specific module.
pub(super) fn find_type_key_by_name(
    modules: &[crate::loader::LoadedModule],
    mod_idx: usize,
    name: &str,
) -> u32 {
    if mod_idx >= modules.len() {
        return u32::MAX;
    }
    let module = &modules[mod_idx].module;
    for (idx, td) in module.type_defs.iter().enumerate() {
        if let Ok(td_name) = writ_module::heap::read_string(&module.string_heap, td.name) {
            if td_name == name {
                return ((mod_idx as u32) << 16) | (idx as u32);
            }
        }
    }
    u32::MAX
}

/// Resolve a contract_idx (from the instruction) to a global contract_key.
pub(super) fn resolve_contract_key_from_idx(
    contract_idx: u32,
    modules: &[crate::loader::LoadedModule],
    current_module_idx: usize,
) -> u32 {
    let token = writ_module::MetadataToken(contract_idx);
    let table_id = token.table_id();
    let row = match token.row_index() {
        Some(r) => r - 1,
        None => return u32::MAX,
    };

    match table_id {
        10 => ((current_module_idx as u32) << 16) | row,
        3 => {
            if let Some(resolved) = modules[current_module_idx].resolved_refs.contracts.get(&row) {
                ((resolved.module_idx as u32) << 16) | (resolved.contractdef_idx as u32)
            } else {
                u32::MAX
            }
        }
        _ => u32::MAX,
    }
}

/// Derive the type_args_hash for CALL_VIRT dispatch from a contract_idx.
pub(super) fn resolve_type_args_hash(
    contract_idx: u32,
    modules: &[crate::loader::LoadedModule],
    current_module_idx: usize,
) -> u32 {
    if contract_idx == 0 {
        return 0;
    }

    let token = writ_module::MetadataToken(contract_idx);
    let table_id = token.table_id();
    let row = match token.row_index() {
        Some(r) => r - 1,
        None => return 0,
    };

    match table_id {
        10 => contract_idx,
        3 => {
            if let Some(resolved) = modules[current_module_idx].resolved_refs.contracts.get(&row) {
                let contractdef_row_1based = (resolved.contractdef_idx as u32) + 1;
                writ_module::MetadataToken::new(10, contractdef_row_1based).0
            } else {
                0
            }
        }
        _ => 0,
    }
}
