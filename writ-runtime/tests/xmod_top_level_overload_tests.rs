use std::collections::BTreeSet;

use writ_module::signature::{TypeSignature, decode_method_signature};
use writ_module::tables::TableId;
use writ_module::{Instruction, MetadataToken};
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState, Value};

fn assert_top_level_overloads(library_source: &'static str) {
    let library_bytes =
        writ_compiler::compile_source(library_source).expect("overloaded library should compile");
    let library =
        writ_module::Module::from_bytes(&library_bytes).expect("decode overloaded library");

    let user_bytes = writ_compiler::compile_with_libraries(
        r#"
            pub fn main() -> int {
                let from_int = pick(7);
                let from_string = pick("seven");
                return from_int + from_string;
            }
        "#,
        &[&library],
    )
    .expect("both imported overloads should type-check");
    let user = writ_module::Module::from_bytes(&user_bytes).expect("decode overload consumer");

    let pick_refs: Vec<_> = user
        .method_refs
        .iter()
        .enumerate()
        .filter_map(|(index, method)| {
            (writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("pick"))
                .then_some((index, method))
        })
        .collect();
    assert_eq!(pick_refs.len(), 2, "one MethodRef per selected overload");

    let mut parameter_kinds = BTreeSet::new();
    let pick_tokens: BTreeSet<u32> = pick_refs
        .iter()
        .map(|(index, method)| {
            let signature = writ_module::heap::read_blob(&user.blob_heap, method.signature)
                .expect("read MethodRef signature");
            let (params, ret) =
                decode_method_signature(signature).expect("decode MethodRef signature");
            assert_eq!(ret, TypeSignature::Int);
            let parameter = match params.as_slice() {
                [TypeSignature::Int] => "int",
                [TypeSignature::String] => "string",
                other => panic!("unexpected pick signature: {other:?}"),
            };
            parameter_kinds.insert(parameter);
            MetadataToken::new(TableId::MethodRef.as_u8(), (*index + 1) as u32).0
        })
        .collect();
    assert_eq!(parameter_kinds, BTreeSet::from(["int", "string"]));

    let main_idx = user
        .top_level_method_indices()
        .into_iter()
        .find(|index| {
            writ_module::heap::read_string(&user.string_heap, user.method_defs[*index].name).ok()
                == Some("main")
        })
        .expect("main MethodDef");
    let body = &user.method_bodies[main_idx];
    let mut cursor = std::io::Cursor::new(&body.code);
    let called_pick_refs: BTreeSet<u32> = std::iter::from_fn(|| {
        ((cursor.position() as usize) < body.code.len())
            .then(|| Instruction::decode(&mut cursor).expect("decode instruction"))
    })
    .filter_map(|instruction| match instruction {
        Instruction::Call { method_idx, .. } if pick_tokens.contains(&method_idx) => {
            Some(method_idx)
        }
        _ => None,
    })
    .collect();
    assert_eq!(
        called_pick_refs, pick_tokens,
        "calls must use both MethodRefs"
    );

    let mut runtime = RuntimeBuilder::new(user)
        .with_library(library)
        .build()
        .expect("link overload consumer");
    let task = runtime.spawn_task(main_idx, vec![]).expect("spawn main");
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(42)));
}

#[test]
fn imported_top_level_overloads_execute() {
    assert_top_level_overloads(
        r#"
            pub fn pick(value: int) -> int { return 11; }
            pub fn pick(value: string) -> int { return 31; }
        "#,
    );
}

#[test]
fn imported_top_level_overloads_execute_in_reversed_declaration_order() {
    assert_top_level_overloads(
        r#"
            pub fn pick(value: string) -> int { return 31; }
            pub fn pick(value: int) -> int { return 11; }
        "#,
    );
}
