use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature, encode_type_signature};
use writ_module::tables::TypeDefKind;
use writ_module::{Instruction, MetadataToken, ModuleBuilder};
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

fn zero_arg_method_signature() -> Vec<u8> {
    encode_method_signature(&[], &TypeSignature::Void).expect("encode method signature")
}

#[test]
fn call_virt_places_receiver_in_callee_register_zero() {
    let mut builder = ModuleBuilder::new("call-virt-self");
    let receiver = builder.add_type_def("Receiver", "test", TypeDefKind::Class, 0);
    builder.add_field_def("value", &[0x01], 0);

    let read = builder.add_contract_def("Read", "test");
    builder.add_contract_method("read", &[], 0);

    let implementation = builder.add_impl_def(receiver, read);
    builder.add_impl_method(
        implementation,
        "read",
        &zero_arg_method_signature(),
        0,
        2,
        body(
            &[
                Instruction::GetField {
                    r_dst: 1,
                    r_obj: 0,
                    field_token: 0x0500_0001,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let main = builder.add_method(
        "main",
        &[],
        0,
        3,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: receiver.0,
                    field_count: 1,
                    r_base: 0,
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
                Instruction::CallVirt {
                    r_dst: 2,
                    r_obj: 0,
                    contract_idx: read.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
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
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task),
        Some(TaskState::Completed),
        "CALL_VIRT failed: {:?}",
        runtime.crash_info(task)
    );
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn list_set_and_custom_iterable_dispatch_sequences_complete() {
    let mut builder = ModuleBuilder::new("generic-iterable-dispatch");

    let list = builder.add_type_def("List", "", TypeDefKind::Class, 0);
    let list_iterator = builder.add_type_def("ListIterator", "", TypeDefKind::Class, 0);
    let set = builder.add_type_def("Set", "", TypeDefKind::Class, 0);
    let set_iterator = builder.add_type_def("SetIterator", "", TypeDefKind::Class, 0);
    let counter = builder.add_type_def("Counter", "", TypeDefKind::Class, 0);
    let counter_iterator = builder.add_type_def("CounterIterator", "", TypeDefKind::Class, 0);

    for row in 1..14 {
        builder.add_contract_def(&format!("Padding{row}"), "writ");
    }
    let iterable = builder.add_contract_def("Iterable", "writ");
    assert_eq!(iterable, MetadataToken::new(10, 14));
    builder.add_contract_method("iterator", &[], 0);
    let iterator = builder.add_contract_def("Iterator", "writ");
    assert_eq!(iterator, MetadataToken::new(10, 15));
    builder.add_contract_method("next", &[], 0);

    for (collection, cursor, label) in [
        (list, list_iterator, "list"),
        (set, set_iterator, "set"),
        (counter, counter_iterator, "custom"),
    ] {
        let iterable_impl = builder.add_impl_def(collection, iterable);
        builder.add_impl_method(
            iterable_impl,
            &format!("{label}_iterator"),
            &zero_arg_method_signature(),
            0,
            2,
            body(
                &[
                    Instruction::New {
                        r_dst: 1,
                        type_idx: cursor.0,
                        field_count: 0,
                        r_base: 0,
                    },
                    Instruction::Ret { r_src: 1 },
                ],
                2,
            ),
        );

        let iterator_impl = builder.add_impl_def(cursor, iterator);
        builder.add_impl_method(
            iterator_impl,
            &format!("{label}_next"),
            &zero_arg_method_signature(),
            0,
            2,
            body(
                &[
                    Instruction::LoadNull { r_dst: 1 },
                    Instruction::Ret { r_src: 1 },
                ],
                2,
            ),
        );
    }

    let mut instructions = Vec::new();
    for collection in [list, set, counter] {
        instructions.extend([
            Instruction::New {
                r_dst: 0,
                type_idx: collection.0,
                field_count: 0,
                r_base: 0,
            },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: iterable.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 1,
                contract_idx: iterator.0,
                slot: 0,
                r_base: 1,
                argc: 1,
            },
            Instruction::IsNone { r_dst: 3, r_opt: 2 },
        ]);
    }
    instructions.extend([
        Instruction::LoadInt { r_dst: 4, value: 3 },
        Instruction::Ret { r_src: 4 },
    ]);

    let main = builder.add_method("main", &[], 0, 5, body(&instructions, 5));
    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().expect("build runtime");
    let task = runtime
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task),
        Some(TaskState::Completed),
        "generic Iterable/Iterator CALL_VIRT dispatch failed: {:?}",
        runtime.crash_info(task)
    );
    assert_eq!(runtime.return_value(task), Some(Value::Int(3)));
}

fn generic_type_spec(
    builder: &mut ModuleBuilder,
    namespace: &str,
    name: &str,
    args: Vec<TypeSignature>,
) -> MetadataToken {
    let signature = encode_type_signature(&TypeSignature::Generic {
        namespace: namespace.to_string(),
        name: name.to_string(),
        args,
    })
    .expect("encode TypeSpec signature");
    builder.add_type_spec(&signature)
}

fn add_constant_impl(
    builder: &mut ModuleBuilder,
    target: MetadataToken,
    contract: MetadataToken,
    name: &str,
    value: i64,
) {
    let implementation = builder.add_impl_def(target, contract);
    builder.add_impl_method(
        implementation,
        name,
        &zero_arg_method_signature(),
        0,
        2,
        body(
            &[
                Instruction::LoadInt { r_dst: 1, value },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );
}

#[test]
fn cross_module_typespec_dispatch_distinguishes_named_specializations() {
    let mut library = ModuleBuilder::new("generic-dispatch-library");
    let receiver = library.add_type_def("Receiver", "lib", TypeDefKind::Class, 0);
    let arg_a = library.add_type_def("ArgA", "lib", TypeDefKind::Class, 0);
    let arg_b = library.add_type_def("ArgB", "lib", TypeDefKind::Class, 0);
    let _select = library.add_contract_def("Select", "lib");
    library.add_contract_method("select", &[], 0);

    let library_specialization = |argument| {
        encode_type_signature(&TypeSignature::Generic {
            namespace: "lib".to_string(),
            name: "Select".to_string(),
            args: vec![TypeSignature::Named(argument)],
        })
        .unwrap()
    };
    let select_a = library.add_type_spec(&library_specialization(arg_a));
    let select_b = library.add_type_spec(&library_specialization(arg_b));

    for (contract, value, name) in [(select_a, 11, "select_a"), (select_b, 22, "select_b")] {
        let implementation = library.add_impl_def(receiver, contract);
        library.add_impl_method(
            implementation,
            name,
            &zero_arg_method_signature(),
            0,
            2,
            body(
                &[
                    Instruction::LoadInt { r_dst: 1, value },
                    Instruction::Ret { r_src: 1 },
                ],
                2,
            ),
        );
    }
    let library = library.build();

    let mut user = ModuleBuilder::new("generic-dispatch-user");
    let library_ref = user.add_module_ref("generic-dispatch-library", "1.0.0");
    let receiver_ref = user.add_type_ref(library_ref, "Receiver", "lib");
    let arg_a_ref = user.add_type_ref(library_ref, "ArgA", "lib");
    let arg_b_ref = user.add_type_ref(library_ref, "ArgB", "lib");
    user.add_type_ref(library_ref, "Select", "lib");

    let user_specialization = |argument| {
        encode_type_signature(&TypeSignature::Generic {
            namespace: "lib".to_string(),
            name: "Select".to_string(),
            args: vec![TypeSignature::Named(argument)],
        })
        .unwrap()
    };
    let select_a_ref = user.add_type_spec(&user_specialization(arg_a_ref));
    let select_b_ref = user.add_type_spec(&user_specialization(arg_b_ref));

    let main = user.add_method(
        "main",
        &[],
        0,
        4,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: receiver_ref.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: select_a_ref.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::CallVirt {
                    r_dst: 2,
                    r_obj: 0,
                    contract_idx: select_b_ref.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::AddI {
                    r_dst: 3,
                    r_a: 1,
                    r_b: 2,
                },
                Instruction::Ret { r_src: 3 },
            ],
            4,
        ),
    );

    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library)
        .build()
        .expect("build cross-module runtime");
    let task = runtime
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task),
        Some(TaskState::Completed),
        "TypeSpec dispatch failed: {:?}",
        runtime.crash_info(task)
    );
    assert_eq!(runtime.return_value(task), Some(Value::Int(33)));
}

#[test]
fn target_typespec_dispatch_distinguishes_same_bare_contract() {
    let mut builder = ModuleBuilder::new("target-typespec-dispatch");
    builder.add_type_def("Crate", "test", TypeDefKind::Class, 0);
    let carries = builder.add_contract_def("Carries", "test");
    builder.add_contract_method("value", &[], 0);

    let crate_int = generic_type_spec(&mut builder, "test", "Crate", vec![TypeSignature::Int]);
    let crate_string =
        generic_type_spec(&mut builder, "test", "Crate", vec![TypeSignature::String]);
    add_constant_impl(&mut builder, crate_int, carries, "int_value", 11);
    add_constant_impl(&mut builder, crate_string, carries, "string_value", 22);

    let main = builder.add_method(
        "main",
        &[],
        0,
        5,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: crate_int.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: carries.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::New {
                    r_dst: 2,
                    type_idx: crate_string.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 3,
                    r_obj: 2,
                    contract_idx: carries.0,
                    slot: 0,
                    r_base: 2,
                    argc: 1,
                },
                Instruction::AddI {
                    r_dst: 4,
                    r_a: 1,
                    r_b: 3,
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
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(33)));
}

#[test]
fn disjoint_open_target_patterns_do_not_collapse_to_one_dispatch_entry() {
    let mut builder = ModuleBuilder::new("open-pattern-dispatch");
    builder.add_type_def("Crate", "test", TypeDefKind::Class, 0);
    builder.add_type_def("Box", "test", TypeDefKind::Class, 0);
    builder.add_type_def("Pair", "test", TypeDefKind::Class, 0);
    let carries = builder.add_contract_def("Carries", "test");
    builder.add_contract_method("value", &[], 0);

    let boxed_param = TypeSignature::Generic {
        namespace: "test".to_string(),
        name: "Box".to_string(),
        args: vec![TypeSignature::GenericParam(0)],
    };
    let paired_param = TypeSignature::Generic {
        namespace: "test".to_string(),
        name: "Pair".to_string(),
        args: vec![TypeSignature::GenericParam(0), TypeSignature::Int],
    };
    let boxed_pattern = generic_type_spec(&mut builder, "test", "Crate", vec![boxed_param]);
    let paired_pattern = generic_type_spec(&mut builder, "test", "Crate", vec![paired_param]);
    add_constant_impl(&mut builder, boxed_pattern, carries, "boxed", 11);
    add_constant_impl(&mut builder, paired_pattern, carries, "paired", 22);

    let boxed_int = TypeSignature::Generic {
        namespace: "test".to_string(),
        name: "Box".to_string(),
        args: vec![TypeSignature::Int],
    };
    let paired_string = TypeSignature::Generic {
        namespace: "test".to_string(),
        name: "Pair".to_string(),
        args: vec![TypeSignature::String, TypeSignature::Int],
    };
    let crate_boxed_int = generic_type_spec(&mut builder, "test", "Crate", vec![boxed_int]);
    let crate_paired_string = generic_type_spec(&mut builder, "test", "Crate", vec![paired_string]);

    let main = builder.add_method(
        "main",
        &[],
        0,
        5,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: crate_boxed_int.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: carries.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::New {
                    r_dst: 2,
                    type_idx: crate_paired_string.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 3,
                    r_obj: 2,
                    contract_idx: carries.0,
                    slot: 0,
                    r_base: 2,
                    argc: 1,
                },
                Instruction::AddI {
                    r_dst: 4,
                    r_a: 1,
                    r_b: 3,
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
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(33)));
}

#[test]
fn correlated_target_and_contract_parameters_reject_mismatched_call() {
    let mut builder = ModuleBuilder::new("correlated-pattern-dispatch");
    builder.add_type_def("Crate", "test", TypeDefKind::Class, 0);
    builder.add_contract_def("Carries", "test");
    builder.add_contract_method("value", &[], 0);

    let crate_param = generic_type_spec(
        &mut builder,
        "test",
        "Crate",
        vec![TypeSignature::GenericParam(0)],
    );
    let carries_param = generic_type_spec(
        &mut builder,
        "test",
        "Carries",
        vec![TypeSignature::GenericParam(0)],
    );
    add_constant_impl(&mut builder, crate_param, carries_param, "value", 1);

    let crate_int = generic_type_spec(&mut builder, "test", "Crate", vec![TypeSignature::Int]);
    let carries_string =
        generic_type_spec(&mut builder, "test", "Carries", vec![TypeSignature::String]);
    let main = builder.add_method(
        "main",
        &[],
        0,
        2,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: crate_int.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: carries_string.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(task).expect("dispatch must fail closed");
    assert!(
        crash.message.contains("CALL_VIRT: no implementation"),
        "unexpected crash: {}",
        crash.message
    );
}

#[test]
fn ambiguous_structural_specializations_crash_deterministically() {
    let mut builder = ModuleBuilder::new("ambiguous-pattern-dispatch");
    builder.add_type_def("Crate", "test", TypeDefKind::Class, 0);
    builder.add_contract_def("Carries", "test");
    builder.add_contract_method("value", &[], 0);

    let crate_param = generic_type_spec(
        &mut builder,
        "test",
        "Crate",
        vec![TypeSignature::GenericParam(0)],
    );
    let carries_param = generic_type_spec(
        &mut builder,
        "test",
        "Carries",
        vec![TypeSignature::GenericParam(0)],
    );
    add_constant_impl(&mut builder, crate_param, carries_param, "first", 1);
    add_constant_impl(&mut builder, crate_param, carries_param, "second", 2);

    let crate_int = generic_type_spec(&mut builder, "test", "Crate", vec![TypeSignature::Int]);
    let carries_int = generic_type_spec(&mut builder, "test", "Carries", vec![TypeSignature::Int]);
    let main = builder.add_method(
        "main",
        &[],
        0,
        2,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: crate_int.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: carries_int.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime
        .crash_info(task)
        .expect("ambiguous dispatch must crash");
    assert!(
        crash
            .message
            .contains("ambiguous implementation (2 matches)"),
        "unexpected crash: {}",
        crash.message
    );
}

#[test]
fn malformed_contract_typespec_fails_closed() {
    let mut builder = ModuleBuilder::new("malformed-contract-typespec");
    let receiver = builder.add_type_def("Receiver", "test", TypeDefKind::Class, 0);
    let contract = builder.add_contract_def("Contract", "test");
    builder.add_contract_method("value", &[], 0);
    add_constant_impl(&mut builder, receiver, contract, "value", 1);
    let malformed = builder.add_type_spec(&[0xff]);

    let main = builder.add_method(
        "main",
        &[],
        0,
        2,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: receiver.0,
                    field_count: 0,
                    r_base: 0,
                },
                Instruction::CallVirt {
                    r_dst: 1,
                    r_obj: 0,
                    contract_idx: malformed.0,
                    slot: 0,
                    r_base: 0,
                    argc: 1,
                },
                Instruction::Ret { r_src: 1 },
            ],
            2,
        ),
    );

    let mut runtime = RuntimeBuilder::new(builder.build())
        .build()
        .expect("build runtime");
    let task = runtime
        .spawn_task(main.row_index().expect("main row") as usize - 1, vec![])
        .expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task), Some(TaskState::Cancelled));
    let crash = runtime
        .crash_info(task)
        .expect("malformed TypeSpec must crash");
    assert!(
        crash.message.contains("unresolved contract token")
            || crash.message.contains("malformed contract TypeSpec"),
        "unexpected crash: {}",
        crash.message
    );
}
