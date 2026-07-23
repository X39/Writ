use crate::value::Value;

use super::{ExecContext, ExecutionResult};

pub(super) fn exec_spawn_task(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let (module_idx, method_idx, args) =
        match prepare_spawn(ctx, "SPAWN_TASK", r_dst, method_idx, r_base, argc) {
            Ok(spawn) => spawn,
            Err(message) => return ExecutionResult::Crash(message),
        };
    ExecutionResult::SpawnChild {
        r_dst,
        module_idx,
        method_idx,
        args,
    }
}

fn prepare_spawn(
    ctx: &mut ExecContext<'_>,
    opcode: &str,
    r_dst: u16,
    method_token: u32,
    r_base: u16,
    argc: u16,
) -> Result<(usize, usize, Vec<Value>), String> {
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    super::calls::validate_call_site_registers(opcode, caller_register_count, r_dst, r_base, argc)?;
    let (module_idx, method_idx) =
        super::calls::resolve_call_target(method_token, ctx.modules, ctx.current_module_idx)
            .map_err(|message| format!("{opcode}: {message}"))?;
    let (register_count, param_count) =
        super::calls::checked_method_register_count(opcode, ctx.modules, module_idx, method_idx)?;
    let target_module = &ctx.modules[module_idx];
    let method = &target_module.module.method_defs[method_idx];
    if method.flags & writ_module::tables::METHOD_FLAG_INTRINSIC != 0 {
        return Err(format!(
            "{opcode}: MethodDef {method_idx} in module {module_idx} is runtime-intrinsic and has no spawnable IL body"
        ));
    }
    if target_module.decoded_bodies[method_idx].is_empty() {
        return Err(format!(
            "{opcode}: MethodDef {method_idx} in module {module_idx} has no executable bytecode body"
        ));
    }
    super::calls::validate_method_param_count(opcode, param_count, argc as usize)?;
    super::calls::validate_callee_register_capacity(opcode, register_count, argc, 0)?;

    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }
    Ok((module_idx, method_idx, args))
}

pub(super) fn exec_join(ctx: &mut ExecContext<'_>, r_dst: u16, r_task: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    match crate::value::unpack_task_id(&frame.registers[r_task as usize]) {
        Some(target) => ExecutionResult::JoinTask { r_dst, target },
        None => ExecutionResult::Crash("JOIN: invalid task handle".into()),
    }
}

pub(super) fn exec_cancel(ctx: &mut ExecContext<'_>, r_task: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    match crate::value::unpack_task_id(&frame.registers[r_task as usize]) {
        Some(target) => ExecutionResult::CancelTask { target },
        None => ExecutionResult::Crash("CANCEL: invalid task handle".into()),
    }
}

pub(super) fn exec_defer_push(ctx: &mut ExecContext<'_>, method_idx: usize) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.defer_stack.push(method_idx);
    ExecutionResult::Continue
}

pub(super) fn exec_defer_pop(ctx: &mut ExecContext<'_>) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.defer_stack.pop();
    ExecutionResult::Continue
}

pub(super) fn exec_load_global(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    global_idx: u32,
) -> ExecutionResult {
    let idx = global_idx as usize;
    let Some(module_globals) = ctx.globals.get(ctx.current_module_idx) else {
        return ExecutionResult::Crash(format!(
            "LoadGlobal: module index {} out of range",
            ctx.current_module_idx
        ));
    };
    if idx < module_globals.len() {
        let val = module_globals[idx];
        let frame = ctx.task.call_stack.last_mut().unwrap();
        frame.registers[r_dst as usize] = val;
        ExecutionResult::Continue
    } else {
        ExecutionResult::Crash(format!("LoadGlobal: index {} out of range", idx))
    }
}

pub(super) fn exec_store_global(
    ctx: &mut ExecContext<'_>,
    global_idx: u32,
    r_src: u16,
) -> ExecutionResult {
    let idx = global_idx as usize;
    let val = ctx.task.call_stack.last().unwrap().registers[r_src as usize];
    let Some(module_globals) = ctx.globals.get_mut(ctx.current_module_idx) else {
        return ExecutionResult::Crash(format!(
            "StoreGlobal: module index {} out of range",
            ctx.current_module_idx
        ));
    };
    if idx < module_globals.len() {
        module_globals[idx] = val;
        ExecutionResult::Continue
    } else {
        ExecutionResult::Crash(format!("StoreGlobal: index {} out of range", idx))
    }
}

pub(super) fn exec_atomic_end(ctx: &mut ExecContext<'_>) -> ExecutionResult {
    if ctx.task.atomic_depth == 0 {
        return ExecutionResult::Crash("ATOMIC_END without matching ATOMIC_BEGIN".into());
    }
    ctx.task.atomic_depth -= 1;
    // Lock release handled by scheduler
    ExecutionResult::Continue
}
