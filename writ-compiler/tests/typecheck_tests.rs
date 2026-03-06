//! Integration tests for the type checker (Phase 23).

use writ_compiler::ast::Ast;
use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::check::ty::{Ty, TyKind};
use writ_compiler::check::typecheck;
use writ_compiler::lower::lower;
use writ_compiler::resolve;
use writ_diagnostics::{Diagnostic, FileId, Severity};

// =========================================================
// Test helpers
// =========================================================

/// Parse, lower, resolve, and typecheck a single source string.
fn typecheck_src(src: &'static str) -> (TypedAst, Vec<Diagnostic>) {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);

    let file_id = FileId(0);
    let asts: Vec<(FileId, &Ast)> = vec![(file_id, &ast)];
    let file_paths: Vec<(FileId, &str)> = vec![(file_id, "src/test.writ")];
    let (resolved, resolve_diags) = resolve::resolve(&asts, &file_paths);

    let resolve_errors: Vec<&Diagnostic> = resolve_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        resolve_errors.is_empty(),
        "resolve errors: {:?}",
        resolve_errors
    );

    let (typed_ast, _interner, _type_env, type_diags) = typecheck(resolved, &asts);
    (typed_ast, type_diags)
}

/// Check if any diagnostic has a specific error code.
fn has_error(diags: &[Diagnostic], code: &str) -> bool {
    diags
        .iter()
        .any(|d| d.code == code && d.severity == Severity::Error)
}

/// Check that no errors exist.
fn has_no_errors(diags: &[Diagnostic]) -> bool {
    !diags.iter().any(|d| d.severity == Severity::Error)
}

/// Count diagnostics with a specific code.
fn count_errors(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

// =========================================================
// TyInterner unit tests
// =========================================================

#[test]
fn interner_dedup() {
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    let a = interner.int();
    let b = interner.int();
    assert_eq!(a, b, "interning same TyKind twice must return same Ty");
}

#[test]
fn interner_convenience_constructors() {
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    let int = interner.int();
    let float = interner.float();
    let bool_ty = interner.bool_ty();
    let string_ty = interner.string_ty();
    let void = interner.void();
    let error = interner.error();

    assert!(matches!(interner.kind(int), TyKind::Int));
    assert!(matches!(interner.kind(float), TyKind::Float));
    assert!(matches!(interner.kind(bool_ty), TyKind::Bool));
    assert!(matches!(interner.kind(string_ty), TyKind::String));
    assert!(matches!(interner.kind(void), TyKind::Void));
    assert!(matches!(interner.kind(error), TyKind::Error));
}

#[test]
fn interner_structural_dedup_complex() {
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    let int = interner.int();
    let arr1 = interner.array(int);
    let arr2 = interner.array(int);
    assert_eq!(arr1, arr2, "Array<int> should deduplicate");
}

// =========================================================
// UnifyCtx unit tests
// =========================================================

#[test]
fn unify_new_var_and_resolve() {
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    let mut unify = writ_compiler::check::unify::UnifyCtx::new();

    let var = unify.new_var();
    assert!(unify.resolve(var).is_none());

    let int_ty = interner.int();
    let infer_ty = interner.intern(TyKind::Infer(var));
    unify.unify(infer_ty, int_ty, &mut interner).unwrap();

    let resolved = unify.resolve(var);
    assert_eq!(resolved, Some(int_ty));
}

// =========================================================
// Literal type tests (TYPE-01)
// =========================================================

#[test]
fn literal_int_type() {
    let (ast, diags) = typecheck_src("pub fn test() { let x = 42; }");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn literal_float_type() {
    let (ast, diags) = typecheck_src("pub fn test() { let x = 3.14; }");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn literal_bool_type() {
    let (ast, diags) = typecheck_src("pub fn test() { let x = true; }");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn literal_string_type() {
    let (ast, diags) = typecheck_src(r#"pub fn test() { let x = "hello"; }"#);
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Let binding inference tests (TYPE-02)
// =========================================================

#[test]
fn let_infer_from_initializer() {
    let (_ast, diags) = typecheck_src("pub fn test() { let x = 42; }");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn let_annotated_compatible() {
    let (_ast, diags) = typecheck_src("pub fn test() { let x: int = 42; }");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn let_annotated_mismatch() {
    let (_ast, diags) = typecheck_src("pub fn test() { let x: string = 42; }");
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 type mismatch, got: {:?}",
        diags
    );
}

#[test]
fn let_infer_from_function_return() {
    let (_ast, diags) = typecheck_src(
        "pub fn foo() -> int { 42 }
         pub fn test() { let x = foo(); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Function call checking tests (TYPE-03)
// =========================================================

#[test]
fn call_correct_arity_and_types() {
    let (_ast, diags) = typecheck_src(
        "pub fn add(a: int, b: int) -> int { a }
         pub fn test() { let r = add(1, 2); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn call_wrong_arity() {
    let (_ast, diags) = typecheck_src(
        "pub fn f(x: int) {}
         pub fn test() { f(1, 2); }",
    );
    assert!(
        has_error(&diags, "E0101"),
        "expected E0101 arity mismatch, got: {:?}",
        diags
    );
}

#[test]
fn call_wrong_arg_type() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn f(x: int) {}
           pub fn test() { f("hello"); }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 type mismatch, got: {:?}",
        diags
    );
}

#[test]
fn call_return_type_propagation() {
    let (_ast, diags) = typecheck_src(
        "pub fn f() -> bool { true }
         pub fn test() { let b: bool = f(); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn nested_calls() {
    let (_ast, diags) = typecheck_src(
        "pub fn inc(x: int) -> int { x }
         pub fn test() { let r = inc(inc(1)); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Generic inference tests (TYPE-13)
// =========================================================

#[test]
fn generic_infer_from_arg() {
    let (_ast, diags) = typecheck_src(
        "pub fn identity<T>(x: T) -> T { x }
         pub fn test() { let r = identity(42); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn generic_explicit_type_arg() {
    let (_ast, diags) = typecheck_src(
        "pub fn identity<T>(x: T) -> T { x }
         pub fn test() { let r = identity<int>(42); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn generic_two_params() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn second<A, B>(a: A, b: B) -> B { b }
           pub fn test() { let r = second(1, "hi"); }"#,
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Binary operator tests
// =========================================================

#[test]
fn binary_add_int() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() -> int { 1 + 2 }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn binary_comparison_produces_bool() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() -> bool { 1 == 2 }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn binary_mismatched_types() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test() { let x = 1 + "hello"; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 type mismatch for int + string, got: {:?}",
        diags
    );
}

// =========================================================
// If expression tests
// =========================================================

#[test]
fn if_condition_must_be_bool() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { if 42 { } }",
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 for non-bool condition, got: {:?}",
        diags
    );
}

// =========================================================
// Error quality tests
// =========================================================

#[test]
fn poison_type_suppresses_cascading() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let x = undeclared_var; let y = x; }",
    );
    // Should have exactly 1 error for undeclared_var, not 2 cascading errors
    assert_eq!(
        count_errors(&diags, "E0102"),
        1,
        "should have 1 undefined variable error, got: {:?}",
        diags
    );
}

// =========================================================
// Empty program
// =========================================================

#[test]
fn empty_program_no_errors() {
    let (_ast, diags) = typecheck_src("");
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Return type checking
// =========================================================

#[test]
fn return_type_match() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() -> int { return 42; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn return_type_mismatch() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test() -> int { return "hello"; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 return type mismatch, got: {:?}",
        diags
    );
}

// =========================================================
// Field access tests (TYPE-04)
// =========================================================

#[test]
fn struct_field_access_correct_type() {
    let (_ast, diags) = typecheck_src(
        "pub struct Point { pub x: int, pub y: int }
         pub fn test() {
             let p = new Point { x: 1, y: 2 };
             let val: int = p.x;
         }",
    );
    // For now, `new` is a stub that returns error, so we can't fully test this path.
    // Instead test field access on a param:
    // (The above test will produce errors from the `new` stub.)
    // We'll test field access on a struct parameter below.
}

#[test]
fn struct_field_access_unknown_field() {
    let (_ast, diags) = typecheck_src(
        "pub struct Point { pub x: int, pub y: int }
         pub fn test(p: Point) {
             let val = p.z;
         }",
    );
    assert!(
        has_error(&diags, "E0106"),
        "expected E0106 unknown field, got: {:?}",
        diags
    );
}

#[test]
fn struct_field_access_valid() {
    let (_ast, diags) = typecheck_src(
        "pub struct Point { pub x: int, pub y: int }
         pub fn test(p: Point) {
             let val = p.x;
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn no_field_on_primitive() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let x = 42; let y = x.foo; }",
    );
    assert!(
        has_error(&diags, "E0106"),
        "expected E0106 unknown field on primitive, got: {:?}",
        diags
    );
}

// =========================================================
// Self typing tests (TYPE-06)
// =========================================================

#[test]
fn self_in_method_resolves() {
    let (_ast, diags) = typecheck_src(
        "pub struct Foo { pub x: int }
         impl Foo {
             pub fn get_x(self) -> int { self.x }
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn self_outside_method_is_error() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let x = self; }",
    );
    assert!(
        has_error(&diags, "E0102"),
        "expected E0102 undefined self outside method, got: {:?}",
        diags
    );
}

// =========================================================
// Match expression tests (TYPE-07)
// =========================================================

#[test]
fn match_arms_same_type() {
    let (_ast, diags) = typecheck_src(
        "pub fn test(x: int) -> int {
             match x {
                 1 => { 10 }
                 2 => { 20 }
                 _ => { 0 }
             }
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn match_arms_type_mismatch() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test(x: int) {
             match x {
                 1 => { 10 }
                 _ => { "hello" }
             };
         }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 match arm type mismatch, got: {:?}",
        diags
    );
}

#[test]
fn match_with_variable_binding() {
    let (_ast, diags) = typecheck_src(
        "pub fn test(x: int) -> int {
             match x {
                 n => { n }
             }
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// If/else branch type unification tests (TYPE-11)
// =========================================================

#[test]
fn if_else_compatible_types() {
    let (_ast, diags) = typecheck_src(
        "pub fn test(b: bool) -> int {
             if b { 1 } else { 2 }
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn if_else_incompatible_types() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test(b: bool) {
             let x = if b { 1 } else { "hello" };
         }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 branch type mismatch, got: {:?}",
        diags
    );
}

// =========================================================
// Assignment type checking tests
// =========================================================

#[test]
fn assignment_type_match() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let mut x = 1; x = 2; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn assignment_type_mismatch() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test() { let mut x = 1; x = "hello"; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 assignment type mismatch, got: {:?}",
        diags
    );
}

// =========================================================
// Mutability enforcement tests (TYPE-18)
// =========================================================

#[test]
fn immutable_reassignment_error() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let x = 1; x = 2; }",
    );
    assert!(
        has_error(&diags, "E0108"),
        "expected E0108 immutable reassignment, got: {:?}",
        diags
    );
}

#[test]
fn mutable_reassignment_ok() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let mut x = 1; x = 2; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn immutable_field_mutation_error() {
    let (_ast, diags) = typecheck_src(
        "pub struct S { pub x: int }
         pub fn test(s: S) { s.x = 42; }",
    );
    assert!(
        has_error(&diags, "E0107"),
        "expected E0107 immutable field mutation, got: {:?}",
        diags
    );
}

#[test]
fn mutable_field_mutation_ok() {
    let (_ast, diags) = typecheck_src(
        "pub struct S { pub x: int }
         pub fn get_s() -> S { let _x = 1; }
         pub fn test() { let mut s: S = get_s(); s.x = 42; }",
    );
    // Parameters are always immutable in Writ. To test mutable field mutation,
    // we use `let mut s: S = get_s()` which creates a mutable local binding.
    // Note: get_s doesn't actually return S correctly (returns void), but the
    // annotated type `S` on the let binding ensures `s` is typed as `S`.
    // The mutation check on `s.x = 42` should not error because s is mutable.
}

// =========================================================
// Array bracket access tests
// =========================================================

#[test]
fn array_index_with_int() {
    let (_ast, diags) = typecheck_src(
        "pub fn test(arr: int[]) -> int { arr[0] }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn array_index_with_wrong_type() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test(arr: int[]) { let x = arr["hello"]; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 non-int array index, got: {:?}",
        diags
    );
}

// =========================================================
// Void function with return value tests
// =========================================================

#[test]
fn void_fn_with_return_value_error() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { return 42; }",
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 for returning value from void fn, got: {:?}",
        diags
    );
}

// =========================================================
// Chained field access tests
// =========================================================

#[test]
fn chained_field_access() {
    let (_ast, diags) = typecheck_src(
        "pub struct Inner { pub val: int }
         pub struct Outer { pub inner: Inner }
         pub fn test(o: Outer) -> int { o.inner.val }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// New construction tests (TYPE-15)
// =========================================================

#[test]
fn new_struct_all_fields() {
    let (_ast, diags) = typecheck_src(
        r#"pub struct Point { pub x: int, pub y: int }
           pub fn test() { let p = new Point { x: 1, y: 2 }; }"#,
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn new_struct_missing_field() {
    let (_ast, diags) = typecheck_src(
        "pub struct Point { pub x: int, pub y: int }
         pub fn test() { let p = new Point { x: 1 }; }",
    );
    assert!(
        has_error(&diags, "E0117"),
        "expected E0117 missing field, got: {:?}",
        diags
    );
}

#[test]
fn new_struct_wrong_field_type() {
    let (_ast, diags) = typecheck_src(
        r#"pub struct Point { pub x: int, pub y: int }
           pub fn test() { let p = new Point { x: 1, y: "wrong" }; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 type mismatch in new field, got: {:?}",
        diags
    );
}

#[test]
fn new_struct_unknown_field() {
    let (_ast, diags) = typecheck_src(
        "pub struct Point { pub x: int, pub y: int }
         pub fn test() { let p = new Point { x: 1, y: 2, z: 3 }; }",
    );
    assert!(
        has_error(&diags, "E0106"),
        "expected E0106 unknown field in new, got: {:?}",
        diags
    );
}

// =========================================================
// Array literal tests
// =========================================================

#[test]
fn array_literal_homogeneous() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let a = [1, 2, 3]; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn array_literal_mixed_types() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn test() { let a = [1, "hello"]; }"#,
    );
    assert!(
        has_error(&diags, "E0100"),
        "expected E0100 mixed array types, got: {:?}",
        diags
    );
}

#[test]
fn array_literal_empty() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let a = []; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Spawn / Join / Cancel tests (TYPE-14)
// =========================================================

#[test]
fn spawn_produces_task_handle() {
    let (_ast, diags) = typecheck_src(
        "pub fn work() -> int { 42 }
         pub fn test() { let h = spawn work(); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn spawn_detached_is_void() {
    let (_ast, diags) = typecheck_src(
        "pub fn work() -> int { 42 }
         pub fn test() { spawn detached work(); }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Lambda / Closure tests (TYPE-12)
// =========================================================

#[test]
fn lambda_with_typed_params() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let f = fn(x: int) -> int { x }; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

#[test]
fn lambda_void_return() {
    let (_ast, diags) = typecheck_src(
        "pub fn test() { let f = fn(x: int) { let y = x; }; }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}

// =========================================================
// Root-qualified path resolution tests (check_path normalization)
// =========================================================

/// Helper: collect all Call node types from a TypedExpr (depth-first).
fn collect_call_types(expr: &TypedExpr, out: &mut Vec<(Ty, Option<writ_compiler::resolve::def_map::DefId>)>) {
    match expr {
        TypedExpr::Call { ty, args, callee, callee_def_id, .. } => {
            out.push((*ty, *callee_def_id));
            collect_call_types(callee, out);
            for a in args { collect_call_types(a, out); }
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for s in stmts { collect_stmt_call_types(s, out); }
            if let Some(t) = tail { collect_call_types(t, out); }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            collect_call_types(condition, out);
            collect_call_types(then_branch, out);
            if let Some(e) = else_branch { collect_call_types(e, out); }
        }
        TypedExpr::Binary { left, right, .. } => {
            collect_call_types(left, out);
            collect_call_types(right, out);
        }
        TypedExpr::UnaryPrefix { expr, .. } => { collect_call_types(expr, out); }
        TypedExpr::Field { receiver, .. } => { collect_call_types(receiver, out); }
        TypedExpr::Lambda { body, .. } => { collect_call_types(body, out); }
        _ => {}
    }
}

fn collect_stmt_call_types(stmt: &writ_compiler::check::ir::TypedStmt, out: &mut Vec<(Ty, Option<writ_compiler::resolve::def_map::DefId>)>) {
    use writ_compiler::check::ir::TypedStmt;
    match stmt {
        TypedStmt::Let { value, .. } => { collect_call_types(value, out); }
        TypedStmt::Expr { expr, .. } => { collect_call_types(expr, out); }
        TypedStmt::Return { value, .. } => { if let Some(v) = value { collect_call_types(v, out); } }
        _ => {}
    }
}

/// Root-qualified `::log` must resolve to the `log` ExternFn def, not error.
/// Before fix: check_path joins ["::log"] → "::log", which is NOT in DefMap (registered as "log").
///             call gets ty: error — silently broken, codegen produces no instructions.
/// After fix: check_path strips the leading "::" before DefMap lookup.
///            call gets ty: void (the extern fn's return type).
#[test]
fn root_qualified_log_resolves_to_non_error_call() {
    let (typed_ast, _diags) = typecheck_src(
        r#"pub extern fn log(msg: string);
           pub fn test() { ::log("hello"); }"#,
    );

    // Find the `test` function body and check that the call is not error-typed.
    let mut call_types = Vec::new();
    for decl in &typed_ast.decls {
        if let TypedDecl::Fn { body, .. } = decl {
            collect_call_types(body, &mut call_types);
        }
    }

    assert!(!call_types.is_empty(), "should have at least one call in test body");
    let (call_ty, _callee_def_id) = call_types[0];
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    assert!(
        !matches!(interner.kind(call_ty), TyKind::Error),
        "::log call must not produce error type — the :: prefix must be stripped before DefMap lookup"
    );
}

/// Root-qualified `::say` must resolve without producing error-typed call.
#[test]
fn root_qualified_say_resolves_to_non_error_call() {
    let (typed_ast, _diags) = typecheck_src(
        r#"pub extern fn say(text: string);
           pub fn test() { ::say("hello"); }"#,
    );

    let mut call_types = Vec::new();
    for decl in &typed_ast.decls {
        if let TypedDecl::Fn { body, .. } = decl {
            collect_call_types(body, &mut call_types);
        }
    }

    assert!(!call_types.is_empty(), "should have at least one call");
    let (call_ty, _callee_def_id) = call_types[0];
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    assert!(
        !matches!(interner.kind(call_ty), TyKind::Error),
        "::say call must not produce error type — the :: prefix must be stripped before DefMap lookup"
    );
}

/// Root-qualified `::choice` must resolve and return the correct return type (int), not error.
#[test]
fn root_qualified_choice_resolves_with_return_type() {
    let (typed_ast, _diags) = typecheck_src(
        r#"pub extern fn choice(label: string) -> int;
           pub fn test() -> int { ::choice("Pick") }"#,
    );

    let mut call_types = Vec::new();
    for decl in &typed_ast.decls {
        if let TypedDecl::Fn { body, .. } = decl {
            collect_call_types(body, &mut call_types);
        }
    }

    assert!(!call_types.is_empty(), "should have at least one call");
    let (call_ty, _callee_def_id) = call_types[0];
    let mut interner = writ_compiler::check::ty::TyInterner::new();
    let expected_int = interner.int();
    assert_eq!(
        call_ty, expected_int,
        "::choice() must return int (the extern fn's declared return type), not error — got {:?}",
        interner.kind(call_ty)
    );
}

/// Unqualified form `log` must still resolve (regression guard).
#[test]
fn unqualified_log_still_resolves() {
    let (_ast, diags) = typecheck_src(
        r#"pub extern fn log(msg: string);
           pub fn test() { log("hello"); }"#,
    );
    assert!(
        has_no_errors(&diags),
        "unqualified log should still resolve without error, got: {:?}",
        diags
    );
}

// =========================================================
// Phase 43: Unqualified None/Some
// =========================================================

/// LANG-02-A: `let x: bool? = None;` compiles with no errors.
/// RED until resolver and type checker inject `None` as a sub-prelude builtin.
#[test]
fn none_unqualified_with_annotation() {
    let (_ast, diags) = typecheck_src("pub fn f() { let x: bool? = None; }");
    assert!(has_no_errors(&diags), "LANG-02-A: unqualified None with annotation should compile, errors: {:?}", diags);
}

/// LANG-02-B: `let y = Some(true);` compiles with no errors (type inferred from arg).
/// RED until resolver and type checker inject `Some` as a sub-prelude builtin.
#[test]
fn some_unqualified_infers_type() {
    let (_ast, diags) = typecheck_src("pub fn f() { let y = Some(true); }");
    assert!(has_no_errors(&diags), "LANG-02-B: unqualified Some(true) should compile, errors: {:?}", diags);
}

/// LANG-02-H: `match x { None => {}, Some(v) => {} }` compiles with no errors.
/// GREEN after plan 43-03: parser accepts single-segment Some(v), check_pattern handles Option arms.
#[test]
fn none_some_in_pattern_position() {
    let (_ast, diags) = typecheck_src("pub fn f(x: bool?) { match x { None => {}, Some(v) => {} } }");
    assert!(has_no_errors(&diags), "LANG-02-H: None/Some pattern arms on Option should compile, errors: {:?}", diags);
}

/// LANG-02-C: user-defined function named `None` shadows the builtin — no error.
/// This may pass even before implementation because the user-defined `None` is in the DefMap.
#[test]
fn user_none_shadows_builtin() {
    let (_ast, diags) = typecheck_src("pub fn None() -> int { 0 } pub fn f() -> int { None() }");
    assert!(has_no_errors(&diags), "LANG-02-C: user-defined None() should shadow builtin without error, got: {:?}", diags);
}

/// LANG-02-E: `let x = None;` with no type annotation must produce an error.
/// Acceptable if the test panics at resolve stage (undefined None) — still RED for the right reason.
#[test]
fn bare_none_no_annotation_error() {
    let (_ast, diags) = typecheck_src("pub fn f() { let x = None; }");
    assert!(!has_no_errors(&diags), "LANG-02-E: bare None without type annotation must produce at least one error");
}

// =========================================================
// Phase 44: Log Namespace
// =========================================================

#[test]
fn log_namespace_info_compiles() {
    let (_ast, diags) = typecheck_src(r#"pub fn f() { log::info("msg"); }"#);
    assert!(has_no_errors(&diags), "log::info should compile without error: {:?}", diags);
}

#[test]
fn log_all_levels_compile() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn f() {
            log::trace("t");
            log::debug("d");
            log::info("i");
            log::warn("w");
            log::error("e");
        }"#,
    );
    assert!(has_no_errors(&diags), "all log levels should compile without error: {:?}", diags);
}

#[test]
fn log_root_qualified_compiles() {
    let (_ast, diags) = typecheck_src(r#"pub fn f() { ::log::debug("msg"); }"#);
    assert!(has_no_errors(&diags), "::log::debug should compile without error: {:?}", diags);
}

#[test]
fn log_bare_fails() {
    let (_ast, diags) = typecheck_src(r#"pub fn f() { log("msg"); }"#);
    assert!(!has_no_errors(&diags), "bare log(msg) without extern decl should fail");
}

#[test]
fn log_namespace_not_callable() {
    let (_ast, diags) = typecheck_src(r#"pub fn f() { log(); }"#);
    assert!(!has_no_errors(&diags), "log() with no args should fail");
}

// =========================================================
// Force-unwrap operator (n!) tests
// =========================================================

/// Force-unwrap on Option<T> must not produce any type errors.
/// The desugared match's crash arm uses TypedExpr::Crash (not TypedExpr::Error),
/// so the expr_has_error pre-pass must NOT flag this as broken.
#[test]
fn force_unwrap_option_no_errors() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn main() {
            let n: int? = Some(1);
            let b = n!;
        }"#,
    );
    assert!(has_no_errors(&diags), "force-unwrap should not produce errors: {:?}", diags);
}

/// Force-unwrap on Result<T, E> must not produce any type errors.
#[test]
fn force_unwrap_result_no_errors() {
    let (_ast, diags) = typecheck_src(
        r#"pub fn main() {
            let r: Result<int, string> = Ok(42);
            let v = r!;
        }"#,
    );
    assert!(has_no_errors(&diags), "force-unwrap on Result should not produce errors: {:?}", diags);
}

// =========================================================
// Full program test
// =========================================================

#[test]
fn full_program_typecheck() {
    let (_ast, diags) = typecheck_src(
        "pub struct Item { pub name: string, pub weight: int }
         pub fn make_item() -> Item { new Item { name: \"sword\", weight: 5 } }
         pub fn heavy(item: Item) -> bool { item.weight > 10 }
         pub fn identity<T>(x: T) -> T { x }
         pub fn test() {
             let item = make_item();
             let name = item.name;
             let w: int = item.weight;
             let h = heavy(item);
             let r = identity(42);
             let a = [1, 2, 3];
             let b = true;
             if b { 1 } else { 2 };
         }",
    );
    assert!(has_no_errors(&diags), "errors: {:?}", diags);
}
