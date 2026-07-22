use writ_module::ModuleBuilder;
use writ_module::instruction::{ArrayDefaultKind, Instruction};
use writ_module::module::MethodBody;
use writ_module::tables::TypeDefKind;
use writ_runtime::{ExecutionLimit, NullHost, Runtime, RuntimeBuilder, TaskId, TaskState, Value};

fn encode(instructions: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instruction in instructions {
        instruction.encode(&mut code).unwrap();
    }
    code
}

fn run(instructions: &[Instruction], reg_count: u16) -> (Runtime<NullHost>, TaskId) {
    let mut builder = ModuleBuilder::new("array-default-tests");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &[0],
        0,
        reg_count,
        MethodBody {
            register_types: vec![0; reg_count as usize],
            code: encode(instructions),
            debug_locals: vec![],
            source_spans: vec![],
        },
    );
    let mut runtime = RuntimeBuilder::new(builder.build()).build().unwrap();
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    (runtime, task_id)
}

fn grow_and_load(kind: ArrayDefaultKind) -> Value {
    let (runtime, task_id) = run(
        &[
            Instruction::NewArray {
                r_dst: 0,
                elem_type: kind.operand(),
            },
            Instruction::LoadInt { r_dst: 1, value: 1 },
            Instruction::ArrayResize {
                r_arr: 0,
                r_new_len: 1,
            },
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::ArrayLoad {
                r_dst: 3,
                r_arr: 0,
                r_idx: 2,
            },
            Instruction::Ret { r_src: 3 },
        ],
        4,
    );
    assert_eq!(runtime.task_state(task_id), Some(TaskState::Completed));
    runtime.return_value(task_id).unwrap()
}

#[test]
fn resize_uses_primitive_and_reference_defaults() {
    assert_eq!(grow_and_load(ArrayDefaultKind::Int), Value::Int(0));
    assert_eq!(grow_and_load(ArrayDefaultKind::Float), Value::Float(0.0));
    assert_eq!(grow_and_load(ArrayDefaultKind::Bool), Value::Bool(false));
    assert_eq!(grow_and_load(ArrayDefaultKind::NullReference), Value::Void);
}

#[test]
fn resize_uses_empty_string_default() {
    let (runtime, task_id) = run(
        &[
            Instruction::NewArray {
                r_dst: 0,
                elem_type: ArrayDefaultKind::String.operand(),
            },
            Instruction::LoadInt { r_dst: 1, value: 1 },
            Instruction::ArrayResize {
                r_arr: 0,
                r_new_len: 1,
            },
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::ArrayLoad {
                r_dst: 3,
                r_arr: 0,
                r_idx: 2,
            },
            Instruction::StrLen { r_dst: 4, r_str: 3 },
            Instruction::Ret { r_src: 4 },
        ],
        5,
    );
    assert_eq!(runtime.return_value(task_id), Some(Value::Int(0)));
}

#[test]
fn empty_slice_preserves_inferred_default_kind() {
    let (runtime, task_id) = run(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 2.5,
            },
            Instruction::ArrayInit {
                r_dst: 1,
                elem_type: ArrayDefaultKind::Unavailable.operand(),
                count: 1,
                r_base: 0,
            },
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::ArraySlice {
                r_dst: 3,
                r_arr: 1,
                r_start: 2,
                r_end: 2,
            },
            Instruction::LoadInt { r_dst: 4, value: 1 },
            Instruction::ArrayResize {
                r_arr: 3,
                r_new_len: 4,
            },
            Instruction::ArrayLoad {
                r_dst: 5,
                r_arr: 3,
                r_idx: 2,
            },
            Instruction::Ret { r_src: 5 },
        ],
        6,
    );
    assert_eq!(runtime.return_value(task_id), Some(Value::Float(0.0)));
}

#[test]
fn filled_array_refines_erased_default_kind() {
    let (runtime, task_id) = run(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 9.0,
            },
            Instruction::LoadInt { r_dst: 1, value: 1 },
            Instruction::NewArrayFilled {
                r_dst: 2,
                elem_type: ArrayDefaultKind::Unavailable.operand(),
                r_len: 1,
                r_fill: 0,
            },
            Instruction::LoadInt { r_dst: 1, value: 2 },
            Instruction::ArrayResize {
                r_arr: 2,
                r_new_len: 1,
            },
            Instruction::LoadInt { r_dst: 3, value: 1 },
            Instruction::ArrayLoad {
                r_dst: 4,
                r_arr: 2,
                r_idx: 3,
            },
            Instruction::Ret { r_src: 4 },
        ],
        5,
    );
    assert_eq!(runtime.return_value(task_id), Some(Value::Float(0.0)));
}

#[test]
fn unavailable_default_crashes_instead_of_inventing_a_value() {
    let (runtime, task_id) = run(
        &[
            Instruction::NewArray {
                r_dst: 0,
                elem_type: ArrayDefaultKind::Unavailable.operand(),
            },
            Instruction::LoadInt { r_dst: 1, value: 1 },
            Instruction::ArrayResize {
                r_arr: 0,
                r_new_len: 1,
            },
            Instruction::RetVoid,
        ],
        2,
    );
    assert_eq!(runtime.task_state(task_id), Some(TaskState::Cancelled));
    assert!(
        runtime
            .crash_info(task_id)
            .unwrap()
            .message
            .contains("element type has no runtime default")
    );
}

#[test]
fn runtime_created_string_arrays_keep_string_defaults() {
    let (runtime, task_id) = run(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 101,
            },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            Instruction::LoadInt { r_dst: 0, value: 0 },
            Instruction::I2s { r_dst: 2, r_src: 0 },
            Instruction::StrSplit {
                r_dst: 3,
                r_str: 1,
                r_sep: 2,
            },
            Instruction::LoadInt { r_dst: 4, value: 0 },
            Instruction::ArraySlice {
                r_dst: 5,
                r_arr: 3,
                r_start: 4,
                r_end: 4,
            },
            Instruction::LoadInt { r_dst: 6, value: 1 },
            Instruction::ArrayResize {
                r_arr: 5,
                r_new_len: 6,
            },
            Instruction::ArrayLoad {
                r_dst: 7,
                r_arr: 5,
                r_idx: 4,
            },
            Instruction::StrLen { r_dst: 8, r_str: 7 },
            Instruction::Ret { r_src: 8 },
        ],
        9,
    );
    assert_eq!(runtime.return_value(task_id), Some(Value::Int(0)));
}
