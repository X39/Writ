use crate::heap::HeapObject;
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
    }
}
