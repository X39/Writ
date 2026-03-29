//! Integration tests for Runtime::construct_value and ExternHandler::ImmediateWithHeap.
//!
//! Tests HOST-01, HOST-02, HOST-03 requirements.

use writ_module::module::MethodBody;
use writ_module::{ModuleBuilder, TypeDefKind};
use writ_runtime::{NullHost, Runtime, RuntimeBuilder, Value};

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build a minimal module with a single struct type named "MyStruct" with
/// one int field, and a single no-op main method.
fn build_runtime_with_struct() -> Runtime<NullHost> {
    let mut builder = ModuleBuilder::new("test");
    // type_def for MyStruct (kind=0 is Struct)
    builder.add_type_def("MyStruct", "", TypeDefKind::Struct, 0);
    // one field: int32 (signature byte 0x08)
    builder.add_field_def("x", &[0x08], 0);
    // a no-op main method
    let body = MethodBody {
        register_types: vec![],
        code: {
            let mut code = Vec::new();
            writ_module::Instruction::RetVoid.encode(&mut code).unwrap();
            code
        },
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 0, body);
    let module = builder.build();
    RuntimeBuilder::new(module).build().unwrap()
}

/// Build a module with a class type (let's use kind=2 Entity, which exercises
/// a heap-ref return rather than Struct).
/// Actually we need to test struct (kind=0) and another non-struct/class.
/// For now: struct with 2 fields (int + bool).
fn build_runtime_with_two_field_struct() -> Runtime<NullHost> {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("Point", "", TypeDefKind::Struct, 0);
    builder.add_field_def("x", &[0x08], 0); // int32
    builder.add_field_def("y", &[0x08], 0); // int32
    let body = MethodBody {
        register_types: vec![],
        code: {
            let mut code = Vec::new();
            writ_module::Instruction::RetVoid.encode(&mut code).unwrap();
            code
        },
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 0, body);
    let module = builder.build();
    RuntimeBuilder::new(module).build().unwrap()
}

// ── Task 1 Tests: construct_value ─────────────────────────────────────────

#[test]
fn construct_value_struct_success() {
    let mut runtime = build_runtime_with_struct();
    let result = runtime.construct_value("MyStruct", vec![Value::Int(42)]);
    assert!(result.is_ok(), "expected Ok but got {:?}", result);
    // The result should be a Value::Ref (heap-allocated struct)
    match result.unwrap() {
        Value::Ref(_) => {} // expected: struct is heap-allocated
        other => panic!("expected Value::Ref, got {:?}", other),
    }
}

#[test]
fn construct_value_type_not_found() {
    let mut runtime = build_runtime_with_struct();
    let result = runtime.construct_value("Missing", vec![]);
    assert!(result.is_err(), "expected Err but got Ok");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("not found"),
        "expected 'not found' in error, got: {}",
        msg
    );
}

#[test]
fn construct_value_wrong_field_count() {
    let mut runtime = build_runtime_with_struct();
    // MyStruct has 1 field, provide 0
    let result = runtime.construct_value("MyStruct", vec![]);
    assert!(result.is_err(), "expected Err but got Ok");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("1") && (msg.contains("0") || msg.contains("provided")),
        "expected field count mismatch message, got: {}",
        msg
    );
}

#[test]
fn construct_value_type_mismatch() {
    let mut runtime = build_runtime_with_struct();
    // MyStruct has 1 int field, provide bool
    let result = runtime.construct_value("MyStruct", vec![Value::Bool(true)]);
    assert!(result.is_err(), "expected Err but got Ok");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("type mismatch"),
        "expected 'type mismatch' in error, got: {}",
        msg
    );
}

#[test]
fn construct_value_two_fields_success() {
    let mut runtime = build_runtime_with_two_field_struct();
    let result = runtime.construct_value("Point", vec![Value::Int(10), Value::Int(20)]);
    assert!(result.is_ok(), "expected Ok but got {:?}", result);
    match result.unwrap() {
        Value::Ref(_) => {}
        other => panic!("expected Value::Ref, got {:?}", other),
    }
}

#[test]
fn construct_value_accepts_void_as_uninitialized() {
    // Value::Void should be accepted for any field type (uninitialized)
    let mut runtime = build_runtime_with_struct();
    let result = runtime.construct_value("MyStruct", vec![Value::Void]);
    assert!(
        result.is_ok(),
        "Value::Void should be accepted for uninitialized fields, got: {:?}",
        result
    );
}

// ── Task 2 Tests: ImmediateWithHeap will be added after Task 2 is implemented ──
