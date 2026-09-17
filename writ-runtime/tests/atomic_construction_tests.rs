use writ_module::module::MethodBody;
use writ_module::tables::{FIELD_FLAG_READONLY, TypeDefKind};
use writ_module::{Instruction, ModuleBuilder};
use writ_runtime::{
    ExecutionLimit, HostRequest, HostResponse, LogLevel, RequestId, Runtime, RuntimeBuilder,
    RuntimeHost, TaskState, Value,
};

#[derive(Default)]
struct RecordingHost {
    requests: usize,
}

impl RuntimeHost for RecordingHost {
    fn on_request(&mut self, _id: RequestId, _request: &HostRequest) -> HostResponse {
        self.requests += 1;
        HostResponse::Confirmed
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}
}

fn encode(instructions: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instruction in instructions {
        instruction.encode(&mut code).expect("encode instruction");
    }
    code
}

fn runtime_with_type(
    kind: TypeDefKind,
    field_flags: &[u16],
    register_count: u16,
    instructions: &[Instruction],
) -> Runtime<RecordingHost> {
    let mut builder = ModuleBuilder::new("atomic-construction");
    builder.add_type_def("Value", "test", kind, 0);
    for (index, flags) in field_flags.iter().enumerate() {
        builder.add_field_def(&format!("field_{index}"), &[0x01], *flags);
    }
    builder.add_method(
        "main",
        &[0],
        0,
        register_count,
        MethodBody {
            register_types: vec![0; register_count as usize],
            code: encode(instructions),
            debug_locals: vec![],
            source_spans: vec![],
        },
    );
    RuntimeBuilder::new(builder.build())
        .with_host(RecordingHost::default())
        .build()
        .expect("build runtime")
}

fn run(runtime: &mut Runtime<RecordingHost>) -> writ_runtime::TaskId {
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);
    task
}

#[test]
fn new_wrong_field_count_crashes_without_allocation_or_destination_write() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Struct,
        &[0],
        1,
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 0,
                r_base: 0,
            },
            Instruction::RetVoid,
        ],
    );
    let heap_before = runtime.heap().object_count();
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.heap().object_count(), heap_before);
    assert_eq!(runtime.host().requests, 0);
}

#[test]
fn zero_field_new_ignores_r_base() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Struct,
        &[],
        1,
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 0,
                r_base: u16::MAX,
            },
            Instruction::Ret { r_src: 0 },
        ],
    );
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert!(matches!(
        runtime.return_value(task),
        Some(Value::Struct { .. })
    ));
}

#[test]
fn new_out_of_range_field_block_crashes_without_allocation() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Class,
        &[0],
        2,
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 1,
                r_base: 2,
            },
            Instruction::RetVoid,
        ],
    );
    let heap_before = runtime.heap().object_count();
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.heap().object_count(), heap_before);
}

#[test]
fn new_rejects_entity_kind_without_allocating_or_registering_an_entity() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[],
        1,
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 0,
                r_base: 0,
            },
            Instruction::RetVoid,
        ],
    );
    let heap_before = runtime.heap().object_count();
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.heap().object_count(), heap_before);
    assert_eq!(runtime.entity_registry().slot_count(), 0);
}

#[test]
fn readonly_field_is_initialized_atomically_and_rejects_later_set_field() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Struct,
        &[FIELD_FLAG_READONLY],
        3,
        &[
            Instruction::LoadInt {
                r_dst: 1,
                value: 41,
            },
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 1,
                r_base: 1,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 42,
            },
            Instruction::SetField {
                r_obj: 0,
                field_token: 0x0500_0001,
                r_val: 1,
            },
            Instruction::RetVoid,
        ],
    );
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    runtime.run_task(task, ExecutionLimit::Instructions(3));
    assert_eq!(runtime.task_state(task), Some(TaskState::Ready));
    let value = runtime
        .register_value(task, 0)
        .expect("constructed value remains in destination");
    let Value::Struct { href, .. } = value else {
        panic!("expected constructed struct, got {value:?}");
    };
    runtime.run_task(task, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(
        runtime
            .heap()
            .get_field(href, 0)
            .expect("read initialized field"),
        Value::Int(41)
    );
    assert!(
        runtime
            .crash_info(task)
            .expect("readonly write crash")
            .message
            .contains("read-only field 'field_0'")
    );
}

#[test]
fn spawn_entity_wrong_field_count_has_no_heap_registry_host_or_destination_side_effects() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[0],
        1,
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::SpawnEntity {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 0,
                r_base: 0,
            },
            Instruction::RetVoid,
        ],
    );
    let heap_before = runtime.heap().object_count();
    let slots_before = runtime.entity_registry().slot_count();
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.heap().object_count(), heap_before);
    assert_eq!(runtime.entity_registry().slot_count(), slots_before);
    assert_eq!(runtime.host().requests, 0);
}

#[test]
fn zero_field_spawn_entity_ignores_r_base() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[],
        1,
        &[
            Instruction::SpawnEntity {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 0,
                r_base: u16::MAX,
            },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::Ret { r_src: 0 },
        ],
    );
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert!(matches!(runtime.return_value(task), Some(Value::Entity(_))));
    assert_eq!(runtime.host().requests, 2);
}

#[test]
fn init_entity_rejects_entity_register_out_of_bounds() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[],
        1,
        &[
            Instruction::InitEntity { r_entity: 1 },
            Instruction::RetVoid,
        ],
    );
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.host().requests, 0);
    let crash = runtime
        .crash_info(task)
        .expect("out-of-range INIT_ENTITY must crash");
    assert!(
        crash
            .message
            .contains("INIT_ENTITY: entity register r1 is out of range"),
        "unexpected crash: {}",
        crash.message
    );
}

#[test]
fn spawn_entity_publishes_complete_fields_before_init() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[FIELD_FLAG_READONLY],
        3,
        &[
            Instruction::LoadInt {
                r_dst: 1,
                value: 73,
            },
            Instruction::SpawnEntity {
                r_dst: 0,
                type_idx: 0x0200_0001,
                field_count: 1,
                r_base: 1,
            },
            Instruction::GetField {
                r_dst: 2,
                r_obj: 0,
                field_token: 0x0500_0001,
            },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::Ret { r_src: 2 },
        ],
    );
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    runtime.run_task(task, ExecutionLimit::Instructions(2));
    assert_eq!(runtime.task_state(task), Some(TaskState::Ready));
    let gc_stats = runtime.collect_garbage();
    assert_eq!(
        gc_stats.objects_freed, 0,
        "pending entity field storage must remain a GC root"
    );
    runtime.run_task(task, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(73)));
    assert_eq!(runtime.entity_registry().alive_count(), 1);
    assert_eq!(runtime.host().requests, 2);
}

#[test]
fn get_or_create_rejects_script_fields_without_side_effects() {
    let mut runtime = runtime_with_type(
        TypeDefKind::Entity,
        &[0],
        1,
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::GetOrCreate {
                r_dst: 0,
                type_idx: 0x0200_0001,
            },
            Instruction::RetVoid,
        ],
    );
    let heap_before = runtime.heap().object_count();
    let slots_before = runtime.entity_registry().slot_count();
    let task = run(&mut runtime);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(runtime.heap().object_count(), heap_before);
    assert_eq!(runtime.entity_registry().slot_count(), slots_before);
    assert_eq!(runtime.host().requests, 0);
}

#[test]
fn forged_get_field_registers_crash_instead_of_panicking() {
    for instruction in [
        Instruction::GetField {
            r_dst: 1,
            r_obj: 0,
            field_token: 0x0500_0001,
        },
        Instruction::GetField {
            r_dst: 0,
            r_obj: 1,
            field_token: 0x0500_0001,
        },
    ] {
        let mut runtime = runtime_with_type(
            TypeDefKind::Struct,
            &[0],
            1,
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: 0x0200_0001,
                    field_count: 1,
                    r_base: 0,
                },
                instruction,
                Instruction::RetVoid,
            ],
        );
        let task = run(&mut runtime);
        assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
        assert!(runtime.crash_info(task).is_some());
    }
}

#[test]
fn forged_set_field_registers_crash_without_mutating_the_object() {
    for instruction in [
        Instruction::SetField {
            r_obj: 1,
            field_token: 0x0500_0001,
            r_val: 0,
        },
        Instruction::SetField {
            r_obj: 0,
            field_token: 0x0500_0001,
            r_val: 1,
        },
    ] {
        let mut runtime = runtime_with_type(
            TypeDefKind::Struct,
            &[0],
            1,
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: 0x0200_0001,
                    field_count: 1,
                    r_base: 0,
                },
                instruction,
                Instruction::RetVoid,
            ],
        );
        let task = runtime.spawn_task(0, vec![]).expect("spawn main");
        runtime.run_task(task, ExecutionLimit::Instructions(1));
        assert_eq!(runtime.task_state(task), Some(TaskState::Ready));
        let value = runtime.register_value(task, 0).expect("constructed struct");
        let Value::Struct { href, .. } = value else {
            panic!("expected struct value, got {value:?}");
        };
        runtime.run_task(task, ExecutionLimit::None);
        assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
        assert_eq!(
            runtime.heap().get_field(href, 0).expect("read field"),
            Value::Void
        );
    }
}
