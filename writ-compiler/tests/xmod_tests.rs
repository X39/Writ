//! Cross-module type resolution integration tests.
//!
//! Verifies that `compile_with_libraries` correctly injects library types
//! into the DefMap (so name resolution succeeds) and method signatures into
//! the TypeEnv (so type checking succeeds).
//!
//! Smoke tests from Plan 01 (122-01) are retained. Comprehensive tests covering
//! type reference, method call, field access, generic types, and error paths are
//! added in Plan 02 (122-02) — XMOD-06.

/// Helper: compile source to bytes using the no-library path.
fn compile(src: &str) -> Vec<u8> {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    writ_compiler::compile_source(src_static).expect("compile failed")
}

/// Helper: compile source with library modules.
fn compile_with_libs(src: &str, libs: &[&writ_module::Module]) -> Result<Vec<u8>, String> {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    writ_compiler::compile_with_libraries(src_static, libs)
}

/// Smoke test: compile a library defining a simple struct, then compile user
/// code that declares a variable of that type. Verifies that DefMap injection
/// and type-checking recognize the library type without errors.
///
/// This exercises XMOD-01 (DefMap injection) and XMOD-02 (user code references
/// a library type).
#[test]
fn xmod_smoke_type_reference() {
    // Compile a library defining a simple struct
    let lib_src = r#"
        pub struct Point { pub x: int, pub y: int }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    // Compile user code that references the library type as a parameter type
    // (avoids `new` construction which requires field resolution)
    let user_src = r#"
        pub fn get_x(p: Point) -> int {
            return p.x;
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(
        result.is_ok(),
        "expected compile success, got: {:?}",
        result.err()
    );
}

/// Smoke test: compile_with_libraries works when called with no library modules.
/// Verifies that the empty-slice code path does not break existing behavior.
#[test]
fn xmod_no_libraries() {
    let src = r#"
        pub fn add(a: int, b: int) -> int {
            return a + b;
        }
    "#;
    let result = compile_with_libs(src, &[]);
    assert!(
        result.is_ok(),
        "expected compile success with no libraries, got: {:?}",
        result.err()
    );
}

/// Smoke test: compile user code that uses a method on a library struct.
/// Verifies that inject_library_sigs populates impl_index so method calls resolve.
#[test]
fn xmod_smoke_method_call() {
    // Library with a struct and an impl block containing a method
    let lib_src = r#"
        pub struct Counter { pub value: int }
        impl Counter {
            pub fn get(self) -> int {
                return self.value;
            }
        }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    // User code calls a library method — this requires TypeEnv.impl_index to be populated
    let user_src = r#"
        pub fn read_counter(c: Counter) -> int {
            let value = c.get();
            return value;
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(
        result.is_ok(),
        "expected compile success for method call on library type, got: {:?}",
        result.as_ref().err()
    );

    let user_module = writ_module::Module::from_bytes(&result.unwrap()).unwrap();
    let method_ref = user_module
        .method_refs
        .iter()
        .find(|method_ref| {
            method_ref.parent.table_id() == writ_module::tables::TableId::TypeRef.as_u8()
                && writ_module::heap::read_string(&user_module.string_heap, method_ref.name)
                    .unwrap_or("")
                    == "get"
        })
        .expect("get MethodRef");
    assert_ne!(
        method_ref.flags & writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER,
        0,
    );

    let method_index = user_module
        .method_defs
        .iter()
        .position(|method| {
            writ_module::heap::read_string(&user_module.string_heap, method.name).unwrap_or("")
                == "read_counter"
        })
        .unwrap();
    let body = &user_module.method_bodies[method_index];
    let mut cursor = std::io::Cursor::new(&body.code);
    let mut instructions = Vec::new();
    while (cursor.position() as usize) < body.code.len() {
        instructions.push(writ_module::Instruction::decode(&mut cursor).unwrap());
    }
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        writ_module::Instruction::Call { method_idx, .. }
            if writ_module::MetadataToken(*method_idx).table_id()
                == writ_module::tables::TableId::MethodRef.as_u8()
    )));
    assert!(
        !instructions.iter().any(|instruction| matches!(
            instruction,
            writ_module::Instruction::CallIndirect { .. }
        ))
    );
}

#[test]
fn xmod_instance_param_count_does_not_shift_paramdef_ranges() {
    let lib_src = r#"
        pub struct Counter {}
        impl Counter {
            pub fn add(self, delta: int) -> int { return delta; }
        }
        pub fn later(value: int) -> int { return value; }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let user_src = r#"
        pub fn use_library(c: Counter) -> int {
            return c.add(1) + later(2);
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(
        result.is_ok(),
        "instance self must not consume a ParamDef row: {:?}",
        result.err()
    );
}

// =============================================================================
// XMOD-06: Comprehensive cross-module integration tests (Plan 02)
// =============================================================================

/// XMOD-06: Field access on library struct type-checks correctly.
///
/// Verifies inject_library_sigs populates struct_fields so `.field` access resolves.
#[test]
fn xmod_field_access() {
    let lib_src = r#"
        pub struct Pair { pub first: int, pub second: string }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let user_src = r#"
        pub fn get_second(p: Pair) -> string {
            return p.second;
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    let user_bytes = result.unwrap_or_else(|error| {
        panic!("expected compile success for field access on library type: {error}")
    });
    let user_module = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let second_ref = user_module
        .field_refs
        .iter()
        .position(|field_ref| {
            writ_module::heap::read_string(&user_module.string_heap, field_ref.name).ok()
                == Some("second")
        })
        .expect("the second imported field must have a FieldRef row");
    let expected_operand = writ_module::MetadataToken::new(
        writ_module::tables::TableId::FieldRef.as_u8(),
        (second_ref + 1) as u32,
    )
    .0;
    let method_idx = user_module
        .top_level_method_indices()
        .into_iter()
        .find(|index| {
            writ_module::heap::read_string(
                &user_module.string_heap,
                user_module.method_defs[*index].name,
            )
            .ok()
                == Some("get_second")
        })
        .unwrap();
    let mut cursor = std::io::Cursor::new(&user_module.method_bodies[method_idx].code);
    let mut operand = None;
    while (cursor.position() as usize) < user_module.method_bodies[method_idx].code.len() {
        if let writ_module::Instruction::GetField { field_idx, .. } =
            writ_module::Instruction::decode(&mut cursor).unwrap()
        {
            operand = Some(field_idx);
        }
    }
    assert_eq!(operand, Some(expected_operand));
    assert_eq!(
        writ_module::MetadataToken(operand.unwrap()).table_id(),
        writ_module::tables::TableId::FieldRef.as_u8(),
        "imported fields must not be emitted as raw local ordinals"
    );
}

/// XMOD-06: Multiple library modules can be loaded simultaneously.
///
/// Verifies that inject_module_types and inject_library_sigs handle multiple
/// library slices without DefId collisions.
#[test]
fn xmod_multiple_libraries() {
    let lib_a = r#"
        pub struct Vec2 { pub x: int, pub y: int }
    "#;
    let lib_b = r#"
        pub struct Color { pub r: int, pub g: int, pub b: int }
    "#;
    let bytes_a = compile(lib_a);
    let bytes_b = compile(lib_b);
    let mod_a = writ_module::Module::from_bytes(&bytes_a).unwrap();
    let mod_b = writ_module::Module::from_bytes(&bytes_b).unwrap();

    let user_src = r#"
        pub fn get_x(v: Vec2) -> int {
            return v.x;
        }
        pub fn get_r(c: Color) -> int {
            return c.r;
        }
    "#;
    let result = compile_with_libs(user_src, &[&mod_a, &mod_b]);
    assert!(
        result.is_ok(),
        "expected compile success with multiple libraries, got: {:?}",
        result.err()
    );
}

/// XMOD-06: Type-not-found error when referencing a non-existent library type.
///
/// Verifies the compiler produces a clear error (not a panic) for unknown types.
#[test]
fn xmod_type_not_found_error() {
    let lib_src = r#"
        pub struct RealType { pub x: int }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let user_src = r#"
        pub fn use_unknown(x: NonExistentType) -> int {
            return 0;
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(
        result.is_err(),
        "expected compile failure for unknown type, got Ok"
    );
}

/// XMOD-06: Library function callable from user code (top-level fn).
///
/// Verifies that inject_module_types injects top-level functions into DefMap
/// and inject_library_sigs populates their signatures so calls type-check.
#[test]
fn xmod_top_level_function_call() {
    let lib_src = r#"
        pub fn add_ints(a: int, b: int) -> int {
            return a + b;
        }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let user_src = r#"
        pub fn main() -> int {
            let result = add_ints(3, 4);
            return result;
        }
    "#;
    let result =
        compile_with_libs(user_src, &[&lib_module]).expect("library top-level call should compile");
    let user = writ_module::Module::from_bytes(&result).unwrap();
    let method_ref = user
        .method_refs
        .iter()
        .find(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("add_ints")
        })
        .expect("top-level MethodRef");
    assert_eq!(
        method_ref.parent.table_id(),
        writ_module::tables::TableId::ModuleRef.as_u8()
    );
    assert_eq!(
        method_ref.flags, 0,
        "top-level calls have no implicit receiver"
    );
    let body = &user.method_bodies[user.top_level_method_indices()[0]];
    let mut cursor = std::io::Cursor::new(&body.code);
    assert!(
        std::iter::from_fn(|| {
            ((cursor.position() as usize) < body.code.len())
                .then(|| writ_module::Instruction::decode(&mut cursor).unwrap())
        })
        .any(|instruction| matches!(
            instruction,
            writ_module::Instruction::Call { method_idx, argc: 2, .. }
                if method_idx != 0
                    && writ_module::MetadataToken(method_idx).table_id()
                        == writ_module::tables::TableId::MethodRef.as_u8()
        ))
    );
}

/// XMOD-06: Library class with impl methods type-checks from user code.
///
/// Verifies that inject_library_sigs populates impl_index for class types
/// with methods, enabling method call type resolution.
#[test]
fn xmod_class_method_call() {
    let lib_src = r#"
        pub class Counter { pub value: int }
        impl Counter {
            pub fn increment(mut self) {
                self.value = self.value + 1;
            }
            pub fn get(self) -> int {
                return self.value;
            }
        }
        pub fn new_counter() -> Counter {
            return new Counter { value: 0 };
        }
    "#;
    let lib_bytes = compile(lib_src);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let user_src = r#"
        pub fn use_counter(c: Counter) -> int {
            c.increment();
            return c.get();
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(
        result.is_ok(),
        "expected compile success for class method call on library type, got: {:?}",
        result.err()
    );
}

/// A top-level factory after a type and impl must not be absorbed into either
/// owner's legacy method-list range.
#[test]
fn xmod_mixed_module_factory_is_top_level() {
    let lib_bytes = compile(
        r#"
        pub class Widget { pub value: int }
        impl Widget {
            pub fn get(self) -> int { self.value }
        }
        pub fn make_widget(value: int) -> Widget {
            new Widget { value: value }
        }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let top_level_names: Vec<&str> = lib_module
        .top_level_method_indices()
        .into_iter()
        .map(|idx| {
            writ_module::heap::read_string(
                &lib_module.string_heap,
                lib_module.method_defs[idx].name,
            )
            .unwrap()
        })
        .collect();
    assert!(top_level_names.contains(&"make_widget"));
    assert!(!top_level_names.contains(&"get"));

    let result = compile_with_libs(
        r#"
        pub fn use_factory() -> int {
            let widget: Widget = make_widget(7);
            widget.get()
        }
    "#,
        &[&lib_module],
    );
    assert!(
        result.is_ok(),
        "factory must remain callable: {:?}",
        result.err()
    );
}

/// Each impl owns exactly its declared methods, even when several impl blocks
/// target the same type.
#[test]
fn xmod_multiple_impl_blocks_are_disjoint() {
    let lib_bytes = compile(
        r#"
        pub class Counter { pub value: int }
        impl Counter {
            pub fn get(self) -> int { self.value }
        }
        impl Counter {
            pub fn set(mut self, value: int) { self.value = value; }
        }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let impl_method_names: Vec<Vec<&str>> = (0..lib_module.impl_defs.len())
        .map(|impl_idx| {
            lib_module
                .impl_method_indices(impl_idx)
                .into_iter()
                .map(|method_idx| {
                    writ_module::heap::read_string(
                        &lib_module.string_heap,
                        lib_module.method_defs[method_idx].name,
                    )
                    .unwrap()
                })
                .collect()
        })
        .collect();
    assert!(impl_method_names.iter().any(|names| names == &["get"]));
    assert!(impl_method_names.iter().any(|names| names == &["set"]));

    let result = compile_with_libs(
        r#"
        pub fn use_both(counter: Counter) -> int {
            counter.set(9);
            counter.get()
        }
    "#,
        &[&lib_module],
    );
    assert!(
        result.is_ok(),
        "both impls must remain visible: {:?}",
        result.err()
    );
}

#[test]
fn xmod_array_signature_checks_nested_element_type() {
    let lib_bytes = compile(
        r#"
        pub fn consume_ints(values: int[]) -> int { 0 }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let valid = compile_with_libs(
        r#"pub fn valid() -> int { consume_ints([1, 2, 3]) }"#,
        &[&lib_module],
    );
    assert!(
        valid.is_ok(),
        "matching array type must pass: {:?}",
        valid.err()
    );

    let invalid = compile_with_libs(
        r#"pub fn invalid() -> int { consume_ints(["wrong"]) }"#,
        &[&lib_module],
    );
    assert!(invalid.is_err(), "array element mismatch must be rejected");
}

#[test]
fn xmod_generic_instance_signature_checks_arguments_and_return() {
    let lib_bytes = compile(
        r#"
        pub fn consume_option(value: Option<int>) -> int { 0 }
        pub fn echo_option(value: Option<int>) -> Option<int> { value }
        pub fn consume_result(value: Result<int, string>) -> int { 0 }
        pub fn echo_result(value: Result<int, string>) -> Result<int, string> { value }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let valid = compile_with_libs(
        r#"
        pub fn valid(option: Option<int>, result: Result<int, string>) -> Option<int> {
            consume_option(option);
            consume_result(result);
            return echo_option(option);
        }
    "#,
        &[&lib_module],
    );
    assert!(
        valid.is_ok(),
        "matching generic types must pass: {:?}",
        valid.err()
    );

    let wrong_argument = compile_with_libs(
        r#"pub fn invalid(value: Option<string>) -> int { consume_option(value) }"#,
        &[&lib_module],
    );
    assert!(
        wrong_argument.is_err(),
        "generic argument mismatch must be rejected"
    );

    let wrong_result = compile_with_libs(
        r#"pub fn invalid(value: Result<string, string>) -> int { consume_result(value) }"#,
        &[&lib_module],
    );
    assert!(
        wrong_result.is_err(),
        "Result argument mismatch must be rejected"
    );

    let wrong_return = compile_with_libs(
        r#"pub fn invalid(value: Option<int>) -> Option<string> { return echo_option(value); }"#,
        &[&lib_module],
    );
    assert!(
        wrong_return.is_err(),
        "generic return mismatch must be rejected"
    );
}

#[test]
fn xmod_user_generic_constructor_decodes_by_module_qualified_identity() {
    let lib_bytes = compile(
        r#"
        pub struct Crate<T> { pub value: T }
        pub fn consume_crate(value: Crate<int>) -> int { 0 }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let valid = compile_with_libs(
        r#"pub fn valid(value: Crate<int>) -> int { consume_crate(value) }"#,
        &[&lib_module],
    );
    assert!(
        valid.is_ok(),
        "user generic constructor must resolve nominally: {:?}",
        valid.err()
    );

    let wrong_argument = compile_with_libs(
        r#"pub fn invalid(value: Crate<string>) -> int { consume_crate(value) }"#,
        &[&lib_module],
    );
    assert!(
        wrong_argument.is_err(),
        "user generic argument mismatch must be rejected across modules"
    );

    let valid_field = compile_with_libs(
        r#"pub fn read(value: Crate<string>) -> string { return value.value; }"#,
        &[&lib_module],
    );
    assert!(
        valid_field.is_ok(),
        "library generic fields must specialize: {:?}",
        valid_field.err()
    );

    let wrong_field = compile_with_libs(
        r#"pub fn invalid(value: Crate<string>) -> int { return value.value; }"#,
        &[&lib_module],
    );
    assert!(
        wrong_field.is_err(),
        "library generic field type mismatch must be rejected"
    );
}

#[test]
fn user_generic_instance_register_metadata_retains_arguments() {
    let bytes = compile(
        r#"
        pub struct Crate<T> { pub value: T }
        pub fn echo(value: Crate<int>) -> Crate<int> { value }
        pub fn wrap<T>(value: T) -> Crate<T> {
            let wrapped = new Crate<T> { value: value };
            return wrapped;
        }
        "#,
    );
    let module = writ_module::Module::from_bytes(&bytes).unwrap();

    let signatures: Vec<_> = module
        .method_bodies
        .iter()
        .flat_map(|body| body.register_types.iter())
        .filter_map(|offset| writ_module::heap::read_blob(&module.blob_heap, *offset).ok())
        .filter_map(|blob| writ_module::signature::decode_type_signature(blob).ok())
        .collect();

    assert!(
        signatures.iter().any(|signature| matches!(
            signature,
            writ_module::signature::TypeSignature::Generic { name, args, .. }
                if name == "Crate"
                    && matches!(args.as_slice(), [writ_module::signature::TypeSignature::Int])
        )),
        "register signatures: {signatures:?}"
    );
    assert!(signatures.iter().any(|signature| matches!(
        signature,
        writ_module::signature::TypeSignature::Generic { name, args, .. }
            if name == "Crate"
                && matches!(args.as_slice(), [writ_module::signature::TypeSignature::GenericParam(0)])
    )), "generic body register signatures: {signatures:?}");
    assert!(
        signatures.iter().all(|signature| !matches!(
            signature,
            writ_module::signature::TypeSignature::Generic { name, args, .. }
                if name == "Crate"
                    && matches!(args.as_slice(), [writ_module::signature::TypeSignature::Void])
        )),
        "generic metadata must never silently degrade to Error/Void: {signatures:?}"
    );
}

#[test]
fn impl_method_generic_metadata_uses_combined_scope_ordinal() {
    let bytes = compile(
        r#"
        pub class Crate<T> {}
        impl<T> Crate<T> {
            fn choose<U>(self, value: U) -> U { return value; }
        }
        "#,
    );
    let module = writ_module::Module::from_bytes(&bytes).unwrap();
    let (method_index, method) = module
        .method_defs
        .iter()
        .enumerate()
        .find(|(_, method)| {
            writ_module::heap::read_string(&module.string_heap, method.name).ok() == Some("choose")
        })
        .expect("choose MethodDef");
    let signature = writ_module::heap::read_blob(&module.blob_heap, method.signature).unwrap();
    let (params, ret) = writ_module::signature::decode_method_signature(signature).unwrap();
    assert!(matches!(
        params.as_slice(),
        [writ_module::signature::TypeSignature::GenericParam(1)]
    ));
    assert!(matches!(
        ret,
        writ_module::signature::TypeSignature::GenericParam(1)
    ));

    let owner = writ_module::MetadataToken::new(
        writ_module::tables::TableId::MethodDef.as_u8(),
        (method_index + 1) as u32,
    );
    let generic = module
        .generic_params
        .iter()
        .find(|generic| generic.owner == owner)
        .expect("choose GenericParam");
    assert_eq!(generic.ordinal, 1);
    assert_eq!(
        writ_module::heap::read_string(&module.string_heap, generic.name).ok(),
        Some("U")
    );

    let user = compile_with_libs(
        r#"
        pub fn choose_string(value: Crate<int>) -> string {
            return value.choose("ok");
        }
        "#,
        &[&module],
    );
    assert!(
        user.is_ok(),
        "library method generic must instantiate after its impl prefix: {:?}",
        user.err()
    );
}

#[test]
fn xmod_impl_typespecs_preserve_target_and_contract_arguments() {
    let library_bytes = compile(
        r#"
        pub contract Carries<T> { fn get(self) -> T; }
        pub class Crate<T> { pub value: T }
        impl<T> Carries<T> for Crate<T> {
            fn get(self) -> T { return self.value; }
        }
        impl Crate<int> {
            fn int_only(self) -> int { return 1; }
        }
        "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();

    let impl_signatures: Vec<_> = library
        .impl_defs
        .iter()
        .flat_map(|implementation| [implementation.type_token, implementation.contract])
        .filter(|token| token.table_id() == writ_module::tables::TableId::TypeSpec.as_u8())
        .filter_map(|token| token.row_index())
        .filter_map(|row| library.type_specs.get((row - 1) as usize))
        .filter_map(|type_spec| {
            writ_module::heap::read_blob(&library.blob_heap, type_spec.signature).ok()
        })
        .filter_map(|blob| writ_module::signature::decode_type_signature(blob).ok())
        .collect();
    assert!(impl_signatures.iter().any(|signature| matches!(
        signature,
        writ_module::signature::TypeSignature::Generic { name, args, .. }
            if name == "Carries"
                && matches!(args.as_slice(), [writ_module::signature::TypeSignature::GenericParam(0)])
    )), "ImplDef contract TypeSpec signatures: {impl_signatures:?}");
    assert!(
        impl_signatures.iter().any(|signature| matches!(
            signature,
            writ_module::signature::TypeSignature::Generic { name, args, .. }
                if name == "Crate"
                    && matches!(args.as_slice(), [writ_module::signature::TypeSignature::Int])
        )),
        "ImplDef target TypeSpec signatures: {impl_signatures:?}"
    );

    let valid = compile_with_libs(
        r#"
        pub fn use_impls(value: Crate<int>) -> int {
            let carried: Carries<int> = value;
            return carried.get() + value.int_only();
        }
        "#,
        &[&library],
    );
    assert!(
        valid.is_ok(),
        "specialized library impls must remain usable: {:?}",
        valid.err()
    );

    let wrong_contract = compile_with_libs(
        r#"
        pub fn wrong(value: Crate<int>) {
            let carried: Carries<string> = value;
        }
        "#,
        &[&library],
    );
    assert!(
        wrong_contract.is_err(),
        "contract arguments must survive library decoding"
    );

    let wrong_target = compile_with_libs(
        r#"
        pub fn wrong(value: Crate<string>) -> int {
            return value.int_only();
        }
        "#,
        &[&library],
    );
    assert!(
        wrong_target.is_err(),
        "specialized impl targets must survive library decoding"
    );
}

#[test]
fn xmod_bare_imported_impl_target_uses_typeref_token() {
    let library_bytes = compile(r#"pub class Foreign {}"#);
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();

    let user_bytes = compile_with_libs(
        r#"
        impl Foreign {
            pub fn marker(self) -> int { return 1; }
        }
        "#,
        &[&library],
    )
    .expect("foreign inherent impl must compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let marker = user
        .method_defs
        .iter()
        .find(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("marker")
        })
        .expect("marker MethodDef");
    assert_eq!(
        marker.owner.table_id(),
        writ_module::tables::TableId::ImplDef.as_u8()
    );
    let impl_index = marker.owner.row_index().expect("ImplDef row") as usize - 1;
    let target = user.impl_defs[impl_index].type_token;
    assert!(!target.is_null(), "imported impl target must never be NULL");
    assert_eq!(
        target.table_id(),
        writ_module::tables::TableId::TypeRef.as_u8(),
        "bare imported target must retain its registered TypeRef"
    );
}

#[test]
fn xmod_named_signature_resolves_typeref_token_table() {
    let types_bytes = compile(r#"pub struct Remote {}"#);
    let types_module = writ_module::Module::from_bytes(&types_bytes).unwrap();

    let bridge_bytes = compile_with_libs(
        r#"pub fn echo_remote(value: Remote) -> Remote { value }"#,
        &[&types_module],
    )
    .expect("bridge module should compile against Remote");
    let bridge_module = writ_module::Module::from_bytes(&bridge_bytes).unwrap();
    let echo_idx = bridge_module.top_level_method_indices()[0];
    let signature = writ_module::heap::read_blob(
        &bridge_module.blob_heap,
        bridge_module.method_defs[echo_idx].signature,
    )
    .unwrap();
    let (params, ret) = writ_module::signature::decode_method_signature(signature).unwrap();
    assert!(matches!(
        params.as_slice(),
        [writ_module::signature::TypeSignature::Named(token)] if token.table_id() == 3
    ));
    assert!(matches!(
        ret,
        writ_module::signature::TypeSignature::Named(token) if token.table_id() == 3
    ));

    let valid = compile_with_libs(
        r#"pub fn valid(value: Remote) -> Remote { return echo_remote(value); }"#,
        &[&types_module, &bridge_module],
    );
    assert!(
        valid.is_ok(),
        "TypeRef-backed signature must resolve: {:?}",
        valid.err()
    );

    let invalid = compile_with_libs(
        r#"pub fn invalid() -> Remote { return echo_remote(1); }"#,
        &[&types_module, &bridge_module],
    );
    assert!(
        invalid.is_err(),
        "TypeRef-backed argument mismatch must fail"
    );
}

#[test]
fn xmod_generic_parameter_signature_remains_inferable() {
    let lib_bytes = compile(r#"pub fn identity<T>(value: T) -> T { value }"#);
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();
    let identity_idx = lib_module.top_level_method_indices()[0];
    let identity_blob = writ_module::heap::read_blob(
        &lib_module.blob_heap,
        lib_module.method_defs[identity_idx].signature,
    )
    .unwrap();
    let (params, ret) = writ_module::signature::decode_method_signature(identity_blob).unwrap();
    assert_eq!(
        params,
        vec![writ_module::signature::TypeSignature::GenericParam(0)]
    );
    assert_eq!(ret, writ_module::signature::TypeSignature::GenericParam(0));
    assert!(lib_module.generic_params.iter().any(|param| {
        param.owner_kind == 1
            && param.owner.row_index() == Some((identity_idx + 1) as u32)
            && param.ordinal == 0
    }));

    let valid = compile_with_libs(r#"pub fn valid() -> int { identity(42) }"#, &[&lib_module]);
    assert!(
        valid.is_ok(),
        "generic parameter must infer: {:?}",
        valid.err()
    );

    let invalid = compile_with_libs(
        r#"pub fn invalid() -> string { return identity(42); }"#,
        &[&lib_module],
    );
    assert!(
        invalid.is_err(),
        "instantiated return mismatch must be rejected"
    );
}

#[test]
fn xmod_function_signature_checks_parameter_and_return_types() {
    let lib_bytes = compile(
        r#"
        pub fn apply(callback: fn(int) -> int, value: int) -> int {
            callback(value)
        }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();

    let valid = compile_with_libs(
        r#"
        pub fn valid() -> int {
            apply(fn(value: int) -> int { value }, 1)
        }
    "#,
        &[&lib_module],
    );
    assert!(
        valid.is_ok(),
        "matching callback must pass: {:?}",
        valid.err()
    );

    let invalid = compile_with_libs(
        r#"
        pub fn invalid() -> int {
            apply(fn(value: string) -> int { 0 }, 1)
        }
    "#,
        &[&lib_module],
    );
    assert!(
        invalid.is_err(),
        "callback parameter mismatch must be rejected"
    );
}

#[test]
fn xmod_contract_signature_resolves_contractdef_token_table() {
    let lib_bytes = compile(
        r#"
        pub contract Speakable {
            fn speak(self) -> string;
        }
        pub struct Local {}
        pub fn accept_speakable(value: Speakable) -> int { 0 }
        pub fn echo_local(value: Local) -> Local { value }
    "#,
    );
    let lib_module = writ_module::Module::from_bytes(&lib_bytes).unwrap();
    let method_idx = lib_module
        .top_level_method_indices()
        .into_iter()
        .find(|&idx| {
            writ_module::heap::read_string(
                &lib_module.string_heap,
                lib_module.method_defs[idx].name,
            )
            .unwrap()
                == "accept_speakable"
        })
        .unwrap();
    let signature = writ_module::heap::read_blob(
        &lib_module.blob_heap,
        lib_module.method_defs[method_idx].signature,
    )
    .unwrap();
    let (params, _) = writ_module::signature::decode_method_signature(signature).unwrap();
    assert!(matches!(
        params.as_slice(),
        [writ_module::signature::TypeSignature::Named(token)] if token.table_id() == 10
    ));

    let local_idx = lib_module
        .top_level_method_indices()
        .into_iter()
        .find(|&idx| {
            writ_module::heap::read_string(
                &lib_module.string_heap,
                lib_module.method_defs[idx].name,
            )
            .unwrap()
                == "echo_local"
        })
        .unwrap();
    let local_signature = writ_module::heap::read_blob(
        &lib_module.blob_heap,
        lib_module.method_defs[local_idx].signature,
    )
    .unwrap();
    let (local_params, local_ret) =
        writ_module::signature::decode_method_signature(local_signature).unwrap();
    assert!(matches!(
        local_params.as_slice(),
        [writ_module::signature::TypeSignature::Named(token)] if token.table_id() == 2
    ));
    assert!(matches!(
        local_ret,
        writ_module::signature::TypeSignature::Named(token) if token.table_id() == 2
    ));

    let valid = compile_with_libs(
        r#"pub fn valid(value: Speakable) -> int { accept_speakable(value) }"#,
        &[&lib_module],
    );
    assert!(
        valid.is_ok(),
        "ContractDef-backed signature must resolve: {:?}",
        valid.err()
    );

    let invalid = compile_with_libs(
        r#"pub fn invalid() -> int { accept_speakable(1) }"#,
        &[&lib_module],
    );
    assert!(invalid.is_err(), "non-contract argument must be rejected");
}

#[test]
fn xmod_method_overloads_emit_distinct_signature_refs_and_calls() {
    use writ_module::signature::TypeSignature;

    let library_bytes = compile(
        r#"
        pub class Picker {}
        impl Picker {
            pub fn choose(self, value: int) -> int { return value; }
            pub fn choose(self, value: string) -> int { return 2; }
        }
    "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let user_bytes = compile_with_libs(
        r#"
        pub fn choose_int(value: Picker) -> int {
            let selected = value.choose(1);
            return selected;
        }
        pub fn choose_string(value: Picker) -> int {
            let selected = value.choose("one");
            return selected;
        }
    "#,
        &[&library],
    )
    .expect("cross-module overloads should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();

    let choose_refs: Vec<_> = user
        .method_refs
        .iter()
        .filter(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("choose")
        })
        .map(|method| {
            let blob = writ_module::heap::read_blob(&user.blob_heap, method.signature).unwrap();
            writ_module::signature::decode_method_signature(blob)
                .unwrap()
                .0
        })
        .collect();
    assert_eq!(choose_refs.len(), 2);
    assert!(
        choose_refs
            .iter()
            .any(|params| { matches!(params.as_slice(), [TypeSignature::Int]) })
    );
    assert!(
        choose_refs
            .iter()
            .any(|params| { matches!(params.as_slice(), [TypeSignature::String]) })
    );

    let mut called_refs = Vec::new();
    for body in &user.method_bodies {
        let mut cursor = std::io::Cursor::new(&body.code);
        while (cursor.position() as usize) < body.code.len() {
            if let writ_module::Instruction::Call { method_idx, .. } =
                writ_module::Instruction::decode(&mut cursor).unwrap()
            {
                let token = writ_module::MetadataToken(method_idx);
                if token.table_id() == writ_module::tables::TableId::MethodRef.as_u8() {
                    called_refs.push(method_idx);
                }
            }
        }
    }
    called_refs.sort_unstable();
    called_refs.dedup();
    assert_eq!(called_refs.len(), 2);
}

#[test]
fn xmod_specialized_impl_methods_keep_distinct_typespec_parents() {
    let library_bytes = compile(
        r#"
        pub class Crate<T> {}
        impl Crate<int> { pub fn marker(self) -> int { return 1; } }
        impl Crate<string> { pub fn marker(self) -> int { return 2; } }
    "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let user_bytes = compile_with_libs(
        r#"
        pub fn int_marker(value: Crate<int>) -> int {
            let result = value.marker();
            return result;
        }
        pub fn string_marker(value: Crate<string>) -> int {
            let result = value.marker();
            return result;
        }
    "#,
        &[&library],
    )
    .expect("disjoint specialized methods should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let parents: Vec<_> = user
        .method_refs
        .iter()
        .filter(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("marker")
        })
        .map(|method| method.parent)
        .collect();
    assert_eq!(parents.len(), 2);
    assert!(
        parents
            .iter()
            .all(|parent| { parent.table_id() == writ_module::tables::TableId::TypeSpec.as_u8() })
    );
    assert_ne!(parents[0], parents[1]);
}

#[test]
fn xmod_generic_impl_call_uses_open_parent_and_signature() {
    use writ_module::signature::TypeSignature;

    let library_bytes = compile(
        r#"
        pub class Crate<T> {}
        impl<T> Crate<T> {
            pub fn choose<U>(self, value: U) -> U { return value; }
        }
    "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let user_bytes = compile_with_libs(
        r#"
        pub fn choose<T, U>(value: Crate<T>, item: U) -> U {
            let selected = value.choose(item);
            return selected;
        }
        pub fn choose_int(value: Crate<int>) -> int {
            let selected = value.choose(7);
            return selected;
        }
    "#,
        &[&library],
    )
    .expect("open generic method call should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let method_ref = user
        .method_refs
        .iter()
        .find(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("choose")
        })
        .expect("choose MethodRef");
    assert_eq!(
        method_ref.parent.table_id(),
        writ_module::tables::TableId::TypeSpec.as_u8()
    );
    let blob = writ_module::heap::read_blob(&user.blob_heap, method_ref.signature).unwrap();
    let (params, ret) = writ_module::signature::decode_method_signature(blob).unwrap();
    assert!(matches!(
        params.as_slice(),
        [TypeSignature::GenericParam(_)]
    ));
    assert!(matches!(ret, TypeSignature::GenericParam(_)));
    for body in &user.method_bodies {
        let mut cursor = std::io::Cursor::new(&body.code);
        while (cursor.position() as usize) < body.code.len() {
            if let writ_module::Instruction::Call { method_idx, .. } =
                writ_module::Instruction::decode(&mut cursor).unwrap()
            {
                assert_ne!(method_idx, 0);
            }
        }
    }
}

#[test]
fn xmod_static_methodref_does_not_prepend_qualified_receiver() {
    let library_bytes = compile(
        r#"
        pub class Utility {}
        impl Utility {
            pub fn identity(value: int) -> int { return value; }
        }
    "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let user_bytes = compile_with_libs(
        r#"
        pub fn main() -> int {
            let utility = new Utility {};
            let value = utility.identity(7);
            return value;
        }
    "#,
        &[&library],
    )
    .expect("qualified static library method should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let identity_ref = user
        .method_refs
        .iter()
        .find(|method| {
            writ_module::heap::read_string(&user.string_heap, method.name).ok() == Some("identity")
        })
        .expect("identity MethodRef");
    assert_eq!(
        identity_ref.flags & writ_module::tables::METHOD_REF_FLAG_HAS_RECEIVER,
        0,
    );
    let body = &user.method_bodies[user.top_level_method_indices()[0]];
    let mut cursor = std::io::Cursor::new(&body.code);
    let mut call = None;
    while (cursor.position() as usize) < body.code.len() {
        if let instruction @ writ_module::Instruction::Call { .. } =
            writ_module::Instruction::decode(&mut cursor).unwrap()
        {
            call = Some(instruction);
        }
    }
    assert!(matches!(
        call,
        Some(writ_module::Instruction::Call { argc: 1, .. })
    ));
}

#[test]
fn xmod_spawn_uses_methodrefs_and_the_declared_receiver_abi() {
    let library_bytes = compile(
        r#"
        pub class Worker {}
        impl Worker {
            pub fn choose(self, value: int) -> int { return value; }
            pub fn choose(self, value: string) -> int { return 2; }
            pub fn select(value: int) -> int { return value; }
        }
    "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let user_bytes = compile_with_libs(
        r#"
        pub fn start(worker: Worker) {
            let instance_task = spawn worker.choose(7);
            let static_task = spawn worker.select(9);
        }
    "#,
        &[&library],
    )
    .expect("cross-module concrete spawn calls should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let body = &user.method_bodies[user.top_level_method_indices()[0]];
    let mut cursor = std::io::Cursor::new(&body.code);
    let mut spawns = Vec::new();
    while (cursor.position() as usize) < body.code.len() {
        match writ_module::Instruction::decode(&mut cursor).unwrap() {
            writ_module::Instruction::SpawnTask {
                method_idx, argc, ..
            } => {
                spawns.push((method_idx, argc));
            }
            _ => {}
        }
    }

    assert_eq!(spawns.len(), 2);
    for (method_idx, _) in &spawns {
        let token = writ_module::MetadataToken(*method_idx);
        assert!(!token.is_null(), "spawn target must not be the null token");
        assert_eq!(
            token.table_id(),
            writ_module::tables::TableId::MethodRef.as_u8(),
            "cross-module spawn target must be a MethodRef",
        );
    }

    assert!(spawns.iter().any(|(_, argc)| *argc == 2));
    assert!(spawns.iter().any(|(_, argc)| *argc == 1));
}

#[test]
fn xmod_dialogue_transition_preserves_and_requires_dialogue_identity() {
    let library_bytes = compile(
        r#"
        pub dlg destination {}
        pub fn helper() {}
        "#,
    );
    let library = writ_module::Module::from_bytes(&library_bytes).unwrap();
    let destination = library
        .method_defs
        .iter()
        .find(|method| {
            writ_module::heap::read_string(&library.string_heap, method.name).ok()
                == Some("destination")
        })
        .expect("destination MethodDef");
    assert_ne!(
        destination.flags & writ_module::tables::METHOD_FLAG_DIALOGUE,
        0,
        "compiled dialogue methods must retain cross-module identity",
    );

    let user_bytes = compile_with_libs("pub dlg start { -> destination }", &[&library])
        .expect("a transition to a dependency dialogue should compile");
    let user = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let body = &user.method_bodies[user.top_level_method_indices()[0]];
    let mut cursor = std::io::Cursor::new(&body.code);
    assert!(
        std::iter::from_fn(|| {
            ((cursor.position() as usize) < body.code.len())
                .then(|| writ_module::Instruction::decode(&mut cursor).unwrap())
        })
        .any(|instruction| matches!(
            instruction,
            writ_module::Instruction::TailCall { method_idx, .. }
                if writ_module::MetadataToken(method_idx).table_id()
                    == writ_module::tables::TableId::MethodRef.as_u8()
        ))
    );

    let error = compile_with_libs("pub dlg start { -> helper }", &[&library])
        .expect_err("a dependency function is not a dialogue target");
    assert!(
        error.contains("declared with `fn`, not `dlg`"),
        "unexpected error: {error}",
    );
}
