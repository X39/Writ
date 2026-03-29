use crate::heap::HeapObject;
use crate::reflection::ReflectionIndex;
use crate::value::Value;

use super::{helpers, ExecContext, ExecutionResult, IntrinsicId};

/// Execute an intrinsic operation and store the result in r_dst.
///
/// For binary operators: self = r_obj = r_base, argument = r_base+1.
/// For unary operators: self = r_obj = r_base.
pub(super) fn execute_intrinsic(
    ctx: &mut ExecContext<'_>,
    id: IntrinsicId,
    r_dst: u16,
    r_obj: u16,
    r_base: u16,
    _argc: u16,
) -> ExecutionResult {
    match id {
        // ── Int arithmetic ────────────────────────────────────
        IntrinsicId::IntAdd => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a.wrapping_add(b));
            ExecutionResult::Continue
        }
        IntrinsicId::IntSub => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a.wrapping_sub(b));
            ExecutionResult::Continue
        }
        IntrinsicId::IntMul => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a.wrapping_mul(b));
            ExecutionResult::Continue
        }
        IntrinsicId::IntDiv => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            if b == 0 {
                return ExecutionResult::Crash("division by zero".into());
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a / b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntMod => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            if b == 0 {
                return ExecutionResult::Crash("division by zero".into());
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a % b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntNeg => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(-a);
            ExecutionResult::Continue
        }
        IntrinsicId::IntNot => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(!a);
            ExecutionResult::Continue
        }
        IntrinsicId::IntEq => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(a == b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntOrd => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(a < b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntBitAnd => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a & b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntBitOr => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a | b);
            ExecutionResult::Continue
        }
        IntrinsicId::IntIntoFloat => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a as f64);
            ExecutionResult::Continue
        }
        IntrinsicId::IntIntoString => {
            let a = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = a.to_string();
            let href = ctx.heap.alloc_string(&s);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }

        // ── Float arithmetic ──────────────────────────────────
        IntrinsicId::FloatAdd => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a + b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatSub => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a - b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatMul => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a * b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatDiv => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a / b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatMod => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(a % b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatNeg => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(-a);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatEq => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(a == b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatOrd => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(a < b);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatIntoInt => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(a as i64);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatIntoString => {
            let a = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = a.to_string();
            let href = ctx.heap.alloc_string(&s);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }

        // ── Bool ──────────────────────────────────────────────
        IntrinsicId::BoolEq => {
            let a = helpers::extract_bool(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let b = helpers::extract_bool(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(a == b);
            ExecutionResult::Continue
        }
        IntrinsicId::BoolNot => {
            let a = helpers::extract_bool(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(!a);
            ExecutionResult::Continue
        }
        IntrinsicId::BoolIntoString => {
            let a = helpers::extract_bool(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = a.to_string();
            let href = ctx.heap.alloc_string(&s);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }

        // ── String ────────────────────────────────────────────
        IntrinsicId::StringAdd => {
            let href_a = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let href_b = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let sa = match ctx.heap.read_string(href_a) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("StringAdd: left operand not a string".into()),
            };
            let sb = match ctx.heap.read_string(href_b) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("StringAdd: right operand not a string".into()),
            };
            let result = format!("{}{}", sa, sb);
            let href = ctx.heap.alloc_string(&result);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        IntrinsicId::StringEq => {
            let href_a = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let href_b = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let sa = ctx.heap.read_string(href_a).map(|s| s.to_string()).unwrap_or_default();
            let sb = ctx.heap.read_string(href_b).map(|s| s.to_string()).unwrap_or_default();
            let eq = sa == sb;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(eq);
            ExecutionResult::Continue
        }
        IntrinsicId::StringOrd => {
            let href_a = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let href_b = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]);
            let sa = ctx.heap.read_string(href_a).map(|s| s.to_string()).unwrap_or_default();
            let sb = ctx.heap.read_string(href_b).map(|s| s.to_string()).unwrap_or_default();
            let lt = sa < sb;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(lt);
            ExecutionResult::Continue
        }
        IntrinsicId::StringIndexChar => {
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let idx = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]) as usize;
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("StringIndexChar: not a string".into()),
            };
            if idx >= s.len() {
                return ExecutionResult::Crash(format!("string index {} out of bounds (len {})", idx, s.len()));
            }
            let ch = &s[idx..idx + 1];
            let href = ctx.heap.alloc_string(ch);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        IntrinsicId::StringIndexRange => {
            // Range-based string slicing (placeholder for full Range support)
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("StringIndexRange: not a string".into()),
            };
            let result_href = ctx.heap.alloc_string(&s);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(result_href);
            ExecutionResult::Continue
        }
        IntrinsicId::StringIntoString => {
            // Identity conversion
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = frame.registers[r_obj as usize];
            ExecutionResult::Continue
        }
        IntrinsicId::StringIntoInt => {
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("string.into_int(): not a string".into()),
            };
            let v: i64 = match s.trim().parse() {
                Ok(n) => n,
                Err(_) => return ExecutionResult::Crash(format!("string.into_int(): cannot parse {:?} as int", s)),
            };
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(v);
            ExecutionResult::Continue
        }
        IntrinsicId::StringIntoFloat => {
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("string.into_float(): not a string".into()),
            };
            let v: f64 = match s.trim().parse() {
                Ok(n) => n,
                Err(_) => return ExecutionResult::Crash(format!("string.into_float(): cannot parse {:?} as float", s)),
            };
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Float(v);
            ExecutionResult::Continue
        }
        IntrinsicId::StringIntoBool => {
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("string.into_bool(): not a string".into()),
            };
            let v: bool = match s.trim() {
                "true" => true,
                "false" => false,
                other => return ExecutionResult::Crash(format!("string.into_bool(): cannot parse {:?} as bool", other)),
            };
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(v);
            ExecutionResult::Continue
        }

        // ── Array ─────────────────────────────────────────────
        IntrinsicId::ArrayIndex => {
            let arr_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let idx = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]) as usize;
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
                _ => ExecutionResult::Crash("ArrayIndex: not an array".into()),
            }
        }
        IntrinsicId::ArrayIndexSet => {
            let arr_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let idx = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]) as usize;
            let val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2];
            match ctx.heap.get_object_mut(arr_ref) {
                Ok(HeapObject::Array { elements, .. }) => {
                    if idx < elements.len() {
                        elements[idx] = val;
                        let frame = ctx.task.call_stack.last_mut().unwrap();
                        frame.registers[r_dst as usize] = Value::Void;
                        ExecutionResult::Continue
                    } else {
                        ExecutionResult::Crash(format!("array index {} out of bounds (len {})", idx, elements.len()))
                    }
                }
                _ => ExecutionResult::Crash("ArrayIndexSet: not an array".into()),
            }
        }
        IntrinsicId::ArraySlice => {
            let arr_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let start = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]) as usize;
            let end = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2]) as usize;
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
        IntrinsicId::ArrayIterable => {
            // Return the array itself as its own iterator (simplified)
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = frame.registers[r_obj as usize];
            ExecutionResult::Continue
        }

        // ── Reflection get_type ────────────────────────────────
        IntrinsicId::IntGetType => {
            let href = ctx.reflection.get_or_alloc_primitive_type("Int", ctx.heap);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatGetType => {
            let href = ctx.reflection.get_or_alloc_primitive_type("Float", ctx.heap);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        IntrinsicId::BoolGetType => {
            let href = ctx.reflection.get_or_alloc_primitive_type("Bool", ctx.heap);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        IntrinsicId::StringGetType => {
            let href = ctx.reflection.get_or_alloc_primitive_type("String", ctx.heap);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }

        // ── Reflection — Type methods ─────────────────────────

        IntrinsicId::TypeFields => {
            // r_obj = Type heap object; recover (module_idx, typedef_idx) from reverse map.
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (module_idx, typedef_idx) = match ctx.reflection.lookup_type_identity(type_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("TypeFields: not a Type object".into()),
            };
            // Iterate field range for this typedef and allocate FieldInfo objects.
            let (field_start, field_end) = crate::reflection::ReflectionIndex::typedef_field_range_pub(
                ctx.modules, module_idx, typedef_idx
            );
            let arr_href = ctx.heap.alloc_array(0);
            for offset in 0..(field_end.saturating_sub(field_start)) {
                let fi = ctx.reflection.get_or_alloc_field_info(
                    module_idx, typedef_idx, offset, ctx.heap, ctx.modules
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(fi));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        IntrinsicId::TypeMethods => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (module_idx, typedef_idx) = match ctx.reflection.lookup_type_identity(type_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("TypeMethods: not a Type object".into()),
            };
            let (method_start, method_end) = crate::reflection::ReflectionIndex::typedef_method_range_pub(
                ctx.modules, module_idx, typedef_idx
            );
            let arr_href = ctx.heap.alloc_array(0);
            for method_idx in method_start..method_end {
                let mi = ctx.reflection.get_or_alloc_method_info(
                    module_idx, method_idx, ctx.heap, ctx.modules
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(mi));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        IntrinsicId::TypeAttributes => {
            // Replicate Domain::query_attributes_on logic directly using ctx.modules.
            use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};

            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (module_idx, typedef_idx) = match ctx.reflection.lookup_type_identity(type_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("TypeAttributes: not a Type object".into()),
            };

            // Collect attribute rows for this typedef
            let mut attr_data: Vec<(String, Vec<writ_module::attr::AttrValue>)> = Vec::new();
            {
                let module = &ctx.modules[module_idx].module;
                let target_row = (typedef_idx + 1) as u32;
                for row in &module.attribute_defs {
                    if row.owner_kind == ATTR_OWNER_KIND_DECL { continue; }
                    if row.owner.table_id() != TableId::TypeDef.as_u8() { continue; }
                    if row.owner.row_index() != Some(target_row) { continue; }
                    let name = writ_module::heap::read_string(&module.string_heap, row.name)
                        .unwrap_or("<unknown>").to_owned();
                    let args = if row.value == 0 {
                        Vec::new()
                    } else {
                        match writ_module::heap::read_blob(&module.blob_heap, row.value) {
                            Ok(blob) => writ_module::attr::decode_attr_args(blob).unwrap_or_default(),
                            Err(_) => Vec::new(),
                        }
                    };
                    attr_data.push((name, args));
                }
            }

            let arr_href = ctx.heap.alloc_array(0);
            for (ordinal, (name, args)) in attr_data.iter().enumerate() {
                let ai = ctx.reflection.get_or_alloc_attribute_info(
                    name, args, ordinal, module_idx, typedef_idx, ctx.heap
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(ai));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        IntrinsicId::TypeContracts => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (module_idx, typedef_idx) = match ctx.reflection.lookup_type_identity(type_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("TypeContracts: not a Type object".into()),
            };

            // Collect ImplDef rows that reference this typedef
            let mut impl_contract_idxs: Vec<usize> = Vec::new();
            {
                let module = &ctx.modules[module_idx].module;
                let target_type_row = (typedef_idx + 1) as u32;
                for impl_def in &module.impl_defs {
                    // impl_def.type_token is a TypeDef token (table_id=2, row=typedef_idx+1)
                    if impl_def.type_token.table_id() == 2 {
                        if impl_def.type_token.row_index() == Some(target_type_row) {
                            // contract is a ContractDef token (table_id=10, row=contract_idx+1)
                            if impl_def.contract.table_id() == 10 {
                                if let Some(row) = impl_def.contract.row_index() {
                                    impl_contract_idxs.push((row - 1) as usize);
                                }
                            }
                        }
                    }
                }
            }

            let arr_href = ctx.heap.alloc_array(0);
            for contract_idx in impl_contract_idxs {
                let ci = ctx.reflection.get_or_alloc_contract_info(
                    module_idx, contract_idx, ctx.heap, ctx.modules
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(ci));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        IntrinsicId::TypeImplements => {
            // r_obj = Type heap object, r_base+1 = contract Type heap object to check
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let contract_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1]
            );
            let (module_idx, typedef_idx) = match ctx.reflection.lookup_type_identity(type_href) {
                Some(id) => id,
                None => {
                    let frame = ctx.task.call_stack.last_mut().unwrap();
                    frame.registers[r_dst as usize] = Value::Bool(false);
                    return ExecutionResult::Continue;
                }
            };
            // Look up the contract name from the contract_href Type object (field 0 = name)
            let contract_name = match ctx.heap.get_field(contract_href, 0) {
                Ok(Value::Ref(name_href)) => {
                    ctx.heap.read_string(name_href).unwrap_or("").to_owned()
                }
                _ => String::new(),
            };

            let mut found = false;
            {
                let module = &ctx.modules[module_idx].module;
                let target_type_row = (typedef_idx + 1) as u32;
                'outer: for impl_def in &module.impl_defs {
                    if impl_def.type_token.table_id() == 2
                        && impl_def.type_token.row_index() == Some(target_type_row)
                        && impl_def.contract.table_id() == 10
                    {
                        if let Some(row) = impl_def.contract.row_index() {
                            let contract_idx = (row - 1) as usize;
                            if contract_idx < module.contract_defs.len() {
                                let cd = &module.contract_defs[contract_idx];
                                let name = writ_module::heap::read_string(
                                    &module.string_heap, cd.name
                                ).unwrap_or("");
                                if name == contract_name {
                                    found = true;
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Bool(found);
            ExecutionResult::Continue
        }

        IntrinsicId::TypeGetName => {
            // Read field 0 (name) from the Type heap object
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(type_href, 0)
                .unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::TypeGetNamespace => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(type_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::TypeGetKind => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(type_href, 2).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::TypeGetIsGeneric => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(type_href, 3).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        // ── Reflection — FieldInfo methods ────────────────────

        IntrinsicId::FieldInfoGet => {
            // r_obj = FieldInfo heap object (always a class/Ref), r_base+1 = instance (may be Struct or Ref)
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
            // Extract HeapRef from either Value::Ref or Value::Struct
            let instance_href = match instance_val {
                Value::Ref(href) => href,
                Value::Struct { href, .. } => href,
                _ => return ExecutionResult::Crash(
                    "FieldInfo.get: instance argument is not a struct or ref".into()
                ),
            };
            // Recover field identity from reverse map
            let (_module_idx, _typedef_idx, field_offset) =
                match ctx.reflection.lookup_field_identity(fi_href) {
                    Some(id) => id,
                    None => return ExecutionResult::Crash(
                        "FieldInfo.get: not a FieldInfo object".into()
                    ),
                };
            // field_offset within the typedef is the heap object field index.
            // The struct's field at offset N corresponds to heap field index N.
            let val = ctx.heap.get_field(instance_href, field_offset)
                .unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::FieldInfoGetName => {
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(fi_href, 0).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::FieldInfoGetDeclaredType => {
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(fi_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::FieldInfoGetIsMutable => {
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(fi_href, 2).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::FieldInfoSet => {
            // r_obj = FieldInfo heap object, r_base+1 = instance (Ref or Struct), r_base+2 = new value
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
            let new_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2];

            let instance_href = match instance_val {
                Value::Ref(href) => href,
                Value::Struct { href, .. } => href,
                _ => return ExecutionResult::Crash(
                    "FieldInfo.set: instance argument is not a struct or ref".into()
                ),
            };

            let (module_idx, typedef_idx, field_offset) =
                match ctx.reflection.lookup_field_identity(fi_href) {
                    Some(id) => id,
                    None => return ExecutionResult::Crash(
                        "FieldInfo.set: not a FieldInfo object".into()
                    ),
                };

            // Compute the absolute field index in the module's field_defs table
            let td = &ctx.modules[module_idx].module.type_defs[typedef_idx];
            let abs_idx = td.field_list.saturating_sub(1) as usize + field_offset;

            // Check readonly flag: bit 0x01 = FIELD_FLAG_READONLY (let field)
            let flags = ctx.modules[module_idx].module.field_defs[abs_idx].flags;
            if flags & 0x01 != 0 {
                // Field is readonly — read the field name for the error message
                let name_offset = ctx.modules[module_idx].module.field_defs[abs_idx].name;
                let field_name = writ_module::heap::read_string(
                    &ctx.modules[module_idx].module.string_heap,
                    name_offset,
                ).unwrap_or("unknown").to_owned();
                return ExecutionResult::Crash(
                    format!("Reflection write to immutable field '{}'", field_name)
                );
            }

            // Write the new value
            match ctx.heap.set_field(instance_href, field_offset, new_val) {
                Ok(()) => {}
                Err(e) => return ExecutionResult::Crash(format!("FieldInfo.set: {}", e)),
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Void;
            ExecutionResult::Continue
        }

        // ── Reflection — MethodInfo methods ───────────────────

        IntrinsicId::MethodInfoGetName => {
            let mi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(mi_href, 0).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::MethodInfoGetReturnType => {
            let mi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(mi_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::MethodInfoGetParameters => {
            let mi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(mi_href, 2).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::MethodInfoInvoke => {
            // r_obj = MethodInfo heap object, r_base+1 = instance (self), r_base+2 = args Array
            let mi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
            let args_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2];

            let args_href = match args_val {
                Value::Ref(href) => href,
                _ => return ExecutionResult::Crash(
                    "MethodInfo.invoke: args argument is not an array ref".into()
                ),
            };

            let (method_module_idx, method_idx) =
                match ctx.reflection.lookup_method_identity(mi_href) {
                    Some(id) => id,
                    None => return ExecutionResult::Crash(
                        "MethodInfo.invoke: not a MethodInfo object".into()
                    ),
                };

            if method_idx >= ctx.modules[method_module_idx].decoded_bodies.len() {
                return ExecutionResult::Crash(format!(
                    "MethodInfo.invoke: method index {} out of range", method_idx
                ));
            }

            // Extract args from the Array heap object
            let args: Vec<Value> = match ctx.heap.get_object(args_href) {
                Ok(HeapObject::Array { elements, .. }) => elements.clone(),
                _ => return ExecutionResult::Crash(
                    "MethodInfo.invoke: args is not an Array object".into()
                ),
            };

            // Validate argument count against method's param_count
            let param_count = ctx.modules[method_module_idx].module.method_defs[method_idx].param_count as usize;
            if args.len() != param_count {
                return ExecutionResult::Crash(format!(
                    "MethodInfo.invoke: expected {} args, got {}", param_count, args.len()
                ));
            }

            // Push a call frame for the target method; return Continue so the scheduler drives it
            let reg_count = ctx.modules[method_module_idx].module.method_bodies[method_idx].register_types.len();
            ctx.task.call_stack.push(crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst));
            let stack_len = ctx.task.call_stack.len();
            let callee = &mut ctx.task.call_stack[stack_len - 1];

            // r0 = instance (self), r1..rN = args
            if reg_count > 0 {
                callee.registers[0] = instance_val;
            }
            for (i, arg) in args.into_iter().enumerate() {
                let slot = i + 1;
                if slot < reg_count {
                    callee.registers[slot] = arg;
                }
            }

            ExecutionResult::Continue
        }

        // ── Reflection — Generic type queries (Phase 108) ─────

        IntrinsicId::TypeTypeArgs => {
            let type_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            // Field 4 is the type_args Array
            let val = ctx.heap.get_field(type_href, 4).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        // ── Reflection — Per-member attributes (Phase 108) ────

        IntrinsicId::MethodInfoAttributes => {
            use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};
            let mi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (method_module_idx, method_idx) = match ctx.reflection.lookup_method_identity(mi_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("MethodInfoAttributes: not a MethodInfo".into()),
            };
            let target_row = (method_idx + 1) as u32;
            let mut attr_data: Vec<(String, Vec<writ_module::attr::AttrValue>)> = Vec::new();
            {
                let module = &ctx.modules[method_module_idx].module;
                for row in &module.attribute_defs {
                    if row.owner_kind == ATTR_OWNER_KIND_DECL { continue; }
                    if row.owner.table_id() != TableId::MethodDef.as_u8() { continue; }
                    if row.owner.row_index() != Some(target_row) { continue; }
                    let name = writ_module::heap::read_string(&module.string_heap, row.name)
                        .unwrap_or("<unknown>").to_owned();
                    let args = if row.value == 0 {
                        Vec::new()
                    } else {
                        match writ_module::heap::read_blob(&module.blob_heap, row.value) {
                            Ok(blob) => writ_module::attr::decode_attr_args(blob).unwrap_or_default(),
                            Err(_) => Vec::new(),
                        }
                    };
                    attr_data.push((name, args));
                }
            }
            let arr_href = ctx.heap.alloc_array(0);
            for (ordinal, (name, args)) in attr_data.iter().enumerate() {
                let ai = ctx.reflection.get_or_alloc_method_attribute_info(
                    name, args, ordinal, method_module_idx, method_idx, ctx.heap
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(ai));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        IntrinsicId::FieldInfoAttributes => {
            use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};
            let fi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let (module_idx, typedef_idx, field_offset) = match ctx.reflection.lookup_field_identity(fi_href) {
                Some(id) => id,
                None => return ExecutionResult::Crash("FieldInfoAttributes: not a FieldInfo".into()),
            };
            let (field_start, _) = ReflectionIndex::typedef_field_range_pub(
                ctx.modules, module_idx, typedef_idx
            );
            let abs_field_idx = field_start + field_offset;
            let target_row = (abs_field_idx + 1) as u32;
            let mut attr_data: Vec<(String, Vec<writ_module::attr::AttrValue>)> = Vec::new();
            {
                let module = &ctx.modules[module_idx].module;
                for row in &module.attribute_defs {
                    if row.owner_kind == ATTR_OWNER_KIND_DECL { continue; }
                    if row.owner.table_id() != TableId::FieldDef.as_u8() { continue; }
                    if row.owner.row_index() != Some(target_row) { continue; }
                    let name = writ_module::heap::read_string(&module.string_heap, row.name)
                        .unwrap_or("<unknown>").to_owned();
                    let args = if row.value == 0 {
                        Vec::new()
                    } else {
                        match writ_module::heap::read_blob(&module.blob_heap, row.value) {
                            Ok(blob) => writ_module::attr::decode_attr_args(blob).unwrap_or_default(),
                            Err(_) => Vec::new(),
                        }
                    };
                    attr_data.push((name, args));
                }
            }
            let arr_href = ctx.heap.alloc_array(0);
            for (ordinal, (name, args)) in attr_data.iter().enumerate() {
                let ai = ctx.reflection.get_or_alloc_field_attribute_info(
                    name, args, ordinal, module_idx, abs_field_idx, ctx.heap
                );
                if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_href) {
                    elements.push(Value::Ref(ai));
                }
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(arr_href);
            ExecutionResult::Continue
        }

        // ── Reflection — ParameterInfo methods ────────────────

        IntrinsicId::ParameterInfoGetName => {
            let pi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(pi_href, 0).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::ParameterInfoGetType => {
            let pi_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(pi_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        // ── Reflection — AttributeInfo methods ────────────────

        IntrinsicId::AttributeInfoGetName => {
            let ai_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(ai_href, 0).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::AttributeInfoGetArgs => {
            let ai_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(ai_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        // ── Reflection — ContractInfo methods ─────────────────

        IntrinsicId::ContractInfoGetName => {
            let ci_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(ci_href, 0).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        IntrinsicId::ContractInfoGetType => {
            let ci_href = helpers::extract_ref(
                &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
            );
            let val = ctx.heap.get_field(ci_href, 1).unwrap_or(Value::Void);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }

        // ── Hashable — hash() for primitive types (Phase 116) ─
        IntrinsicId::IntHash => {
            let v = helpers::extract_int(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(v);
            ExecutionResult::Continue
        }
        IntrinsicId::FloatHash => {
            let f = helpers::extract_float(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let hash = f.to_bits() as i64;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(hash);
            ExecutionResult::Continue
        }
        IntrinsicId::BoolHash => {
            let b = helpers::extract_bool(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let hash = if b { 1i64 } else { 0i64 };
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(hash);
            ExecutionResult::Continue
        }
        IntrinsicId::StringHash => {
            let href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
            let s = match ctx.heap.read_string(href) {
                Ok(s) => s.to_string(),
                Err(_) => return ExecutionResult::Crash("StringHash: not a string".into()),
            };
            // FNV-1a hash
            let mut hash: u64 = 0xcbf29ce484222325;
            for byte in s.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Int(hash as i64);
            ExecutionResult::Continue
        }
    }
}
