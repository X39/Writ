//! Collection pass: walk TypedAst + DefMap + original ASTs, populate ModuleBuilder.
//!
//! This mirrors the pattern in `check/env.rs` — we need the original ASTs for
//! field/param/hook details that TypedDecl doesn't carry.

use rustc_hash::FxHashMap;
use writ_diagnostics::FileId;

use crate::check::ir::{TypedAst, TypedDecl};
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap};

use super::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle};

mod types;
mod functions;
mod contracts;
mod builtins;
mod walker;
mod globals;
mod encoding;
mod lookup;

use types::{collect_struct, collect_entity, collect_enum, collect_class, collect_extern_struct};
use functions::{collect_fn, collect_extern_fn, collect_component};
use contracts::{collect_contract, collect_impl, collect_extern_class, collect_extern_component};
use globals::{collect_const, collect_global};
use encoding::{collect_exports, collect_attributes, collect_locale_defs, collect_component_slots};
use walker::collect_called_def_ids;

pub use builtins::{inject_log_extern_defs, inject_dialogue_extern_defs};

/// Collect all definitions from the TypedAst into the ModuleBuilder.
pub fn collect_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &crate::ast::Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    diags: &mut Vec<writ_diagnostics::Diagnostic>,
) {
    let def_map = &typed_ast.def_map;

    // 1. ModuleDef: always exactly 1 row.
    let module_name = find_module_name(def_map);
    builder.set_module_def(&module_name, "0.1.0", 0);

    // 2. ModuleRef: always emit writ-runtime.
    builder.add_module_ref("writ-runtime", "1.0.0");

    // 3. Walk TypedDecl list and emit rows.
    // We need to track TypeDefHandles for linking children.
    let mut typedef_handles: FxHashMap<DefId, TypeDefHandle> = FxHashMap::default();
    let mut methoddef_handles: FxHashMap<DefId, MethodDefHandle> = FxHashMap::default();

    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Struct { def_id } => {
                collect_struct(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Class { def_id } => {
                collect_class(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Entity { def_id } => {
                collect_entity(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Enum { def_id } => {
                collect_enum(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Fn { def_id, .. } => {
                collect_fn(*def_id, def_map, asts, interner, builder, &mut methoddef_handles, diags);
            }
            TypedDecl::Contract { def_id } => {
                collect_contract(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::Impl { def_id, methods } => {
                collect_impl(
                    *def_id,
                    methods,
                    def_map,
                    asts,
                    interner,
                    builder,
                    &typedef_handles,
                    &mut methoddef_handles,
                    diags,
                );
            }
            TypedDecl::Component { def_id } => {
                collect_component(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::ExternFn { def_id } => {
                collect_extern_fn(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::ExternStruct { def_id } => {
                collect_extern_struct(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::ExternClass { def_id } => {
                collect_extern_class(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::ExternComponent { def_id } => {
                collect_extern_component(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Const { def_id, .. } => {
                collect_const(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::Global { def_id, .. } => {
                collect_global(*def_id, def_map, asts, interner, builder, diags);
            }
        }
    }

    // 4. Component slots: walk entity decls for component slots.
    collect_component_slots(typed_ast, asts, def_map, builder, &typedef_handles);

    // 5. LocaleDef: collected in collect_post_finalize() after token assignment.

    // Note: ExportDef and AttributeDef are collected in collect_post_finalize()
    // after token assignment, because they depend on resolved MetadataTokens.

    // 6. Inject synthetic ExternDef rows for log-level builtins AFTER all user-declared
    //    externs so that existing user extern token indices are not shifted.
    //    Only inject those actually referenced by the source code.
    let called_ids = collect_called_def_ids(typed_ast);
    inject_log_extern_defs(def_map, builder, &called_ids);
    inject_dialogue_extern_defs(def_map, builder, &called_ids);
}

/// Collect exports and attributes that depend on finalized tokens.
///
/// Must be called after `builder.finalize()`.
pub fn collect_post_finalize(
    typed_ast: &TypedAst,
    asts: &[(FileId, &crate::ast::Ast)],
    builder: &mut ModuleBuilder,
) {
    let def_map = &typed_ast.def_map;

    // ExportDef: walk DefMap.by_fqn for all pub-visible items.
    collect_exports(def_map, builder);

    // Attributes: walk all decls and emit AttributeDef rows.
    collect_attributes(typed_ast, asts, builder);

    // LocaleDef: walk all Fn decls for [Locale("tag")] attribute overrides.
    collect_locale_defs(typed_ast, asts, builder);
}

// =============================================================================
// Module name
// =============================================================================

fn find_module_name(def_map: &DefMap) -> String {
    use writ_diagnostics::FileId;
    // Use the first non-synthetic namespace found, or "main".
    // Skip entries with FileId(u32::MAX) — those are synthetic builtins (e.g. log:: levels).
    for entry in def_map.arena.iter() {
        if entry.1.file_id == FileId(u32::MAX) {
            continue; // skip synthetic entries
        }
        if !entry.1.namespace.is_empty() {
            // Return the root namespace segment.
            let ns = &entry.1.namespace;
            if let Some(root) = ns.split("::").next() {
                return root.to_string();
            }
        }
    }
    "main".to_string()
}

// =============================================================================
// Type signature encoding helper
// =============================================================================

/// Build a generic param name-to-index map.
pub(super) fn build_generic_map(generics: &[String]) -> rustc_hash::FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect()
}
