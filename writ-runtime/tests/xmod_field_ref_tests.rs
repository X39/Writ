use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature, encode_type_signature};
use writ_module::tables::{TableId, TypeDefKind};
use writ_module::{Instruction, MetadataToken, Module, ModuleBuilder};
use writ_runtime::{Domain, ExecutionLimit, RuntimeBuilder, TaskState, Value};

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

fn compile(source: &'static str) -> Module {
    let bytes = writ_compiler::compile_source(source).expect("library should compile");
    Module::from_bytes(&bytes).expect("library module should decode")
}

fn main_method(module: &Module) -> usize {
    module
        .top_level_method_indices()
        .into_iter()
        .find(|index| {
            writ_module::heap::read_string(&module.string_heap, module.method_defs[*index].name)
                .ok()
                == Some("main")
        })
        .expect("main method")
}

#[test]
fn compiled_cross_module_second_field_set_and_get_execute() {
    let library = compile(
        r#"
            pub class Pair {
                pub first: int,
                pub second: int
            }
        "#,
    );
    let user_bytes = writ_compiler::compile_with_libraries(
        r#"
            pub fn main() -> int {
                let mut pair = new Pair { first: 1, second: 2 };
                pair.second = 42;
                return pair.second;
            }
        "#,
        &[&library],
    )
    .expect("consumer should compile");
    let user = Module::from_bytes(&user_bytes).expect("consumer module should decode");
    let main = main_method(&user);

    let mut cursor = std::io::Cursor::new(&user.method_bodies[main].code);
    let mut field_operands = Vec::new();
    while (cursor.position() as usize) < user.method_bodies[main].code.len() {
        match Instruction::decode(&mut cursor).expect("decode main instruction") {
            Instruction::GetField { field_idx, .. } | Instruction::SetField { field_idx, .. } => {
                field_operands.push(field_idx)
            }
            _ => {}
        }
    }
    assert!(!field_operands.is_empty());
    assert!(
        field_operands
            .iter()
            .all(|operand| { MetadataToken(*operand).table_id() == TableId::FieldRef.as_u8() })
    );

    let mut runtime = RuntimeBuilder::new(user)
        .with_library(library)
        .build()
        .expect("consumer should link");
    let task = runtime.spawn_task(main, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn fieldref_row_order_is_independent_from_target_field_layout() {
    let int_sig = encode_type_signature(&TypeSignature::Int).unwrap();
    let method_sig = encode_method_signature(&[], &TypeSignature::Int).unwrap();

    let mut library = ModuleBuilder::new("field-layout-library");
    library.add_type_def("Pair", "test", TypeDefKind::Class, 0);
    library.add_field_def("first", &int_sig, 0);
    library.add_field_def("second", &int_sig, 0);
    let library = library.build();

    let mut user = ModuleBuilder::new("field-layout-user");
    let library_ref = user.add_module_ref("field-layout-library", "1.0.0");
    let pair_ref = user.add_type_ref(library_ref, "Pair", "test");
    let second_ref = user.add_field_ref(pair_ref, "second", &int_sig);
    user.add_field_ref(pair_ref, "first", &int_sig);
    user.add_method(
        "main",
        &method_sig,
        0,
        3,
        body(
            &[
                Instruction::New {
                    r_dst: 0,
                    type_idx: pair_ref.0,
                },
                Instruction::LoadInt {
                    r_dst: 1,
                    value: 42,
                },
                Instruction::SetField {
                    r_obj: 0,
                    field_idx: second_ref.0,
                    r_val: 1,
                },
                Instruction::GetField {
                    r_dst: 2,
                    r_obj: 0,
                    field_idx: second_ref.0,
                },
                Instruction::Ret { r_src: 2 },
            ],
            3,
        ),
    );

    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library)
        .build()
        .expect("FieldRefs should link");
    let task = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn same_name_fieldref_with_wrong_type_is_rejected() {
    let int_sig = encode_type_signature(&TypeSignature::Int).unwrap();
    let string_sig = encode_type_signature(&TypeSignature::String).unwrap();
    let mut library = ModuleBuilder::new("field-type-library");
    library.add_type_def("Record", "test", TypeDefKind::Struct, 0);
    library.add_field_def("value", &int_sig, 0);

    let mut user = ModuleBuilder::new("field-type-user");
    let library_ref = user.add_module_ref("field-type-library", "1.0.0");
    let record_ref = user.add_type_ref(library_ref, "Record", "test");
    user.add_field_ref(record_ref, "value", &string_sig);

    let mut domain = Domain::new();
    domain.add_module(library.build()).unwrap();
    domain.add_module(user.build()).unwrap();
    let error = domain.resolve_refs().unwrap_err().to_string();
    assert!(error.contains("unresolved field reference"), "{error}");
    assert!(error.contains("value"), "{error}");
}

#[test]
fn malformed_fieldref_signature_is_rejected() {
    let int_sig = encode_type_signature(&TypeSignature::Int).unwrap();
    let mut library = ModuleBuilder::new("malformed-field-library");
    library.add_type_def("Record", "test", TypeDefKind::Struct, 0);
    library.add_field_def("value", &int_sig, 0);

    let mut user = ModuleBuilder::new("malformed-field-user");
    let library_ref = user.add_module_ref("malformed-field-library", "1.0.0");
    let record_ref = user.add_type_ref(library_ref, "Record", "test");
    user.add_field_ref(record_ref, "value", &[0xff]);

    let mut domain = Domain::new();
    domain.add_module(library.build()).unwrap();
    domain.add_module(user.build()).unwrap();
    let error = domain.resolve_refs().unwrap_err().to_string();
    assert!(error.contains("invalid FieldRef type signature"), "{error}");
}

#[test]
fn fieldref_named_type_signature_uses_canonical_cross_module_identity() {
    let mut library = ModuleBuilder::new("canonical-field-library");
    let payload = library.add_type_def("Payload", "test", TypeDefKind::Class, 0);
    let holder = library.add_type_def("Holder", "test", TypeDefKind::Struct, 0);
    let definition_sig = encode_type_signature(&TypeSignature::Named(payload)).unwrap();
    library.add_field_def("payload", &definition_sig, 0);

    let mut user = ModuleBuilder::new("canonical-field-user");
    let library_ref = user.add_module_ref("canonical-field-library", "1.0.0");
    let payload_ref = user.add_type_ref(library_ref, "Payload", "test");
    let holder_ref = user.add_type_ref(library_ref, "Holder", "test");
    let reference_sig = encode_type_signature(&TypeSignature::Named(payload_ref)).unwrap();
    user.add_field_ref(holder_ref, "payload", &reference_sig);

    let mut domain = Domain::new();
    domain.add_module(library.build()).unwrap();
    domain.add_module(user.build()).unwrap();
    domain
        .resolve_refs()
        .expect("canonical signatures should match");
    let resolved = domain.modules[1].resolved_refs.fields[&0];
    assert_eq!(resolved.module_idx, 0);
    assert_eq!(
        resolved.owner_type_idx,
        holder.row_index().unwrap() as usize - 1
    );
    assert_eq!(resolved.field_offset, 0);
}

#[test]
fn fieldref_typespec_parent_fails_explicitly() {
    let int_sig = encode_type_signature(&TypeSignature::Int).unwrap();
    let mut library = ModuleBuilder::new("generic-field-library");
    library.add_type_def("Box", "test", TypeDefKind::Struct, 0);
    library.add_field_def("value", &int_sig, 0);

    let mut user = ModuleBuilder::new("generic-field-user");
    let library_ref = user.add_module_ref("generic-field-library", "1.0.0");
    let box_ref = user.add_type_ref(library_ref, "Box", "test");
    let parent_sig = encode_type_signature(&TypeSignature::Generic {
        namespace: "test".into(),
        name: "Box".into(),
        args: vec![TypeSignature::Int],
    })
    .unwrap();
    let box_spec = user.add_type_spec(&parent_sig);
    user.add_field_ref(box_spec, "value", &int_sig);

    let mut domain = Domain::new();
    domain.add_module(library.build()).unwrap();
    domain.add_module(user.build()).unwrap();
    let error = domain.resolve_refs().unwrap_err().to_string();
    assert!(
        error.contains("FieldRef TypeSpec parents are not supported"),
        "{error}"
    );

    assert_eq!(box_ref.table_id(), TableId::TypeRef.as_u8());
}
