//! End-to-end regressions for checked-call identity reaching IL emission.

use std::io::Cursor;

use writ_module::tables::TableId;
use writ_module::{Instruction, MetadataToken, Module};

fn compile(src: &str) -> Module {
    let src: &'static str = Box::leak(src.to_owned().into_boxed_str());
    let bytes = writ_compiler::compile_source(src).expect("source should compile");
    Module::from_bytes(&bytes).expect("compiled module should decode")
}

fn string(module: &Module, offset: u32) -> &str {
    writ_module::heap::read_string(&module.string_heap, offset).unwrap_or("")
}

fn method_instructions(module: &Module, name: &str) -> Vec<Instruction> {
    let method_index = module
        .method_defs
        .iter()
        .position(|method| string(module, method.name) == name)
        .unwrap_or_else(|| panic!("missing MethodDef `{name}`"));
    let body_index = module.method_defs[..=method_index]
        .iter()
        .filter(|method| method.body_size != 0)
        .count()
        .checked_sub(1)
        .unwrap_or_else(|| panic!("MethodDef `{name}` has no body"));
    let body = &module.method_bodies[body_index];
    let mut cursor = Cursor::new(&body.code);
    std::iter::from_fn(|| {
        ((cursor.position() as usize) < body.code.len())
            .then(|| Instruction::decode(&mut cursor).expect("instruction should decode"))
    })
    .collect()
}

#[test]
fn generic_impl_call_specializes_receiver_bound_signature() {
    let module = compile(
        r#"
        class Holder<T> { mut value: T }
        impl<T> Holder<T> {
            fn set(mut self, value: T) { self.value = value; }
        }
        fn main() {
            let mut value: Holder<int> = new Holder<int> { value: 0 };
            value.set(1);
        }
        "#,
    );

    let set_index = module
        .method_defs
        .iter()
        .position(|method| string(&module, method.name) == "set")
        .expect("set MethodDef");
    let set_token = MetadataToken::new(TableId::MethodDef.as_u8(), (set_index + 1) as u32).0;
    let instructions = method_instructions(&module, "main");

    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instruction::Call { method_idx, argc: 2, .. } if *method_idx == set_token
    )));
    assert!(
        !instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::CallIndirect { .. }))
    );
}

#[test]
fn range_construction_emits_atomic_ordered_field_block() {
    let module = compile(
        r#"
        fn main() {
            let range = 0..=10;
        }
        "#,
    );
    let instructions = method_instructions(&module, "main");
    assert!(
        !instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::SetField { .. })),
        "Range construction must not expose a partially initialized object"
    );
    let new_index = instructions
        .iter()
        .position(|instruction| matches!(instruction, Instruction::New { .. }))
        .expect("Range construction should emit NEW");
    let Instruction::New {
        field_count,
        r_base,
        ..
    } = &instructions[new_index]
    else {
        unreachable!()
    };
    let r_base = *r_base;
    assert_eq!(*field_count, 4);
    assert!(
        new_index >= 4,
        "all four Range values must be evaluated before NEW"
    );
    assert!(matches!(
        instructions[new_index - 4],
        Instruction::LoadInt {
            r_dst,
            value: 0
        } if r_dst == r_base
    ));
    assert!(matches!(
        instructions[new_index - 3],
        Instruction::LoadInt {
            r_dst,
            value: 10
        } if r_dst == r_base + 1
    ));
    assert!(matches!(
        instructions[new_index - 2],
        Instruction::LoadTrue { r_dst } if r_dst == r_base + 2
    ));
    assert!(matches!(
        instructions[new_index - 1],
        Instruction::LoadTrue { r_dst } if r_dst == r_base + 3
    ));
}
