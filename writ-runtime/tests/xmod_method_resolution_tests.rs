use writ_module::Instruction;
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState, Value};

#[test]
fn imported_inherent_method_wins_before_more_specific_contract_impl() {
    let library_bytes = writ_compiler::compile_source(
        r#"
            pub contract Marker { fn marker(self) -> int; }
            pub class Crate<T> {}

            impl<T> Crate<T> {
                pub fn marker(self) -> int { return 11; }
            }

            impl Marker for Crate<int> {
                pub fn marker(self) -> int { return 22; }
            }

            pub fn make() -> Crate<int> {
                return new Crate<int> {};
            }
        "#,
    )
    .expect("resolution-policy library should compile");
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();

    let user_bytes = writ_compiler::compile_with_libraries(
        r#"
            pub fn main() -> int {
                let value = make();
                return value.marker();
            }
        "#,
        &[&library],
    )
    .expect("consumer should select the inherent method");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let main_idx = user
        .top_level_method_indices()
        .into_iter()
        .find(|index| {
            writ_module::heap::read_string(&user.string_heap, user.method_defs[*index].name).ok()
                == Some("main")
        })
        .unwrap();

    let mut cursor = std::io::Cursor::new(&user.method_bodies[main_idx].code);
    while (cursor.position() as usize) < user.method_bodies[main_idx].code.len() {
        if let Instruction::Call { method_idx, .. } = Instruction::decode(&mut cursor).unwrap() {
            assert_ne!(method_idx, 0, "a checked direct call must never use the null token");
        }
    }

    let mut runtime = RuntimeBuilder::new(user)
        .with_library(library)
        .build()
        .expect("consumer should link");
    let task = runtime.spawn_task(main_idx, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(task), Some(Value::Int(11)));
}
