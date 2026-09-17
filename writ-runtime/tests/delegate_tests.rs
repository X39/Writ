use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature};
use writ_module::tables::{METHOD_FLAG_INTRINSIC, TypeDefKind};
use writ_module::{Instruction, MetadataToken, Module, ModuleBuilder};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState, Value};

const METHOD_STATIC: u16 = 1 << 1;

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

fn signature(params: &[TypeSignature], ret: TypeSignature) -> Vec<u8> {
    encode_method_signature(params, &ret).expect("encode method signature")
}

fn row_index(token: MetadataToken) -> usize {
    token.row_index().expect("non-null token") as usize - 1
}

fn assert_crash(module: Module, main: MetadataToken, expected: &str) {
    let mut runtime = RuntimeBuilder::new(module).build().expect("build runtime");
    let task = runtime
        .spawn_task(row_index(main), vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(task).expect("delegate call must crash");
    assert!(
        crash.message.contains(expected),
        "expected crash containing {expected:?}, got {:?}",
        crash.message
    );
}

fn assert_crash_with_libraries(
    module: Module,
    libraries: Vec<Module>,
    main: MetadataToken,
    expected: &str,
) {
    let mut builder = RuntimeBuilder::new(module);
    for library in libraries {
        builder = builder.with_library(library);
    }
    let mut runtime = builder.build().expect("build runtime");
    let task = runtime
        .spawn_task(row_index(main), vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime
        .crash_info(task)
        .expect("delegate operation must crash");
    assert!(
        crash.message.contains(expected),
        "expected crash containing {expected:?}, got {:?}",
        crash.message
    );
}

#[test]
fn call_indirect_prepends_captured_target_before_explicit_arguments() {
    let mut builder = ModuleBuilder::new("delegate-capture");
    let env = builder.add_type_def("Env", "test", TypeDefKind::Class, 0);
    builder.add_field_def("captured", &[0x01], 0);

    let invoke = builder.add_type_method(
        env,
        "invoke",
        &signature(&[TypeSignature::Int], TypeSignature::Int),
        0,
        3,
        body(
            &[
                Instruction::GetField {
                    r_dst: 2,
                    r_obj: 0,
                    field_token: 0x0500_0001,
                },
                Instruction::AddI {
                    r_dst: 2,
                    r_a: 2,
                    r_b: 1,
                },
                Instruction::Ret { r_src: 2 },
            ],
            3,
        ),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Int),
        0,
        5,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: env.0,
                    field_count: 1,
                    r_base: 0,
                },
                Instruction::LoadInt {
                    r_dst: 1,
                    value: 40,
                },
                Instruction::SetField {
                    r_obj: 0,
                    field_token: 0x0500_0001,
                    r_val: 1,
                },
                Instruction::NewDelegate {
                    r_dst: 2,
                    method_idx: invoke.0,
                    r_target: 0,
                },
                Instruction::LoadInt { r_dst: 3, value: 2 },
                Instruction::CallIndirect {
                    r_dst: 4,
                    r_delegate: 2,
                    r_base: 3,
                    argc: 1,
                },
                Instruction::Ret { r_src: 4 },
            ],
            5,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(row_index(main), vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn call_indirect_with_null_target_passes_only_explicit_arguments() {
    let mut builder = ModuleBuilder::new("delegate-null-target");
    let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[], TypeSignature::Int),
        METHOD_STATIC,
        1,
        body(
            &[
                Instruction::LoadInt {
                    r_dst: 0,
                    value: 77,
                },
                Instruction::Ret { r_src: 0 },
            ],
            1,
        ),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Int),
        0,
        3,
        body(
            &[
                Instruction::NewDelegate {
                    r_dst: 1,
                    method_idx: invoke.0,
                    r_target: 0,
                },
                Instruction::CallIndirect {
                    r_dst: 2,
                    r_delegate: 1,
                    r_base: 0,
                    argc: 0,
                },
                Instruction::Ret { r_src: 2 },
            ],
            3,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(row_index(main), vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(77)));
}

#[test]
fn delegate_resolves_cross_module_method_ref_when_created() {
    let method_signature = signature(&[TypeSignature::Int], TypeSignature::Int);

    let mut library = ModuleBuilder::new("delegate-library");
    let exports = library.add_type_def("Exports", "lib", TypeDefKind::Struct, 0);
    library.add_type_method(
        exports,
        "identity",
        &method_signature,
        METHOD_STATIC,
        1,
        body(&[Instruction::Ret { r_src: 0 }], 1),
    );

    let mut user = ModuleBuilder::new("delegate-user");
    user.add_type_def("Main", "test", TypeDefKind::Struct, 0);
    let library_ref = user.add_module_ref("delegate-library", "1.0.0");
    let exports_ref = user.add_type_ref(library_ref, "Exports", "lib");
    let identity_ref =
        user.add_method_ref_with_flags(exports_ref, "identity", &method_signature, 0);
    let main = user.add_method(
        "main",
        &signature(&[], TypeSignature::Int),
        0,
        4,
        body(
            &[
                Instruction::NewDelegate {
                    r_dst: 1,
                    method_idx: identity_ref.0,
                    r_target: 0,
                },
                Instruction::LoadInt {
                    r_dst: 2,
                    value: 91,
                },
                Instruction::CallIndirect {
                    r_dst: 3,
                    r_delegate: 1,
                    r_base: 2,
                    argc: 1,
                },
                Instruction::Ret { r_src: 3 },
            ],
            4,
        ),
    );

    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(row_index(main), vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(91)));
}

#[test]
fn new_delegate_rejects_bound_static_method_ref() {
    let method_signature = signature(&[TypeSignature::Int], TypeSignature::Void);

    let mut library = ModuleBuilder::new("delegate-static-library");
    let exports = library.add_type_def("Exports", "lib", TypeDefKind::Struct, 0);
    library.add_type_method(
        exports,
        "consume",
        &method_signature,
        METHOD_STATIC,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    let mut user = ModuleBuilder::new("delegate-static-user");
    let library_ref = user.add_module_ref("delegate-static-library", "1.0.0");
    let exports_ref = user.add_type_ref(library_ref, "Exports", "lib");
    let consume_ref = user.add_method_ref_with_flags(exports_ref, "consume", &method_signature, 0);
    let main = user.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        2,
        body(
            &[
                Instruction::LoadInt {
                    r_dst: 0,
                    value: 42,
                },
                Instruction::NewDelegate {
                    r_dst: 1,
                    method_idx: consume_ref.0,
                    r_target: 0,
                },
                Instruction::RetVoid,
            ],
            2,
        ),
    );

    assert_crash_with_libraries(
        user.build(),
        vec![library.build()],
        main,
        "requires a null delegate target",
    );
}

#[test]
fn new_delegate_rejects_unbound_instance_method_ref() {
    let method_signature = signature(&[], TypeSignature::Void);

    let mut library = ModuleBuilder::new("delegate-instance-library");
    let receiver = library.add_type_def("Receiver", "lib", TypeDefKind::Class, 0);
    library.add_type_method(
        receiver,
        "invoke",
        &method_signature,
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );

    let mut user = ModuleBuilder::new("delegate-instance-user");
    let library_ref = user.add_module_ref("delegate-instance-library", "1.0.0");
    let receiver_ref = user.add_type_ref(library_ref, "Receiver", "lib");
    let invoke_ref = user.add_method_ref_with_flags(
        receiver_ref,
        "invoke",
        &method_signature,
        writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER,
    );
    let main = user.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        1,
        body(
            &[
                Instruction::NewDelegate {
                    r_dst: 0,
                    method_idx: invoke_ref.0,
                    r_target: 0,
                },
                Instruction::RetVoid,
            ],
            1,
        ),
    );

    assert_crash_with_libraries(
        user.build(),
        vec![library.build()],
        main,
        "requires a non-null delegate target",
    );
}

#[test]
fn call_indirect_rejects_bound_static_delegate_allocated_via_heap() {
    let mut builder = ModuleBuilder::new("delegate-bound-static-heap");
    let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[TypeSignature::Int], TypeSignature::Void),
        METHOD_STATIC,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let main = builder.add_method(
        "main",
        &signature(&[TypeSignature::Int], TypeSignature::Void),
        0,
        2,
        body(
            &[
                Instruction::CallIndirect {
                    r_dst: 1,
                    r_delegate: 0,
                    r_base: 1,
                    argc: 0,
                },
                Instruction::RetVoid,
            ],
            2,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let user_module_idx = runtime.user_module_idx();
    let delegate =
        runtime
            .heap_mut()
            .alloc_delegate(user_module_idx, row_index(invoke), Some(Value::Int(42)));
    let task = runtime
        .spawn_task(row_index(main), vec![Value::Ref(delegate)])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(task).expect("delegate call must crash");
    assert!(
        crash.message.contains("requires a null delegate target"),
        "unexpected crash: {}",
        crash.message
    );
}

#[test]
fn call_indirect_rejects_unbound_instance_delegate_allocated_via_heap() {
    let mut builder = ModuleBuilder::new("delegate-unbound-instance-heap");
    let owner = builder.add_type_def("Receiver", "test", TypeDefKind::Class, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[], TypeSignature::Void),
        0,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let main = builder.add_method(
        "main",
        &signature(
            &[TypeSignature::Int, TypeSignature::Int],
            TypeSignature::Void,
        ),
        0,
        2,
        body(
            &[
                Instruction::CallIndirect {
                    r_dst: 0,
                    r_delegate: 0,
                    r_base: 1,
                    argc: 1,
                },
                Instruction::RetVoid,
            ],
            2,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let user_module_idx = runtime.user_module_idx();
    let delegate = runtime
        .heap_mut()
        .alloc_delegate(user_module_idx, row_index(invoke), None);
    let task = runtime
        .spawn_task(row_index(main), vec![Value::Ref(delegate), Value::Int(7)])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(task).expect("delegate call must crash");
    assert!(
        crash
            .message
            .contains("requires a non-null delegate target"),
        "unexpected crash: {}",
        crash.message
    );
}

#[test]
fn new_delegate_rejects_target_register_out_of_bounds() {
    let mut builder = ModuleBuilder::new("delegate-target-bounds");
    let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[], TypeSignature::Void),
        METHOD_STATIC,
        0,
        body(&[Instruction::RetVoid], 0),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        1,
        body(
            &[Instruction::NewDelegate {
                r_dst: 0,
                method_idx: invoke.0,
                r_target: 1,
            }],
            1,
        ),
    );

    assert_crash(builder.build(), main, "NEW_DELEGATE: target register r1");
}

#[test]
fn new_delegate_rejects_mismatched_method_body_metadata() {
    let mut builder = ModuleBuilder::new("delegate-method-metadata");
    let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[], TypeSignature::Void),
        METHOD_STATIC,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        1,
        body(
            &[Instruction::NewDelegate {
                r_dst: 0,
                method_idx: invoke.0,
                r_target: 0,
            }],
            1,
        ),
    );
    let mut module = builder.build();
    module.method_defs[row_index(invoke)].reg_count = 2;

    assert_crash(
        module,
        main,
        "NEW_DELEGATE: MethodDef.reg_count 2 does not match",
    );
}

#[test]
fn call_indirect_rejects_exact_arity_mismatch() {
    let mut builder = ModuleBuilder::new("delegate-arity");
    let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[TypeSignature::Int], TypeSignature::Void),
        METHOD_STATIC,
        1,
        body(&[Instruction::RetVoid], 1),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        2,
        body(
            &[
                Instruction::NewDelegate {
                    r_dst: 1,
                    method_idx: invoke.0,
                    r_target: 0,
                },
                Instruction::CallIndirect {
                    r_dst: 0,
                    r_delegate: 1,
                    r_base: 0,
                    argc: 0,
                },
            ],
            2,
        ),
    );

    assert_crash(
        builder.build(),
        main,
        "CALL_INDIRECT: argument count 0 does not match MethodDef.param_count 1",
    );
}

#[test]
fn call_indirect_rejects_implicit_target_beyond_callee_registers() {
    let mut builder = ModuleBuilder::new("delegate-target-capacity");
    let owner = builder.add_type_def("Receiver", "test", TypeDefKind::Class, 0);
    let invoke = builder.add_type_method(
        owner,
        "invoke",
        &signature(&[], TypeSignature::Void),
        0,
        0,
        body(&[Instruction::RetVoid], 0),
    );
    let main = builder.add_method(
        "main",
        &signature(&[], TypeSignature::Void),
        0,
        2,
        body(
            &[
                Instruction::LoadInt { r_dst: 0, value: 1 },
                Instruction::NewDelegate {
                    r_dst: 1,
                    method_idx: invoke.0,
                    r_target: 0,
                },
                Instruction::CallIndirect {
                    r_dst: 0,
                    r_delegate: 1,
                    r_base: 0,
                    argc: 0,
                },
            ],
            2,
        ),
    );

    assert_crash(
        builder.build(),
        main,
        "CALL_INDIRECT: 1 arguments exceed callee register count 0",
    );
}

#[test]
fn new_delegate_rejects_non_executable_method_before_allocation() {
    for (case, flags, expected) in [
        (
            "intrinsic",
            METHOD_STATIC | METHOD_FLAG_INTRINSIC,
            "runtime-intrinsic",
        ),
        ("empty", METHOD_STATIC, "no executable bytecode body"),
    ] {
        let mut builder = ModuleBuilder::new(&format!("delegate-new-{case}"));
        let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
        let invoke = builder.add_type_method(
            owner,
            "invoke",
            &signature(&[], TypeSignature::Void),
            flags,
            0,
            body(&[], 0),
        );
        let main = builder.add_method(
            "main",
            &signature(&[], TypeSignature::Void),
            0,
            1,
            body(
                &[Instruction::NewDelegate {
                    r_dst: 0,
                    method_idx: invoke.0,
                    r_target: 0,
                }],
                1,
            ),
        );

        let mut runtime = RuntimeBuilder::new(builder.build())
            .build()
            .expect("build runtime");
        let heap_before = runtime.heap().object_count();
        let task = runtime
            .spawn_task(row_index(main), vec![])
            .expect("spawn main");
        runtime.run_task(task, ExecutionLimit::Instructions(1));

        assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
        assert_eq!(
            runtime.heap().object_count(),
            heap_before,
            "NEW_DELEGATE must reject {case} targets before allocating"
        );
        let crash = runtime
            .crash_info(task)
            .expect("delegate creation must crash");
        assert!(
            crash.message.contains(expected),
            "expected {case} crash containing {expected:?}, got {:?}",
            crash.message
        );
    }
}

#[test]
fn call_indirect_rejects_non_executable_method_before_frame_push() {
    for (case, flags, expected) in [
        (
            "intrinsic",
            METHOD_STATIC | METHOD_FLAG_INTRINSIC,
            "runtime-intrinsic",
        ),
        ("empty", METHOD_STATIC, "no executable bytecode body"),
    ] {
        let mut builder = ModuleBuilder::new(&format!("delegate-call-{case}"));
        let owner = builder.add_type_def("Functions", "test", TypeDefKind::Struct, 0);
        let invoke = builder.add_type_method(
            owner,
            "invoke",
            &signature(&[], TypeSignature::Void),
            flags,
            0,
            body(&[], 0),
        );
        let main = builder.add_method(
            "main",
            &signature(&[TypeSignature::Int], TypeSignature::Void),
            0,
            1,
            body(
                &[Instruction::CallIndirect {
                    r_dst: 0,
                    r_delegate: 0,
                    r_base: 0,
                    argc: 0,
                }],
                1,
            ),
        );

        let mut runtime = RuntimeBuilder::new(builder.build())
            .build()
            .expect("build runtime");
        let user_module_idx = runtime.user_module_idx();
        let delegate = runtime
            .heap_mut()
            .alloc_delegate(user_module_idx, row_index(invoke), None);
        let task = runtime
            .spawn_task(row_index(main), vec![Value::Ref(delegate)])
            .expect("spawn main");
        runtime.run_task(task, ExecutionLimit::Instructions(1));

        assert_eq!(
            runtime.task_state(task),
            Some(TaskState::Cancelled),
            "CALL_INDIRECT must reject {case} targets in the calling instruction"
        );
        let crash = runtime.crash_info(task).expect("delegate call must crash");
        assert!(
            crash.message.contains(expected),
            "expected {case} crash containing {expected:?}, got {:?}",
            crash.message
        );
    }
}
