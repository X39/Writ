use crate::entity::{EntityState, EntityTypeIdentity};
use crate::host::{HostRequest, HostResponse, LogLevel, RequestId};
use crate::value::Value;
use writ_module::instruction::ArrayDefaultKind;

use super::{ExecContext, ExecutionResult, helpers};

// ── Entity Instructions ────────────────────────────────────────

pub(super) fn exec_spawn_entity(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
    field_count: u16,
    r_base: u16,
) -> ExecutionResult {
    let (type_identity, expected_field_count) = match resolve_entity_type(ctx, type_idx) {
        Ok(resolved) => resolved,
        Err(error) => return ExecutionResult::Crash(format!("SpawnEntity: {error}")),
    };
    let (r_dst, fields) = match collect_entity_constructor_fields(
        ctx,
        "SpawnEntity",
        r_dst,
        field_count,
        r_base,
        expected_field_count,
    ) {
        Ok(validated) => validated,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let data_ref = ctx.heap.alloc_struct_initialized(u32::MAX, None, fields);
    let entity_id = ctx
        .entity_registry
        .begin_spawn_resolved(type_idx, type_identity);
    if let Err(error) = ctx.entity_registry.set_data_ref(entity_id, data_ref) {
        return ExecutionResult::Crash(format!(
            "SpawnEntity: failed to publish initialized field storage: {error}"
        ));
    }
    ctx.task.call_stack.last_mut().unwrap().registers[r_dst] = Value::Entity(entity_id);
    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::EntitySpawn {
        task_id: ctx.task.id,
        type_idx,
    };
    let _ = ctx.host.on_request(req_id, &req);
    ExecutionResult::Continue
}

fn collect_entity_constructor_fields(
    ctx: &ExecContext<'_>,
    opcode: &str,
    r_dst: u16,
    field_count: u16,
    r_base: u16,
    expected_field_count: usize,
) -> Result<(usize, Vec<Value>), String> {
    let registers = &ctx
        .task
        .call_stack
        .last()
        .ok_or_else(|| format!("{opcode}: task has no active call frame"))?
        .registers;
    let r_dst = r_dst as usize;
    if r_dst >= registers.len() {
        return Err(format!(
            "{opcode}: destination register r{r_dst} exceeds caller register count {}",
            registers.len()
        ));
    }
    if field_count as usize != expected_field_count {
        return Err(format!(
            "{opcode}: field count {} does not match TypeDef field count {expected_field_count}",
            field_count
        ));
    }
    if field_count == 0 {
        return Ok((r_dst, Vec::new()));
    }
    let start = r_base as usize;
    let end = start
        .checked_add(field_count as usize)
        .ok_or_else(|| format!("{opcode}: field register range overflow"))?;
    if end > registers.len() {
        return Err(format!(
            "{opcode}: field register range r{start}..r{end} exceeds caller register count {}",
            registers.len()
        ));
    }
    Ok((r_dst, registers[start..end].to_vec()))
}

pub(super) fn exec_init_entity(ctx: &mut ExecContext<'_>, r_entity: u16) -> ExecutionResult {
    let frame = match ctx.task.call_stack.last() {
        Some(frame) => frame,
        None => {
            return ExecutionResult::Crash("INIT_ENTITY: task has no active call frame".into());
        }
    };
    let entity_value = match frame.registers.get(r_entity as usize) {
        Some(value) => value,
        None => {
            return ExecutionResult::Crash(format!(
                "INIT_ENTITY: entity register r{r_entity} is out of range for frame with {} registers",
                frame.registers.len()
            ));
        }
    };
    let entity_id = helpers::extract_entity(entity_value);

    // The field state was installed atomically by SPAWN_ENTITY. INIT_ENTITY only
    // transitions lifecycle state and dispatches the creation hook.
    match ctx.entity_registry.commit_init(entity_id) {
        Ok(()) => {}
        Err(e) => return ExecutionResult::Crash(format!("InitEntity: {}", e)),
    }

    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::InitEntity {
        task_id: ctx.task.id,
        entity: entity_id,
    };
    let response = ctx.host.on_request(req_id, &req);
    if let HostResponse::Error(e) = response {
        return ExecutionResult::Crash(format!("InitEntity failed: {:?}", e));
    }

    // Dispatch on_create lifecycle hook if the entity type defines one.
    if let Ok(Some(type_identity)) = ctx.entity_registry.get_type_identity(entity_id) {
        let module = &ctx.modules[type_identity.module_idx];
        if let Some(hook_idx) =
            find_hook_by_name(&module.module, type_identity.type_def_idx, "on_create")
        {
            push_hook_frame(
                ctx.task,
                type_identity.module_idx,
                hook_idx,
                &module.module,
                Value::Entity(entity_id),
            );
        }
    }

    ExecutionResult::Continue
}

pub(super) fn exec_destroy_entity(ctx: &mut ExecContext<'_>, r_entity: u16) -> ExecutionResult {
    let entity_id =
        helpers::extract_entity(&ctx.task.call_stack.last().unwrap().registers[r_entity as usize]);

    // Check if we're in the second phase (on_destroy hook has already run)
    if ctx.entity_registry.get_state(entity_id) == Some(EntityState::Destroying) {
        if let Err(e) = ctx.entity_registry.complete_destroy(entity_id) {
            return ExecutionResult::Crash(format!("DestroyEntity complete: {}", e));
        }
        let req_id = RequestId(*ctx.next_request_id);
        *ctx.next_request_id += 1;
        let req = HostRequest::DestroyEntity {
            task_id: ctx.task.id,
            entity: entity_id,
        };
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

    let type_identity = ctx
        .entity_registry
        .get_type_identity(entity_id)
        .ok()
        .flatten();

    if let Err(e) = ctx.entity_registry.begin_destroy(entity_id) {
        return ExecutionResult::Crash(format!("DestroyEntity: {}", e));
    }

    // Decrement PC so DESTROY_ENTITY re-executes after the hook frame returns.
    ctx.task.call_stack.last_mut().unwrap().pc -= 1;

    // Dispatch on_destroy lifecycle hook if the entity type defines one.
    if let Some(type_identity) = type_identity {
        let module = &ctx.modules[type_identity.module_idx];
        if let Some(hook_idx) =
            find_hook_by_name(&module.module, type_identity.type_def_idx, "on_destroy")
        {
            push_hook_frame(
                ctx.task,
                type_identity.module_idx,
                hook_idx,
                &module.module,
                Value::Entity(entity_id),
            );
        }
    }

    ExecutionResult::Continue
}

fn resolve_entity_type(
    ctx: &ExecContext<'_>,
    type_idx: u32,
) -> Result<(EntityTypeIdentity, usize), String> {
    let (module_idx, type_def_idx) = crate::type_specs::resolve_type_location(
        ctx.current_module_idx,
        writ_module::MetadataToken(type_idx),
        ctx.modules,
    )
    .ok_or_else(|| format!("type token 0x{type_idx:08x} did not resolve to a TypeDef"))?;
    let module = &ctx.modules[module_idx].module;
    let type_def = module
        .type_defs
        .get(type_def_idx)
        .ok_or_else(|| format!("resolved TypeDef row {} is out of range", type_def_idx + 1))?;
    if writ_module::TypeDefKind::from_u8(type_def.kind) != Some(writ_module::TypeDefKind::Entity) {
        return Err(format!(
            "resolved TypeDef row {} is not an entity",
            type_def_idx + 1
        ));
    }
    Ok((
        EntityTypeIdentity {
            module_idx,
            type_def_idx,
        },
        helpers::get_type_field_count(module, type_def_idx)?,
    ))
}

pub(super) fn exec_get_component(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_entity: u16,
    comp_type_idx: u32,
) -> ExecutionResult {
    let entity =
        helpers::extract_entity(&ctx.task.call_stack.last().unwrap().registers[r_entity as usize]);
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::GetComponent {
        task_id: ctx.task.id,
        entity,
        comp_type_idx,
    };
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
        HostResponse::Error(e) => ExecutionResult::Crash(format!("host request failed: {:?}", e)),
        HostResponse::Suspend => {
            ctx.task.pending_request = Some((req_id, req));
            ctx.task.pending_r_dst = r_dst;
            ExecutionResult::Suspended(req_id)
        }
    }
}

pub(super) fn exec_get_or_create(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
) -> ExecutionResult {
    let (type_identity, field_count) = match resolve_entity_type(ctx, type_idx) {
        Ok(resolved) => resolved,
        Err(error) => return ExecutionResult::Crash(format!("GetOrCreate: {error}")),
    };
    if field_count != 0 {
        return ExecutionResult::Crash(format!(
            "GetOrCreate: entity TypeDef has {field_count} script field(s); singleton construction requires explicit field initialization"
        ));
    }
    // Check singleton map first
    if let Some(existing) = ctx.entity_registry.get_resolved_singleton(type_identity)
        && ctx.entity_registry.is_alive(existing)
    {
        let frame = ctx.task.call_stack.last_mut().unwrap();
        frame.registers[r_dst as usize] = Value::Entity(existing);
        return ExecutionResult::Continue;
    }
    // Create new entity and register as singleton
    let entity_id = ctx
        .entity_registry
        .allocate_resolved(type_idx, type_identity);
    let data_ref = ctx
        .heap
        .alloc_struct_initialized(u32::MAX, None, Vec::new());
    let _ = ctx.entity_registry.set_data_ref(entity_id, data_ref);
    ctx.entity_registry
        .register_resolved_singleton(type_identity, entity_id);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Entity(entity_id);
    // Notify host
    let req_id = RequestId(*ctx.next_request_id);
    *ctx.next_request_id += 1;
    let req = HostRequest::GetOrCreate {
        task_id: ctx.task.id,
        type_idx,
    };
    let _ = ctx.host.on_request(req_id, &req);
    ExecutionResult::Continue
}

pub(super) fn exec_find_all(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    _type_idx: u32,
) -> ExecutionResult {
    // FindAll returns an array of entities — stub with empty array
    let href = ctx
        .heap
        .alloc_array(ArrayDefaultKind::NullReference.operand());
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_entity_is_alive(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_entity: u16,
) -> ExecutionResult {
    let entity_id =
        helpers::extract_entity(&ctx.task.call_stack.last().unwrap().registers[r_entity as usize]);
    let is_alive = ctx.entity_registry.is_alive(entity_id);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_alive);
    ExecutionResult::Continue
}

// ──── Lifecycle Hook Helpers ──────────────────────────────────────────

/// Scan a TypeDef's explicitly owned methods for a method with the given name.
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
    for idx in module.type_method_indices(type_idx) {
        let md_name =
            writ_module::heap::read_string(&module.string_heap, module.method_defs[idx].name)
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
    module_idx: usize,
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
    let mut frame = crate::frame::CallFrame::new_in_module(
        module_idx,
        hook_method_idx,
        reg_count,
        HOOK_RETURN_SINK,
    );
    frame.registers[0] = entity_handle;
    task.call_stack.push(frame);
}

/// Log a secondary crash during destroy (used by exec_destroy_entity internally).
#[allow(dead_code)]
fn log_secondary_crash(host: &mut dyn crate::host::RuntimeHost, msg: &str) {
    host.on_log(LogLevel::Error, msg);
}
