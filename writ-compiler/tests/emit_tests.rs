//! Integration tests for the IL metadata emission (Phase 24).
//!
//! These tests verify that the emit module correctly populates all metadata
//! tables from TypedAst + original ASTs, and that token assignment is correct.

use writ_compiler::ast::Ast;
use writ_compiler::check::typecheck;
use writ_compiler::emit;
use writ_compiler::emit::metadata::{MetadataToken, TableId, TypeDefKind};
use writ_compiler::emit::module_builder::ModuleBuilder;
use writ_compiler::lower::lower;
use writ_compiler::resolve;
use writ_diagnostics::{Diagnostic, FileId, Severity};
use writ_module::tables::{FIELD_FLAG_PUBLIC, FIELD_FLAG_READONLY};

// =========================================================
// Test helpers
// =========================================================

/// Parse, lower, resolve, typecheck, and emit a single source string.
/// Returns the finalized ModuleBuilder and diagnostics.
fn emit_src(src: &'static str) -> (ModuleBuilder, Vec<Diagnostic>) {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);

    let file_id = FileId(0);
    let asts: Vec<(FileId, &Ast)> = vec![(file_id, &ast)];
    let file_paths: Vec<(FileId, &str)> = vec![(file_id, "src/test.writ")];
    let (resolved, resolve_diags) = resolve::resolve(&asts, &file_paths, &[]);

    let resolve_errors: Vec<&Diagnostic> = resolve_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        resolve_errors.is_empty(),
        "resolve errors: {:?}",
        resolve_errors
    );

    let (typed_ast, interner, _type_env, type_diags) = typecheck(resolved, &asts, &[]);
    let type_errors: Vec<&Diagnostic> = type_diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        type_errors.is_empty(),
        "type errors: {:?}",
        type_errors
    );

    let (builder, emit_diags) = emit::emit(&typed_ast, &asts, &interner);
    (builder, emit_diags)
}

// =========================================================
// ModuleDef tests
// =========================================================

#[test]
fn module_def_always_present() {
    let (builder, diags) = emit_src("fn main() {}");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert!(builder.module_def.is_some(), "ModuleDef row must be present");
}

// =========================================================
// TypeDef tests
// =========================================================

#[test]
fn struct_emits_typedef() {
    let (builder, diags) = emit_src("struct Point { x: int, y: int }");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.type_def_count(), 1, "should have 1 TypeDef for Point");
    assert_eq!(
        builder.typedef_kind(0),
        TypeDefKind::Struct as u8,
        "TypeDef kind should be Struct"
    );
}

#[test]
fn struct_fields_emit_fielddefs() {
    let (builder, diags) = emit_src("struct Point { x: int, y: int }");
    assert!(diags.is_empty());
    assert_eq!(builder.field_def_count(), 2, "Point should have 2 FieldDefs");
}

#[test]
fn source_field_flags_keep_visibility_distinct_from_readonly() {
    let (builder, diags) = emit_src("struct Point { pub x: int, y: int }");
    assert!(diags.is_empty());

    let flags: Vec<u16> = builder.finalized_field_defs().map(|field| field.flags).collect();
    assert_eq!(flags.len(), 2);
    assert_ne!(flags[0] & FIELD_FLAG_PUBLIC, 0, "pub field must carry visibility bit");
    assert_eq!(flags[1] & FIELD_FLAG_PUBLIC, 0, "private field must not carry visibility bit");
    assert!(
        flags.iter().all(|flags| flags & FIELD_FLAG_READONLY == 0),
        "source grammar has no read-only field modifier"
    );
}

#[test]
fn compiler_builder_finalizes_empty_field_ranges_with_next_indices() {
    let mut builder = ModuleBuilder::new();
    builder.add_typedef("LeadingEmpty", "", TypeDefKind::Class, 0, None);
    let list = builder.add_typedef("List", "", TypeDefKind::Class, 0, None);
    builder.add_fielddef(list, "items", 0, 0);
    builder.add_typedef("__closure_0", "", TypeDefKind::Class, 0, None);
    let pair = builder.add_typedef("Pair", "", TypeDefKind::Struct, 0, None);
    builder.add_fielddef(pair, "left", 0, 0);
    builder.add_fielddef(pair, "right", 0, 0);
    builder.add_typedef("TrailingEmpty", "", TypeDefKind::Class, 0, None);

    builder.finalize();

    let starts: Vec<u32> = (0..builder.type_def_count())
        .map(|idx| builder.typedef_field_list(idx))
        .collect();
    assert_eq!(starts, vec![1, 1, 2, 2, 4]);
    assert!(starts.iter().all(|start| *start > 0));
    assert!(starts.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn entity_emits_typedef() {
    let (builder, diags) = emit_src(
        r#"
        entity Guard {
            health: int = 100
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.type_def_count(), 1, "should have 1 TypeDef for Guard");
    assert_eq!(
        builder.typedef_kind(0),
        TypeDefKind::Entity as u8,
        "TypeDef kind should be Entity"
    );
}

#[test]
fn enum_emits_typedef() {
    let (builder, diags) = emit_src(
        r#"
        enum Color {
            Red,
            Green,
            Blue,
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.type_def_count(), 1, "should have 1 TypeDef for Color");
    assert_eq!(
        builder.typedef_kind(0),
        TypeDefKind::Enum as u8,
        "TypeDef kind should be Enum"
    );
}

// =========================================================
// MethodDef tests
// =========================================================

#[test]
fn fn_emits_methoddef() {
    let (builder, diags) = emit_src("fn greet(name: string) -> string { name }");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.method_def_count(), 1, "should have 1 MethodDef for greet");
}

#[test]
fn fn_params_emit_paramdefs() {
    let (builder, diags) = emit_src("fn add(a: int, b: int) -> int { a }");
    assert!(diags.is_empty());
    assert_eq!(builder.param_def_count(), 2, "add should have 2 ParamDefs");
}

#[test]
fn methoddef_param_count_matches_entry_register_layout() {
    let (builder, diags) = emit_src(
        r#"
        struct Counter {}

        impl Counter {
            fn instance(self, value: int) -> int { return value; }
            fn static_method(value: int) -> int { return value; }
        }

        fn free_function(value: int) -> int { return value; }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);

    let param_count = |name: &str| {
        builder
            .finalized_method_defs()
            .find(|row| builder.string_heap.get_str(row.name) == name)
            .unwrap_or_else(|| panic!("missing MethodDef for {name}"))
            .param_count
    };

    assert_eq!(param_count("instance"), 2, "r0=self, r1=value");
    assert_eq!(param_count("static_method"), 1, "r0=value");
    assert_eq!(param_count("free_function"), 1, "r0=value");
    assert_eq!(
        param_count("get_type"),
        1,
        "r0=self for synthetic Reflectable method"
    );
}

// =========================================================
// ContractDef tests
// =========================================================

#[test]
fn contract_emits_contractdef_and_methods() {
    let (builder, diags) = emit_src(
        r#"
        contract Printable {
            fn display(self) -> string;
            fn debug(self) -> string;
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.contract_def_count(), 1, "should have 1 ContractDef");
    assert_eq!(
        builder.contract_method_count(),
        2,
        "Printable should have 2 ContractMethods"
    );
}

#[test]
fn contract_method_slots_assigned() {
    let (builder, diags) = emit_src(
        r#"
        contract Printable {
            fn display(self) -> string;
            fn debug(self) -> string;
        }
        "#,
    );
    assert!(diags.is_empty());
    // Slots should be 0 and 1 in declaration order
    assert_eq!(builder.contract_method_slot(0), 0, "first method slot should be 0");
    assert_eq!(builder.contract_method_slot(1), 1, "second method slot should be 1");
}

#[test]
fn compiler_builder_finalizes_contract_child_ranges_with_next_indices() {
    let mut builder = ModuleBuilder::new();

    // Keep a non-contract GenericParam ahead of the contract-owned rows to
    // verify that generic_param_list stores an absolute table row index.
    let generic_type = builder.add_typedef("Box", "", TypeDefKind::Class, 0, None);
    builder.add_generic_param(TableId::TypeDef, generic_type.0, 0, "T");

    let first = builder.add_contract_def("First", "", None);
    let _middle_empty = builder.add_contract_def("MiddleEmpty", "", None);
    let third = builder.add_contract_def("Third", "", None);
    let _trailing_empty = builder.add_contract_def("TrailingEmpty", "", None);

    // Deliberately collect children out of parent order; finalize must group
    // them while retaining correct starts for the intervening empty contracts.
    builder.add_contract_method(third, "third", 0, 17);
    builder.add_contract_method(first, "first_a", 0, 17);
    builder.add_contract_method(first, "first_b", 0, 17);
    builder.add_generic_param(TableId::ContractDef, third.0, 0, "U");
    builder.add_generic_param(TableId::ContractDef, first.0, 0, "T");
    builder.add_generic_param(TableId::ContractDef, first.0, 1, "E");

    writ_compiler::emit::slots::assign_vtable_slots(&mut builder);
    builder.finalize();

    let contract_defs = builder.finalized_contract_defs();
    let method_starts: Vec<u32> = contract_defs.iter().map(|row| row.method_list).collect();
    let generic_starts: Vec<u32> = contract_defs
        .iter()
        .map(|row| row.generic_param_list)
        .collect();

    assert_eq!(method_starts, vec![1, 3, 3, 4]);
    assert_eq!(generic_starts, vec![2, 4, 4, 5]);
    assert!(method_starts.iter().all(|start| *start > 0));
    assert!(generic_starts.iter().all(|start| *start > 0));

    let method_counts: Vec<u32> = method_starts
        .iter()
        .enumerate()
        .map(|(idx, start)| {
            let end = method_starts
                .get(idx + 1)
                .copied()
                .unwrap_or(builder.contract_method_count() as u32 + 1);
            end - start
        })
        .collect();
    let generic_counts: Vec<u32> = generic_starts
        .iter()
        .enumerate()
        .map(|(idx, start)| {
            let end = generic_starts
                .get(idx + 1)
                .copied()
                .unwrap_or(builder.generic_param_count() as u32 + 1);
            end - start
        })
        .collect();

    assert_eq!(method_counts, vec![2, 0, 1, 0]);
    assert_eq!(generic_counts, vec![2, 0, 1, 0]);
    assert_eq!(
        (0..builder.contract_method_count())
            .map(|idx| builder.contract_method_slot(idx))
            .collect::<Vec<_>>(),
        vec![0, 1, 0]
    );
}

// =========================================================
// ImplDef tests
// =========================================================

#[test]
fn impl_emits_impldef() {
    let (builder, diags) = emit_src(
        r#"
        struct Foo { x: int }

        impl Foo {
            fn make() -> int { 42 }
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    // ImplDef rows: 1 user impl + 1 Reflectable auto-impl for struct Foo = 2
    assert_eq!(builder.impl_def_count(), 2, "should have 2 ImplDefs (1 user + 1 Reflectable auto-impl)");
}

// =========================================================
// Reflectable auto-impl tests
// =========================================================

#[test]
fn reflectable_auto_impl_three_types() {
    let (builder, diags) = emit_src(
        r#"
        struct Point { x: int, y: int }
        enum Color { Red, Blue }
        entity Guard { name: string }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    // 3 user types -> 3 Reflectable auto-impls (no user impls in this source)
    assert_eq!(builder.impl_def_count(), 3, "should have 3 Reflectable auto-impls");
    // 3 synthetic get_type() MethodDefs (one per user type)
    assert_eq!(builder.method_def_count(), 3, "should have 3 get_type() MethodDefs");
    let reflectable_token = MetadataToken(builder.type_ref_token_by_name("Reflectable"));
    assert!(!reflectable_token.is_null(), "Reflectable TypeRef is missing");
    assert!(
        builder
            .finalized_impl_defs()
            .iter()
            .all(|implementation| implementation.contract_token == reflectable_token),
        "auto-impls must reference writ-runtime::Reflectable through a TypeRef"
    );
}

#[test]
fn method_owner_invariant_holds() {
    // Reflectable methods belong to their ImplDef, not directly to the user TypeDef.
    // Version 6 records that relationship on each MethodDef row, so method_list is
    // only a derived first-row index on the actual owner.
    let (builder, diags) = emit_src(
        r#"
        struct Foo { x: int }
        struct Bar { y: float }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.impl_def_count(), 2, "2 Reflectable auto-impls");
    assert_eq!(builder.type_def_count(), 2, "2 TypeDefs");
    assert_eq!(builder.typedef_method_list(0), 0, "Foo has no direct methods");
    assert_eq!(builder.typedef_method_list(1), 0, "Bar has no direct methods");

    let impls = builder.finalized_impl_defs();
    assert!(impls.iter().all(|row| row.method_list > 0));

    let owners: Vec<_> = builder
        .finalized_method_defs()
        .map(|row| row.owner)
        .collect();
    assert_eq!(owners.len(), 2);
    assert!(owners.iter().all(|owner| owner.table() == TableId::ImplDef));
    assert_eq!(owners[0].row(), 1);
    assert_eq!(owners[1].row(), 2);
}

// =========================================================
// GlobalDef tests
// =========================================================

#[test]
fn const_emits_globaldef() {
    let (builder, diags) = emit_src("const MAX: int = 100;");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.global_def_count(), 1, "should have 1 GlobalDef for MAX");
}

#[test]
fn global_mut_emits_globaldef() {
    let (builder, diags) = emit_src("global mut counter: int = 0;");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.global_def_count(), 1, "should have 1 GlobalDef for counter");
}

// =========================================================
// ExternDef tests
// =========================================================

#[test]
fn extern_fn_emits_externdef() {
    let (builder, diags) = emit_src("extern fn print(msg: string);");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    // 1 user-declared extern (print). Log-level externs are only emitted when referenced.
    assert_eq!(builder.extern_def_count(), 1, "should have 1 ExternDef: print");
    // The user-declared extern must be present by name
    let has_print = builder
        .extern_defs
        .iter()
        .any(|row| builder.string_heap.get_str(row.name) == "print");
    assert!(has_print, "expected ExternDef named 'print'");
}

// =========================================================
// ChoiceOption rename tests
// =========================================================

#[test]
fn choice_option_emits_externdef() {
    let (builder, diags) = emit_src(
        r#"
        dlg ask(narrator: Entity) {
            @narrator What do you think?
            $ choice {
                "Good!" { @narrator Great! }
                "Bad" { @narrator Sorry. }
            }
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    // At least one ExternDef row must be named "ChoiceOption"
    let has_choice_option = builder
        .extern_defs
        .iter()
        .any(|row| builder.string_heap.get_str(row.name) == "ChoiceOption");
    assert!(
        has_choice_option,
        "expected ExternDef named ChoiceOption, got: {:?}",
        builder
            .extern_defs
            .iter()
            .map(|r| builder.string_heap.get_str(r.name))
            .collect::<Vec<_>>()
    );
}

// =========================================================
// Token assignment tests
// =========================================================

#[test]
fn typedef_tokens_are_one_based() {
    let (builder, diags) = emit_src(
        r#"
        struct A {}
        struct B {}
        struct C {}
        "#,
    );
    assert!(diags.is_empty());
    // Canonical core declarations are injected into every compilation and also
    // contribute entries to def_token_map. Restrict this assertion to the three
    // user TypeDefs; their rows must remain one-based regardless of core size.
    let typedef_tokens: Vec<_> = builder
        .def_token_map
        .values()
        .filter(|t| t.table() == TableId::TypeDef)
        .collect();
    assert_eq!(typedef_tokens.len(), 3, "should have 3 TypeDef tokens");
    for token in &typedef_tokens {
        assert!(token.row() >= 1 && token.row() <= 3, "row should be 1-3, got {}", token.row());
    }
}

#[test]
fn fn_tokens_are_methoddef() {
    let (builder, diags) = emit_src(
        r#"
        fn foo() {}
        fn bar() {}
        "#,
    );
    assert!(diags.is_empty());
    assert_eq!(builder.method_def_count(), 2);
    // def_token_map now includes 2 MethodDef (foo/bar) + 5 ExternDef (log levels)
    let method_tokens: Vec<_> = builder
        .def_token_map
        .values()
        .filter(|t| t.table() == TableId::MethodDef)
        .collect();
    assert_eq!(method_tokens.len(), 2, "should have 2 MethodDef tokens");
    for token in &method_tokens {
        assert!(token.row() >= 1 && token.row() <= 2, "row should be 1-2, got {}", token.row());
    }
}

// =========================================================
// ExportDef tests
// =========================================================

#[test]
fn pub_items_emit_exportdef() {
    let (builder, diags) = emit_src(
        r#"
        pub struct Visible {}
        struct Hidden {}
        "#,
    );
    assert!(diags.is_empty());
    // Canonical core exports are injected into every compilation. Assert the
    // visibility behavior of this source without depending on the core's size.
    let export_names: Vec<_> = builder
        .export_defs
        .iter()
        .map(|row| builder.string_heap.get_str(row.name))
        .collect();
    assert_eq!(
        export_names.iter().filter(|name| **name == "Visible").count(),
        1,
        "Visible should be exported exactly once"
    );
    assert!(
        !export_names.contains(&"Hidden"),
        "Hidden must not be exported"
    );
}

// =========================================================
// Combined scenario
// =========================================================

#[test]
fn combined_struct_fn_const() {
    let (builder, diags) = emit_src(
        r#"
        const PI: float = 3.14;
        struct Circle { radius: float }
        fn area(r: float) -> float { r }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(builder.type_def_count(), 1, "1 TypeDef (Circle)");
    assert_eq!(builder.field_def_count(), 1, "1 FieldDef (radius)");
    // 2 MethodDefs: 1 user fn (area) + 1 Reflectable get_type() for Circle
    assert_eq!(builder.method_def_count(), 2, "2 MethodDefs (area + Circle.get_type)");
    assert_eq!(builder.param_def_count(), 1, "1 ParamDef (r)");
    assert_eq!(builder.global_def_count(), 1, "1 GlobalDef (PI)");
}

// =========================================================
// MetadataToken encode/decode
// =========================================================

#[test]
fn metadata_token_roundtrip() {
    let token = MetadataToken::new(TableId::MethodDef, 42);
    assert_eq!(token.table(), TableId::MethodDef);
    assert_eq!(token.row(), 42);
    assert!(!token.is_null());
}

#[test]
fn metadata_token_null() {
    let token = MetadataToken::NULL;
    assert!(token.is_null());
    assert_eq!(token.row(), 0);
}

// =========================================================
// Heap tests
// =========================================================

#[test]
fn string_heap_deduplication() {
    let mut heap = emit::heaps::StringHeap::new();
    let off1 = heap.intern("hello");
    let off2 = heap.intern("hello");
    let off3 = heap.intern("world");
    assert_eq!(off1, off2, "duplicate strings should have same offset");
    assert_ne!(off1, off3, "different strings should have different offsets");
}

#[test]
fn blob_heap_deduplication() {
    let mut heap = emit::heaps::BlobHeap::new();
    let off1 = heap.intern(&[0x01, 0x02, 0x03]);
    let off2 = heap.intern(&[0x01, 0x02, 0x03]);
    let off3 = heap.intern(&[0x04, 0x05]);
    assert_eq!(off1, off2, "duplicate blobs should have same offset");
    assert_ne!(off1, off3, "different blobs should have different offsets");
}

// =========================================================
// ModuleRef tests
// =========================================================

#[test]
fn writ_runtime_moduleref_always_present() {
    let (builder, diags) = emit_src("fn main() {}");
    assert!(diags.is_empty());
    assert!(
        !builder.module_refs.is_empty(),
        "should have at least 1 ModuleRef (writ-runtime)"
    );
}

// =========================================================
// GenericParam tests
// =========================================================

#[test]
fn generic_struct_emits_generic_params() {
    let (builder, diags) = emit_src(
        r#"
        struct Wrapper<T> { value: T }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(
        builder.generic_param_count(),
        1,
        "Wrapper<T> should have 1 GenericParam"
    );
}

#[test]
fn generic_fn_emits_generic_params() {
    let (builder, diags) = emit_src(
        r#"
        fn identity<T>(x: T) -> T { x }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    assert_eq!(
        builder.generic_param_count(),
        1,
        "identity<T> should have 1 GenericParam"
    );
}

// =========================================================
// Lifecycle hook tests
// =========================================================

#[test]
fn entity_hooks_emit_methoddefs() {
    let (builder, diags) = emit_src(
        r#"
        entity Item {
            name: string = "default"

            on create {
                let x: int = 1;
            }

            on interact(who: Entity) {
                let x: int = 2;
            }
        }
        "#,
    );
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);
    // The hook should generate a MethodDef
    assert!(
        builder.method_def_count() >= 1,
        "should have at least 1 MethodDef for on_create hook"
    );

    let hook_param_count = |name: &str| {
        builder
            .finalized_method_defs()
            .find(|row| builder.string_heap.get_str(row.name) == name)
            .unwrap_or_else(|| panic!("missing MethodDef for {name}"))
            .param_count
    };
    assert_eq!(hook_param_count("on_OnCreate"), 1, "r0=implicit self");
    assert_eq!(
        hook_param_count("on_OnInteract"),
        2,
        "r0=implicit self, r1=who"
    );
}

#[test]
fn class_hook_param_count_includes_implicit_self() {
    let (builder, diags) = emit_src("class Panel { on create { } }");
    assert!(diags.is_empty(), "unexpected emit diags: {:?}", diags);

    let hook = builder
        .finalized_method_defs()
        .find(|row| builder.string_heap.get_str(row.name) == "on_create")
        .expect("missing class hook MethodDef");
    assert_eq!(hook.param_count, 1, "r0=implicit self");
}

// =========================================================
// Empty program
// =========================================================

#[test]
fn empty_program_has_module_def() {
    let (builder, diags) = emit_src("");
    assert!(diags.is_empty());
    assert!(builder.module_def.is_some(), "even empty programs have a ModuleDef");
    assert_eq!(builder.type_def_count(), 0);
    assert_eq!(builder.method_def_count(), 0);
}

// =========================================================
// LocaleDef tests
// =========================================================

/// A dlg with a [Locale("ja")] override compiled via the full pipeline
/// (including the $-suffix lowering path) produces one LocaleDef row.
/// Uses empty dlg bodies to avoid Entity/say resolution requirements.
#[test]
fn locale_override_dlg_emits_locale_def() {
    // Use dlg syntax with empty bodies. The base greet() lowers to fn "greet";
    // the locale override lowers to fn "greet$ja" via lower_dialogue's suffix logic.
    // collect_locale_defs should detect the $ suffix, extract base name "greet",
    // find its MethodDef token, and emit 1 LocaleDef row.
    let (builder, diags) = emit_src(
        r#"
dlg greet() {}

[Locale("ja")]
dlg greet() {}
"#,
    );
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == writ_diagnostics::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(
        builder.locale_defs.len(),
        1,
        "one [Locale] dlg override should produce 1 LocaleDef row"
    );
}

/// Two locale overrides produce two LocaleDef rows.
#[test]
fn two_locale_overrides_emit_two_locale_defs() {
    let (builder, diags) = emit_src(
        r#"
dlg greet() {}

[Locale("ja")]
dlg greet() {}

[Locale("de")]
dlg greet() {}
"#,
    );
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == writ_diagnostics::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(
        builder.locale_defs.len(),
        2,
        "two [Locale] dlg overrides should produce 2 LocaleDef rows"
    );
}

/// A source with no [Locale] attributes produces zero LocaleDef rows.
#[test]
fn no_locale_attr_emits_zero_locale_defs() {
    let (builder, _) = emit_src(
        r#"
fn hello() -> int {
    return 42;
}
"#,
    );
    assert_eq!(
        builder.locale_defs.len(),
        0,
        "no Locale attrs means 0 LocaleDef rows"
    );
}

// =========================================================
// GenericConstraint tests
// =========================================================

/// A function with a single bound `<T: Equivalent>` produces exactly 1 GenericConstraint row.
#[test]
fn emit_generic_constraint_table() {
    let (builder, diags) = emit_src(
        r#"pub contract Equivalent { fn is_eq(self) -> bool; }
           pub struct Bar { pub x: int }
           impl Equivalent for Bar { fn is_eq(self) -> bool { true } }
           pub fn check_eq<T: Equivalent>(a: T, b: T) -> bool { true }
           pub fn test() {
               let a = new Bar { x: 1 };
               let b = new Bar { x: 2 };
               check_eq(a, b);
           }"#,
    );
    assert!(diags.is_empty(), "unexpected diags: {:?}", diags);
    let constraints: Vec<_> = builder.finalized_generic_constraints().to_vec();
    assert_eq!(constraints.len(), 1, "expected 1 GenericConstraint row, got {}", constraints.len());
    assert_eq!(constraints[0].param_row, 1, "param_row should be 1-based");
    assert_ne!(constraints[0].constraint, MetadataToken::NULL, "constraint token should be resolved");
}

/// A function with two bounds `<T: Equivalent + Comparable>` produces exactly 2 GenericConstraint rows.
#[test]
fn emit_generic_multi_constraint() {
    let (builder, diags) = emit_src(
        r#"pub contract Equivalent { fn is_eq(self) -> bool; }
           pub contract Comparable { fn cmp(self) -> int; }
           pub struct Pair { pub x: int }
           impl Equivalent for Pair { fn is_eq(self) -> bool { true } }
           impl Comparable for Pair { fn cmp(self) -> int { 0 } }
           pub fn compare<T: Equivalent + Comparable>(a: T, b: T) -> bool { true }
           pub fn test() {
               let a = new Pair { x: 1 };
               let b = new Pair { x: 2 };
               compare(a, b);
           }"#,
    );
    assert!(diags.is_empty(), "unexpected diags: {:?}", diags);
    let constraints: Vec<_> = builder.finalized_generic_constraints().to_vec();
    assert_eq!(constraints.len(), 2, "expected 2 GenericConstraint rows for Equivalent + Comparable, got {}", constraints.len());
}
