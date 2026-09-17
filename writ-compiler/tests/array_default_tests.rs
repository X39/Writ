use std::io::Cursor;

use writ_module::instruction::{ArrayDefaultKind, Instruction};

fn array_construction_operands(source: &'static str) -> Vec<(u16, u32)> {
    let bytes = writ_compiler::compile_source(source).expect("source should compile");
    let module = writ_module::Module::from_bytes(&bytes).expect("module should decode");
    let mut operands = Vec::new();
    for body in &module.method_bodies {
        let mut cursor = Cursor::new(body.code.as_slice());
        while (cursor.position() as usize) < body.code.len() {
            let instruction = Instruction::decode(&mut cursor).expect("instruction should decode");
            let opcode = instruction.opcode();
            match instruction {
                Instruction::NewArray { default_kind, .. }
                | Instruction::ArrayInit { default_kind, .. }
                | Instruction::NewArraySized { default_kind, .. }
                | Instruction::NewArrayFilled { default_kind, .. } => {
                    operands.push((opcode, default_kind));
                }
                _ => {}
            }
        }
    }
    operands
}

#[test]
fn annotated_empty_arrays_emit_type_correct_default_kinds() {
    let operands = array_construction_operands(
        r#"
fn main() {
    let ints: int[] = [];
    let floats: float[] = [];
    let bools: bool[] = [];
    let strings: string[] = [];
}
"#,
    );
    assert_eq!(
        operands,
        vec![
            (0x0900, ArrayDefaultKind::Int.operand()),
            (0x0900, ArrayDefaultKind::Float.operand()),
            (0x0900, ArrayDefaultKind::Bool.operand()),
            (0x0900, ArrayDefaultKind::String.operand()),
        ]
    );
}

#[test]
fn generic_constructor_substitutes_concrete_array_element_type() {
    let operands = array_construction_operands(
        r#"
pub class Bucket<T> { items: T[] }

fn main() {
    let bucket: Bucket<float> = new Bucket<float> { items: [] };
}
"#,
    );
    assert_eq!(operands, vec![(0x0900, ArrayDefaultKind::Float.operand())]);
}

#[test]
fn reference_arrays_emit_null_reference_default_kind() {
    let operands = array_construction_operands(
        r#"
pub class Item {}

fn main() {
    let items: Item[] = [];
}
"#,
    );
    assert_eq!(
        operands,
        vec![(0x0900, ArrayDefaultKind::NullReference.operand())]
    );
}

#[test]
fn nonempty_erased_generic_array_defers_to_runtime_value_inference() {
    let operands = array_construction_operands(
        r#"
fn collect<T>(value: T) -> T[] {
    [value]
}
"#,
    );
    assert_eq!(
        operands,
        vec![(0x0901, ArrayDefaultKind::Unavailable.operand())]
    );
}
