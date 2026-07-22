use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature};
use writ_module::tables::TypeDefKind;
use writ_module::{Instruction, Module, ModuleBuilder};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState};

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
            Instruction::LoadInt {
                r_dst: 1,
                value: 7,
            },
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

    assert_crash(module, 1, "receiver register r0 must equal argument base r1");
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
                Instruction::LoadInt {
                    r_dst: 0,
                    value: 7,
                },
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

    assert_crash(
        builder.build(),
        0,
        "CALL_INDIRECT: delegate register r1",
    );
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
