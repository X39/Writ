use writ_module::module::MethodBody;
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
            &[],
            0,
            2,
            body(
                &[
                    Instruction::New {
                        r_dst: 1,
                        type_idx: cursor.0,
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
            &[],
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
            Instruction::IsNone {
                r_dst: 3,
                r_opt: 2,
            },
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
