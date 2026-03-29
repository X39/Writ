use super::{ExecContext, ExecutionResult};

pub(super) fn exec_spawn_task(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let decoded_idx = match super::decode_method_token(method_idx) {
        Some(idx) => idx,
        None => return ExecutionResult::Crash("SpawnTask: null method token".into()),
    };
    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }
    ExecutionResult::SpawnChild {
        r_dst,
        method_idx: decoded_idx,
        args,
    }
}

pub(super) fn exec_spawn_detached(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let decoded_idx = match super::decode_method_token(method_idx) {
        Some(idx) => idx,
        None => return ExecutionResult::Crash("SpawnDetached: null method token".into()),
    };
    let mut args = Vec::with_capacity(argc as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..argc as usize {
            args.push(frame.registers[r_base as usize + i]);
        }
    }
    ExecutionResult::SpawnDetachedTask {
        r_dst,
        method_idx: decoded_idx,
        args,
    }
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

pub(super) fn exec_load_global(ctx: &mut ExecContext<'_>, r_dst: u16, global_idx: u32) -> ExecutionResult {
    let idx = global_idx as usize;
    if idx < ctx.globals.len() {
        let val = ctx.globals[idx];
        let frame = ctx.task.call_stack.last_mut().unwrap();
        frame.registers[r_dst as usize] = val;
        ExecutionResult::Continue
    } else {
        ExecutionResult::Crash(format!("LoadGlobal: index {} out of range", idx))
    }
}

pub(super) fn exec_store_global(ctx: &mut ExecContext<'_>, global_idx: u32, r_src: u16) -> ExecutionResult {
    let idx = global_idx as usize;
    let val = ctx.task.call_stack.last().unwrap().registers[r_src as usize];
    if idx < ctx.globals.len() {
        ctx.globals[idx] = val;
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
