use crate::heap::HeapObject;
use crate::host::{LogLevel, RequestId};
use crate::value::Value;

use super::{helpers, intrinsics, DispatchKey, DispatchTarget, ExecContext, ExecutionResult};

#[inline]
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

    // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
    ctx.task.call_stack.push(crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst));
    let stack_len = ctx.task.call_stack.len();
    let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
    let caller = bottom.last().unwrap();
    let callee = &mut top[0];
    // SAFETY: The compiler guarantees argc <= callee reg_count and r_base + argc <= caller
    // reg_count for every CALL instruction it emits. Both frames were sized from these
    // values at creation time, so all indices are in-bounds.
    for i in 0..argc as usize {
        unsafe {
            *callee.registers.get_unchecked_mut(i) =
                *caller.registers.get_unchecked(r_base as usize + i);
        }
    }

    if ctx.host.debug_enabled() {
        ctx.host.on_function_enter(ctx.task.id, method_idx as u32);
    }
    ExecutionResult::Continue
}

#[inline]
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

            // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
            ctx.task.call_stack.push(crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst));
            let stack_len = ctx.task.call_stack.len();
            let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
            let caller = bottom.last().unwrap();
            let callee = &mut top[0];
            // SAFETY: The compiler guarantees argc <= callee reg_count and r_base + argc <= caller
            // reg_count for every CALL instruction it emits. Both frames were sized from these
            // values at creation time, so all indices are in-bounds.
            for i in 0..argc as usize {
                unsafe {
                    *callee.registers.get_unchecked_mut(i) =
                        *caller.registers.get_unchecked(r_base as usize + i);
                }
            }

            if ctx.host.debug_enabled() {
                ctx.host.on_function_enter(ctx.task.id, method_idx as u32);
            }
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

#[inline]
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

    // Try Speaker contract dispatch for entity arguments before building display_args.
    // If an entity's type implements Speaker, speaker_name(self) is called synchronously
    // and the result used instead of the type name.
    let mut speaker_overrides: Vec<(usize, String)> = Vec::new();
    for (i, v) in args.iter().enumerate() {
        if let Value::Entity(_) = v {
            if let Some(name) = try_speaker_dispatch(
                *v,
                r_dst,
                ctx.task,
                ctx.modules,
                ctx.dispatch_table,
                ctx.heap,
                ctx.host,
                ctx.globals,
                ctx.next_request_id,
                ctx.entity_registry,
                ctx.pool,
                ctx.reflection,
            ) {
                speaker_overrides.push((i, name));
            }
        }
    }

    // Pre-resolve args to human-readable strings before issuing HostRequest.
    // Entity values use Speaker override if available, otherwise type name from TypeDefs.
    let display_args: Vec<String> = args.iter().enumerate().map(|(i, v)| {
        // Check for Speaker contract override first
        if let Some((_, name)) = speaker_overrides.iter().find(|(idx, _)| *idx == i) {
            return name.clone();
        }
        match v {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Ref(href) => ctx.heap.read_string(*href)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "<ref>".to_string()),
            Value::Void => "void".to_string(),
            Value::Entity(e) => resolve_entity_display_name(*e, ctx.entity_registry, ctx.modules),
            Value::Struct { type_idx, .. } => format!("<struct@{}>", type_idx),
        }
    }).collect();

    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;

    let req = crate::host::HostRequest::ExternCall {
        task_id: ctx.task.id,
        extern_idx,
        args,
        display_args,
    };

    // Try heap-aware dispatch first (for ImmediateWithHeap handlers)
    let response = if let Some(resp) = ctx.host.on_extern_call_with_heap(req_id, &req, ctx.heap) {
        resp
    } else {
        ctx.host.on_request(req_id, &req)
    };
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
        crate::host::HostResponse::Suspend => {
            // Park this task — the host will call Runtime::confirm() later.
            ctx.task.pending_request = Some((req_id, req));
            ctx.task.pending_r_dst = r_dst;
            ExecutionResult::Suspended(req_id)
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

#[inline]
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

    // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
    ctx.task.call_stack.push(crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst));
    let stack_len = ctx.task.call_stack.len();
    let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
    let caller = bottom.last().unwrap();
    let callee = &mut top[0];
    // SAFETY: The compiler guarantees argc <= callee reg_count and r_base + argc <= caller
    // reg_count for every CALL instruction it emits. Both frames were sized from these
    // values at creation time, so all indices are in-bounds.
    for i in 0..argc as usize {
        unsafe {
            *callee.registers.get_unchecked_mut(i) =
                *caller.registers.get_unchecked(r_base as usize + i);
        }
    }

    if ctx.host.debug_enabled() {
        ctx.host.on_function_enter(ctx.task.id, method_idx as u32);
    }
    ExecutionResult::Continue
}

#[inline]
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

    // Collect args into stack-resident buffer (no heap allocation for argc <= 32)
    const MAX_INLINE_ARGC: usize = 32;
    let argc_usize = argc as usize;
    let mut arg_buf: [Value; MAX_INLINE_ARGC] = std::array::from_fn(|_| Value::Void);
    let mut heap_args: Option<Vec<Value>> = None;
    {
        let frame = ctx.task.call_stack.last().unwrap();
        if argc_usize > MAX_INLINE_ARGC {
            let mut hv = Vec::with_capacity(argc_usize);
            for i in 0..argc_usize {
                hv.push(frame.registers[r_base as usize + i]);
            }
            heap_args = Some(hv);
        } else {
            for i in 0..argc_usize {
                arg_buf[i] = frame.registers[r_base as usize + i];
            }
        }
    }

    // Execute defers before replacing frame (LIFO order)
    while let Some(handler_pc) = ctx.task.call_stack.last_mut().unwrap().defer_stack.pop() {
        if let Err(secondary) = super::execute_defer_handler(
            ctx.task, handler_pc, ctx.modules, ctx.current_module_idx,
            ctx.dispatch_table, ctx.heap, ctx.host, ctx.globals,
            ctx.next_request_id, ctx.entity_registry, ctx.pool, ctx.reflection,
        ) {
            ctx.host.on_log(
                LogLevel::Error,
                &format!("secondary crash in defer during tail call: {}", secondary),
            );
        }
    }

    // Replace current frame in-place (reuse existing Vec allocation via clear+resize)
    let current = ctx.task.call_stack.last_mut().unwrap();
    current.method_idx = method_idx;
    current.pc = 0;
    current.registers.clear();
    current.registers.resize(reg_count, Value::Void);
    if let Some(hv) = heap_args {
        for (i, v) in hv.into_iter().enumerate() {
            if i < current.registers.len() {
                current.registers[i] = v;
            }
        }
    } else {
        for i in 0..argc_usize {
            current.registers[i] = arg_buf[i];
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
                Ok(HeapObject::Struct { type_key, .. }) => *type_key,
                Ok(HeapObject::Boxed(inner)) => {
                    resolve_runtime_type_key(*inner, heap, modules)
                }
                _ => u32::MAX,
            }
        }
        Value::Entity(_) => find_type_key_by_name(modules, 0, "Entity"),
        Value::Void => u32::MAX,
        Value::Struct { type_idx, .. } => type_idx,
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
        if let Ok(td_name) = writ_module::heap::read_string(&module.string_heap, td.name)
            && td_name == name {
                return ((mod_idx as u32) << 16) | (idx as u32);
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

/// Resolve an entity's display name, checking for Speaker contract override first.
///
/// If the entity's type implements the `Speaker` contract, executes
/// `speaker_name(self) -> string` via synchronous sub-call and uses the result.
/// Otherwise falls back to the entity's type name from the TypeDef table.
fn resolve_entity_display_name(
    entity_id: crate::value::EntityId,
    entity_registry: &crate::entity::EntityRegistry,
    modules: &[crate::loader::LoadedModule],
) -> String {
    // Look up entity's type_idx (may fail for destroyed/stale handles)
    let type_idx = match entity_registry.get_type_idx(entity_id) {
        Ok(idx) => idx,
        Err(_) => return format!("<entity@{}>", entity_id.index),
    };

    // Strip table bits and convert to 0-based index (type_idx may be a metadata token
    // with table_id in high bits, or a raw 1-based row index)
    let row = (type_idx & 0x00FF_FFFF).saturating_sub(1) as usize;

    // Search modules in reverse order (user modules first, then virtual module)
    // so that user-defined entity types are found before writ-runtime builtins.
    for loaded in modules.iter().rev() {
        let module = &loaded.module;
        if row < module.type_defs.len() {
            if let Ok(name) = writ_module::heap::read_string(&module.string_heap, module.type_defs[row].name) {
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }

    format!("<entity@{}>", entity_id.index)
}

/// Try to resolve a Speaker contract display name for an entity via synchronous sub-execution.
///
/// Looks up the Speaker contract in the dispatch table for the entity's concrete type.
/// If found, pushes a call frame for `speaker_name(self)`, runs the VM loop until
/// the frame returns, and extracts the resulting string. Returns None if the entity's
/// type does not implement Speaker, or if the sub-call fails.
#[allow(clippy::too_many_arguments)]
fn try_speaker_dispatch(
    entity_val: Value,
    r_dst: u16,
    task: &mut crate::task::Task,
    modules: &[crate::loader::LoadedModule],
    dispatch_table: &super::DispatchTable,
    heap: &mut dyn crate::gc::GcHeap,
    host: &mut dyn crate::host::RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut crate::entity::EntityRegistry,
    pool: &mut crate::frame::RegisterPool,
    reflection: &mut crate::reflection::ReflectionIndex,
) -> Option<String> {
    // Find Speaker contract key in module 0 (writ-runtime)
    let speaker_key = find_contract_key_by_name(modules, 0, "Speaker")?;

    // Find entity's concrete type key from entity registry (not the base Entity type).
    // Speaker impls are on concrete entity types (e.g. Merchant), not the base Entity.
    let entity_id = match entity_val {
        Value::Entity(eid) => eid,
        _ => return None,
    };
    let raw_type_idx = entity_registry.get_type_idx(entity_id).ok()?;
    // type_idx is a 1-based row in the user module (module 1). Convert to type_key.
    let row_0based = (raw_type_idx & 0x00FF_FFFF).saturating_sub(1);
    // User module is the last module loaded (index = modules.len() - 1)
    let user_mod_idx = modules.len().saturating_sub(1) as u32;
    let type_key = (user_mod_idx << 16) | row_0based;

    // Look up Speaker::speaker_name (slot 0) in dispatch table
    let key = super::DispatchKey {
        type_key,
        contract_key: speaker_key,
        slot: 0,
        type_args_hash: 0,
    };
    let target = dispatch_table
        .get(&key)
        .or_else(|| dispatch_table.get_any(type_key, speaker_key, 0))?;

    let (target_module_idx, method_idx) = match target {
        super::DispatchTarget::Method {
            module_idx,
            method_idx,
        } => (*module_idx, *method_idx),
        _ => return None,
    };

    // Validate method exists
    let target_module = modules.get(target_module_idx)?;
    if method_idx >= target_module.decoded_bodies.len() {
        return None;
    }
    let reg_count = target_module.module.method_bodies[method_idx]
        .register_types
        .len();

    // Push speaker_name call frame with r_dst as return register
    let saved_depth = task.call_stack.len();
    task.call_stack.push(crate::frame::CallFrame::with_pool(
        pool,
        method_idx,
        reg_count,
        r_dst,
    ));
    // Set self parameter (register 0) to the entity value
    if let Some(frame) = task.call_stack.last_mut() {
        if !frame.registers.is_empty() {
            frame.registers[0] = entity_val;
        }
    }

    // Run VM loop until the speaker_name frame returns
    loop {
        if task.call_stack.len() <= saved_depth {
            // Frame was popped by RET — result written to caller's r_dst
            break;
        }
        let result = super::execute_one(
            task,
            modules,
            target_module_idx,
            dispatch_table,
            heap,
            host,
            globals,
            next_request_id,
            entity_registry,
            pool,
            reflection,
        );
        match result {
            super::ExecutionResult::Continue => continue,
            super::ExecutionResult::Crash(_) => {
                // Clean up: pop any extra frames pushed during the sub-call
                while task.call_stack.len() > saved_depth {
                    let f = task.call_stack.pop().unwrap();
                    pool.release(f.registers);
                }
                return None;
            }
            _ => continue,
        }
    }

    // Read result from caller frame's r_dst register
    let result_val = task
        .call_stack
        .last()
        .and_then(|f| f.registers.get(r_dst as usize))
        .copied();

    match result_val {
        Some(Value::Ref(href)) => heap.read_string(href).ok().map(|s| s.to_string()),
        _ => None,
    }
}

/// Find a contract_key by name in a specific module.
fn find_contract_key_by_name(
    modules: &[crate::loader::LoadedModule],
    mod_idx: usize,
    name: &str,
) -> Option<u32> {
    let module = &modules.get(mod_idx)?.module;
    for (idx, cd) in module.contract_defs.iter().enumerate() {
        if let Ok(cd_name) = writ_module::heap::read_string(&module.string_heap, cd.name) {
            if cd_name == name {
                return Some(((mod_idx as u32) << 16) | (idx as u32));
            }
        }
    }
    None
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
