use crate::heap::HeapObject;
use crate::value::Value;

use super::{helpers, ExecContext, ExecutionResult};

// ── Struct Object Model ────────────────────────────────────────

pub(super) fn exec_new(ctx: &mut ExecContext<'_>, r_dst: u16, type_idx: u32) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let field_count = helpers::get_type_field_count(&module.module, type_idx);

    // type_idx is a MetadataToken: high byte = table ID, low 24 bits = 1-based row.
    // Strip the table bits to get the 1-based row, then convert to 0-based array index.
    let row = type_idx & 0x00FF_FFFF;
    let idx = row.saturating_sub(1) as usize;
    let kind_u8 = module.module.type_defs.get(idx).map(|t| t.kind);

    match kind_u8.and_then(writ_module::TypeDefKind::from_u8) {
        Some(writ_module::TypeDefKind::Struct) => {
            // kind=0: value-type struct — inline in register, no heap allocation
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::InlineStruct {
                type_idx,
                fields: vec![Value::Void; field_count],
            };
            ExecutionResult::Continue
        }
        Some(writ_module::TypeDefKind::Class) | Some(writ_module::TypeDefKind::Entity) => {
            // kind=4 (class) or kind=2 (entity): heap allocation (existing behavior preserved)
            let href = ctx.heap.alloc_struct(field_count);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        Some(other) => ExecutionResult::Crash(format!(
            "NEW: type_idx {} has kind {:?}, expected struct or class",
            type_idx, other
        )),
        None => ExecutionResult::Crash(format!(
            "NEW: type_idx {} is out of range",
            type_idx
        )),
    }
}

pub(super) fn exec_get_field(
    ctx: &mut ExecContext<'_>,
    r_dst: u16,
    r_obj: u16,
    field_idx: u32,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let obj_val = &frame.registers[r_obj as usize];
    match obj_val {
        Value::InlineStruct { fields, .. } => {
            let idx = field_idx as usize;
            match fields.get(idx) {
                Some(val) => {
                    let val = val.clone();
                    let frame = ctx.task.call_stack.last_mut().unwrap();
                    frame.registers[r_dst as usize] = val;
                    ExecutionResult::Continue
                }
                None => ExecutionResult::Crash(format!(
                    "GetField: field index {} out of range for struct with {} fields",
                    idx,
                    fields.len()
                )),
            }
        }
        Value::Ref(_) | Value::Entity(_) => {
            // Existing heap/entity path
            let href = helpers::extract_ref(obj_val);
            match ctx.heap.get_field(href, field_idx as usize) {
                Ok(val) => {
                    let frame = ctx.task.call_stack.last_mut().unwrap();
                    frame.registers[r_dst as usize] = val;
                    ExecutionResult::Continue
                }
                Err(e) => ExecutionResult::Crash(format!("GetField: {}", e)),
            }
        }
        other => ExecutionResult::Crash(format!(
            "GetField: expected struct or class, got {:?}",
            other
        )),
    }
}

pub(super) fn exec_set_field(
    ctx: &mut ExecContext<'_>,
    r_obj: u16,
    field_idx: u32,
    r_val: u16,
) -> ExecutionResult {
    let idx = field_idx as usize;
    // Clone the value to store BEFORE taking mutable reference to the object register
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize].clone();

    let frame = ctx.task.call_stack.last_mut().unwrap();
    match &mut frame.registers[r_obj as usize] {
        Value::InlineStruct { fields, .. } => {
            if idx < fields.len() {
                fields[idx] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!(
                    "SetField: field index {} out of range for struct with {} fields",
                    idx,
                    fields.len()
                ))
            }
        }
        Value::Ref(href) => {
            // Copy href (HeapRef is Copy) so we can drop the frame borrow
            let href = *href;
            // End the mutable borrow of frame by shadowing it
            let _ = frame;
            match ctx.heap.set_field(href, idx, val) {
                Ok(()) => ExecutionResult::Continue,
                Err(e) => ExecutionResult::Crash(format!("SetField: {}", e)),
            }
        }
        _ => ExecutionResult::Crash("SetField: not a struct or class".into()),
    }
}

// ── Arrays ─────────────────────────────────────────────────────

pub(super) fn exec_new_array(ctx: &mut ExecContext<'_>, r_dst: u16, elem_type: u32) -> ExecutionResult {
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
            elements.push(frame.registers[r_base as usize + i].clone());
        }
    }
    let idx = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array { elements: elems, .. }) = ctx.heap.get_object_mut(idx) {
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
                let val = elements[idx].clone();
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("array index {} out of bounds (len {})", idx, elements.len()))
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
    let val = frame.registers[r_val as usize].clone();
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if idx < elements.len() {
                elements[idx] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("array index {} out of bounds (len {})", idx, elements.len()))
            }
        }
        _ => ExecutionResult::Crash("ArrayStore: not an array".into()),
    }
}

pub(super) fn exec_array_len(ctx: &mut ExecContext<'_>, r_dst: u16, r_arr: u16) -> ExecutionResult {
    let arr_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_arr as usize]);
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

pub(super) fn exec_array_add(ctx: &mut ExecContext<'_>, r_arr: u16, r_val: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let val = frame.registers[r_val as usize].clone();
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            elements.push(val);
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("ArrayAdd: not an array".into()),
    }
}

pub(super) fn exec_array_remove(ctx: &mut ExecContext<'_>, r_arr: u16, r_idx: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let idx = helpers::extract_int(&frame.registers[r_idx as usize]) as usize;
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if idx < elements.len() {
                elements.remove(idx);
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("ArrayRemove: index {} out of bounds", idx))
            }
        }
        _ => ExecutionResult::Crash("ArrayRemove: not an array".into()),
    }
}

pub(super) fn exec_array_insert(
    ctx: &mut ExecContext<'_>,
    r_arr: u16,
    r_idx: u16,
    r_val: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let idx = helpers::extract_int(&frame.registers[r_idx as usize]) as usize;
    let val = frame.registers[r_val as usize].clone();
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, .. }) => {
            if idx <= elements.len() {
                elements.insert(idx, val);
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("ArrayInsert: index {} out of bounds", idx))
            }
        }
        _ => ExecutionResult::Crash("ArrayInsert: not an array".into()),
    }
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
        Ok(HeapObject::Array { elem_type, elements }) => {
            let et = *elem_type;
            if start <= end && end <= elements.len() {
                let slice = elements[start..end].to_vec();
                let new_href = ctx.heap.alloc_array(et);
                if let Ok(HeapObject::Array { elements: elems, .. }) = ctx.heap.get_object_mut(new_href) {
                    *elems = slice;
                }
                let frame = ctx.task.call_stack.last_mut().unwrap();
                frame.registers[r_dst as usize] = Value::Ref(new_href);
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!("ArraySlice: range {}..{} out of bounds", start, end))
            }
        }
        _ => ExecutionResult::Crash("ArraySlice: not an array".into()),
    }
}

// ── Option ─────────────────────────────────────────────────────

pub(super) fn exec_wrap_some(ctx: &mut ExecContext<'_>, r_dst: u16, r_val: u16) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize].clone();
    let href = ctx.heap.alloc_enum(0, 1, vec![val]); // tag 1 = Some
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_unwrap(ctx: &mut ExecContext<'_>, r_dst: u16, r_opt: u16) -> ExecutionResult {
    let opt_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
    match ctx.heap.get_object(opt_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 1 && !fields.is_empty() {
                let val = fields[0].clone();
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
    let opt_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
    let is_some = match ctx.heap.get_object(opt_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 1,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_some);
    ExecutionResult::Continue
}

pub(super) fn exec_is_none(ctx: &mut ExecContext<'_>, r_dst: u16, r_opt: u16) -> ExecutionResult {
    let opt_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_opt as usize]);
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
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize].clone();
    let href = ctx.heap.alloc_enum(0, 0, vec![val]); // tag 0 = Ok
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_wrap_err(ctx: &mut ExecContext<'_>, r_dst: u16, r_err: u16) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_err as usize].clone();
    let href = ctx.heap.alloc_enum(0, 1, vec![val]); // tag 1 = Err
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_unwrap_ok(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 0 && !fields.is_empty() {
                let val = fields[0].clone();
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
    let res_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    let is_ok = match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 0,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_ok);
    ExecutionResult::Continue
}

pub(super) fn exec_is_err(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    let is_err = match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, .. }) => *tag == 1,
        _ => false,
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(is_err);
    ExecutionResult::Continue
}

pub(super) fn exec_extract_err(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
    match ctx.heap.get_object(res_ref) {
        Ok(HeapObject::Enum { tag, fields, .. }) => {
            if *tag == 1 && !fields.is_empty() {
                let val = fields[0].clone();
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
            fields.push(frame.registers[r_base as usize + i].clone());
        }
    }
    let href = ctx.heap.alloc_enum(type_idx, tag, fields);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_get_tag(ctx: &mut ExecContext<'_>, r_dst: u16, r_enum: u16) -> ExecutionResult {
    let enum_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_enum as usize]);
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
    let enum_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_enum as usize]);
    match ctx.heap.get_object(enum_ref) {
        Ok(HeapObject::Enum { fields, .. }) => {
            let idx = field_idx as usize;
            if idx < fields.len() {
                let val = fields[idx].clone();
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
