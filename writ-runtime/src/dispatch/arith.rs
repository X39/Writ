use crate::value::Value;

use super::{helpers, ExecContext, ExecutionResult};

// ── Meta / Crash ───────────────────────────────────────────────

pub(super) fn exec_crash(ctx: &mut ExecContext<'_>, r_msg: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let msg = match frame.registers[r_msg as usize] {
        Value::Ref(href) => match ctx.heap.read_string(href) {
            Ok(s) => s.to_string(),
            Err(_) => format!("crash with non-string value in r{}", r_msg),
        },
        _ => format!("crash with non-string value in r{}", r_msg),
    };
    ExecutionResult::Crash(msg)
}

// ── Data Movement ──────────────────────────────────────────────

pub(super) fn exec_mov(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = frame.registers[r_src as usize];
    ExecutionResult::Continue
}

pub(super) fn exec_load_int(ctx: &mut ExecContext<'_>, r_dst: u16, value: i64) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Int(value);
    ExecutionResult::Continue
}

pub(super) fn exec_load_float(ctx: &mut ExecContext<'_>, r_dst: u16, value: f64) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Float(value);
    ExecutionResult::Continue
}

pub(super) fn exec_load_true(ctx: &mut ExecContext<'_>, r_dst: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(true);
    ExecutionResult::Continue
}

pub(super) fn exec_load_false(ctx: &mut ExecContext<'_>, r_dst: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(false);
    ExecutionResult::Continue
}

pub(super) fn exec_load_string(ctx: &mut ExecContext<'_>, r_dst: u16, string_idx: u32) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let s = match writ_module::heap::read_string(&module.module.string_heap, string_idx) {
        Ok(s) => s.to_string(),
        Err(_) => return ExecutionResult::Crash(format!("invalid string index {}", string_idx)),
    };
    let href = ctx.heap.alloc_string(&s);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_load_null(ctx: &mut ExecContext<'_>, r_dst: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Void;
    ExecutionResult::Continue
}

// ── Control Flow ───────────────────────────────────────────────

pub(super) fn exec_br(ctx: &mut ExecContext<'_>, offset: i32) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.pc = offset as usize;
    ExecutionResult::Continue
}

pub(super) fn exec_br_true(ctx: &mut ExecContext<'_>, r_cond: u16, offset: i32) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    if helpers::extract_bool(&frame.registers[r_cond as usize]) {
        frame.pc = offset as usize;
    }
    ExecutionResult::Continue
}

pub(super) fn exec_br_false(ctx: &mut ExecContext<'_>, r_cond: u16, offset: i32) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    if !helpers::extract_bool(&frame.registers[r_cond as usize]) {
        frame.pc = offset as usize;
    }
    ExecutionResult::Continue
}

pub(super) fn exec_switch(ctx: &mut ExecContext<'_>, r_tag: u16, offsets: Vec<i32>) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let tag = helpers::extract_int(&frame.registers[r_tag as usize]) as usize;
    if tag >= offsets.len() {
        return ExecutionResult::Crash(format!(
            "switch tag {} out of range (0..{})",
            tag,
            offsets.len()
        ));
    }
    frame.pc = offsets[tag] as usize;
    ExecutionResult::Continue
}

// ── Integer Arithmetic ─────────────────────────────────────────

pub(super) fn exec_add_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a.wrapping_add(b));
    ExecutionResult::Continue
}

pub(super) fn exec_sub_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a.wrapping_sub(b));
    ExecutionResult::Continue
}

pub(super) fn exec_mul_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a.wrapping_mul(b));
    ExecutionResult::Continue
}

pub(super) fn exec_div_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    if b == 0 {
        return ExecutionResult::Crash("division by zero".into());
    }
    frame.registers[r_dst as usize] = Value::Int(a / b);
    ExecutionResult::Continue
}

pub(super) fn exec_mod_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    if b == 0 {
        return ExecutionResult::Crash("division by zero".into());
    }
    frame.registers[r_dst as usize] = Value::Int(a % b);
    ExecutionResult::Continue
}

pub(super) fn exec_neg_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let v = helpers::extract_int(&frame.registers[r_src as usize]);
    frame.registers[r_dst as usize] = Value::Int(-v);
    ExecutionResult::Continue
}

// ── Float Arithmetic ───────────────────────────────────────────

pub(super) fn exec_add_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Float(a + b);
    ExecutionResult::Continue
}

pub(super) fn exec_sub_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Float(a - b);
    ExecutionResult::Continue
}

pub(super) fn exec_mul_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Float(a * b);
    ExecutionResult::Continue
}

pub(super) fn exec_div_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Float(a / b);
    ExecutionResult::Continue
}

pub(super) fn exec_mod_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Float(a % b);
    ExecutionResult::Continue
}

pub(super) fn exec_neg_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let v = helpers::extract_float(&frame.registers[r_src as usize]);
    frame.registers[r_dst as usize] = Value::Float(-v);
    ExecutionResult::Continue
}

// ── Bitwise & Logical ──────────────────────────────────────────

pub(super) fn exec_bit_and(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a & b);
    ExecutionResult::Continue
}

pub(super) fn exec_bit_or(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a | b);
    ExecutionResult::Continue
}

pub(super) fn exec_shl(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a << (b & 63));
    ExecutionResult::Continue
}

pub(super) fn exec_shr(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Int(a >> (b & 63));
    ExecutionResult::Continue
}

/// Logical NOT. Operand must be bool (spec §52_3_4_bitwise_logical.md).
pub(super) fn exec_not(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let v = helpers::extract_bool(&frame.registers[r_src as usize]);
    frame.registers[r_dst as usize] = Value::Bool(!v);
    ExecutionResult::Continue
}

// ── Comparison ─────────────────────────────────────────────────

pub(super) fn exec_cmp_eq_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Bool(a == b);
    ExecutionResult::Continue
}

pub(super) fn exec_cmp_eq_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Bool(a == b);
    ExecutionResult::Continue
}

pub(super) fn exec_cmp_eq_b(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_bool(&frame.registers[r_a as usize]);
    let b = helpers::extract_bool(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Bool(a == b);
    ExecutionResult::Continue
}

pub(super) fn exec_cmp_eq_s(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    // CRITICAL: Compare string CONTENT, not HeapRef indices
    let frame = ctx.task.call_stack.last().unwrap();
    let href_a = helpers::extract_ref(&frame.registers[r_a as usize]);
    let href_b = helpers::extract_ref(&frame.registers[r_b as usize]);
    let sa = match ctx.heap.read_string(href_a) {
        Ok(s) => s.to_string(),
        Err(_) => return ExecutionResult::Crash("CmpEqS: left operand is not a string".into()),
    };
    let sb = match ctx.heap.read_string(href_b) {
        Ok(s) => s.to_string(),
        Err(_) => return ExecutionResult::Crash("CmpEqS: right operand is not a string".into()),
    };
    let eq = sa == sb;
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Bool(eq);
    ExecutionResult::Continue
}

pub(super) fn exec_cmp_lt_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_int(&frame.registers[r_a as usize]);
    let b = helpers::extract_int(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Bool(a < b);
    ExecutionResult::Continue
}

pub(super) fn exec_cmp_lt_f(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let a = helpers::extract_float(&frame.registers[r_a as usize]);
    let b = helpers::extract_float(&frame.registers[r_b as usize]);
    frame.registers[r_dst as usize] = Value::Bool(a < b);
    ExecutionResult::Continue
}

// ── Conversion ─────────────────────────────────────────────────

pub(super) fn exec_i2f(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let v = helpers::extract_int(&frame.registers[r_src as usize]);
    frame.registers[r_dst as usize] = Value::Float(v as f64);
    ExecutionResult::Continue
}

pub(super) fn exec_f2i(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    let v = helpers::extract_float(&frame.registers[r_src as usize]);
    frame.registers[r_dst as usize] = Value::Int(v as i64);
    ExecutionResult::Continue
}

pub(super) fn exec_i2s(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let v = helpers::extract_int(&frame.registers[r_src as usize]);
    let s = v.to_string();
    let href = ctx.heap.alloc_string(&s);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_f2s(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let v = helpers::extract_float(&frame.registers[r_src as usize]);
    let s = v.to_string();
    let href = ctx.heap.alloc_string(&s);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_b2s(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let v = helpers::extract_bool(&frame.registers[r_src as usize]);
    let s = v.to_string();
    let href = ctx.heap.alloc_string(&s);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_convert(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    // Placeholder: just copy value (full conversion needs type system from Phase 19)
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = frame.registers[r_src as usize];
    ExecutionResult::Continue
}

// ── Strings ────────────────────────────────────────────────────

pub(super) fn exec_str_concat(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let href_a = helpers::extract_ref(&frame.registers[r_a as usize]);
    let href_b = helpers::extract_ref(&frame.registers[r_b as usize]);
    let sa = match ctx.heap.read_string(href_a) {
        Ok(s) => s.to_string(),
        Err(_) => return ExecutionResult::Crash("StrConcat: left operand not a string".into()),
    };
    let sb = match ctx.heap.read_string(href_b) {
        Ok(s) => s.to_string(),
        Err(_) => return ExecutionResult::Crash("StrConcat: right operand not a string".into()),
    };
    let result = format!("{}{}", sa, sb);
    let href = ctx.heap.alloc_string(&result);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_str_build(ctx: &mut ExecContext<'_>, r_dst: u16, count: u16, r_base: u16) -> ExecutionResult {
    let mut parts = Vec::with_capacity(count as usize);
    {
        let frame = ctx.task.call_stack.last().unwrap();
        for i in 0..count as usize {
            let href = helpers::extract_ref(&frame.registers[r_base as usize + i]);
            match ctx.heap.read_string(href) {
                Ok(s) => parts.push(s.to_string()),
                Err(_) => return ExecutionResult::Crash(format!("StrBuild: argument {} not a string", i)),
            }
        }
    }
    let result = parts.concat();
    let href = ctx.heap.alloc_string(&result);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_str_len(ctx: &mut ExecContext<'_>, r_dst: u16, r_str: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let href = helpers::extract_ref(&frame.registers[r_str as usize]);
    let len = match ctx.heap.read_string(href) {
        Ok(s) => s.len() as i64,
        Err(_) => return ExecutionResult::Crash("StrLen: not a string".into()),
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Int(len);
    ExecutionResult::Continue
}

// ── Boxing ─────────────────────────────────────────────────────

pub(super) fn exec_box(ctx: &mut ExecContext<'_>, r_dst: u16, r_val: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let val = frame.registers[r_val as usize];
    let href = ctx.heap.alloc_boxed(val);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}

pub(super) fn exec_unbox(ctx: &mut ExecContext<'_>, r_dst: u16, r_boxed: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let href = helpers::extract_ref(&frame.registers[r_boxed as usize]);
    match ctx.heap.get_object(href) {
        Ok(crate::heap::HeapObject::Boxed(val)) => {
            let val = *val;
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("Unbox: not a boxed value".into()),
    }
}
