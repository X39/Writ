//! Regression coverage for canonical `writ-runtime` compiler injection.

use writ_module::signature::TypeSignature;
use writ_module::{MetadataToken, Module, ModuleBuilder, TypeDefKind};

fn compile(src: &str) -> Module {
    let src: &'static str = Box::leak(src.to_owned().into_boxed_str());
    let bytes = writ_compiler::compile_source(src).expect("source should compile");
    Module::from_bytes(&bytes).expect("compiled module should decode")
}

fn compile_with_libraries(src: &str, libraries: &[&Module]) -> Module {
    let src: &'static str = Box::leak(src.to_owned().into_boxed_str());
    let bytes = writ_compiler::compile_with_libraries(src, libraries)
        .expect("source should compile with libraries");
    Module::from_bytes(&bytes).expect("compiled module should decode")
}

fn string(module: &Module, offset: u32) -> &str {
    writ_module::heap::read_string(&module.string_heap, offset).unwrap_or("")
}

fn module_ref_indices(module: &Module, name: &str) -> Vec<usize> {
    module
        .module_refs
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (string(module, row.name) == name).then_some(index))
        .collect()
}

fn module_ref_names(module: &Module) -> Vec<&str> {
    module
        .module_refs
        .iter()
        .map(|row| string(module, row.name))
        .collect()
}

fn type_ref_index(module: &Module, namespace: &str, name: &str) -> Option<usize> {
    module.type_refs.iter().position(|row| {
        string(module, row.namespace) == namespace && string(module, row.name) == name
    })
}

fn top_level_method_signature(module: &Module, name: &str) -> Vec<u8> {
    let method_index = module
        .top_level_method_indices()
        .into_iter()
        .find(|&index| string(module, module.method_defs[index].name) == name)
        .expect("top-level method should exist");
    writ_module::heap::read_blob(
        &module.blob_heap,
        module.method_defs[method_index].signature,
    )
    .expect("method signature should exist")
    .to_vec()
}

#[test]
fn compile_source_injects_core_through_codegen() {
    let module = compile(r#"pub fn inspect(value: writ::FieldInfo) -> writ::FieldInfo { value }"#);

    let runtime_refs = module_ref_indices(&module, "writ-runtime");
    assert_eq!(runtime_refs.len(), 1, "core ModuleRef must be unique");

    let field_info_index = type_ref_index(&module, "writ", "FieldInfo")
        .expect("core FieldInfo must reach emitted TypeRefs");
    assert_eq!(
        module.type_refs[field_info_index].scope.row_index(),
        Some((runtime_refs[0] + 1) as u32),
        "FieldInfo must be scoped to writ-runtime",
    );

    let signature = top_level_method_signature(&module, "inspect");
    let (params, ret) = writ_module::signature::decode_method_signature(&signature)
        .expect("inspect signature should decode");
    let field_info_token = MetadataToken::new(3, (field_info_index + 1) as u32);
    assert_eq!(params, vec![TypeSignature::Named(field_info_token)]);
    assert_eq!(ret, TypeSignature::Named(field_info_token));
}

#[test]
fn compile_source_injects_core_field_metadata_into_typecheck() {
    let module = compile(r#"pub fn field_name(value: writ::FieldInfo) -> string { value.name }"#);

    assert!(
        module
            .top_level_method_indices()
            .into_iter()
            .any(|index| string(&module, module.method_defs[index].name) == "field_name"),
        "FieldInfo.name access must typecheck and reach codegen",
    );
}

#[test]
fn typecheck_boundary_injects_core_into_an_external_def_map() {
    let resolved = writ_compiler::resolve::ir::NameResolvedAst {
        decls: Vec::new(),
        def_map: writ_compiler::resolve::def_map::DefMap::new(),
    };

    let (typed_ast, _interner, type_env, diagnostics) =
        writ_compiler::check::typecheck(resolved, &[], &[]);

    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    let field_info = typed_ast
        .def_map
        .get("writ::FieldInfo")
        .expect("typecheck must inject the implicit core DefMap entries");
    assert_eq!(
        type_env
            .struct_fields
            .get(&field_info)
            .expect("typecheck must load core field metadata")
            .len(),
        3,
    );
}

#[test]
fn explicit_libraries_and_implicit_core_both_reach_codegen() {
    let mut library_builder = ModuleBuilder::new("types");
    library_builder.add_type_def("Remote", "external", TypeDefKind::Struct, 0);
    let library = library_builder.build();

    let mut second_builder = ModuleBuilder::new("more-types");
    second_builder.add_type_def("Marker", "external", TypeDefKind::Struct, 0);
    let second_library = second_builder.build();

    let module = compile_with_libraries(
        r#"pub fn bridge(remote: external::Remote, marker: external::Marker, field: writ::FieldInfo) -> external::Remote { remote }"#,
        &[&library, &second_library],
    );

    let library_refs = module_ref_indices(&module, "types");
    let second_library_refs = module_ref_indices(&module, "more-types");
    let runtime_refs = module_ref_indices(&module, "writ-runtime");
    assert_eq!(library_refs.len(), 1, "explicit ModuleRef must be retained");
    assert_eq!(second_library_refs.len(), 1);
    assert_eq!(
        runtime_refs.len(),
        1,
        "implicit core ModuleRef must be unique"
    );
    assert_eq!(
        module_ref_names(&module),
        vec!["types", "more-types", "writ-runtime"],
        "canonical core must be appended after explicit dependencies",
    );

    let remote_index = type_ref_index(&module, "external", "Remote")
        .expect("explicit library TypeRef must be emitted");
    let marker_index = type_ref_index(&module, "external", "Marker")
        .expect("second explicit library TypeRef must be emitted");
    let field_info_index = type_ref_index(&module, "writ", "FieldInfo")
        .expect("implicit core TypeRef must be emitted");
    assert_eq!(
        module.type_refs[remote_index].scope.row_index(),
        Some((library_refs[0] + 1) as u32),
    );
    assert_eq!(
        module.type_refs[marker_index].scope.row_index(),
        Some((second_library_refs[0] + 1) as u32),
    );
    assert_eq!(
        module.type_refs[field_info_index].scope.row_index(),
        Some((runtime_refs[0] + 1) as u32),
    );

    let signature = top_level_method_signature(&module, "bridge");
    let (params, ret) = writ_module::signature::decode_method_signature(&signature)
        .expect("bridge signature should decode");
    let remote_token = MetadataToken::new(3, (remote_index + 1) as u32);
    let marker_token = MetadataToken::new(3, (marker_index + 1) as u32);
    let field_info_token = MetadataToken::new(3, (field_info_index + 1) as u32);
    assert_eq!(
        params,
        vec![
            TypeSignature::Named(remote_token),
            TypeSignature::Named(marker_token),
            TypeSignature::Named(field_info_token),
        ],
    );
    assert_eq!(ret, TypeSignature::Named(remote_token));
}

#[test]
fn first_explicit_core_is_authoritative() {
    let mut first = writ_module::build_writ_runtime_module();
    let version = writ_module::heap::intern_string(&mut first.string_heap, "9.9.9");
    first.header.module_version = version;
    first.module_defs[0].version = version;

    let mut second_builder = ModuleBuilder::new("writ-runtime");
    second_builder.add_type_def("SecondOnly", "writ", TypeDefKind::Struct, 0);
    let second = second_builder.build();
    let before = ModuleBuilder::new("before-core").build();
    let after = ModuleBuilder::new("after-core").build();

    let module = compile_with_libraries(
        r#"pub fn echo(value: writ::FieldInfo) -> writ::FieldInfo { value }"#,
        &[&before, &first, &after, &second],
    );

    let runtime_refs = module_ref_indices(&module, "writ-runtime");
    assert_eq!(runtime_refs.len(), 1);
    assert_eq!(
        module_ref_names(&module),
        vec!["before-core", "writ-runtime", "after-core"],
        "the first explicit core must retain its supplied position",
    );
    assert_eq!(
        string(&module, module.module_refs[runtime_refs[0]].min_version),
        "9.9.9",
    );
    assert!(type_ref_index(&module, "writ", "FieldInfo").is_some());
    assert!(type_ref_index(&module, "writ", "SecondOnly").is_none());
}
