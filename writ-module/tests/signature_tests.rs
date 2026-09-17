use writ_module::signature::{
    TypeSignature, decode_method_signature, decode_type_signature, encode_method_signature,
    encode_type_signature,
};
use writ_module::{DecodeError, EncodeError, MetadataToken};

#[test]
fn recursive_method_signature_round_trips_without_side_lookups() {
    let option_of_ints = TypeSignature::Generic {
        namespace: "writ".into(),
        name: "Option".into(),
        args: vec![TypeSignature::Array(Box::new(TypeSignature::Int))],
    };
    let generic_result = TypeSignature::Generic {
        namespace: "writ".into(),
        name: "Result".into(),
        args: vec![TypeSignature::GenericParam(0), TypeSignature::String],
    };
    let callback = TypeSignature::Function {
        params: vec![option_of_ints.clone(), generic_result.clone()],
        ret: Box::new(TypeSignature::Generic {
            namespace: "writ".into(),
            name: "TaskHandle".into(),
            args: vec![TypeSignature::Bool],
        }),
    };
    let ret = TypeSignature::Named(MetadataToken::new(2, 17));

    let bytes = encode_method_signature(&[option_of_ints, generic_result, callback], &ret)
        .expect("signature should encode");
    let decoded = decode_method_signature(&bytes).expect("signature should decode");

    assert_eq!(decoded.1, ret);
    assert_eq!(decoded.0.len(), 3);
    assert!(matches!(decoded.0[0], TypeSignature::Generic { .. }));
    assert!(matches!(decoded.0[1], TypeSignature::Generic { .. }));
    assert!(matches!(decoded.0[2], TypeSignature::Function { .. }));
}

#[test]
fn generic_constructor_namespace_is_part_of_identity() {
    let left = TypeSignature::Generic {
        namespace: "alpha::collections".into(),
        name: "List".into(),
        args: vec![TypeSignature::Int],
    };
    let right = TypeSignature::Generic {
        namespace: "beta::collections".into(),
        name: "List".into(),
        args: vec![TypeSignature::Int],
    };

    let left_bytes = encode_type_signature(&left).unwrap();
    let right_bytes = encode_type_signature(&right).unwrap();
    assert_ne!(left_bytes, right_bytes);
    assert_eq!(decode_type_signature(&left_bytes).unwrap(), left);
    assert_eq!(decode_type_signature(&right_bytes).unwrap(), right);
}

#[test]
fn decoder_rejects_truncated_and_trailing_payloads() {
    let function = TypeSignature::Function {
        params: vec![TypeSignature::Int],
        ret: Box::new(TypeSignature::String),
    };
    let mut bytes = encode_type_signature(&function).unwrap();
    bytes.pop();
    assert!(matches!(
        decode_type_signature(&bytes),
        Err(DecodeError::UnexpectedEof)
    ));

    let mut bytes = encode_type_signature(&TypeSignature::Bool).unwrap();
    bytes.push(0x00);
    assert!(matches!(
        decode_type_signature(&bytes),
        Err(DecodeError::InvalidTypeSignature(_))
    ));
}

#[test]
fn encoder_rejects_excessive_nesting_before_recursing_unboundedly() {
    let mut signature = TypeSignature::Int;
    for _ in 0..65 {
        signature = TypeSignature::Array(Box::new(signature));
    }

    assert!(matches!(
        encode_type_signature(&signature),
        Err(EncodeError::TypeSignatureTooDeep)
    ));
}
