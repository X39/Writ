//! End-to-end regressions for checked-call identity reaching IL emission.

use std::io::Cursor;

use writ_module::signature::TypeSignature;
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
        class Holder<T> { value: T }
        impl<T> Holder<T> {
            fn set(mut self, value: T) { self.value = value; }
        }
        fn main() {
            let value: Holder<int> = new Holder<int> { value: 0 };
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
fn range_construction_emits_runtime_fieldref_tokens() {
    let module = compile(
        r#"
        fn main() {
            let range = 0..=10;
        }
        "#,
    );
    let range_type_ref = module
        .type_refs
        .iter()
        .position(|row| {
            string(&module, row.namespace) == "writ" && string(&module, row.name) == "Range"
        })
        .expect("writ::Range TypeRef");
    let range_parent = MetadataToken::new(TableId::TypeRef.as_u8(), (range_type_ref + 1) as u32);
    let field_tokens: Vec<_> = method_instructions(&module, "main")
        .into_iter()
        .filter_map(|instruction| match instruction {
            Instruction::SetField { field_token, .. } => Some(field_token),
            _ => None,
        })
        .collect();

    assert_eq!(field_tokens.len(), 4, "one write per Range field");
    let mut fields = Vec::new();
    for encoded in field_tokens {
        let token = MetadataToken(encoded);
        assert_eq!(
            token.table_id(),
            TableId::FieldRef.as_u8(),
            "Range construction must use a FieldRef token"
        );
        let row = token.row_index().expect("non-null FieldRef") as usize - 1;
        let field = module.field_refs.get(row).expect("FieldRef row");
        assert_eq!(field.parent, range_parent);
        let signature = writ_module::heap::read_blob(&module.blob_heap, field.type_sig)
            .expect("FieldRef type signature");
        fields.push((
            string(&module, field.name).to_owned(),
            writ_module::signature::decode_type_signature(signature)
                .expect("canonical FieldRef type signature"),
        ));
    }

    assert_eq!(
        fields,
        vec![
            ("start".to_owned(), TypeSignature::GenericParam(0)),
            ("end".to_owned(), TypeSignature::GenericParam(0)),
            ("start_inclusive".to_owned(), TypeSignature::Bool),
            ("end_inclusive".to_owned(), TypeSignature::Bool),
        ]
    );
}
