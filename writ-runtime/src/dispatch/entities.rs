use crate::entity::EntityState;
use crate::host::{HostRequest, HostResponse, LogLevel, RequestId};
use crate::value::Value;

use super::{helpers, ExecContext, ExecutionResult};

// ── Entity Instructions ────────────────────────────────────────

pub(super) fn exec_spawn_entity(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let entity_id = ctx.entity_registry.begin_spawn(type_idx);
    let field_count = helpers::get_type_field_count(&module.module, type_idx);
    let data_ref = ctx.heap.alloc_struct(field_count);
    let _ = ctx.entity_registry.set_data_ref(entity_id, data_ref);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Entity(entity_id);
    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::EntitySpawn { task_id: ctx.task.id, type_idx };
    let _ = ctx.host.on_request(req_id, &req);
    ExecutionResult::Continue
}

pub(super) fn exec_init_entity(ctx: &mut ExecContext<'_>, r_entity: u16) -> ExecutionResult {
    let entity_id = helpers::extract_entity(
        &ctx.task.call_stack.last().unwrap().registers[r_entity as usize],
    );

    // Commit initialization: flush buffered field writes
    let field_writes = match ctx.entity_registry.commit_init(entity_id) {
        Ok(writes) => writes,
        Err(e) => return ExecutionResult::Crash(format!("InitEntity: {}", e)),
    };

    // Apply buffered field writes to the heap object
    if let Ok(Some(data_ref)) = ctx.entity_registry.get_data_ref(entity_id) {
        for (field_idx, value) in field_writes {
            let _ = ctx.heap.set_field(data_ref, field_idx as usize, value);
        }
    }

    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::InitEntity { task_id: ctx.task.id, entity: entity_id };
    let response = ctx.host.on_request(req_id, &req);
    if let HostResponse::Error(e) = response {
        return ExecutionResult::Crash(format!("InitEntity failed: {:?}", e));
    }

    // Dispatch on_create lifecycle hook if the entity type defines one.
    let module = &ctx.modules[ctx.current_module_idx];
    if let Ok(type_idx_raw) = ctx.entity_registry.get_type_idx(entity_id) {
        let type_idx_0based = (type_idx_raw as usize).saturating_sub(1);
        if let Some(hook_idx) = find_hook_by_name(&module.module, type_idx_0based, "on_create") {
            push_hook_frame(ctx.task, hook_idx, &module.module, Value::Entity(entity_id));
        }
    }

    ExecutionResult::Continue
}

pub(super) fn exec_destroy_entity(ctx: &mut ExecContext<'_>, r_entity: u16) -> ExecutionResult {
    let entity_id = helpers::extract_entity(
        &ctx.task.call_stack.last().unwrap().registers[r_entity as usize],
    );

    // Check if we're in the second phase (on_destroy hook has already run)
    if ctx.entity_registry.get_state(entity_id) == Some(EntityState::Destroying) {
        if let Err(e) = ctx.entity_registry.complete_destroy(entity_id) {
            return ExecutionResult::Crash(format!("DestroyEntity complete: {}", e));
        }
        let req_id = RequestId(*ctx.next_request_id);
        *ctx.next_request_id += 1;
        let req = HostRequest::DestroyEntity { task_id: ctx.task.id, entity: entity_id };
        let _ = ctx.host.on_request(req_id, &req);
        return ExecutionResult::Continue;
    }

    // Validate entity is alive
    if !ctx.entity_registry.is_alive(entity_id) {
        return ExecutionResult::Crash(format!(
            "DestroyEntity: entity (idx={}, gen={}) is not alive (stale or already destroyed)",
            entity_id.index, entity_id.generation
        ));
    }

    let type_idx_raw = ctx.entity_registry.get_type_idx(entity_id).unwrap_or(0);

    if let Err(e) = ctx.entity_registry.begin_destroy(entity_id) {
        return ExecutionResult::Crash(format!("DestroyEntity: {}", e));
    }

    // Decrement PC so DESTROY_ENTITY re-executes after the hook frame returns.
    ctx.task.call_stack.last_mut().unwrap().pc -= 1;

    // Dispatch on_destroy lifecycle hook if the entity type defines one.
    let module = &ctx.modules[ctx.current_module_idx];
    let type_idx_0based = (type_idx_raw as usize).saturating_sub(1);
    if let Some(hook_idx) = find_hook_by_name(&module.module, type_idx_0based, "on_destroy") {
        push_hook_frame(ctx.task, hook_idx, &module.module, Value::Entity(entity_id));
    }

    ExecutionResult::Continue
}

pub(super) fn exec_get_component(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_entity: u16,
    comp_type_idx: u32,
) -> ExecutionResult {
    let entity = helpers::extract_entity(
        &ctx.task.call_stack.last().unwrap().registers[r_entity as usize],
    );
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::GetComponent { task_id: ctx.task.id, entity, comp_type_idx };
    let response = ctx.host.on_request(req_id, &req);
    match response {
        HostResponse::Value(val) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }
        HostResponse::EntityHandle(eid) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Entity(eid);
            ExecutionResult::Continue
        }
        HostResponse::Confirmed => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Void;
            ExecutionResult::Continue
        }
        HostResponse::Error(e) => {
            ExecutionResult::Crash(format!("host request failed: {:?}", e))
        }
    }
}

pub(super) fn exec_get_or_create(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
) -> ExecutionResult {
    // Check singleton map first
    if let Some(existing) = ctx.entity_registry.get_singleton(type_idx) {
        if ctx.entity_registry.is_alive(existing) {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Entity(existing);
            return ExecutionResult::Continue;
        }
    }
    // Create new entity and register as singleton
    let module = &ctx.modules[ctx.current_module_idx];
    let entity_id = ctx.entity_registry.allocate(type_idx);
    let field_count = helpers::get_type_field_count(&module.module, type_idx);
    let data_ref = ctx.heap.alloc_struct(field_count);
    let _ = ctx.entity_registry.set_data_ref(entity_id, data_ref);
    ctx.entity_registry.register_singleton(type_idx, entity_id);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Entity(entity_id);
    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::GetOrCreate { task_id: ctx.task.id, type_idx };
    let _ = ctx.host.on_request(req_id, &req);
    ExecutionResult::Continue
}

pub(super) fn exec_find_all(ctx: &mut ExecContext<'_>, r_dst: u16, _type_idx: u32) -> ExecutionResult {
    // FindAll returns an array of entities — stub with empty array
    let href = ctx.heap.alloc_array(0);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_entity_is_alive(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_entity: u16,
) -> ExecutionResult {
    let entity_id = helpers::extract_entity(
        &ctx.task.call_stack.last().unwrap().registers[r_entity as usize],
    );
    let is_alive = ctx.entity_registry.is_alive(entity_id);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_alive);
    ExecutionResult::Continue
}

// ──── Lifecycle Hook Helpers ──────────────────────────────────────────

/// Scan a TypeDef's method range for a method with the given name.
///
/// `type_idx` is a 0-based index into `module.type_defs`.
pub(super) fn find_hook_by_name(
    module: &writ_module::Module,
    type_idx: usize,
    name: &str,
) -> Option<usize> {
    if type_idx >= module.type_defs.len() {
        return None;
    }
    let td = &module.type_defs[type_idx];
    let method_start = td.method_list.saturating_sub(1) as usize;
    let method_end = if type_idx + 1 < module.type_defs.len() {
        module.type_defs[type_idx + 1].method_list.saturating_sub(1) as usize
    } else {
        module.method_defs.len()
    };
    for idx in method_start..method_end {
        let md_name = writ_module::heap::read_string(&module.string_heap, module.method_defs[idx].name)
            .unwrap_or("");
        if md_name == name {
            return Some(idx);
        }
    }
    None
}

/// A sentinel return_register value meaning "discard the return value".
///
/// Lifecycle hook frames (on_create, on_destroy) do not return meaningful values.
pub(super) const HOOK_RETURN_SINK: u16 = u16::MAX;

/// Push a lifecycle hook call frame onto the task's call stack.
pub(super) fn push_hook_frame(
    task: &mut crate::task::Task,
    hook_method_idx: usize,
    module: &writ_module::Module,
    entity_handle: Value,
) {
    let reg_count = if hook_method_idx < module.method_bodies.len() {
        module.method_bodies[hook_method_idx].register_types.len()
    } else {
        1
    };
    let reg_count = reg_count.max(1);
    let mut frame = crate::frame::CallFrame::new(hook_method_idx, reg_count, HOOK_RETURN_SINK);
    frame.registers[0] = entity_handle;
    task.call_stack.push(frame);
}

/// Log a secondary crash during destroy (used by exec_destroy_entity internally).
#[allow(dead_code)]
fn log_secondary_crash(host: &mut dyn crate::host::RuntimeHost, msg: &str) {
    host.on_log(LogLevel::Error, msg);
}
