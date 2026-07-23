use crate::heap::HeapObject;
use crate::value::Value;
use writ_module::instruction::ArrayDefaultKind;
use writ_module::tables::FIELD_FLAG_READONLY;

use super::{ExecContext, ExecutionResult, helpers};

// ── Struct Object Model ────────────────────────────────────────

pub(super) fn exec_new(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
    field_count: u16,
    r_base: u16,
) -> ExecutionResult {
    let token = writ_module::MetadataToken(type_idx);
    let table_id = token.table_id();
    let row = token.row_index().unwrap_or(0);
    let type_spec = (table_id == 4).then_some((ctx.current_module_idx, type_idx));
    if !matches!(table_id, 2 | 3 | 4) || row == 0 {
        return ExecutionResult::Crash(format!("NEW: unsupported type token 0x{type_idx:08x}"));
    }

    // Resolve the target module and typedef index.
    // TypeDef tokens (table 2) reference the current module directly.
    // TypeRef tokens (table 3) require cross-module resolution.
    let (target_module_idx, target_typedef_idx) = if table_id == 4 {
        match crate::type_specs::resolve_type_location(ctx.current_module_idx, token, ctx.modules) {
            Some(location) => location,
            None => {
                return ExecutionResult::Crash(format!(
                    "NEW: TypeSpec token 0x{type_idx:08x} did not resolve to a TypeDef"
                ));
            }
        }
    } else if table_id == 3 {
        // TypeRef — resolve through the domain's cross-module resolution
        let module = &ctx.modules[ctx.current_module_idx];
        let typeref_row_0based = row.saturating_sub(1) as u32;
        if let Some(resolved) = module.resolved_refs.types.get(&typeref_row_0based) {
            (resolved.module_idx, resolved.typedef_idx as usize)
        } else {
            return ExecutionResult::Crash(format!(
                "NEW: TypeRef row {} not resolved",
                typeref_row_0based
            ));
        }
    } else {
        // TypeDef — current module, convert 1-based row to 0-based index
        (ctx.current_module_idx, row.saturating_sub(1) as usize)
    };

    let target_module = &ctx.modules[target_module_idx];
    let expected_field_count =
        match helpers::get_type_field_count(&target_module.module, target_typedef_idx) {
            Ok(field_count) => field_count,
            Err(error) => return ExecutionResult::Crash(format!("NEW: {error}")),
        };
    let kind_u8 = target_module
        .module
        .type_defs
        .get(target_typedef_idx)
        .map(|t| t.kind);
    let kind = match kind_u8.and_then(writ_module::TypeDefKind::from_u8) {
        Some(writ_module::TypeDefKind::Struct) => writ_module::TypeDefKind::Struct,
        Some(writ_module::TypeDefKind::Class) => writ_module::TypeDefKind::Class,
        Some(other) => {
            return ExecutionResult::Crash(format!(
                "NEW: type token 0x{type_idx:08x} has kind {other:?}; expected struct or class"
            ));
        }
        None => {
            return ExecutionResult::Crash(format!(
                "NEW: type token 0x{type_idx:08x} resolved outside the TypeDef table"
            ));
        }
    };
    let (r_dst, fields) = match collect_constructor_fields(
        ctx,
        "NEW",
        r_dst,
        field_count,
        r_base,
        expected_field_count,
    ) {
        Ok(validated) => validated,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let runtime_type_key = ((target_module_idx as u32) << 16) | target_typedef_idx as u32;
    let href = ctx
        .heap
        .alloc_struct_initialized(runtime_type_key, type_spec, fields);

    let value = match kind {
        writ_module::TypeDefKind::Struct => {
            // kind=0: value-type struct — heap allocation with Copy-semantic HeapRef.
            // Keep the canonical base key and optional TypeSpec on the heap so
            // cross-module struct dispatch does not depend on a local token value.
            Value::Struct { type_idx, href }
        }
        writ_module::TypeDefKind::Class => {
            // kind=4 (class): heap allocation.
            // Encode the type_key as (target_module_idx << 16) | target_typedef_idx so that
            // CALL_VIRT can resolve the dispatch table entry from the runtime object type.
            Value::Ref(href)
        }
        _ => unreachable!("NEW kind was validated before allocation"),
    };
    ctx.task.call_stack.last_mut().unwrap().registers[r_dst] = value;
    ExecutionResult::Continue
}

fn collect_constructor_fields(
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

struct ResolvedFieldTarget {
    href: crate::value::HeapRef,
    field_offset: usize,
    flags: u16,
    name: String,
}

fn checked_field_register(
    registers: &[Value],
    opcode: &str,
    register: u16,
    role: &str,
) -> Result<usize, String> {
    let register = register as usize;
    if register >= registers.len() {
        return Err(format!(
            "{opcode}: {role} register r{register} exceeds caller register count {}",
            registers.len()
        ));
    }
    Ok(register)
}

fn resolve_field_target(
    ctx: &ExecContext<'_>,
    object: Value,
    field_operand: u32,
) -> Result<ResolvedFieldTarget, String> {
    let (href, entity_identity, is_entity) = match object {
        Value::Struct { href, .. } | Value::Ref(href) => (href, None, false),
        Value::Entity(entity) => {
            let href = ctx
                .entity_registry
                .get_data_ref(entity)
                .map_err(|error| format!("invalid entity receiver: {error}"))?
                .ok_or_else(|| "entity receiver has no script-field storage".to_string())?;
            let identity = ctx
                .entity_registry
                .get_type_identity(entity)
                .map_err(|error| format!("invalid entity receiver: {error}"))?;
            (href, identity, true)
        }
        other => {
            return Err(format!(
                "expected struct, class, or entity receiver, got {other:?}"
            ));
        }
    };

    let token = writ_module::MetadataToken(field_operand);
    if token.is_null() {
        return Err("null field token".into());
    }
    let row = token
        .row_index()
        .and_then(|row| row.checked_sub(1))
        .ok_or_else(|| format!("field token 0x{field_operand:08x} has row zero"))?
        as usize;

    let (target_module_idx, owner_type_idx, field_def_idx, field_offset) =
        match writ_module::tables::TableId::from_u8(token.table_id()) {
            Some(writ_module::tables::TableId::FieldDef) => {
                let location = ctx.modules[ctx.current_module_idx]
                    .field_def_locations
                    .get(row)
                    .ok_or_else(|| format!("FieldDef row {} is out of range", row + 1))?;
                (
                    ctx.current_module_idx,
                    location.owner_type_idx,
                    row,
                    location.field_offset,
                )
            }
            Some(writ_module::tables::TableId::FieldRef) => {
                let current_module = &ctx.modules[ctx.current_module_idx];
                current_module
                    .module
                    .field_refs
                    .get(row)
                    .ok_or_else(|| format!("FieldRef row {} is out of range", row + 1))?;
                let resolved = current_module
                    .resolved_refs
                    .fields
                    .get(&(row as u32))
                    .ok_or_else(|| format!("unresolved FieldRef row {}", row + 1))?;
                let target_module = ctx.modules.get(resolved.module_idx).ok_or_else(|| {
                    format!(
                        "FieldRef row {} resolved to missing module {}",
                        row + 1,
                        resolved.module_idx
                    )
                })?;
                let location = target_module
                    .field_def_locations
                    .get(resolved.field_idx)
                    .ok_or_else(|| {
                        format!(
                            "FieldRef row {} resolved to out-of-range FieldDef row {}",
                            row + 1,
                            resolved.field_idx + 1
                        )
                    })?;
                if location.owner_type_idx != resolved.owner_type_idx
                    || location.field_offset != resolved.field_offset
                {
                    return Err(format!(
                        "FieldRef row {} resolved to inconsistent field metadata",
                        row + 1
                    ));
                }
                (
                    resolved.module_idx,
                    resolved.owner_type_idx,
                    resolved.field_idx,
                    resolved.field_offset,
                )
            }
            _ => {
                return Err(format!(
                    "field token 0x{field_operand:08x} uses table {}; expected FieldDef (5) or FieldRef (6)",
                    token.table_id()
                ));
            }
        };
    let target_module = &ctx.modules[target_module_idx].module;
    let field_def = target_module.field_defs.get(field_def_idx).ok_or_else(|| {
        format!(
            "resolved FieldDef row {} is out of range",
            field_def_idx + 1
        )
    })?;
    let field_name = writ_module::heap::read_string(&target_module.string_heap, field_def.name)
        .unwrap_or("<invalid field name>")
        .to_string();

    let expected_identity = crate::entity::EntityTypeIdentity {
        module_idx: target_module_idx,
        type_def_idx: owner_type_idx,
    };
    if let Some(actual_identity) = entity_identity {
        if actual_identity != expected_identity {
            return Err(format!(
                "field token owner mismatch: entity type {:?}, expected {:?}",
                actual_identity, expected_identity
            ));
        }
        return Ok(ResolvedFieldTarget {
            href,
            field_offset,
            flags: field_def.flags,
            name: field_name,
        });
    }
    if is_entity {
        return Err("field-token entity receiver has no canonical type identity".into());
    }

    match ctx.heap.get_object(href) {
        Ok(HeapObject::Struct { type_key, .. }) if *type_key != u32::MAX => {
            let expected = ((target_module_idx as u32) << 16) | owner_type_idx as u32;
            if *type_key != expected {
                return Err(format!(
                    "field token owner mismatch: object type key 0x{type_key:08x}, expected 0x{expected:08x}"
                ));
            }
        }
        Ok(HeapObject::Struct { .. }) => {
            return Err("field-token receiver has no canonical type identity".into());
        }
        Ok(_) => return Err("field-token receiver is not a struct or class object".into()),
        Err(error) => return Err(error.to_string()),
    }

    Ok(ResolvedFieldTarget {
        href,
        field_offset,
        flags: field_def.flags,
        name: field_name,
    })
}

pub(super) fn exec_get_field(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_obj: u16,
    field_token: u32,
) -> ExecutionResult {
    let frame = match ctx.task.call_stack.last() {
        Some(frame) => frame,
        None => return ExecutionResult::Crash("GetField: task has no active call frame".into()),
    };
    let r_dst = match checked_field_register(&frame.registers, "GetField", r_dst, "destination") {
        Ok(register) => register,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let r_obj = match checked_field_register(&frame.registers, "GetField", r_obj, "object") {
        Ok(register) => register,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let object = frame.registers[r_obj];
    let target = match resolve_field_target(ctx, object, field_token) {
        Ok(target) => target,
        Err(error) => return ExecutionResult::Crash(format!("GetField: {error}")),
    };
    match ctx.heap.get_field(target.href, target.field_offset) {
        Ok(val) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst] = val;
            ExecutionResult::Continue
        }
        Err(error) => ExecutionResult::Crash(format!("GetField: {error}")),
    }
}

pub(super) fn exec_set_field(
    ctx: &mut ExecContext<'_>,
    r_obj: u16,
    field_token: u32,
    r_val: u16,
) -> ExecutionResult {
    let frame = match ctx.task.call_stack.last() {
        Some(frame) => frame,
        None => return ExecutionResult::Crash("SetField: task has no active call frame".into()),
    };
    let r_obj = match checked_field_register(&frame.registers, "SetField", r_obj, "object") {
        Ok(register) => register,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let r_val = match checked_field_register(&frame.registers, "SetField", r_val, "value") {
        Ok(register) => register,
        Err(error) => return ExecutionResult::Crash(error),
    };
    let object = frame.registers[r_obj];
    let val = frame.registers[r_val];
    let target = match resolve_field_target(ctx, object, field_token) {
        Ok(target) => target,
        Err(error) => return ExecutionResult::Crash(format!("SetField: {error}")),
    };
    if target.flags & FIELD_FLAG_READONLY != 0 {
        return ExecutionResult::Crash(format!(
            "SetField: cannot write to read-only field '{}'",
            target.name
        ));
    }
    match ctx.heap.set_field(target.href, target.field_offset, val) {
        Ok(()) => ExecutionResult::Continue,
        Err(error) => ExecutionResult::Crash(format!("SetField: {error}")),
    }
}

// ── Arrays ─────────────────────────────────────────────────────

pub(super) fn exec_new_array(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    elem_type: u32,
) -> ExecutionResult {
    if ArrayDefaultKind::from_operand(elem_type).is_none() {
        return ExecutionResult::Crash(format!("NewArray: invalid array default kind {elem_type}"));
    }
    let href = ctx.heap.alloc_array(elem_type);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_array_init(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    elem_type: u32,
    count: u16,
    r_base: u16,
) -> ExecutionResult {
    let mut elements = Vec::with_capacity(count as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..count as usize {
            elements.push(frame.registers[r_base as usize + i]);
        }
    }
    let elem_type = match resolved_array_kind(elem_type, elements.first(), &*ctx.heap) {
        Ok(kind) => kind.operand(),
        Err(message) => return ExecutionResult::Crash(format!("ArrayInit: {message}")),
    };
    let idx = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array {
        elements: elems, ..
    }) = ctx.heap.get_object_mut(idx)
    {
        *elems = elements;
    }
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(idx);
    ExecutionResult::Continue
}

pub(super) fn exec_array_load(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_arr: u16,
    r_idx: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let idx = helpers::extract_int(&frame.registers[r_idx as usize]) as usize;
    match ctx.heap.get_object(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if idx < elements.len() {
                let val = elements[idx];
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!(
                    "array index {} out of bounds (len {})",
                    idx,
                    elements.len()
                ))
            }
        }
        _ => ExecutionResult::Crash("ArrayLoad: not an array".into()),
    }
}

pub(super) fn exec_array_store(
    ctx: &mut ExecContext<'_>,
    r_arr: u16,
    r_idx: u16,
    r_val: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let idx = helpers::extract_int(&frame.registers[r_idx as usize]) as usize;
    let val = frame.registers[r_val as usize];
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if idx < elements.len() {
                elements[idx] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!(
                    "array index {} out of bounds (len {})",
                    idx,
                    elements.len()
                ))
            }
        }
        _ => ExecutionResult::Crash("ArrayStore: not an array".into()),
    }
}

pub(super) fn exec_array_len(ctx: &mut ExecContext<'_>, r_dst: u16, r_arr: u16) -> ExecutionResult {
    let arr_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_arr as usize]);
    match ctx.heap.get_object(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            let len = elements.len() as i64;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(len);
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("ArrayLen: not an array".into()),
    }
}

pub(super) fn exec_array_resize(
    ctx: &mut ExecContext<'_>,
    r_arr: u16,
    r_new_len: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let new_len = helpers::extract_int(&frame.registers[r_new_len as usize]);
    if new_len < 0 {
        return ExecutionResult::Crash("ArrayResize: negative length".into());
    }
    let (old_len, elem_type) = match ctx.heap.get_object(arr_ref) {
        Ok(HeapObject::Array {
            elements,
            elem_type,
        }) => (elements.len(), *elem_type),
        _ => return ExecutionResult::Crash("ArrayResize: not an array".into()),
    };
    let new_len = new_len as usize;
    let default = if new_len > old_len {
        match default_value_for(elem_type, &mut *ctx.heap) {
            Ok(value) => Some(value),
            Err(message) => return ExecutionResult::Crash(format!("ArrayResize: {message}")),
        }
    } else {
        None
    };
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if let Some(default) = default {
                elements.resize(new_len, default);
            } else {
                elements.truncate(new_len);
            }
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("ArrayResize: not an array".into()),
    }
}

fn resolved_array_kind(
    operand: u32,
    prototype: Option<&Value>,
    heap: &dyn crate::gc::GcHeap,
) -> Result<ArrayDefaultKind, String> {
    let kind = ArrayDefaultKind::from_operand(operand)
        .ok_or_else(|| format!("invalid array default kind {operand}"))?;
    if kind == ArrayDefaultKind::Unavailable {
        Ok(prototype
            .map(|value| infer_array_kind(value, heap))
            .unwrap_or(ArrayDefaultKind::Unavailable))
    } else {
        Ok(kind)
    }
}

fn infer_array_kind(value: &Value, heap: &dyn crate::gc::GcHeap) -> ArrayDefaultKind {
    match value {
        Value::Int(_) => ArrayDefaultKind::Int,
        Value::Float(_) => ArrayDefaultKind::Float,
        Value::Bool(_) => ArrayDefaultKind::Bool,
        Value::Ref(href) => match heap.get_object(*href) {
            Ok(HeapObject::String(_)) => ArrayDefaultKind::String,
            _ => ArrayDefaultKind::NullReference,
        },
        Value::Entity(_) | Value::Void => ArrayDefaultKind::NullReference,
        Value::Struct { .. } => ArrayDefaultKind::Unavailable,
    }
}

fn default_value_for(operand: u32, heap: &mut dyn crate::gc::GcHeap) -> Result<Value, String> {
    match ArrayDefaultKind::from_operand(operand) {
        Some(ArrayDefaultKind::Int) => Ok(Value::Int(0)),
        Some(ArrayDefaultKind::Float) => Ok(Value::Float(0.0)),
        Some(ArrayDefaultKind::Bool) => Ok(Value::Bool(false)),
        Some(ArrayDefaultKind::String) => Ok(Value::Ref(heap.alloc_string(""))),
        Some(ArrayDefaultKind::NullReference) => Ok(Value::Void),
        Some(ArrayDefaultKind::Unavailable) => {
            Err("element type has no runtime default; create it from a concrete value first".into())
        }
        None => Err(format!("invalid array default kind {operand}")),
    }
}

pub(super) fn exec_array_copy(
    ctx: &mut ExecContext<'_>,
    r_dst_arr: u16,
    r_dst_idx: u16,
    r_src_arr: u16,
    r_src_idx: u16,
    r_len: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let dst_ref = helpers::extract_ref(&frame.registers[r_dst_arr as usize]);
    let dst_idx = helpers::extract_int(&frame.registers[r_dst_idx as usize]) as usize;
    let src_ref = helpers::extract_ref(&frame.registers[r_src_arr as usize]);
    let src_idx = helpers::extract_int(&frame.registers[r_src_idx as usize]) as usize;
    let len = helpers::extract_int(&frame.registers[r_len as usize]) as usize;

    if dst_ref == src_ref {
        // Same array — use copy_within for memmove semantics (per D-09)
        match ctx.heap.get_object_mut(dst_ref) {
            Ok(HeapObject::Array { elements, .. }) => {
                if src_idx + len > elements.len() || dst_idx + len > elements.len() {
                    return ExecutionResult::Crash("ArrayCopy: out of bounds".into());
                }
                elements.copy_within(src_idx..src_idx + len, dst_idx);
                ExecutionResult::Continue
            }
            _ => ExecutionResult::Crash("ArrayCopy: not an array".into()),
        }
    } else {
        // Different arrays — clone elements from src, write to dst
        // Read source elements first (immutable borrow)
        let src_elems = match ctx.heap.get_object(src_ref) {
            Ok(HeapObject::Array { elements, .. }) => {
                if src_idx + len > elements.len() {
                    return ExecutionResult::Crash("ArrayCopy: source out of bounds".into());
                }
                elements[src_idx..src_idx + len].to_vec()
            }
            _ => return ExecutionResult::Crash("ArrayCopy: source not an array".into()),
        };
        // Write to destination (mutable borrow)
        match ctx.heap.get_object_mut(dst_ref) {
            Ok(HeapObject::Array { elements, .. }) => {
                if dst_idx + len > elements.len() {
                    return ExecutionResult::Crash("ArrayCopy: destination out of bounds".into());
                }
                elements[dst_idx..dst_idx + len].clone_from_slice(&src_elems);
                ExecutionResult::Continue
            }
            _ => ExecutionResult::Crash("ArrayCopy: destination not an array".into()),
        }
    }
}

pub(super) fn exec_new_array_sized(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    elem_type: u32,
    r_len: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let len = helpers::extract_int(&frame.registers[r_len as usize]);
    if len < 0 {
        return ExecutionResult::Crash("NewArraySized: negative length".into());
    }
    let len = len as usize;
    if ArrayDefaultKind::from_operand(elem_type).is_none() {
        return ExecutionResult::Crash(format!(
            "NewArraySized: invalid array default kind {elem_type}"
        ));
    }
    let default = if len == 0 {
        None
    } else {
        match default_value_for(elem_type, &mut *ctx.heap) {
            Ok(value) => Some(value),
            Err(message) => return ExecutionResult::Crash(format!("NewArraySized: {message}")),
        }
    };
    let elements = default.map(|value| vec![value; len]).unwrap_or_default();
    let href = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array {
        elements: elems, ..
    }) = ctx.heap.get_object_mut(href)
    {
        *elems = elements;
    }
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_new_array_filled(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    elem_type: u32,
    r_len: u16,
    r_fill: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let len = helpers::extract_int(&frame.registers[r_len as usize]);
    if len < 0 {
        return ExecutionResult::Crash("NewArrayFilled: negative length".into());
    }
    let len = len as usize;
    let fill_val = frame.registers[r_fill as usize];
    let elem_type = match resolved_array_kind(elem_type, Some(&fill_val), &*ctx.heap) {
        Ok(kind) => kind.operand(),
        Err(message) => return ExecutionResult::Crash(format!("NewArrayFilled: {message}")),
    };
    let elements = vec![fill_val; len];
    let href = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array {
        elements: elems, ..
    }) = ctx.heap.get_object_mut(href)
    {
        *elems = elements;
    }
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_array_slice(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_arr: u16,
    r_start: u16,
    r_end: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let start = helpers::extract_int(&frame.registers[r_start as usize]) as usize;
    let end = helpers::extract_int(&frame.registers[r_end as usize]) as usize;
    match ctx.heap.get_object(arr_ref) {
        Ok(HeapObject::Array {
            elem_type,
            elements,
        }) => {
            let et = *elem_type;
            if start <= end && end <= elements.len() {
                let slice = elements[start..end].to_vec();
                let new_href = ctx.heap.alloc_array(et);
                if let Ok(HeapObject::Array {
                    elements: elems, ..
                }) = ctx.heap.get_object_mut(new_href)
                {
                    *elems = slice;
                }
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = Value::Ref(new_href);
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!(
                    "ArraySlice: range {}..{} out of bounds",
                    start, end
                ))
            }
        }
        _ => ExecutionResult::Crash("ArraySlice: not an array".into()),
    }
}

// ── Option ─────────────────────────────────────────────────────

pub(super) fn exec_wrap_some(ctx: &mut ExecContext<'_>, r_dst: u16, r_val: u16) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize];
    let href = ctx.heap.alloc_enum(0, 1, vec![val]); // tag 1 = Some
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_unwrap(ctx: &mut ExecContext<'_>, r_dst: u16, r_opt: u16) -> ExecutionResult {
    let opt_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
    match ctx.heap.get_object(opt_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 1 && !fields.is_empty() {
                let val = fields[0];
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash("unwrap called on None".into())
            }
        }
        _ => ExecutionResult::Crash("Unwrap: not an Option".into()),
    }
}

pub(super) fn exec_is_some(ctx: &mut ExecContext<'_>, r_dst: u16, r_opt: u16) -> ExecutionResult {
    let opt_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
    let is_some = match ctx.heap.get_object(opt_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 1,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_some);
    ExecutionResult::Continue
}

pub(super) fn exec_is_none(ctx: &mut ExecContext<'_>, r_dst: u16, r_opt: u16) -> ExecutionResult {
    let opt_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
    let is_none = match ctx.heap.get_object(opt_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 0,
        _ => true,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_none);
    ExecutionResult::Continue
}

// ── Result ─────────────────────────────────────────────────────

pub(super) fn exec_wrap_ok(ctx: &mut ExecContext<'_>, r_dst: u16, r_val: u16) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize];
    let href = ctx.heap.alloc_enum(0, 0, vec![val]); // tag 0 = Ok
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_wrap_err(ctx: &mut ExecContext<'_>, r_dst: u16, r_err: u16) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_err as usize];
    let href = ctx.heap.alloc_enum(0, 1, vec![val]); // tag 1 = Err
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_unwrap_ok(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_result: u16,
) -> ExecutionResult {
    let res_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 0 && !fields.is_empty() {
                let val = fields[0];
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash("unwrap_ok called on Err".into())
            }
        }
        _ => ExecutionResult::Crash("UnwrapOk: not a Result".into()),
    }
}

pub(super) fn exec_is_ok(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    let is_ok = match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 0,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_ok);
    ExecutionResult::Continue
}

pub(super) fn exec_is_err(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    let is_err = match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 1,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_err);
    ExecutionResult::Continue
}

pub(super) fn exec_extract_err(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_result: u16,
) -> ExecutionResult {
    let res_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 1 && !fields.is_empty() {
                let val = fields[0];
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash("ExtractErr called on Ok".into())
            }
        }
        _ => ExecutionResult::Crash("ExtractErr: not a Result".into()),
    }
}

// ── Enum ───────────────────────────────────────────────────────

pub(super) fn exec_new_enum(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    type_idx: u32,
    tag: u16,
    field_count: u16,
    r_base: u16,
) -> ExecutionResult {
    let mut fields = Vec::with_capacity(field_count as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..field_count as usize {
            fields.push(frame.registers[r_base as usize + i]);
        }
    }
    let href = ctx.heap.alloc_enum(type_idx, tag, fields);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_get_tag(ctx: &mut ExecContext<'_>, r_dst: u16, r_enum: u16) -> ExecutionResult {
    let enum_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_enum as usize]);
    match ctx.heap.get_object(enum_ref) {
        Ok(HeapObject::Enum { tag, .. }) => {
            let tag_val = *tag as i64;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(tag_val);
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("GetTag: not an enum".into()),
    }
}

pub(super) fn exec_extract_field(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_enum: u16,
    field_idx: u16,
) -> ExecutionResult {
    let enum_ref =
        helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_enum as usize]);
    match ctx.heap.get_object(enum_ref) {
        Ok(HeapObject::Enum { fields, .. }) => {
            let idx = field_idx as usize;
            if idx < fields.len() {
                let val = fields[idx];
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("ExtractField: index {} out of range", idx))
            }
        }
        _ => ExecutionResult::Crash("ExtractField: not an enum".into()),
    }
}
