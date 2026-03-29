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
    assert!(result.is_ok(), "expected compile success, got: {:?}", result.err());
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
    assert!(result.is_ok(), "expected compile success with no libraries, got: {:?}", result.err());
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
            return c.get();
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(result.is_ok(), "expected compile success for method call on library type, got: {:?}", result.err());
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
        pub fn get_first(p: Pair) -> int {
            return p.first;
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(result.is_ok(), "expected compile success for field access on library type, got: {:?}", result.err());
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
    assert!(result.is_ok(), "expected compile success with multiple libraries, got: {:?}", result.err());
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
    assert!(result.is_err(), "expected compile failure for unknown type, got Ok");
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
            return add_ints(3, 4);
        }
    "#;
    let result = compile_with_libs(user_src, &[&lib_module]);
    assert!(result.is_ok(), "expected compile success calling library top-level fn, got: {:?}", result.err());
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
    assert!(result.is_ok(), "expected compile success for class method call on library type, got: {:?}", result.err());
}
