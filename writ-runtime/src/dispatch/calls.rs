use crate::heap::HeapObject;
use crate::host::{LogLevel, RequestId};
use crate::value::Value;

use super::{DispatchTarget, ExecContext, ExecutionResult, IntrinsicId, helpers, intrinsics};

#[inline]
pub(super) fn exec_call(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    if let Err(message) =
        validate_call_site_registers("CALL", caller_register_count, r_dst, r_base, argc)
    {
        return ExecutionResult::Crash(message);
    }

    let (target_module_idx, method_idx) =
        match resolve_call_target(method_idx, ctx.modules, ctx.current_module_idx) {
            Ok(target) => target,
            Err(message) => return ExecutionResult::Crash(message),
        };
    let (reg_count, param_count) =
        match checked_method_register_count("CALL", ctx.modules, target_module_idx, method_idx) {
            Ok(reg_count) => reg_count,
            Err(message) => return ExecutionResult::Crash(message),
        };
    if let Err(message) = validate_method_param_count("CALL", param_count, argc as usize) {
        return ExecutionResult::Crash(message);
    }
    if let Err(message) = validate_callee_register_capacity("CALL", reg_count, argc, 0) {
        return ExecutionResult::Crash(message);
    }

    // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
    ctx.task
        .call_stack
        .push(crate::frame::CallFrame::with_pool_in_module(
            ctx.pool,
            target_module_idx,
            method_idx,
            reg_count,
            r_dst,
        ));
    let stack_len = ctx.task.call_stack.len();
    let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
    let caller = bottom.last().unwrap();
    let callee = &mut top[0];
    for i in 0..argc as usize {
        callee.registers[i] = caller.registers[r_base as usize + i];
    }

    if ctx.host.debug_enabled() {
        ctx.host
            .on_function_enter(ctx.task.id, target_module_idx, method_idx as u32);
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
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    if let Err(message) =
        validate_call_site_registers("CALL_VIRT", caller_register_count, r_dst, r_base, argc)
    {
        return ExecutionResult::Crash(message);
    }
    let r_obj_idx = match checked_register("CALL_VIRT", caller_register_count, r_obj, "receiver") {
        Ok(r_obj) => r_obj,
        Err(message) => return ExecutionResult::Crash(message),
    };
    if r_obj != r_base {
        return ExecutionResult::Crash(format!(
            "CALL_VIRT: receiver register r{r_obj} must equal argument base r{r_base}"
        ));
    }
    if argc == 0 {
        return ExecutionResult::Crash(
            "CALL_VIRT: argument block must include the receiver".to_string(),
        );
    }
    let obj_val = ctx.task.call_stack.last().unwrap().registers[r_obj_idx];

    // Determine type_key from the object value's runtime type
    let type_key = resolve_runtime_type_key(obj_val, ctx.heap, ctx.modules);
    if type_key == u32::MAX {
        return ExecutionResult::Crash(
            "CALL_VIRT: receiver has no resolvable runtime type".to_string(),
        );
    }

    let target_type = match resolve_runtime_type_spec(obj_val, ctx.heap) {
        Some((module_idx, token)) => {
            let Some(signature) =
                crate::type_specs::type_spec_signature(module_idx, token, ctx.modules)
            else {
                return ExecutionResult::Crash(format!(
                    "CALL_VIRT: malformed receiver TypeSpec token 0x{:08x}",
                    token.0
                ));
            };
            Some((module_idx, signature))
        }
        None => None,
    };

    // Resolve contract_key from the contract_idx in the current module
    let contract_key =
        resolve_contract_key_from_idx(contract_idx, ctx.modules, ctx.current_module_idx);
    if contract_key == u32::MAX {
        return ExecutionResult::Crash(format!(
            "CALL_VIRT: unresolved contract token 0x{contract_idx:08x}"
        ));
    }

    let contract_token = writ_module::MetadataToken(contract_idx);
    let contract_type = if contract_token.table_id() == 4 {
        let Some(signature) = crate::type_specs::type_spec_signature(
            ctx.current_module_idx,
            contract_token,
            ctx.modules,
        ) else {
            return ExecutionResult::Crash(format!(
                "CALL_VIRT: malformed contract TypeSpec token 0x{contract_idx:08x}"
            ));
        };
        Some((ctx.current_module_idx, signature))
    } else {
        None
    };

    let resolved_target = match ctx.dispatch_table.resolve_pattern(
        type_key,
        target_type
            .as_ref()
            .map(|(module_idx, signature)| (*module_idx, signature)),
        contract_key,
        contract_type
            .as_ref()
            .map(|(module_idx, signature)| (*module_idx, signature)),
        slot,
        ctx.modules,
    ) {
        Ok(target) => target.copied(),
        Err(count) => {
            return ExecutionResult::Crash(format!(
                "CALL_VIRT: ambiguous implementation ({count} matches) for type_key=0x{type_key:08x}, contract_key=0x{contract_key:08x}, slot={slot}"
            ));
        }
    };

    match resolved_target {
        Some(DispatchTarget::Method {
            module_idx,
            method_idx,
        }) => {
            let (reg_count, param_count) = match checked_method_register_count(
                "CALL_VIRT",
                ctx.modules,
                module_idx,
                method_idx,
            ) {
                Ok(reg_count) => reg_count,
                Err(message) => return ExecutionResult::Crash(message),
            };
            if let Err(message) =
                validate_method_param_count("CALL_VIRT", param_count, argc as usize)
            {
                return ExecutionResult::Crash(message);
            }
            if let Err(message) = validate_callee_register_capacity("CALL_VIRT", reg_count, argc, 0)
            {
                return ExecutionResult::Crash(message);
            }

            // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
            ctx.task
                .call_stack
                .push(crate::frame::CallFrame::with_pool_in_module(
                    ctx.pool, module_idx, method_idx, reg_count, r_dst,
                ));
            let stack_len = ctx.task.call_stack.len();
            let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
            let caller = bottom.last().unwrap();
            let callee = &mut top[0];
            for i in 0..argc as usize {
                callee.registers[i] = caller.registers[r_base as usize + i];
            }

            if ctx.host.debug_enabled() {
                ctx.host
                    .on_function_enter(ctx.task.id, module_idx, method_idx as u32);
            }
            ExecutionResult::Continue
        }
        Some(DispatchTarget::Intrinsic(id)) => {
            let expected_param_count = intrinsic_param_count(id);
            if argc as usize != expected_param_count {
                return ExecutionResult::Crash(format!(
                    "CALL_VIRT: argument count {argc} does not match intrinsic parameter count {expected_param_count}"
                ));
            }
            intrinsics::execute_intrinsic(ctx, id, r_dst, r_obj, r_base, argc)
        }
        None => ExecutionResult::Crash(format!(
            "CALL_VIRT: no implementation for type_key=0x{:08x}, contract_key=0x{:08x}, slot={}",
            type_key, contract_key, slot
        )),
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
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    if let Err(message) =
        validate_call_site_registers("CALL_EXTERN", caller_register_count, r_dst, r_base, argc)
    {
        return ExecutionResult::Crash(message);
    }

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
    let display_args: Vec<String> = args
        .iter()
        .enumerate()
        .map(|(i, v)| {
            // Check for Speaker contract override first
            if let Some((_, name)) = speaker_overrides.iter().find(|(idx, _)| *idx == i) {
                return name.clone();
            }
            match v {
                Value::Int(n) => n.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Ref(href) => ctx
                    .heap
                    .read_string(*href)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "<ref>".to_string()),
                Value::Void => "void".to_string(),
                Value::Entity(e) => {
                    resolve_entity_display_name(*e, ctx.entity_registry, ctx.modules)
                }
                Value::Struct { type_idx, .. } => format!("<struct@{}>", type_idx),
            }
        })
        .collect();

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
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    let r_dst = match checked_register("NEW_DELEGATE", caller_register_count, r_dst, "destination")
    {
        Ok(r_dst) => r_dst,
        Err(message) => return ExecutionResult::Crash(message),
    };
    let r_target = match checked_register("NEW_DELEGATE", caller_register_count, r_target, "target")
    {
        Ok(r_target) => r_target,
        Err(message) => return ExecutionResult::Crash(message),
    };

    let (target_module_idx, method_idx) =
        match resolve_call_target(method_idx, ctx.modules, ctx.current_module_idx) {
            Ok(target) => target,
            Err(message) => return ExecutionResult::Crash(format!("NEW_DELEGATE: {message}")),
        };
    if let Err(message) =
        checked_delegate_method("NEW_DELEGATE", ctx.modules, target_module_idx, method_idx)
    {
        return ExecutionResult::Crash(message);
    }

    let target = {
        let frame = ctx.task.call_stack.last().unwrap();
        if matches!(frame.registers[r_target], Value::Void) {
            None
        } else {
            Some(frame.registers[r_target])
        }
    };
    if let Err(message) = validate_delegate_binding(
        "NEW_DELEGATE",
        ctx.modules,
        target_module_idx,
        method_idx,
        target.is_some(),
    ) {
        return ExecutionResult::Crash(message);
    }
    let href = ctx
        .heap
        .alloc_delegate(target_module_idx, method_idx, target);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst] = Value::Ref(href);
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
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    if let Err(message) =
        validate_call_site_registers("CALL_INDIRECT", caller_register_count, r_dst, r_base, argc)
    {
        return ExecutionResult::Crash(message);
    }
    let r_delegate = match checked_register(
        "CALL_INDIRECT",
        caller_register_count,
        r_delegate,
        "delegate",
    ) {
        Ok(r_delegate) => r_delegate,
        Err(message) => return ExecutionResult::Crash(message),
    };
    let delegate_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_delegate]);
    let (target_module_idx, method_idx, target) = match ctx.heap.get_object(delegate_ref) {
        Ok(HeapObject::Delegate {
            module_idx,
            method_idx,
            target,
        }) => (*module_idx, *method_idx, *target),
        _ => return ExecutionResult::Crash("CALL_INDIRECT: not a delegate".into()),
    };

    let (reg_count, param_count) = match checked_delegate_method(
        "CALL_INDIRECT",
        ctx.modules,
        target_module_idx,
        method_idx,
    ) {
        Ok(reg_count) => reg_count,
        Err(message) => return ExecutionResult::Crash(message),
    };
    let implicit_argc = match validate_delegate_binding(
        "CALL_INDIRECT",
        ctx.modules,
        target_module_idx,
        method_idx,
        target.is_some(),
    ) {
        Ok(implicit_argc) => implicit_argc,
        Err(message) => return ExecutionResult::Crash(message),
    };
    let actual_param_count = argc as usize + implicit_argc;
    if let Err(message) =
        validate_method_param_count("CALL_INDIRECT", param_count, actual_param_count)
    {
        return ExecutionResult::Crash(message);
    }
    if let Err(message) =
        validate_callee_register_capacity("CALL_INDIRECT", reg_count, argc, implicit_argc)
    {
        return ExecutionResult::Crash(message);
    }

    // Push callee frame immediately, then use split_at_mut for disjoint caller/callee access
    ctx.task
        .call_stack
        .push(crate::frame::CallFrame::with_pool_in_module(
            ctx.pool,
            target_module_idx,
            method_idx,
            reg_count,
            r_dst,
        ));
    let stack_len = ctx.task.call_stack.len();
    let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
    let caller = bottom.last().unwrap();
    let callee = &mut top[0];
    if let Some(target) = target {
        callee.registers[0] = target;
    }
    for i in 0..argc as usize {
        callee.registers[implicit_argc + i] = caller.registers[r_base as usize + i];
    }

    if ctx.host.debug_enabled() {
        ctx.host
            .on_function_enter(ctx.task.id, target_module_idx, method_idx as u32);
    }
    ExecutionResult::Continue
}

pub(super) fn validate_call_site_registers(
    opcode: &str,
    caller_register_count: usize,
    r_dst: u16,
    r_base: u16,
    argc: u16,
) -> Result<(), String> {
    checked_register(opcode, caller_register_count, r_dst, "destination")?;

    validate_argument_registers(opcode, caller_register_count, r_base, argc)
}

pub(super) fn validate_argument_registers(
    opcode: &str,
    caller_register_count: usize,
    r_base: u16,
    argc: u16,
) -> Result<(), String> {
    let start = r_base as usize;
    let end = start
        .checked_add(argc as usize)
        .ok_or_else(|| format!("{opcode}: argument register range overflow"))?;
    if end > caller_register_count {
        return Err(format!(
            "{opcode}: argument register range r{start}..r{end} exceeds caller register count {caller_register_count}"
        ));
    }

    Ok(())
}

fn checked_register(
    opcode: &str,
    register_count: usize,
    register: u16,
    role: &str,
) -> Result<usize, String> {
    let register = register as usize;
    if register >= register_count {
        return Err(format!(
            "{opcode}: {role} register r{register} exceeds caller register count {register_count}"
        ));
    }
    Ok(register)
}

pub(super) fn validate_callee_register_capacity(
    opcode: &str,
    callee_register_count: usize,
    argc: u16,
    implicit_argc: usize,
) -> Result<(), String> {
    let required = (argc as usize)
        .checked_add(implicit_argc)
        .ok_or_else(|| format!("{opcode}: callee register requirement overflow"))?;
    if required > callee_register_count {
        return Err(format!(
            "{opcode}: {required} arguments exceed callee register count {callee_register_count}"
        ));
    }
    Ok(())
}

pub(super) fn validate_method_param_count(
    opcode: &str,
    expected_param_count: usize,
    actual_param_count: usize,
) -> Result<(), String> {
    if actual_param_count != expected_param_count {
        return Err(format!(
            "{opcode}: argument count {actual_param_count} does not match MethodDef.param_count {expected_param_count}"
        ));
    }
    Ok(())
}

fn intrinsic_param_count(id: IntrinsicId) -> usize {
    match id {
        IntrinsicId::IntAdd
        | IntrinsicId::IntSub
        | IntrinsicId::IntMul
        | IntrinsicId::IntDiv
        | IntrinsicId::IntMod
        | IntrinsicId::IntEq
        | IntrinsicId::IntOrd
        | IntrinsicId::IntBitAnd
        | IntrinsicId::IntBitOr
        | IntrinsicId::FloatAdd
        | IntrinsicId::FloatSub
        | IntrinsicId::FloatMul
        | IntrinsicId::FloatDiv
        | IntrinsicId::FloatMod
        | IntrinsicId::FloatEq
        | IntrinsicId::FloatOrd
        | IntrinsicId::BoolEq
        | IntrinsicId::StringAdd
        | IntrinsicId::StringEq
        | IntrinsicId::StringOrd
        | IntrinsicId::StringIndexChar
        | IntrinsicId::ArrayIndex
        | IntrinsicId::TypeImplements
        | IntrinsicId::FieldInfoGet => 2,
        IntrinsicId::StringIndexRange
        | IntrinsicId::ArrayIndexSet
        | IntrinsicId::ArraySlice
        | IntrinsicId::FieldInfoSet
        | IntrinsicId::MethodInfoInvoke => 3,
        _ => 1,
    }
}

pub(super) fn checked_method_register_count(
    opcode: &str,
    modules: &[crate::loader::LoadedModule],
    module_idx: usize,
    method_idx: usize,
) -> Result<(usize, usize), String> {
    let module = modules
        .get(module_idx)
        .ok_or_else(|| format!("{opcode}: target module index {module_idx} out of range"))?;
    let method_def = module.module.method_defs.get(method_idx).ok_or_else(|| {
        format!("{opcode}: MethodDef index {method_idx} out of range in module {module_idx}")
    })?;
    if module.decoded_bodies.get(method_idx).is_none() {
        return Err(format!(
            "{opcode}: decoded method body index {method_idx} out of range in module {module_idx}"
        ));
    }
    let body = module.module.method_bodies.get(method_idx).ok_or_else(|| {
        format!("{opcode}: MethodBody index {method_idx} out of range in module {module_idx}")
    })?;
    let body_register_count = body.register_types.len();
    if method_def.reg_count as usize != body_register_count {
        return Err(format!(
            "{opcode}: MethodDef.reg_count {} does not match MethodBody register count {body_register_count} for method {method_idx} in module {module_idx}",
            method_def.reg_count
        ));
    }
    Ok((body_register_count, method_def.param_count as usize))
}

fn checked_delegate_method(
    opcode: &str,
    modules: &[crate::loader::LoadedModule],
    module_idx: usize,
    method_idx: usize,
) -> Result<(usize, usize), String> {
    let counts = checked_method_register_count(opcode, modules, module_idx, method_idx)?;
    let module = &modules[module_idx];
    let method = &module.module.method_defs[method_idx];
    if method.flags & writ_module::tables::METHOD_FLAG_INTRINSIC != 0 {
        return Err(format!(
            "{opcode}: MethodDef {method_idx} in module {module_idx} is runtime-intrinsic and has no delegate-callable IL body"
        ));
    }
    if module.decoded_bodies[method_idx].is_empty() {
        return Err(format!(
            "{opcode}: MethodDef {method_idx} in module {module_idx} has no executable bytecode body"
        ));
    }
    Ok(counts)
}

fn validate_delegate_binding(
    opcode: &str,
    modules: &[crate::loader::LoadedModule],
    module_idx: usize,
    method_idx: usize,
    has_target: bool,
) -> Result<usize, String> {
    let method = modules
        .get(module_idx)
        .and_then(|module| module.module.method_defs.get(method_idx))
        .ok_or_else(|| {
            format!("{opcode}: MethodDef index {method_idx} out of range in module {module_idx}")
        })?;
    let has_receiver =
        !method.owner.is_null() && method.flags & writ_module::tables::METHOD_FLAG_STATIC == 0;

    match (has_receiver, has_target) {
        (true, false) => Err(format!(
            "{opcode}: instance MethodDef {method_idx} in module {module_idx} requires a non-null delegate target"
        )),
        (false, true) => Err(format!(
            "{opcode}: static or top-level MethodDef {method_idx} in module {module_idx} requires a null delegate target"
        )),
        _ => Ok(usize::from(has_receiver)),
    }
}

#[inline]
pub(super) fn exec_tail_call(
    ctx: &mut ExecContext<'_>,
    method_idx: u32,
    r_base: u16,
    argc: u16,
) -> ExecutionResult {
    let caller_register_count = ctx.task.call_stack.last().unwrap().registers.len();
    if let Err(message) =
        validate_argument_registers("TAIL_CALL", caller_register_count, r_base, argc)
    {
        return ExecutionResult::Crash(message);
    }

    let (target_module_idx, method_idx) =
        match resolve_call_target(method_idx, ctx.modules, ctx.current_module_idx) {
            Ok(target) => target,
            Err(message) => return ExecutionResult::Crash(format!("TAIL_CALL: {message}")),
        };
    let (reg_count, param_count) = match checked_method_register_count(
        "TAIL_CALL",
        ctx.modules,
        target_module_idx,
        method_idx,
    ) {
        Ok(metadata) => metadata,
        Err(message) => return ExecutionResult::Crash(message),
    };
    if let Err(message) = validate_method_param_count("TAIL_CALL", param_count, argc as usize) {
        return ExecutionResult::Crash(message);
    }
    if let Err(message) = validate_callee_register_capacity("TAIL_CALL", reg_count, argc, 0) {
        return ExecutionResult::Crash(message);
    }

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
            ctx.task,
            handler_pc,
            ctx.modules,
            ctx.current_module_idx,
            ctx.dispatch_table,
            ctx.heap,
            ctx.host,
            ctx.globals,
            ctx.next_request_id,
            ctx.entity_registry,
            ctx.pool,
            ctx.reflection,
        ) {
            ctx.host.on_log(
                LogLevel::Error,
                &format!("secondary crash in defer during tail call: {}", secondary),
            );
        }
    }

    // Replace current frame in-place (reuse existing Vec allocation via clear+resize)
    let current = ctx.task.call_stack.last_mut().unwrap();
    current.module_idx = Some(target_module_idx);
    current.method_idx = method_idx;
    current.pc = 0;
    current.registers.clear();
    current.registers.resize(reg_count, Value::Void);
    if let Some(hv) = heap_args {
        for (i, v) in hv.into_iter().enumerate() {
            current.registers[i] = v;
        }
    } else {
        for i in 0..argc_usize {
            current.registers[i] = arg_buf[i];
        }
    }

    ExecutionResult::Continue
}

// ──── CALL_VIRT Helpers ───────────────────────────────────────────────

pub(super) fn resolve_call_target(
    token: u32,
    modules: &[crate::loader::LoadedModule],
    current_module_idx: usize,
) -> Result<(usize, usize), String> {
    let token = writ_module::MetadataToken(token);
    let row = token
        .row_index()
        .ok_or_else(|| "call to null method token".to_string())?
        - 1;

    match token.table_id() {
        7 => Ok((current_module_idx, row as usize)),
        8 => modules[current_module_idx]
            .resolved_refs
            .methods
            .get(&row)
            .map(|resolved| (resolved.module_idx, resolved.method_idx))
            .ok_or_else(|| format!("call to unresolved MethodRef row {}", row)),
        table => Err(format!(
            "call uses unsupported method token table {}",
            table
        )),
    }
}

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
        Value::Ref(href) => match heap.get_object(href) {
            Ok(HeapObject::String(_)) => find_type_key_by_name(modules, 0, "String"),
            Ok(HeapObject::Array { .. }) => find_type_key_by_name(modules, 0, "Array"),
            Ok(HeapObject::Struct { type_key, .. }) => *type_key,
            Ok(HeapObject::Boxed(inner)) => resolve_runtime_type_key(*inner, heap, modules),
            _ => u32::MAX,
        },
        Value::Entity(_) => find_type_key_by_name(modules, 0, "Entity"),
        Value::Void => u32::MAX,
        Value::Struct { href, .. } => match heap.get_object(href) {
            Ok(HeapObject::Struct { type_key, .. }) => *type_key,
            _ => u32::MAX,
        },
    }
}
/// Return the allocation-site TypeSpec retained on a generic class or struct.
fn resolve_runtime_type_spec(
    val: Value,
    heap: &dyn crate::gc::GcHeap,
) -> Option<(usize, writ_module::MetadataToken)> {
    let href = match val {
        Value::Ref(href) | Value::Struct { href, .. } => href,
        _ => return None,
    };
    match heap.get_object(href).ok()? {
        HeapObject::Struct {
            type_spec: Some((module_idx, token)),
            ..
        } => Some((*module_idx, writ_module::MetadataToken(*token))),
        HeapObject::Boxed(inner) => resolve_runtime_type_spec(*inner, heap),
        _ => None,
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
            && td_name == name
        {
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
    crate::type_specs::resolve_contract_key(
        current_module_idx,
        writ_module::MetadataToken(contract_idx),
        modules,
    )
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
    if let Ok(Some(identity)) = entity_registry.get_type_identity(entity_id)
        && let Some(module) = modules
            .get(identity.module_idx)
            .map(|loaded| &loaded.module)
        && let Some(type_def) = module.type_defs.get(identity.type_def_idx)
        && let Ok(name) = writ_module::heap::read_string(&module.string_heap, type_def.name)
        && !name.is_empty()
    {
        return name.to_string();
    }

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
            if let Ok(name) =
                writ_module::heap::read_string(&module.string_heap, module.type_defs[row].name)
            {
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
    globals: &mut Vec<Vec<Value>>,
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
    let type_key = match entity_registry.get_type_identity(entity_id).ok().flatten() {
        Some(identity) => ((identity.module_idx as u32) << 16) | identity.type_def_idx as u32,
        None => {
            // Compatibility fallback for entities created through the public raw registry API.
            let raw_type_idx = entity_registry.get_type_idx(entity_id).ok()?;
            let row_0based = (raw_type_idx & 0x00FF_FFFF).saturating_sub(1);
            let user_mod_idx = modules.len().saturating_sub(1) as u32;
            (user_mod_idx << 16) | row_0based
        }
    };

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
    task.call_stack
        .push(crate::frame::CallFrame::with_pool_in_module(
            pool,
            target_module_idx,
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
