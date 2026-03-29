use crate::heap::HeapObject;
use crate::value::Value;

use super::{helpers, ExecContext, ExecutionResult};

// ── Struct Object Model ────────────────────────────────────────

pub(super) fn exec_new(ctx: &mut ExecContext<'_>, r_dst: u16, type_idx: u32) -> ExecutionResult {
    let table_id = (type_idx >> 24) as u8;
    let row = type_idx & 0x00FF_FFFF;

    // Resolve the target module and typedef index.
    // TypeDef tokens (table 2) reference the current module directly.
    // TypeRef tokens (table 3) require cross-module resolution.
    let (target_module_idx, target_typedef_idx) = if table_id == 3 {
        // TypeRef — resolve through the domain's cross-module resolution
        let module = &ctx.modules[ctx.current_module_idx];
        let typeref_row_0based = row.saturating_sub(1) as u32;
        if let Some(resolved) = module.resolved_refs.types.get(&typeref_row_0based) {
            (resolved.module_idx, resolved.typedef_idx as usize)
        } else {
            return ExecutionResult::Crash(format!(
                "NEW: TypeRef row {} not resolved", typeref_row_0based
            ));
        }
    } else {
        // TypeDef — current module, convert 1-based row to 0-based index
        (ctx.current_module_idx, row.saturating_sub(1) as usize)
    };

    let target_module = &ctx.modules[target_module_idx];
    // For TypeRef tokens, build a synthetic TypeDef token for the resolved typedef index
    // so get_type_field_count can decode it correctly against the target module.
    let resolved_type_idx = if table_id == 3 {
        // Encode as TypeDef token (table_id=2) with 1-based row in the target module
        (2u32 << 24) | ((target_typedef_idx as u32) + 1)
    } else {
        type_idx
    };
    let field_count = helpers::get_type_field_count(&target_module.module, resolved_type_idx);
    let kind_u8 = target_module.module.type_defs.get(target_typedef_idx).map(|t| t.kind);

    match kind_u8.and_then(writ_module::TypeDefKind::from_u8) {
        Some(writ_module::TypeDefKind::Struct) => {
            // kind=0: value-type struct — heap allocation with Copy-semantic HeapRef.
            // The type_key is not needed in the HeapObject because type_idx is carried
            // directly in Value::Struct for virtual dispatch.
            let href = ctx.heap.alloc_struct(u32::MAX, field_count);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Struct { type_idx, href };
            ExecutionResult::Continue
        }
        Some(writ_module::TypeDefKind::Class) | Some(writ_module::TypeDefKind::Entity) => {
            // kind=4 (class) or kind=2 (entity): heap allocation.
            // Encode the type_key as (target_module_idx << 16) | target_typedef_idx so that
            // CALL_VIRT can resolve the dispatch table entry from the runtime object type.
            let class_type_key = ((target_module_idx as u32) << 16) | (target_typedef_idx as u32);
            let href = ctx.heap.alloc_struct(class_type_key, field_count);
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
        Value::Struct { href, .. } => {
            let href = *href;
            match ctx.heap.get_field(href, field_idx as usize) {
                Ok(val) => {
                    let frame = ctx.task.call_stack.last_mut().unwrap();
                    frame.registers[r_dst as usize] = val;
                    ExecutionResult::Continue
                }
                Err(e) => ExecutionResult::Crash(format!("GetField: {}", e)),
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
    // Copy the value to store BEFORE taking mutable reference to the object register
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize];

    let frame = ctx.task.call_stack.last_mut().unwrap();
    match &mut frame.registers[r_obj as usize] {
        Value::Struct { href, .. } => {
            let href = *href;
            let _ = frame;
            match ctx.heap.set_field(href, idx, val) {
                Ok(()) => ExecutionResult::Continue,
                Err(e) => ExecutionResult::Crash(format!("SetField: {}", e)),
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
            elements.push(frame.registers[r_base as usize + i]);
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
                let val = elements[idx];
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
    let val = frame.registers[r_val as usize];
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
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, elem_type }) => {
            let new_len = new_len as usize;
            let et = *elem_type;
            if new_len > elements.len() {
                let default = default_value_for(et);
                elements.resize(new_len, default);
            } else {
                elements.truncate(new_len);
            }
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("ArrayResize: not an array".into()),
    }
}

fn default_value_for(elem_type: u32) -> Value {
    // elem_type encoding: 0=int, 1=float, 2=bool; others (string, reference) → Void
    match elem_type {
        0 => Value::Int(0),
        1 => Value::Float(0.0),
        2 => Value::Bool(false),
        _ => Value::Void,
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
    let default = default_value_for(elem_type);
    let elements = vec![default; len];
    let href = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array { elements: elems, .. }) = ctx.heap.get_object_mut(href) {
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
    let elements = vec![fill_val; len];
    let href = ctx.heap.alloc_array(elem_type);
    if let Ok(HeapObject::Array { elements: elems, .. }) = ctx.heap.get_object_mut(href) {
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
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize];
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

pub(super) fn exec_unwrap_ok(ctx: &mut ExecContext<'_>, r_dst: u16, r_result: u16) -> ExecutionResult {
    let res_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_result as usize]);
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
