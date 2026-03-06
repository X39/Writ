//! Free helper functions for the DAP server.
//!
//! These functions are testable without a live DapServer instance.

use dap::prelude::*;
use writ_module::module::Module;
use writ_module::heap::read_string;
use writ_runtime::{TaskId, Value, GcHeap};

use crate::variables::{format_value, decode_type_blob};

/// Decode a globally-unique DAP frame_id back to (task_idx, display_frame_idx).
/// Encoding: frame_id = task_idx * 10000 + display_frame_idx
pub(super) fn decode_frame_id(frame_id: i64) -> (u32, u32) {
    let task_idx = (frame_id / 10000) as u32;
    let frame_idx = (frame_id % 10000) as u32;
    (task_idx, frame_idx)
}

/// Build DAP Thread entries from active task IDs.
/// Returns a single "terminated" thread if task_ids is empty.
pub(super) fn build_thread_list(
    task_ids: &[TaskId],
    call_stack_fn: impl Fn(TaskId) -> Option<Vec<(usize, usize)>>,
    module: &Module,
) -> Vec<types::Thread> {
    if task_ids.is_empty() {
        return vec![types::Thread { id: 0, name: "terminated".to_string() }];
    }
    task_ids.iter().map(|tid| {
        let name = call_stack_fn(*tid)
            .and_then(|frames| frames.first().map(|&(method_idx, _)| method_idx))
            .and_then(|method_idx| module.method_defs.get(method_idx))
            .and_then(|def| read_string(&module.string_heap, def.name).ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("task-{}", tid.index));
        types::Thread { id: tid.index as i64, name }
    }).collect()
}

/// Collect named local variables for a given frame.
///
/// Arguments:
/// - module: the loaded Module (for debug_locals, string_heap, blob_heap)
/// - method_idx: which method this frame is executing
/// - pc: the current program counter in that method
/// - registers: the register values for this frame (from frame_registers)
/// - heap: the GC heap (for formatting Ref values)
///
/// Returns a Vec of DAP Variable structs.
pub(crate) fn collect_frame_variables(
    module: &Module,
    method_idx: usize,
    pc: usize,
    registers: &[Value],
    heap: &dyn GcHeap,
) -> Vec<types::Variable> {
    let debug_locals = match module.method_bodies.get(method_idx) {
        Some(body) => &body.debug_locals,
        None => return vec![],
    };

    debug_locals.iter()
        // Filter out unnamed temporaries: name offset 0 means the empty string (no name).
        // Only named variables (params and let bindings) should appear in the DAP variables panel.
        .filter(|dl| dl.name != 0)
        .filter(|dl| dl.start_pc <= pc as u32 && (pc as u32) < dl.end_pc)
        .map(|dl| {
            let name = read_string(&module.string_heap, dl.name)
                .unwrap_or("?").to_string();
            let reg_val = registers.get(dl.register as usize)
                .cloned().unwrap_or(Value::Void);
            let value_str = format_value(&reg_val, module, heap);
            let type_str = decode_type_blob(module, dl.type_ref);
            types::Variable {
                name,
                value: value_str,
                type_field: Some(type_str),
                variables_reference: 0,
                ..Default::default()
            }
        })
        .collect()
}

/// Translate instruction-index PC to byte-offset PC.
///
/// `call_stack_frames` returns instruction-index PCs, but `SourceSpan.pc`
/// and `DebugLocal.start_pc`/`end_pc` are byte offsets. This helper bridges
/// the gap using `LoadedModule.byte_offsets`.
pub(crate) fn instr_to_byte_pc(runtime: &writ_runtime::Runtime<crate::debug_host::DebugHost>, method_idx: usize, instr_pc: usize) -> u32 {
    let user_idx = runtime.user_module_idx();
    runtime.domain().modules.get(user_idx)
        .and_then(|m| m.byte_offsets.get(method_idx))
        .and_then(|offsets| offsets.get(instr_pc))
        .copied()
        .unwrap_or(0)
}

/// Look up a local variable by name in the given frame and return its formatted value.
///
/// Returns (value_string, Some(type_string)) if found, or
/// (error_message, None) if no matching local exists.
pub(super) fn evaluate_local(
    module: &Module,
    method_idx: usize,
    pc: usize,
    registers: &[Value],
    heap: &dyn GcHeap,
    expr: &str,
) -> (String, Option<String>) {
    let debug_locals = match module.method_bodies.get(method_idx) {
        Some(body) => &body.debug_locals,
        None => return (format!("'{}' is not a local variable in the current frame", expr), None),
    };

    for dl in debug_locals {
        if dl.start_pc <= pc as u32 && (pc as u32) < dl.end_pc
            && let Ok(name) = read_string(&module.string_heap, dl.name)
                && name == expr {
                    let reg_val = registers.get(dl.register as usize)
                        .cloned().unwrap_or(Value::Void);
                    let value_str = format_value(&reg_val, module, heap);
                    let type_str = decode_type_blob(module, dl.type_ref);
                    return (value_str, Some(type_str));
                }
    }

    (format!("'{}' is not a local variable in the current frame", expr), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::module::{MethodBody, Module};
    use writ_module::tables::MethodDefRow;

    /// Helper: build a Module with method names in the string heap.
    /// Returns a module with method_defs and method_bodies populated.
    fn make_module_with_methods(method_names: &[&str]) -> Module {
        let mut module = Module::new();
        for name in method_names {
            let name_bytes = name.as_bytes();
            let offset = module.string_heap.len() as u32;
            module.string_heap.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            module.string_heap.extend_from_slice(name_bytes);
            module.method_defs.push(MethodDefRow {
                name: offset,
                signature: 0,
                flags: 0,
                body_offset: 0,
                body_size: 0,
                reg_count: 0,
                param_count: 0,
            });
            module.method_bodies.push(MethodBody {
                register_types: vec![],
                code: vec![],
                debug_locals: vec![],
                source_spans: vec![],
            });
        }
        module
    }

    #[test]
    fn test_decode_frame_id() {
        assert_eq!(decode_frame_id(0), (0, 0));
        assert_eq!(decode_frame_id(10003), (1, 3));
        assert_eq!(decode_frame_id(50000), (5, 0));
        // Roundtrip: encode then decode
        for task_idx in [0u32, 1, 5, 99] {
            for frame_idx in [0u32, 1, 7, 9999] {
                let encoded = (task_idx as i64) * 10000 + frame_idx as i64;
                assert_eq!(decode_frame_id(encoded), (task_idx, frame_idx));
            }
        }
    }

    #[test]
    fn test_threads_multi_task() {
        let module = make_module_with_methods(&["main", "worker"]);

        // Two active tasks
        let task0 = TaskId::new(0, 0);
        let task1 = TaskId::new(1, 0);
        let task_ids = vec![task0, task1];

        // Simulate call_stack_frames: task0 has method 0 at bottom, task1 has method 1
        let threads = build_thread_list(&task_ids, |tid| {
            match tid.index {
                0 => Some(vec![(0usize, 0usize)]),
                1 => Some(vec![(1usize, 0usize)]),
                _ => None,
            }
        }, &module);

        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].id, 0);
        assert_eq!(threads[0].name, "main");
        assert_eq!(threads[1].id, 1);
        assert_eq!(threads[1].name, "worker");

        // Empty task list -> terminated
        let threads_empty = build_thread_list(&[], |_| None, &module);
        assert_eq!(threads_empty.len(), 1);
        assert_eq!(threads_empty[0].id, 0);
        assert_eq!(threads_empty[0].name, "terminated");
    }

    // ── Task 2: Scopes / Variables / Evaluate helpers ─────────────────────────

    /// Helper: build a Module with one method containing DebugLocals and a blob heap
    /// for type decoding. locals: (name, register, start_pc, end_pc, type_tag)
    fn make_module_with_locals(locals: &[(&str, u16, u32, u32, u8)]) -> Module {
        use writ_module::module::DebugLocal;
        let mut module = Module::new();

        let mut debug_locals = Vec::new();

        for &(name, register, start_pc, end_pc, type_tag) in locals {
            // Write name to string heap (length-prefixed)
            let name_offset = module.string_heap.len() as u32;
            let name_bytes = name.as_bytes();
            module.string_heap.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            module.string_heap.extend_from_slice(name_bytes);

            // Write type blob (4-byte length prefix + 1-byte tag)
            let type_ref = module.blob_heap.len() as u32;
            module.blob_heap.extend_from_slice(&1u32.to_le_bytes()); // blob length = 1
            module.blob_heap.push(type_tag);

            debug_locals.push(DebugLocal {
                register,
                name: name_offset,
                type_ref,
                start_pc,
                end_pc,
            });
        }

        module.method_bodies = vec![MethodBody {
            register_types: vec![],
            code: vec![],
            debug_locals,
            source_spans: vec![],
        }];
        module.method_defs = vec![MethodDefRow {
            name: 0, signature: 0, flags: 0,
            body_offset: 0, body_size: 0, reg_count: 0, param_count: 0,
        }];
        module
    }

    #[test]
    fn test_scopes_handler() {
        use crate::variables::{make_variables_ref, unpack_variables_ref};
        // Verify the Scopes->Variables pipeline: decode_frame_id produces (task_idx, frame_idx),
        // which feeds into make_variables_ref, and unpack_variables_ref recovers the same pair.
        let frame_id: i64 = 2 * 10000 + 3; // task=2, frame=3
        let (task_idx, frame_idx) = decode_frame_id(frame_id);
        assert_eq!(task_idx, 2);
        assert_eq!(frame_idx, 3);

        let vars_ref = make_variables_ref(task_idx, frame_idx);
        let (unpacked_task, unpacked_frame) = unpack_variables_ref(vars_ref);
        assert_eq!(unpacked_task, 2);
        assert_eq!(unpacked_frame, 3);
    }

    #[test]
    fn test_variables_handler() {
        use writ_runtime::Value;
        use writ_runtime::BumpHeap;

        // type_tag 0x01 = int, 0x02 = float
        let module = make_module_with_locals(&[
            ("x", 0, 0, 10, 0x01),  // active at pc 0..10
            ("y", 1, 5, 15, 0x02),  // active at pc 5..15 (float)
        ]);
        let heap = BumpHeap::new();
        let registers = vec![Value::Int(42), Value::Float(3.14)];

        // At pc=7, both x and y are active
        let vars = collect_frame_variables(&module, 0, 7, &registers, &heap);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "x");
        assert_eq!(vars[0].value, "42");
        assert_eq!(vars[1].name, "y");

        // At pc=3, only x is active (y starts at pc 5)
        let vars_early = collect_frame_variables(&module, 0, 3, &registers, &heap);
        assert_eq!(vars_early.len(), 1);
        assert_eq!(vars_early[0].name, "x");

        // At pc=12, only y is active (x ends at pc 10)
        let vars_late = collect_frame_variables(&module, 0, 12, &registers, &heap);
        assert_eq!(vars_late.len(), 1);
        assert_eq!(vars_late[0].name, "y");

        // At pc=20, neither is active
        let vars_none = collect_frame_variables(&module, 0, 20, &registers, &heap);
        assert!(vars_none.is_empty());
    }

    #[test]
    fn test_evaluate_local_name() {
        use writ_runtime::Value;
        use writ_runtime::BumpHeap;

        let module = make_module_with_locals(&[("x", 0, 0, 10, 0x01)]);
        let heap = BumpHeap::new();
        let registers = vec![Value::Int(42)];

        let (value, ty) = evaluate_local(&module, 0, 5, &registers, &heap, "x");
        assert_eq!(value, "42");
        assert!(ty.is_some());
        assert_eq!(ty.unwrap(), "int");
    }

    #[test]
    fn test_evaluate_unknown() {
        use writ_runtime::Value;
        use writ_runtime::BumpHeap;

        let module = make_module_with_locals(&[("x", 0, 0, 10, 0x01)]);
        let heap = BumpHeap::new();
        let registers = vec![Value::Int(42)];

        let (msg, ty) = evaluate_local(&module, 0, 5, &registers, &heap, "nonexistent");
        assert!(msg.contains("nonexistent"), "got: {}", msg);
        assert!(msg.contains("not a local variable"), "got: {}", msg);
        assert!(ty.is_none());
    }
}
