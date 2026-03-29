use writ_module::attr::{decode_attr_args, encode_attr_args, AttrValue};

fn round_trip(args: &[AttrValue]) -> Vec<AttrValue> {
    let encoded = encode_attr_args(args);
    decode_attr_args(&encoded).expect("decode should succeed")
}

#[test]
fn test_encode_empty_args() {
    assert_eq!(encode_attr_args(&[]), Vec::<u8>::new());
    assert_eq!(decode_attr_args(&[]).unwrap(), vec![]);
}

#[test]
fn test_encode_string_arg() {
    let args = vec![AttrValue::String("hello".into())];
    assert_eq!(round_trip(&args), args);
}

#[test]
fn test_encode_int_arg() {
    let args42 = vec![AttrValue::Int(42)];
    assert_eq!(round_trip(&args42), args42);

    let args_max = vec![AttrValue::Int(i64::MAX)];
    assert_eq!(round_trip(&args_max), args_max);

    let args_min = vec![AttrValue::Int(i64::MIN)];
    assert_eq!(round_trip(&args_min), args_min);
}

#[test]
fn test_encode_bool_arg() {
    let args = vec![AttrValue::Bool(true), AttrValue::Bool(false)];
    assert_eq!(round_trip(&args), args);
}

#[test]
fn test_encode_named_arg() {
    let args = vec![AttrValue::Named {
        name: "msg".into(),
        value: Box::new(AttrValue::String("deprecated".into())),
    }];
    assert_eq!(round_trip(&args), args);
}

#[test]
fn test_round_trip_multi_args() {
    let args = vec![
        AttrValue::String("a".into()),
        AttrValue::Int(1),
        AttrValue::Bool(true),
    ];
    assert_eq!(round_trip(&args), args);
}

#[test]
fn test_decode_invalid_tag() {
    assert!(decode_attr_args(&[0xFF]).is_err());
}
