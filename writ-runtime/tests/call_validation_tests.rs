use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature};
use writ_module::tables::{METHOD_FLAG_INTRINSIC, TypeDefKind};
use writ_module::{Instruction, Module, ModuleBuilder};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState, Value};

fn body(instructions: &[Instruction], register_count: u16) -> MethodBody {
    let mut code = Vec::new();
    for instruction in instructions {
        instruction.encode(&mut code).expect("encode instruction");
    }
    MethodBody {
        register_types: vec![0; register_count as usize],
        code,
        debug_locals: vec![],
        source_spans: vec![],
    }
}

fn signature(parameter_count: usize) -> Vec<u8> {
    encode_method_signature(
        &vec![TypeSignature::Int; parameter_count],
        &TypeSignature::Void,
    )
    .expect("encode method signature")
}

fn returning_int_signature(parameter_count: usize) -> Vec<u8> {
    encode_method_signature(
        &vec![TypeSignature::Int; parameter_count],
        &TypeSignature::Int,
    )
    .expect("encode method signature")
}

fn assert_crash(module: Module, main_method_idx: usize, expected: &str) {
    let mut runtime = RuntimeBuilder::new(module).build().expect("build runtime");
    let task = runtime
        .spawn_task(main_method_idx, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(task).expect("malformed call must crash");
    assert!(
        crash.message.contains(expected),
        "expected crash containing {expected:?}, got {:?}",
        crash.message
    );
}

fn assert_spawn_crash_before_child(module: Module, expected: &str) {
    let mut runtime = RuntimeBuilder::new(module).build().expect("build runtime");
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    assert_eq!(
        runtime.task_count(),
        1,
        "an invalid SPAWN_TASK target must not create a child"
    );
    let crash = runtime
        .crash_info(task)
        .expect("malformed spawn must crash");
    assert!(
        crash.message.contains(expected),
        "expected crash containing {expected:?}, got {:?}",
        crash.message
    );
}

#[test]
fn call_rejects_argument_register_range_out_of_bounds() {
    let mut builder = ModuleBuilder::new("call-source-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::Call {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 1,
                argc: 1,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    assert_crash(builder.build(), 0, "CALL: argument register range");
}

#[test]
fn call_rejects_argument_count_mismatching_method_metadata() {
    let mut builder = ModuleBuilder::new("call-arity");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::Call {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let mut module = builder.build();
    module.method_defs[1].param_count = 1;

    assert_crash(module, 0, "CALL: argument count 0");
}

#[test]
fn call_rejects_mismatched_method_register_metadata() {
    let mut builder = ModuleBuilder::new("call-register-metadata");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::Call {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(0),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let mut module = builder.build();
    module.method_defs[1].reg_count = 2;

    assert_crash(module, 0, "MethodDef.reg_count 2 does not match");
}

fn virtual_call_module(
    main_instructions: &[Instruction],
    main_register_count: u16,
    callee_register_count: u16,
    callee_param_count: u16,
) -> Module {
    let mut builder = ModuleBuilder::new("call-virt-validation");
    let receiver = builder.add_type_def("Receiver", "test", TypeDefKind::Class, 0);
    let contract = builder.add_contract_def("Contract", "test");
    builder.add_contract_method("invoke", &signature(0), 0);
    let implementation = builder.add_impl_def(receiver, contract);
    builder.add_impl_method(
        implementation,
        "invoke",
        &signature(callee_param_count.saturating_sub(1) as usize),
        0,
        callee_register_count,
        body(&[Instruction::RetVoid], callee_register_count),
    );
    builder.add_method(
        "main",
        &signature(0),
        0,
        main_register_count,
        body(main_instructions, main_register_count),
    );
    let mut module = builder.build();
    module.method_defs[0].param_count = callee_param_count;
    module
}

#[test]
fn call_virt_rejects_receiver_register_out_of_bounds() {
    let module = virtual_call_module(
        &[Instruction::CallVirt {
            r_dst: 0,
            r_obj: 1,
            contract_idx: 0x0a00_0001,
            slot: 0,
            r_base: 0,
            argc: 1,
        }],
        1,
        1,
        1,
    );

    assert_crash(module, 1, "CALL_VIRT: receiver register r1");
}

#[test]
fn call_virt_rejects_argument_count_mismatching_method_metadata() {
    let module = virtual_call_module(
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
            },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: 0x0a00_0001,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
        ],
        2,
        2,
        2,
    );

    assert_crash(module, 1, "CALL_VIRT: argument count 1");
}

#[test]
fn call_virt_rejects_callee_register_capacity_overflow() {
    let module = virtual_call_module(
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
            },
            Instruction::LoadInt { r_dst: 1, value: 7 },
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 0,
                contract_idx: 0x0a00_0001,
                slot: 0,
                r_base: 0,
                argc: 2,
            },
        ],
        3,
        1,
        2,
    );

    assert_crash(module, 1, "arguments exceed callee register count 1");
}

#[test]
fn call_virt_rejects_receiver_outside_argument_block() {
    let module = virtual_call_module(
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
            },
            Instruction::CallVirt {
                r_dst: 0,
                r_obj: 0,
                contract_idx: 0x0a00_0001,
                slot: 0,
                r_base: 1,
                argc: 1,
            },
        ],
        2,
        1,
        1,
    );

    assert_crash(
        module,
        1,
        "receiver register r0 must equal argument base r1",
    );
}

#[test]
fn call_virt_rejects_argument_block_without_receiver() {
    let module = virtual_call_module(
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 0x0200_0001,
            },
            Instruction::CallVirt {
                r_dst: 0,
                r_obj: 0,
                contract_idx: 0x0a00_0001,
                slot: 0,
                r_base: 0,
                argc: 0,
            },
        ],
        1,
        1,
        1,
    );

    assert_crash(module, 1, "argument block must include the receiver");
}

#[test]
fn call_virt_rejects_intrinsic_argument_count_mismatch() {
    let mut builder = ModuleBuilder::new("call-virt-intrinsic-arity");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    let runtime_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let add_ref = builder.add_type_ref(runtime_ref, "Add", "writ");
    builder.add_method(
        "main",
        &signature(0),
        0,
        2,
        body(
            &[
                Instruction::LoadInt { r_dst: 0, value: 7 },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: add_ref.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
            ],
            2,
        ),
    );

    assert_crash(
        builder.build(),
        0,
        "argument count 1 does not match intrinsic parameter count 2",
    );
}

#[test]
fn call_extern_rejects_argument_register_range_out_of_bounds() {
    let mut builder = ModuleBuilder::new("call-extern-source-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::CallExtern {
                r_dst: 0,
                extern_idx: 0,
                r_base: 1,
                argc: 1,
            }],
            1,
        ),
    );

    assert_crash(builder.build(), 0, "CALL_EXTERN: argument register range");
}

#[test]
fn call_extern_rejects_destination_register_out_of_bounds() {
    let mut builder = ModuleBuilder::new("call-extern-destination-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::CallExtern {
                r_dst: 1,
                extern_idx: 0,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );

    assert_crash(builder.build(), 0, "CALL_EXTERN: destination register r1");
}

#[test]
fn call_indirect_rejects_delegate_register_out_of_bounds() {
    let mut builder = ModuleBuilder::new("call-indirect-delegate-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::CallIndirect {
                r_dst: 0,
                r_delegate: 1,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );

    assert_crash(builder.build(), 0, "CALL_INDIRECT: delegate register r1");
}

#[test]
fn call_indirect_rejects_argument_register_range_out_of_bounds() {
    let mut builder = ModuleBuilder::new("call-indirect-source-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::CallIndirect {
                r_dst: 0,
                r_delegate: 0,
                r_base: 1,
                argc: 1,
            }],
            1,
        ),
    );

    assert_crash(builder.build(), 0, "CALL_INDIRECT: argument register range");
}

#[test]
fn call_indirect_rejects_callee_register_capacity_overflow() {
    let mut builder = ModuleBuilder::new("call-indirect-callee-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        2,
        body(
            &[
                Instruction::NewDelegate {
                    r_dst: 0,
                    method_idx: 0x0700_0002,
                    r_target: 1,
                },
                Instruction::CallIndirect {
                    r_dst: 1,
                    r_delegate: 0,
                    r_base: 1,
                    argc: 1,
                },
            ],
            2,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        0,
        body(&[Instruction::RetVoid], 0),
    );

    assert_crash(
        builder.build(),
        0,
        "CALL_INDIRECT: 1 arguments exceed callee register count 0",
    );
}

#[test]
fn tail_call_resolves_methodref_and_switches_module() {
    let method_signature = returning_int_signature(0);

    let mut library = ModuleBuilder::new("tail-call-library");
    let worker = library.add_type_def("Worker", "lib", TypeDefKind::Class, 0);
    library.add_type_method(
        worker,
        "answer",
        &method_signature,
        0,
        2,
        body(
            &[
                Instruction::LoadInt {
                    r_dst: 1,
                    value: 42,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let mut user = ModuleBuilder::new("tail-call-user");
    let library_ref = user.add_module_ref("tail-call-library", "1.0.0");
    let worker_ref = user.add_type_ref(library_ref, "Worker", "lib");
    let answer_ref = user.add_method_ref(worker_ref, "answer", &method_signature);
    user.add_method(
        "main",
        &method_signature,
        0,
        1,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: worker_ref.0,
                },
                Instruction::TailCall {
                    method_idx: answer_ref.0,
                    r_base: 0,
                    argc: 1,
                },
            ],
            1,
        ),
    );

    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library.build())
        .build()
        .expect("build runtime");
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn tail_call_rejects_argument_register_range_out_of_bounds() {
    let mut builder = ModuleBuilder::new("tail-call-source-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::TailCall {
                method_idx: 0x0700_0002,
                r_base: 1,
                argc: 1,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    assert_crash(builder.build(), 0, "TAIL_CALL: argument register range");
}

#[test]
fn tail_call_rejects_argument_count_mismatching_method_metadata() {
    let mut builder = ModuleBuilder::new("tail-call-arity");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[
                Instruction::LoadInt { r_dst: 0, value: 7 },
                Instruction::TailCall {
                    method_idx: 0x0700_0002,
                    r_base: 0,
                    argc: 1,
                },
            ],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(0),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    assert_crash(builder.build(), 0, "TAIL_CALL: argument count 1");
}

#[test]
fn spawn_instructions_resolve_methodrefs_in_the_target_module() {
    let method_signature = returning_int_signature(0);

    let mut library = ModuleBuilder::new("spawn-library");
    let worker = library.add_type_def("Worker", "lib", TypeDefKind::Class, 0);
    library.add_type_method(
        worker,
        "answer",
        &method_signature,
        0,
        2,
        body(
            &[
                Instruction::LoadInt {
                    r_dst: 1,
                    value: 42,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let mut user = ModuleBuilder::new("spawn-user");
    let library_ref = user.add_module_ref("spawn-library", "1.0.0");
    let worker_ref = user.add_type_ref(library_ref, "Worker", "lib");
    let answer_ref = user.add_method_ref(worker_ref, "answer", &method_signature);
    user.add_method(
        "main",
        &method_signature,
        0,
        3,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: worker_ref.0,
                },
                Instruction::SpawnTask {
                    r_dst: 1,
                    method_idx: answer_ref.0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::Join {
                    r_dst: 2,
                    r_task: 1,
                },
                Instruction::Ret { r_src: 2 },
            ],
            3,
        ),
    );

    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library.build())
        .build()
        .expect("build runtime");
    let task = runtime.spawn_task(0, vec![]).expect("spawn main");
    for _ in 0..8 {
        runtime.tick(0.0, ExecutionLimit::None);
        if matches!(
            runtime.task_state(task),
            Some(TaskState::Completed | TaskState::Cancelled)
        ) {
            break;
        }
    }

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn spawn_task_rejects_argument_register_range_out_of_bounds() {
    let mut builder = ModuleBuilder::new("spawn-source-bounds");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 1,
                argc: 1,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    assert_crash(builder.build(), 0, "SPAWN_TASK: argument register range");
}

#[test]
fn spawn_task_rejects_argument_count_mismatching_method_metadata() {
    let mut builder = ModuleBuilder::new("spawn-arity");
    builder.add_type_def("Owner", "test", TypeDefKind::Struct, 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );
    builder.add_method(
        "callee",
        &signature(1),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    assert_crash(builder.build(), 0, "SPAWN_TASK: argument count 0");
}

#[test]
fn spawn_task_rejects_null_method_token_before_creating_child() {
    let mut builder = ModuleBuilder::new("spawn-null-target");
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: 0,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );

    assert_spawn_crash_before_child(builder.build(), "call to null method token");
}

#[test]
fn spawn_task_rejects_extern_token_before_creating_child() {
    let mut builder = ModuleBuilder::new("spawn-extern-target");
    let target = builder.add_extern_def("host_work", &signature(0), "host_work", 0);
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: target.0,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );

    assert_spawn_crash_before_child(
        builder.build(),
        "call uses unsupported method token table 16",
    );
}

#[test]
fn spawn_task_rejects_intrinsic_method_before_creating_child() {
    let mut builder = ModuleBuilder::new("spawn-intrinsic-target");
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );
    builder.add_method(
        "intrinsic",
        &signature(0),
        METHOD_FLAG_INTRINSIC,
        0,
        body(&[], 0),
    );

    assert_spawn_crash_before_child(builder.build(), "runtime-intrinsic");
}

#[test]
fn spawn_task_rejects_empty_bytecode_body_before_creating_child() {
    let mut builder = ModuleBuilder::new("spawn-empty-target");
    builder.add_method(
        "main",
        &signature(0),
        0,
        1,
        body(
            &[Instruction::SpawnTask {
                r_dst: 0,
                method_idx: 0x0700_0002,
                r_base: 0,
                argc: 0,
            }],
            1,
        ),
    );
    builder.add_method("empty", &signature(0), 0, 0, body(&[], 0));

    assert_spawn_crash_before_child(builder.build(), "no executable bytecode body");
}
