use writ_module::MetadataToken;

#[test]
fn null_token_is_null() {
    assert!(MetadataToken::NULL.is_null());
}

#[test]
fn null_token_table_id_is_zero() {
    assert_eq!(MetadataToken::NULL.table_id(), 0);
}

#[test]
fn null_token_row_index_is_none() {
    assert_eq!(MetadataToken::NULL.row_index(), None);
}

#[test]
fn token_from_zero_is_null() {
    assert!(MetadataToken(0).is_null());
}

#[test]
fn new_token_encodes_correctly() {
    let tok = MetadataToken::new(2, 5);
    assert_eq!(tok.0, 0x02_000005);
    assert_eq!(tok.table_id(), 2);
    assert_eq!(tok.row_index(), Some(5));
    assert!(!tok.is_null());
}

#[test]
fn new_token_with_different_table() {
    let tok = MetadataToken::new(0x07, 0x0A);
    assert_eq!(tok.0, 0x07_00000A);
    assert_eq!(tok.table_id(), 0x07);
    assert_eq!(tok.row_index(), Some(0x0A));
}

#[test]
fn row_index_one_returns_some_one() {
    let tok = MetadataToken::new(0, 1);
    assert_eq!(tok.row_index(), Some(1));
}

#[test]
fn non_null_token_is_not_null() {
    let tok = MetadataToken::new(3, 42);
    assert!(!tok.is_null());
}

#[test]
#[should_panic(expected = "exceeds 24-bit maximum")]
fn new_token_panics_on_overflow() {
    MetadataToken::new(0, 0x01000000);
}
