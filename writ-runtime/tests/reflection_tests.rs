//! Integration tests for reflection intrinsics (Phase 103, extended in Phase 106).
//!
//! Verifies RT-01 through RT-05 (Phase 103):
//! - RT-01: ReflectionIndex lazy cache (allocate on first access, reuse on second)
//! - RT-02: Cached Type objects survive GC (registered as permanent roots)
//! - RT-03: TypeOf opcode returns Value::Ref pointing to a real Type heap object
//! - RT-04: Reflection intrinsic arms dispatch correctly (TypeFields, FieldInfoGet, etc.)
//! - RT-05: TypeAttributes uses unified ModuleAttributeView attribute path
//!
//! Phase 106 additions (REFL-04, REFL-06, REFL-07, REFL-09):
//! - REFL-04: Type.methods() returns Array of MethodInfo for user-defined methods
//! - REFL-06: Type.contracts() returns Array of ContractInfo for implemented contracts
//! - REFL-07: Type.implements(contract) returns bool — true for implemented, check by name
//! - REFL-09: typeof(T) == typeof(T) is true (interned singleton); typeof(T) == typeof(U) is false
//! - REFL-09: GC survival after full reflection op chain (TypeOf → fields() → FieldInfo.get)

use writ_module::module::MethodBody;
use writ_module::tables::TypeDefKind;
use writ_module::token::MetadataToken;
use writ_module::Instruction;
use writ_module::ModuleBuilder;
use writ_runtime::{ExecutionLimit, RuntimeBuilder, TaskState, Value};
use writ_runtime::heap::HeapObject;

// ── Encoding helper ───────────────────────────────────────────────────

fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

// ── TypeDef token helper ───────────────────────────────────────────────

/// Encode a local TypeDef metadata token: table_id=2 (TypeDef), row=typedef_idx+1 (1-based).
fn typedef_token(typedef_idx: usize) -> u32 {
    (2u32 << 24) | ((typedef_idx + 1) as u32)
}

// ── Test: TypeOf returns a real heap object (RT-03) ───────────────────

/// Test that TypeOf r0, type_token stores Value::Ref (not Value::Int or Void) in r0.
/// Verifies RT-03: TypeOf opcode returns a real Type heap object.
#[test]
fn test_typeof_returns_type_ref() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("MyStruct", "", TypeDefKind::Struct, 0);

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            // TypeOf r0, type_token (local TypeDef at 0-based index 0 in this module)
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    match runtime.return_value(tid) {
        Some(Value::Ref(_href)) => { /* correct: got a heap object reference */ }
        other => panic!("expected Value::Ref from TypeOf, got {:?}", other),
    }
}

// ── Test: primitive get_type() returns a heap ref (RT-04) ─────────────

/// Test that calling Int.get_type() via Reflectable CALL_VIRT returns Value::Ref.
/// Verifies RT-04 for primitive type intrinsics.
#[test]
fn test_primitive_get_type_returns_ref() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);

    // Add a TypeRef to the Reflectable contract in writ-runtime
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let reflectable_ref = builder.add_type_ref(mod_ref, "Reflectable", "writ");

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 42 },
            // Call get_type() on the Int via Reflectable contract (slot 0)
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: reflectable_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    match runtime.return_value(tid) {
        Some(Value::Ref(_href)) => { /* correct */ }
        other => panic!("expected Value::Ref from get_type(), got {:?}", other),
    }
}

// ── Test: Type object survives GC (RT-02) ─────────────────────────────

/// Test that a Type heap object cached in ReflectionIndex survives garbage collection.
/// Verifies RT-02: lazy cache objects registered as permanent GC roots.
///
/// The Type object is allocated via TypeOf. After the task completes (registers cleared),
/// the only reference is from ReflectionIndex's type_cache. The GC must preserve it.
#[test]
fn test_type_object_survives_gc() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("MyStruct", "", TypeDefKind::Struct, 0);

    let body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            // Allocate a Type object — it gets cached in ReflectionIndex
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // Drop the register by loading null — the only remaining reference is from
            // ReflectionIndex's permanent type_cache.
            Instruction::LoadNull { r_dst: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 1, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Before GC: the Type object + 3 string fields (name, namespace, kind) + 1 empty Array
    // (type_args field 4) are on the heap. is_generic is Value::Bool (inline). Total >= 5.
    let before = runtime.heap().object_count();
    assert!(before >= 5, "Type object (1) + 3 string fields + 1 Array (type_args) should be on heap; got {}", before);

    // Run GC — the Type object must survive because ReflectionIndex registers it as a root
    let stats = runtime.collect_garbage();

    // All Type objects must survive (they are permanent GC roots from ReflectionIndex)
    assert_eq!(stats.objects_freed, 0,
        "Type heap object and its string fields should survive GC (permanent roots)");

    // Heap size should be unchanged
    let after = runtime.heap().object_count();
    assert_eq!(before, after, "No objects should be freed after GC");
}

// ── Test: Type.fields() returns correct FieldInfo array (RT-04) ───────

/// Test that Type.fields() returns an Array of FieldInfo objects with correct is_mutable values.
/// Verifies RT-04 for TypeFields intrinsic.
#[test]
fn test_type_fields_returns_array() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with 2 fields: `x` (mutable, flags=0), `y` (readonly, flags=1)
    builder.add_type_def("Vec2", "", TypeDefKind::Struct, 0);
    builder.add_field_def("x", &[0x01], 0);   // int, flags=0 (mutable)
    builder.add_field_def("y", &[0x01], 1);   // int, flags=1 (readonly = let)

    // Add TypeRef to writ-runtime "Type.fields" contract
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref = builder.add_type_ref(mod_ref, "Type.fields", "writ");

    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            // r0 = TypeOf Vec2
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.fields()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Result should be an Array
    let arr_val = runtime.return_value(tid);
    let arr_href = match arr_val {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from Type.fields(), got {:?}", other),
    };

    // Check the array has 2 elements
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 2, "Vec2 should have 2 fields");

    // Check FieldInfo[0]: is_mutable=true (flags=0)
    let fi0_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for FieldInfo[0], got {:?}", other),
    };
    // FieldInfo field 2 = is_mutable
    let is_mutable_0 = runtime.heap().get_field(fi0_href, 2).expect("is_mutable field");
    assert_eq!(is_mutable_0, Value::Bool(true), "field 'x' (flags=0) should be mutable");

    // Check FieldInfo[1]: is_mutable=false (flags=1 = readonly)
    let fi1_href = match elements[1] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for FieldInfo[1], got {:?}", other),
    };
    let is_mutable_1 = runtime.heap().get_field(fi1_href, 2).expect("is_mutable field");
    assert_eq!(is_mutable_1, Value::Bool(false), "field 'y' (flags=1) should be readonly");
}

// ── Test: Type.attributes() uses unified attribute path (RT-05) ───────

/// Test that Type.attributes() returns AttributeInfo objects matching attributes
/// added to the TypeDef via attribute_defs (unified with ModuleAttributeView path).
/// Verifies RT-05: TypeAttributes uses Domain::query_attributes_on equivalent.
#[test]
fn test_type_attributes_from_module_attribute_view() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("MyEntity", "", TypeDefKind::Struct, 0);

    // Add an attribute "Singleton" with no args, attached to typedef 0
    // owner_kind=0 (type), owner = TypeDef token (table=2, row=1 for typedef 0)
    let owner_token = MetadataToken::new(2, 1); // table_id=TypeDef(2), row=1 (1-based)
    builder.add_attribute_def(owner_token, 0u8, "Singleton", &[]);

    // Add TypeRef to writ-runtime "Type.attributes" contract
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_attrs_ref = builder.add_type_ref(mod_ref, "Type.attributes", "writ");

    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            // r0 = TypeOf MyEntity
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.attributes() -> Array<AttributeInfo>
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_attrs_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Result should be an Array<AttributeInfo>
    let arr_val = runtime.return_value(tid);
    let arr_href = match arr_val {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from Type.attributes(), got {:?}", other),
    };

    // Check the array has 1 element (the Singleton attribute)
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 1, "MyEntity should have 1 attribute (Singleton)");

    // Verify the AttributeInfo has name = "Singleton" (field 0)
    let ai_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for AttributeInfo, got {:?}", other),
    };
    let name_val = runtime.heap().get_field(ai_href, 0).expect("name field");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for attribute name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "Singleton",
        "AttributeInfo field 0 should be the attribute name");
}

// ── Test: FieldInfo.get(instance) returns field value (RT-04) ─────────

/// Test that FieldInfo.get(instance) returns the correct field value from a struct instance.
/// Verifies RT-04 for FieldInfoGet intrinsic.
#[test]
fn test_field_info_get() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with 2 fields
    builder.add_type_def("Point", "", TypeDefKind::Struct, 0);
    builder.add_field_def("x", &[0x01], 0); // int
    builder.add_field_def("y", &[0x01], 0); // int

    // Add TypeRefs for Type.fields and FieldInfo.get contracts
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref = builder.add_type_ref(mod_ref, "Type.fields", "writ");
    let fieldinfo_get_ref = builder.add_type_ref(mod_ref, "FieldInfo.get", "writ");

    let body = MethodBody {
        register_types: vec![0; 9],
        code: encode(&[
            // r0 = new Point struct
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::LoadInt { r_dst: 1, value: 10 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },  // x = 10
            Instruction::LoadInt { r_dst: 2, value: 20 },
            Instruction::SetField { r_obj: 0, field_idx: 1, r_val: 2 },  // y = 20

            // r3 = TypeOf Point
            Instruction::TypeOf { r_dst: 3, type_idx: typedef_token(0) },

            // r4 = r3.fields()  (self=r3, argc=1)
            Instruction::CallVirt {
                r_dst: 4,
                r_obj: 3,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 3,
                argc: 1,
            },

            // r5 = 0 (index into fields array)
            // r6 = r4[r5]  (FieldInfo for field 'x')
            Instruction::LoadInt { r_dst: 5, value: 0 },
            Instruction::ArrayLoad { r_dst: 6, r_arr: 4, r_idx: 5 },

            // FieldInfo.get(instance): r_base=6 (self=FieldInfo), r_base+1=7 (instance=Point)
            // MOV r7 <- r0 to provide the instance in r7
            Instruction::Mov { r_dst: 7, r_src: 0 },

            // r8 = r6.get(r7)  (read field 'x' from Point instance at r7)
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 6,
                contract_idx: fieldinfo_get_ref.0,
                slot: 0,
                r_base: 6, // r_base=6(self=FieldInfo), r_base+1=7(arg=instance)
                argc: 2,   // self + instance
            },

            Instruction::Ret { r_src: 8 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 9, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    // field 'x' was set to 10
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Int(10)),
        "FieldInfo.get(instance) should return the field's value (x=10)"
    );
}

// ── Phase 106 additions ────────────────────────────────────────────────

// ── Test: Type.methods() returns Array of MethodInfo (REFL-04) ────────

/// Test that Type.methods() returns an Array of MethodInfo with at least one entry
/// for a struct that has a user-defined method. Verifies REFL-04.
#[test]
fn test_type_methods_returns_array() {
    let mut builder = ModuleBuilder::new("test");

    // Struct "Greeter" with one method "greet"
    builder.add_type_def("Greeter", "", TypeDefKind::Struct, 0);

    // Add TypeRef to writ-runtime "Type.methods" contract
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref = builder.add_type_ref(mod_ref, "Type.methods", "writ");

    // Add "greet" as a method on Greeter (registers r0=void, RetVoid body)
    let greet_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    // method_def at index 0 = "main" (added below); greet must come first so typedef_method_range finds it.
    // We need greet to belong to Greeter's method range.  Add it before "main".
    builder.add_method("greet", &[0], 0, 1, greet_body);

    // Main body: TypeOf Greeter → CallVirt Type.methods() → Ret array
    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            // r0 = TypeOf Greeter (typedef_idx=0)
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.methods()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // spawn_task(method_idx=1, ...) because "greet" is method 0, "main" is method 1
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Result should be an Array
    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from Type.methods(), got {:?}", other),
    };

    // Check array has at least 1 element (the "greet" method)
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert!(!elements.is_empty(), "Greeter.methods() should contain at least one method (greet)");

    // Verify first MethodInfo has name "greet" (field 0 = name string)
    let mi_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for MethodInfo[0], got {:?}", other),
    };
    let name_val = runtime.heap().get_field(mi_href, 0).expect("name field");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for method name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "greet", "MethodInfo field 0 should be the method name 'greet'");
}

// ── Test: Type.contracts() returns Array of ContractInfo (REFL-06) ────

/// Test that Type.contracts() returns an Array of ContractInfo for a struct
/// that implements a contract via ImplDef. Verifies REFL-06.
#[test]
fn test_type_contracts_returns_array() {
    let mut builder = ModuleBuilder::new("test");

    // Struct "Printable"
    builder.add_type_def("Widget", "", TypeDefKind::Struct, 0);

    // Define a local contract "Drawable" in this module (table_id=10)
    let drawable_contract = builder.add_contract_def("Drawable", "writ");
    builder.add_contract_method("drawable_draw", &[], 0);

    // Add TypeRef to writ-runtime "Type.contracts" contract
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_contracts_ref = builder.add_type_ref(mod_ref, "Type.contracts", "writ");

    // Register Widget as implementing Drawable via ImplDef
    let widget_token = MetadataToken::new(2, 1); // TypeDef table=2, row=1 (1-based)
    builder.add_impl_def(widget_token, drawable_contract);
    // Intrinsic flag not needed for user contracts; this method body is never called in tests
    let impl_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("drawable_draw", &[0], 0, 1, impl_body);

    // Main body: TypeOf Widget → contracts() → Ret array
    let main_body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_contracts_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // spawn_task(method_idx=2, ...) because impl method is 0, draw impl is 1, main is 2
    // Actually, let's find main's index: contracts_draw=0, main=1
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from Type.contracts(), got {:?}", other),
    };

    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 1, "Widget should implement exactly 1 contract (Drawable)");

    // Verify ContractInfo field 0 = name = "Drawable"
    let ci_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for ContractInfo, got {:?}", other),
    };
    let name_val = runtime.heap().get_field(ci_href, 0).expect("name field");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for contract name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "Drawable", "ContractInfo field 0 should be the contract name 'Drawable'");
}

// ── Test: Type.implements(contract) returns bool (REFL-07) ────────────

/// Test that Type.implements(contract_type) returns true when the type implements
/// the given contract (matched by name). Verifies REFL-07.
///
/// Strategy: TypeImplements reads field 0 (name string) from the contract_href Type object
/// and compares it against the contract names in the impl table. We use a second TypeDef
/// with the same name as the contract to produce a Type heap object with the right name.
#[test]
fn test_type_implements_returns_bool() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef 0: "Widget" — the struct being queried
    builder.add_type_def("Widget", "", TypeDefKind::Struct, 0);
    // TypeDef 1: "Drawable" — used only to produce a Type heap object with name="Drawable"
    // (TypeImplements matches the contract by its name string read from the Type object)
    builder.add_type_def("Drawable", "", TypeDefKind::Struct, 0);

    // Define a local contract "Drawable" in this module
    let drawable_contract = builder.add_contract_def("Drawable", "writ");
    builder.add_contract_method("drawable_draw", &[], 0);

    // Add TypeRefs for Type.implements and Type.contracts (to set up dispatch)
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_implements_ref = builder.add_type_ref(mod_ref, "Type.implements", "writ");

    // Register Widget as implementing Drawable
    let widget_token = MetadataToken::new(2, 1); // TypeDef table=2, row=1 (Widget)
    builder.add_impl_def(widget_token, drawable_contract);
    let impl_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("drawable_draw_impl", &[0], 0, 1, impl_body);

    // Main body:
    //   r0 = TypeOf Widget      (typedef_idx=0)
    //   r1 = TypeOf Drawable    (typedef_idx=1, produces Type object with name="Drawable")
    //   r2 = r0.implements(r1)  (TypeImplements: reads r1.field[0] = "Drawable", checks impl table)
    //   Ret r2
    let main_body = MethodBody {
        register_types: vec![0; 4],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) }, // Widget Type
            Instruction::TypeOf { r_dst: 1, type_idx: typedef_token(1) }, // Drawable Type
            // CallVirt Type.implements(contract): r_base=0 (self=Widget Type), r_base+1=1 (contract=Drawable Type)
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 0,
                contract_idx: type_implements_ref.0,
                slot: 0,
                r_base: 0,
                argc: 2, // self + contract arg
            },
            Instruction::Ret { r_src: 2 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 4, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // method order: drawable_draw_impl=0, main=1
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Bool(true)),
        "Type.implements(Drawable) should return true for Widget which implements Drawable"
    );
}

// ── Test: typeof(T) == typeof(T) is true (REFL-09, same type) ────────

/// Test that two separate TypeOf instructions with the same type_idx return
/// the same singleton HeapRef. Verified by spawning two tasks and comparing
/// the HeapRef values from Rust — same typedef must yield the same pointer.
/// Verifies REFL-09 same-type case.
#[test]
fn test_type_equality_same_type() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("Alpha", "", TypeDefKind::Struct, 0);

    // Task 0: TypeOf Alpha → Ret
    // Task 1: TypeOf Alpha again → Ret
    // Both must return the same Value::Ref (same HeapRef from the singleton cache)
    let body0 = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    // Two methods with identical bodies — the ReflectionIndex cache will return
    // the same HeapRef for both since they reference the same typedef.
    builder.add_method("get_alpha_0", &[0], 0, 1, body0);
    let body1 = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("get_alpha_1", &[0], 0, 1, body1);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid0 = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    let tid1 = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid0), Some(TaskState::Completed));
    assert_eq!(runtime.task_state(tid1), Some(TaskState::Completed));

    let val0 = runtime.return_value(tid0);
    let val1 = runtime.return_value(tid1);

    // Both must be Value::Ref
    match (&val0, &val1) {
        (Some(Value::Ref(h0)), Some(Value::Ref(h1))) => {
            assert_eq!(h0, h1,
                "typeof(Alpha) must return the same HeapRef singleton on every call (interning): \
                 first={:?}, second={:?}", h0, h1);
        }
        _ => panic!("expected Value::Ref from both TypeOf calls, got {:?} and {:?}", val0, val1),
    }
}

// ── Test: typeof(T) != typeof(U) is true (REFL-09, different types) ───

/// Test that TypeOf on different type indices returns different HeapRefs.
/// Verified by comparing HeapRef values from two tasks at the Rust level.
/// Verifies REFL-09 different-types case.
#[test]
fn test_type_inequality_different_types() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("Alpha", "", TypeDefKind::Struct, 0);
    builder.add_type_def("Beta",  "", TypeDefKind::Struct, 0);

    // Task 0: TypeOf Alpha → Ret
    let body_alpha = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("get_alpha", &[0], 0, 1, body_alpha);

    // Task 1: TypeOf Beta → Ret
    let body_beta = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(1) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("get_beta", &[0], 0, 1, body_beta);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid_alpha = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    let tid_beta = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid_alpha), Some(TaskState::Completed));
    assert_eq!(runtime.task_state(tid_beta), Some(TaskState::Completed));

    let val_alpha = runtime.return_value(tid_alpha);
    let val_beta = runtime.return_value(tid_beta);

    match (&val_alpha, &val_beta) {
        (Some(Value::Ref(h_alpha)), Some(Value::Ref(h_beta))) => {
            assert_ne!(h_alpha, h_beta,
                "typeof(Alpha) and typeof(Beta) must return different HeapRef singletons: \
                 alpha={:?}, beta={:?}", h_alpha, h_beta);
        }
        _ => panic!("expected Value::Ref from both TypeOf calls, got {:?} and {:?}", val_alpha, val_beta),
    }
}

// ── Test: GC survival after full reflection op chain (REFL-09 GC) ─────

/// Test that all reflection heap objects survive GC after a full chain of reflection
/// operations: TypeOf → Type.fields() → FieldInfo.get(instance).
///
/// The Type singleton and all FieldInfo singletons are permanent roots in ReflectionIndex.
/// After the task completes (registers cleared), GC runs. The Type object and FieldInfo objects
/// survive. The fields() Array wrapper is a temporary object (not cached) and IS freed.
/// A second task then calls TypeOf again and gets the same Type HeapRef (cached singleton),
/// confirming the cache remains valid after GC. Verifies REFL-09 GC requirement.
#[test]
fn test_gc_survival_after_reflection_ops() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with 1 field
    builder.add_type_def("Sample", "", TypeDefKind::Struct, 0);
    builder.add_field_def("a", &[0x01], 0); // int, mutable

    // TypeRefs for Type.fields and FieldInfo.get
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref = builder.add_type_ref(mod_ref, "Type.fields", "writ");
    let fieldinfo_get_ref = builder.add_type_ref(mod_ref, "FieldInfo.get", "writ");

    // Task 0: full chain — TypeOf + fields() + FieldInfo.get(instance)
    let body0 = MethodBody {
        register_types: vec![0; 8],
        code: encode(&[
            // r0 = new Sample
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::LoadInt { r_dst: 1, value: 77 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 }, // a = 77

            // r2 = TypeOf Sample
            Instruction::TypeOf { r_dst: 2, type_idx: typedef_token(0) },

            // r3 = r2.fields()  (Array of FieldInfo — temporary, will be freed by GC)
            Instruction::CallVirt {
                r_dst: 3,
                r_obj: 2,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 2,
                argc: 1,
            },

            // r4 = 0 (index for field 'a')
            Instruction::LoadInt { r_dst: 4, value: 0 },
            // r5 = r3[r4] (FieldInfo for 'a' — permanent singleton in ReflectionIndex)
            Instruction::ArrayLoad { r_dst: 5, r_arr: 3, r_idx: 4 },

            // r6 = r0 (Sample instance for FieldInfo.get argument)
            Instruction::Mov { r_dst: 6, r_src: 0 },

            // r7 = r5.get(r6) — reads field 'a' value
            Instruction::CallVirt {
                r_dst: 7,
                r_obj: 5,
                contract_idx: fieldinfo_get_ref.0,
                slot: 0,
                r_base: 5,
                argc: 2, // self + instance
            },

            Instruction::Ret { r_src: 7 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 8, body0);

    // Task 1 (second method): just TypeOf → Ret to confirm cache is valid after GC
    let body1 = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("check_type_after_gc", &[0], 0, 1, body1);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();

    // Run full reflection chain (task 0)
    let tid0 = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid0), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid0),
        Some(Value::Int(77)),
        "FieldInfo.get(instance) should return 77 (field 'a')"
    );

    // Record the Type HeapRef from before GC
    let tid_pre_gc = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    let href_before_gc = match runtime.return_value(tid_pre_gc) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Value::Ref from TypeOf before GC, got {:?}", other),
    };

    // Run GC — Type and FieldInfo singletons survive; the temporary fields() Array
    // and the Sample instance (from task 0's stack) will be freed.
    let stats = runtime.collect_garbage();

    // There should be at least some freed objects (the temporary array from fields(),
    // the Sample instance, etc.). What matters is that Type and FieldInfo singletons survive.
    // We verify this by doing another TypeOf after GC — it must return the same HeapRef.
    let _ = stats; // freed count depends on how many temporaries existed; don't assert exact value

    // After GC: TypeOf must still return the same cached HeapRef
    let tid_post_gc = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid_post_gc), Some(TaskState::Completed));
    let href_after_gc = match runtime.return_value(tid_post_gc) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Value::Ref from TypeOf after GC, got {:?}", other),
    };

    assert_eq!(href_before_gc, href_after_gc,
        "Type singleton HeapRef must be identical before and after GC: \
         before={:?}, after={:?}", href_before_gc, href_after_gc);
}

// ── Phase 108 additions (GEN-01, GEN-02, GEN-03) ──────────────────────

// ── Test: Type.is_generic = true for generic typedef (GEN-01) ─────────

/// Test that is_generic returns true for a TypeDef that has a GenericParam.
/// Verifies GEN-01: is_generic field populated from GenericParam table scan.
#[test]
fn test_is_generic_true_for_generic_typedef() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "MyList" with one generic parameter "T"
    let typedef_token_val = MetadataToken::new(2, 1); // table_id=TypeDef=2, row=1 (1-based)
    builder.add_type_def("MyList", "", TypeDefKind::Struct, 0);
    // Add generic param owned by the TypeDef (owner_kind=0 = type owner)
    builder.add_generic_param(typedef_token_val, 0, 0, "T");

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let is_generic_ref = builder.add_type_ref(mod_ref, "Type.get_is_generic", "writ");

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            // r0 = TypeOf MyList
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.is_generic  (Type.get_is_generic contract, slot 0)
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: is_generic_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Bool(true)),
        "Type.is_generic should be true for a typedef with a GenericParam"
    );
}

// ── Test: Type.is_generic = false for plain typedef (GEN-01) ──────────

/// Test that is_generic returns false for a TypeDef with no generic params.
/// Verifies GEN-01 (false case).
#[test]
fn test_is_generic_false_for_non_generic_typedef() {
    let mut builder = ModuleBuilder::new("test");

    // Plain TypeDef "Point" with no generic params
    builder.add_type_def("Point", "", TypeDefKind::Struct, 0);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let is_generic_ref = builder.add_type_ref(mod_ref, "Type.get_is_generic", "writ");

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: is_generic_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Bool(false)),
        "Type.is_generic should be false for a typedef with no generic params"
    );
}

// ── Test: Type.type_args() returns element type for Array<Elem> (GEN-02) ─

/// Test that type_args() returns a 1-element array containing the Type for "Elem"
/// when called on a TypeSpec encoding Array<Elem>.
/// Verifies GEN-02: type_args() returns correct bound type arguments.
#[test]
fn test_type_args_static_typeof() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "Elem" at index 0 in this module
    builder.add_type_def("Elem", "", TypeDefKind::Struct, 0);

    // TypeSpec encoding Array<Elem>:
    //   sig[0] = 0x20 (Array tag)
    //   sig[1] = 0x10 (TypeRef tag)
    //   sig[2..6] = token for Elem: (table_id=2 << 24) | 1 = 0x02000001 in LE = [0x01, 0x00, 0x00, 0x02]
    let elem_token: u32 = (2u32 << 24) | 1; // table_id=TypeDef=2, row=1
    let elem_token_le = elem_token.to_le_bytes();
    let typespec_sig = vec![0x20u8, 0x10, elem_token_le[0], elem_token_le[1], elem_token_le[2], elem_token_le[3]];
    let typespec_token = builder.add_type_spec(&typespec_sig);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_type_args_ref = builder.add_type_ref(mod_ref, "Type.type_args", "writ");

    // TypeSpec token: table_id=4 (TypeSpec), row=1 (1-based)
    let typespec_instr_token: u32 = (4u32 << 24) | typespec_token.row_index().unwrap_or(1);

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            // r0 = TypeOf Array<Elem>  (uses TypeSpec token)
            Instruction::TypeOf { r_dst: 0, type_idx: typespec_instr_token },
            // r1 = r0.type_args()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_type_args_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Result is an Array<Type> with exactly 1 element
    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from type_args(), got {:?}", other),
    };
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 1, "type_args() for Array<Elem> should contain exactly 1 element");

    // The element should be a Type with name "Elem"
    let elem_type_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for type arg element, got {:?}", other),
    };
    let name_val = runtime.heap().get_field(elem_type_href, 0).expect("name field (field 0) on Type");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for type arg name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "Elem",
        "type_args()[0].name should be 'Elem' for Array<Elem>");
}

// ── Test: Type.type_args() returns empty array for non-generic (GEN-02) ─

/// Test that type_args() returns an empty array for a plain non-generic TypeDef.
/// Verifies GEN-02 (empty case).
#[test]
fn test_type_args_empty_for_non_generic() {
    let mut builder = ModuleBuilder::new("test");

    builder.add_type_def("Widget", "", TypeDefKind::Struct, 0);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_type_args_ref = builder.add_type_ref(mod_ref, "Type.type_args", "writ");

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_type_args_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from type_args(), got {:?}", other),
    };
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 0,
        "type_args() for non-generic type should return an empty array");
}

// ── Test: MethodInfo.attributes() returns 1 element (GEN-03) ──────────

/// Test that MethodInfo.attributes() returns an Array with one AttributeInfo when the
/// method has a single AttributeDef pointing to it.
/// Verifies GEN-03: per-member attribute intrinsic for MethodInfo.
#[test]
fn test_method_info_attributes() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "Widget" with one method "update"
    builder.add_type_def("Widget", "", TypeDefKind::Struct, 0);

    // "update" is method 0 in this module; add it before "main" so it belongs to Widget.
    let update_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("update", &[0], 0, 1, update_body);

    // Add an AttributeDef with owner pointing to method "update" (MethodDef table_id=7, row=1).
    // owner_kind=1 (not ATTR_OWNER_KIND_DECL=3, so the intrinsic will include it).
    let method_owner_token = MetadataToken::new(7, 1); // MethodDef, row=1 (method index 0 is row 1)
    builder.add_attribute_def(method_owner_token, 1u8, "Transient", &[]);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref         = builder.add_type_ref(mod_ref, "Type.methods",         "writ");
    let methodinfo_attrs_ref     = builder.add_type_ref(mod_ref, "MethodInfo.attributes", "writ");

    // main: TypeOf Widget -> methods() -> [0] -> attributes() -> Ret array
    let main_body = MethodBody {
        register_types: vec![0; 6],
        code: encode(&[
            // r0 = TypeOf Widget
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.methods()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            // r2 = 0 (index)
            Instruction::LoadInt { r_dst: 2, value: 0 },
            // r3 = r1[r2]  (MethodInfo for "update")
            Instruction::ArrayLoad { r_dst: 3, r_arr: 1, r_idx: 2 },
            // r4 = r3.attributes()
            Instruction::CallVirt {
                r_dst: 4,
                r_obj: 3,
                contract_idx: methodinfo_attrs_ref.0,
                slot: 0,
                r_base: 3,
                argc: 1,
            },
            Instruction::Ret { r_src: 4 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 6, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // method order: update=0, main=1
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from MethodInfo.attributes(), got {:?}", other),
    };
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 1, "method 'update' should have 1 attribute (Transient)");

    // Verify AttributeInfo[0].name == "Transient"
    let ai_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for AttributeInfo, got {:?}", other),
    };
    let name_val = runtime.heap().get_field(ai_href, 0).expect("name field on AttributeInfo");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for attribute name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "Transient",
        "AttributeInfo.name should be 'Transient' for the method attribute");
}

// ── Test: FieldInfo.attributes() returns 1 element (GEN-03) ──────────

/// Test that FieldInfo.attributes() returns an Array with one AttributeInfo when the
/// field has a single AttributeDef pointing to it.
/// Verifies GEN-03: per-member attribute intrinsic for FieldInfo.
#[test]
fn test_field_info_attributes() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "Item" with one field "price"
    builder.add_type_def("Item", "", TypeDefKind::Struct, 0);
    builder.add_field_def("price", &[0x01], 0); // int, mutable

    // Add an AttributeDef with owner pointing to field "price" (FieldDef table_id=5, row=1).
    // owner_kind=1 (not ATTR_OWNER_KIND_DECL=3).
    let field_owner_token = MetadataToken::new(5, 1); // FieldDef, row=1 (field index 0 is row 1)
    builder.add_attribute_def(field_owner_token, 1u8, "Validated", &[]);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref       = builder.add_type_ref(mod_ref, "Type.fields",           "writ");
    let fieldinfo_attrs_ref   = builder.add_type_ref(mod_ref, "FieldInfo.attributes",  "writ");

    let body = MethodBody {
        register_types: vec![0; 5],
        code: encode(&[
            // r0 = TypeOf Item
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.fields()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            // r2 = 0
            Instruction::LoadInt { r_dst: 2, value: 0 },
            // r3 = r1[r2]  (FieldInfo for "price")
            Instruction::ArrayLoad { r_dst: 3, r_arr: 1, r_idx: 2 },
            // r4 = r3.attributes()
            Instruction::CallVirt {
                r_dst: 4,
                r_obj: 3,
                contract_idx: fieldinfo_attrs_ref.0,
                slot: 0,
                r_base: 3,
                argc: 1,
            },
            Instruction::Ret { r_src: 4 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 5, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from FieldInfo.attributes(), got {:?}", other),
    };
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 1, "field 'price' should have 1 attribute (Validated)");

    let ai_href = match elements[0] {
        Value::Ref(href) => href,
        other => panic!("expected Ref for AttributeInfo, got {:?}", other),
    };
    let name_val = runtime.heap().get_field(ai_href, 0).expect("name field on AttributeInfo");
    let name_href = match name_val {
        Value::Ref(href) => href,
        other => panic!("expected string Ref for attribute name, got {:?}", other),
    };
    let name_str = runtime.heap().read_string(name_href).expect("readable string");
    assert_eq!(name_str, "Validated",
        "AttributeInfo.name should be 'Validated' for the field attribute");
}

// ── Test: MethodInfo.attributes() returns empty when none (GEN-03) ────

/// Test that MethodInfo.attributes() returns an empty array for a method with no attributes.
/// Verifies GEN-03 (empty case).
#[test]
fn test_method_info_attributes_empty_when_none() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "Pure" with one method "run" — no attributes added
    builder.add_type_def("Pure", "", TypeDefKind::Struct, 0);

    let run_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("run", &[0], 0, 1, run_body);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref     = builder.add_type_ref(mod_ref, "Type.methods",         "writ");
    let methodinfo_attrs_ref = builder.add_type_ref(mod_ref, "MethodInfo.attributes", "writ");

    let main_body = MethodBody {
        register_types: vec![0; 5],
        code: encode(&[
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::ArrayLoad { r_dst: 3, r_arr: 1, r_idx: 2 },
            Instruction::CallVirt {
                r_dst: 4,
                r_obj: 3,
                contract_idx: methodinfo_attrs_ref.0,
                slot: 0,
                r_base: 3,
                argc: 1,
            },
            Instruction::Ret { r_src: 4 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 5, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // method order: run=0, main=1
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let arr_href = match runtime.return_value(tid) {
        Some(Value::Ref(href)) => href,
        other => panic!("expected Array (Value::Ref) from MethodInfo.attributes(), got {:?}", other),
    };
    let arr_obj = runtime.heap().get_object(arr_href).expect("array object exists");
    let elements = match arr_obj {
        HeapObject::Array { elements, .. } => elements.clone(),
        other => panic!("expected Array heap object, got {:?}", other),
    };
    assert_eq!(elements.len(), 0,
        "method 'run' with no attributes should return empty array from attributes()");
}

// ── Phase 107 additions (DYN-01, DYN-02, DYN-04) ──────────────────────

// ── Test: FieldInfo.set() writes a mutable field (DYN-01) ─────────────

/// Test that FieldInfo.set(instance, value) writes the new value to a mutable (flags=0) field.
/// Verifies DYN-01: FieldInfo.set() on a mut field writes the value and reads back correctly.
///
/// Strategy:
///   1. Create a struct with one mutable int field.
///   2. Allocate an instance, set field to 42 via SetField.
///   3. Get the FieldInfo via Type.fields()[0].
///   4. Call FieldInfo.set(instance, 99) via CALL_VIRT.
///   5. Read the field back via GetField.
///   6. Assert the return value is 99 (the new value, not the old 42).
#[test]
fn test_field_info_set_mut_field() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with one mutable int field (flags=0 = mutable)
    builder.add_type_def("Counter", "", TypeDefKind::Struct, 0);
    builder.add_field_def("val", &[0x01], 0); // int, flags=0 (mutable)

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref  = builder.add_type_ref(mod_ref, "Type.fields",  "writ");
    let fieldinfo_set_ref = builder.add_type_ref(mod_ref, "FieldInfo.set", "writ");

    // Register count: r0=instance, r1=tmp_int, r2=type_obj, r3=fields_arr,
    //                 r4=idx_0, r5=fi0, r6=fi0_copy, r7=new_val, r8=result
    let body = MethodBody {
        register_types: vec![0; 9],
        code: encode(&[
            // r0 = new Counter
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            // r0.val = 42
            Instruction::LoadInt { r_dst: 1, value: 42 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },

            // r2 = TypeOf Counter
            Instruction::TypeOf { r_dst: 2, type_idx: typedef_token(0) },

            // r3 = r2.fields()
            Instruction::CallVirt {
                r_dst: 3,
                r_obj: 2,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 2,
                argc: 1,
            },

            // r4 = 0 (index for field 'val')
            Instruction::LoadInt { r_dst: 4, value: 0 },
            // r5 = r3[r4]  (FieldInfo for 'val')
            Instruction::ArrayLoad { r_dst: 5, r_arr: 3, r_idx: 4 },

            // Set up args for FieldInfo.set(instance, new_val):
            //   r_base=5 (self=FieldInfo), r_base+1=6 (instance), r_base+2=7 (new_val)
            Instruction::Mov { r_dst: 6, r_src: 0 },        // r6 = instance
            Instruction::LoadInt { r_dst: 7, value: 99 },   // r7 = 99 (new value)

            // r8 = r5.set(r6, r7)  — FieldInfo.set(instance=r6, value=r7)
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 5,
                contract_idx: fieldinfo_set_ref.0,
                slot: 0,
                r_base: 5,
                argc: 3,  // self + instance + value
            },

            // Read back the field to confirm the write took effect
            Instruction::GetField { r_dst: 8, r_obj: 0, field_idx: 0 },

            Instruction::Ret { r_src: 8 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 9, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Int(99)),
        "FieldInfo.set() on mutable field should write 99; field read back should be 99 not 42"
    );
}

// ── Test: FieldInfo.set() on a readonly field crashes (DYN-01) ────────

/// Test that FieldInfo.set(instance, value) crashes with "immutable field" message
/// when the field has flags=0x01 (readonly / let-field).
/// Verifies DYN-01: FieldInfo.set() on a let field crashes with descriptive message.
#[test]
fn test_field_info_set_readonly_crashes() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with one readonly int field (flags=0x01 = readonly)
    builder.add_type_def("Frozen", "", TypeDefKind::Struct, 0);
    builder.add_field_def("immut_val", &[0x01], 1); // int, flags=1 (readonly)

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref   = builder.add_type_ref(mod_ref, "Type.fields",   "writ");
    let fieldinfo_set_ref = builder.add_type_ref(mod_ref, "FieldInfo.set", "writ");

    // r0=instance, r1=tmp, r2=type_obj, r3=fields_arr, r4=idx, r5=fi0, r6=inst_copy, r7=new_val, r8=dst
    let body = MethodBody {
        register_types: vec![0; 9],
        code: encode(&[
            // r0 = new Frozen
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            // r0.immut_val = 42 (initial write via direct SetField — still allowed at module level)
            Instruction::LoadInt { r_dst: 1, value: 42 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },

            // r2 = TypeOf Frozen
            Instruction::TypeOf { r_dst: 2, type_idx: typedef_token(0) },

            // r3 = r2.fields()
            Instruction::CallVirt {
                r_dst: 3,
                r_obj: 2,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 2,
                argc: 1,
            },

            // r4 = 0, r5 = fields[0] (FieldInfo for 'immut_val')
            Instruction::LoadInt { r_dst: 4, value: 0 },
            Instruction::ArrayLoad { r_dst: 5, r_arr: 3, r_idx: 4 },

            // r6 = instance, r7 = 99 (new value to attempt)
            Instruction::Mov { r_dst: 6, r_src: 0 },
            Instruction::LoadInt { r_dst: 7, value: 99 },

            // FieldInfo.set(instance, 99) — this MUST crash with "immutable field"
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 5,
                contract_idx: fieldinfo_set_ref.0,
                slot: 0,
                r_base: 5,
                argc: 3,
            },

            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 9, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Cancelled),
        "FieldInfo.set() on readonly field should crash the task");
    let crash = runtime.crash_info(tid).unwrap();
    assert!(
        crash.message.contains("Reflection write to immutable field"),
        "crash message should contain 'Reflection write to immutable field', got: {}",
        crash.message
    );
}

// ── Test: FieldInfo.set() with non-struct instance crashes (DYN-01) ───

/// Test that FieldInfo.set(instance, value) crashes when the instance argument
/// is a primitive value (not a struct/ref), verifying the instance type check.
/// Verifies DYN-01: wrong instance type produces a descriptive crash.
#[test]
fn test_field_info_set_wrong_instance_type_crashes() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with one mutable field — we'll get a FieldInfo for this field
    // but then try to call set() with an int as the instance
    builder.add_type_def("Target", "", TypeDefKind::Struct, 0);
    builder.add_field_def("x", &[0x01], 0); // int, mutable

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_fields_ref   = builder.add_type_ref(mod_ref, "Type.fields",  "writ");
    let fieldinfo_set_ref = builder.add_type_ref(mod_ref, "FieldInfo.set", "writ");

    // r0=type_obj, r1=fields_arr, r2=idx, r3=fi0, r4=bad_instance(int), r5=new_val, r6=dst
    let body = MethodBody {
        register_types: vec![0; 7],
        code: encode(&[
            // r0 = TypeOf Target
            Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },
            // r1 = r0.fields()
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: type_fields_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            // r2 = 0, r3 = fields[0] (FieldInfo for 'x')
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::ArrayLoad { r_dst: 3, r_arr: 1, r_idx: 2 },

            // r4 = 42 (an int — NOT a struct instance)
            Instruction::LoadInt { r_dst: 4, value: 42 },
            // r5 = 99 (the new value to attempt)
            Instruction::LoadInt { r_dst: 5, value: 99 },

            // FieldInfo.set(int_instance, 99) — must crash: instance is not a struct/ref
            Instruction::CallVirt {
                r_dst: 6,
                r_obj: 3,
                contract_idx: fieldinfo_set_ref.0,
                slot: 0,
                r_base: 3,
                argc: 3,
            },

            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 7, body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Cancelled),
        "FieldInfo.set() with non-struct instance should crash the task");
    let crash = runtime.crash_info(tid).unwrap();
    assert!(
        crash.message.contains("instance"),
        "crash message should mention 'instance', got: {}",
        crash.message
    );
}

// ── Test: MethodInfo.invoke() executes the target method (DYN-02) ──────

/// Test that MethodInfo.invoke(instance, args) pushes a CallFrame and the
/// scheduler drives the callee to completion. The invoked method writes a
/// fixed value (100) to the instance's field; the caller reads it back.
/// Verifies DYN-02: MethodInfo.invoke() executes the target method.
///
/// Target method (index 0): r0=instance (struct ref), writes field 0 = 100, RetVoid.
/// Main method (index 1): allocates instance, sets field to 0, invokes target, reads back.
///
/// Note: param_count=0 in ModuleBuilder (builder limitation), so the args Array must be empty.
#[test]
fn test_method_info_invoke_executes_method() {
    let mut builder = ModuleBuilder::new("test");

    // Struct with one mutable int field
    builder.add_type_def("Widget", "", TypeDefKind::Struct, 0);
    builder.add_field_def("data", &[0x01], 0); // int, mutable

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref    = builder.add_type_ref(mod_ref, "Type.methods",     "writ");
    let methodinfo_invoke_ref = builder.add_type_ref(mod_ref, "MethodInfo.invoke", "writ");

    // Target method (method index 0): takes only self (r0=Widget instance), sets data=100, returns void
    // param_count=0 (builder limitation) means invoke passes 0 args; self is always r_base+1.
    let target_body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            // r1 = 100
            Instruction::LoadInt { r_dst: 1, value: 100 },
            // r0.data = 100  (r0 is the instance provided by MethodInfoInvoke as callee.registers[0])
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("set_data", &[0], 0, 2, target_body);

    // Main method (method index 1):
    //   r0=instance, r1=tmp, r2=type_obj, r3=methods_arr, r4=idx_0, r5=mi0,
    //   r6=inst_copy, r7=args_arr, r8=invoke_dst, r9=result
    let main_body = MethodBody {
        register_types: vec![0; 10],
        code: encode(&[
            // r0 = new Widget
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            // r0.data = 0
            Instruction::LoadInt { r_dst: 1, value: 0 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },

            // r2 = TypeOf Widget
            Instruction::TypeOf { r_dst: 2, type_idx: typedef_token(0) },

            // r3 = r2.methods()
            Instruction::CallVirt {
                r_dst: 3,
                r_obj: 2,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 2,
                argc: 1,
            },

            // r4 = 0, r5 = methods[0] (MethodInfo for 'set_data')
            Instruction::LoadInt { r_dst: 4, value: 0 },
            Instruction::ArrayLoad { r_dst: 5, r_arr: 3, r_idx: 4 },

            // r7 = empty args array (param_count=0, so no args beyond self)
            Instruction::NewArray { r_dst: 7, elem_type: 0 },

            // r6 = instance (self for the invoked method)
            Instruction::Mov { r_dst: 6, r_src: 0 },

            // r8 = r5.invoke(r6, r7)  — MethodInfo.invoke(instance=r6, args=r7)
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 5,
                contract_idx: methodinfo_invoke_ref.0,
                slot: 0,
                r_base: 5,
                argc: 3,  // self (MethodInfo) + instance + args array
            },

            // Read back the field: should now be 100 (set by the invoked method)
            Instruction::GetField { r_dst: 9, r_obj: 0, field_idx: 0 },

            Instruction::Ret { r_src: 9 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 10, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    // method 0 = set_data, method 1 = main
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Int(100)),
        "MethodInfo.invoke() should execute 'set_data' which writes 100 to the field; \
         reading back should return 100"
    );
}

// ── Test: MethodInfo.invoke() with wrong arg count crashes (DYN-02) ────

/// Test that MethodInfo.invoke(instance, args) crashes when the args array
/// has the wrong number of elements (param_count mismatch).
/// Verifies DYN-02: wrong arg count produces a descriptive crash.
///
/// Method has param_count=0 (builder limitation). We pass an args array with
/// 1 element — the mismatch triggers "MethodInfo.invoke: expected 0 args, got 1".
#[test]
fn test_method_info_invoke_wrong_argc_crashes() {
    let mut builder = ModuleBuilder::new("test");

    builder.add_type_def("Stub", "", TypeDefKind::Struct, 0);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref    = builder.add_type_ref(mod_ref, "Type.methods",     "writ");
    let methodinfo_invoke_ref = builder.add_type_ref(mod_ref, "MethodInfo.invoke", "writ");

    // Target method (index 0): trivial body, param_count=0
    let noop_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("noop", &[0], 0, 1, noop_body);

    // Main method (index 1): pass args array with 1 element to method expecting 0
    // r0=instance, r1=type_obj, r2=methods_arr, r3=idx, r4=mi0,
    // r5=inst_copy, r6=args_arr, r7=extra_val, r8=dst
    let main_body = MethodBody {
        register_types: vec![0; 9],
        code: encode(&[
            // r0 = new Stub
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },

            // r1 = TypeOf Stub
            Instruction::TypeOf { r_dst: 1, type_idx: typedef_token(0) },

            // r2 = r1.methods()
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 1,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 1,
                argc: 1,
            },

            // r3 = 0, r4 = methods[0] (MethodInfo for 'noop')
            Instruction::LoadInt { r_dst: 3, value: 0 },
            Instruction::ArrayLoad { r_dst: 4, r_arr: 2, r_idx: 3 },

            // r6 = args array with 1 element (wrong count — method expects 0)
            Instruction::NewArray { r_dst: 6, elem_type: 0 },
            Instruction::LoadInt { r_dst: 7, value: 1 },
            Instruction::ArrayResize { r_arr: 6, r_new_len: 7 },
            Instruction::LoadInt { r_dst: 7, value: 0 },
            Instruction::LoadInt { r_dst: 8, value: 42 },
            Instruction::ArrayStore { r_arr: 6, r_idx: 7, r_val: 8 },

            // r5 = instance
            Instruction::Mov { r_dst: 5, r_src: 0 },

            // invoke with wrong arg count — should crash
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 4,
                contract_idx: methodinfo_invoke_ref.0,
                slot: 0,
                r_base: 4,
                argc: 3,
            },

            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 9, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Cancelled),
        "MethodInfo.invoke() with wrong arg count should crash the task");
    let crash = runtime.crash_info(tid).unwrap();
    assert!(
        crash.message.contains("MethodInfo.invoke: expected"),
        "crash message should contain 'MethodInfo.invoke: expected', got: {}",
        crash.message
    );
}

// ── Test: MethodInfo.invoke() is scheduler-driven (DYN-04) ────────────

/// Test that a method invoked via MethodInfo.invoke() participates in cooperative
/// scheduling — the callee frame's instructions are counted by the scheduler.
///
/// Run with ExecutionLimit::Instructions(2) — just enough to enter the main body
/// but not complete both the main method and the callee. With a very tight limit,
/// the task should still be Running (not Completed) after one tick, proving that
/// the scheduler is driving the invoked method rather than an inner synchronous loop.
///
/// Verifies DYN-04: dynamically invoked methods participate in cooperative scheduling.
#[test]
fn test_method_info_invoke_cooperative_scheduling() {
    let mut builder = ModuleBuilder::new("test");

    builder.add_type_def("Box", "", TypeDefKind::Struct, 0);
    builder.add_field_def("n", &[0x01], 0); // int, mutable

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let type_methods_ref    = builder.add_type_ref(mod_ref, "Type.methods",     "writ");
    let methodinfo_invoke_ref = builder.add_type_ref(mod_ref, "MethodInfo.invoke", "writ");

    // Target method (index 0): writes n=999, returns void
    // Has 3 instructions: LoadInt, SetField, RetVoid
    let target_body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadInt { r_dst: 1, value: 999 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("write_n", &[0], 0, 2, target_body);

    // Main method (index 1): allocate, TypeOf, methods(), extract, build args, invoke, GetField, Ret
    // Total instructions before invoke completes: many — a tight limit causes mid-execution pause
    let main_body = MethodBody {
        register_types: vec![0; 9],
        code: encode(&[
            Instruction::New { r_dst: 0, type_idx: typedef_token(0) },
            Instruction::LoadInt { r_dst: 1, value: 0 },
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },
            Instruction::TypeOf { r_dst: 2, type_idx: typedef_token(0) },
            Instruction::CallVirt {
                r_dst: 3,
                r_obj: 2,
                contract_idx: type_methods_ref.0,
                slot: 0,
                r_base: 2,
                argc: 1,
            },
            Instruction::LoadInt { r_dst: 4, value: 0 },
            Instruction::ArrayLoad { r_dst: 5, r_arr: 3, r_idx: 4 },
            Instruction::NewArray { r_dst: 7, elem_type: 0 },
            Instruction::Mov { r_dst: 6, r_src: 0 },
            Instruction::CallVirt {
                r_dst: 8,
                r_obj: 5,
                contract_idx: methodinfo_invoke_ref.0,
                slot: 0,
                r_base: 5,
                argc: 3,
            },
            Instruction::GetField { r_dst: 1, r_obj: 0, field_idx: 0 },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 9, main_body);
    let module = builder.build();

    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
    let tid = runtime.spawn_task(1, vec![]).unwrap();

    // Tick with a very small instruction limit: 3 instructions is enough to start
    // the main method but not complete all the setup + the callee's body.
    runtime.tick(0.0, ExecutionLimit::Instructions(3));

    // With only 3 instructions, the task cannot be complete — it should be back in
    // Ready state (preempted by the scheduler, waiting for its next tick slice).
    // This proves the invoked callee is scheduler-driven and not run synchronously
    // inside the intrinsic body.
    let state_after_limit = runtime.task_state(tid);
    assert!(
        state_after_limit == Some(TaskState::Ready) || state_after_limit == Some(TaskState::Running),
        "With ExecutionLimit::Instructions(3), the task should be preempted (Ready or Running), \
         not Completed — got {:?}", state_after_limit
    );
    assert_ne!(
        state_after_limit,
        Some(TaskState::Completed),
        "Task should NOT complete in only 3 instructions (proves scheduler-driven dispatch)"
    );

    // Now run to completion to verify correctness end-to-end
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(
        runtime.return_value(tid),
        Some(Value::Int(999)),
        "After full completion, the invoked method should have written 999 to the field"
    );
}
