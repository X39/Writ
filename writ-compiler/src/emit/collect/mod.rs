//! Collection pass: walk TypedAst + DefMap + original ASTs, populate ModuleBuilder.
//!
//! This mirrors the pattern in `check/env.rs` — we need the original ASTs for
//! field/param/hook details that TypedDecl doesn't carry.

use std::collections::HashSet;

use rustc_hash::FxHashMap;
use writ_diagnostics::FileId;

use crate::check::ir::{TypedAst, TypedDecl};
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap};

use super::metadata::{MetadataToken, TableId};
use super::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle, ContractDefHandle};

mod types;
mod functions;
mod contracts;
mod builtins;
mod walker;
mod globals;
mod encoding;
mod lookup;

use types::{collect_struct, collect_entity, collect_enum, collect_class};
use functions::{collect_fn, collect_extern_fn, collect_component};
use contracts::{collect_contract, collect_impl, collect_extern_component, emit_reflectable_auto_impl};
pub(crate) use contracts::{ITERABLE_CONTRACT_TOKEN, ITERATOR_CONTRACT_TOKEN};
use globals::{collect_const, collect_global};
use encoding::{collect_exports, collect_attributes, collect_attribute_decl_defs, collect_locale_defs, collect_component_slots};
use walker::collect_called_def_ids;

pub use builtins::{inject_log_extern_defs, inject_dialogue_extern_defs};

/// Collect all definitions from the TypedAst into the ModuleBuilder.
///
/// `active_conditions` is the set of condition names that are active for this compilation.
/// Any `[Conditional("name")]` function whose condition is active will be emitted; its
/// fallback will be suppressed. If no conditions are active, fallbacks are emitted and
/// conditional variants are suppressed. Multiple active conditions targeting the same
/// fallback produce diagnostic E0010.
/// Info about a Reflectable auto-impl emitted during collect_defs.
///
/// Used to emit synthetic get_type() bodies in emit_all_bodies.
pub struct ReflectableInfo {
    /// The TypeDef's DefId — used to resolve the type_idx token for TYPEOF.
    pub def_id: DefId,
}

pub fn collect_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &crate::ast::Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    diags: &mut Vec<writ_diagnostics::Diagnostic>,
    active_conditions: &HashSet<String>,
    library_modules: &[&writ_module::Module],
) -> Vec<ReflectableInfo> {
    let def_map = &typed_ast.def_map;

    // 1. ModuleDef: always exactly 1 row.
    let module_name = find_module_name(def_map);
    builder.set_module_def(&module_name, "0.1.0", 0);

    // 2. ModuleRef: always emit writ-runtime.
    let runtime_mod_idx = builder.add_module_ref("writ-runtime", "1.0.0");

    // 2b. TypeRef: register Range<T> from writ-runtime so range expressions can construct it.
    builder.add_type_ref(runtime_mod_idx, "Range", "writ");

    // 2c. TypeRef: register the writ-runtime Type class and primitive pseudo-TypeDefs
    //     so typeof() expressions can resolve type_idx tokens.
    builder.add_type_ref(runtime_mod_idx, "Type", "writ");
    builder.add_type_ref(runtime_mod_idx, "Int", "writ");
    builder.add_type_ref(runtime_mod_idx, "Float", "writ");
    builder.add_type_ref(runtime_mod_idx, "Bool", "writ");
    builder.add_type_ref(runtime_mod_idx, "String", "writ");

    // 2d. TypeRef: register contracts used directly by body lowering so that
    //     CALL_VIRT can reference the writ-runtime ContractDef rows through the
    //     normal cross-module resolution path.
    //     These are prelude contracts with no user-module DefId; TypeRef resolution
    //     maps them to the writ-runtime virtual module's ContractDef table.
    builder.add_type_ref(runtime_mod_idx, "Eq", "writ");
    builder.add_type_ref(runtime_mod_idx, "Iterable", "writ");
    builder.add_type_ref(runtime_mod_idx, "Iterator", "writ");

    register_library_type_refs(library_modules, def_map, builder);
    register_provisional_named_tokens(typed_ast, builder);

    // Pre-scan: compute the set of DefIds to skip at emit time.
    // Active conditional variant: emit the conditional fn, skip its fallback.
    // Inactive conditional variant: skip the conditional fn, emit the fallback.
    let mut skipped_def_ids: HashSet<DefId> = HashSet::default();
    // Track which fallbacks have 1+ active conditional pointing at them (for E0010).
    let mut active_for_fallback: FxHashMap<DefId, Vec<DefId>> = FxHashMap::default();

    for (&cond_def_id, cond_name) in &typed_ast.conditional_fns {
        let is_active = active_conditions.contains(cond_name.as_str());
        if is_active {
            // Active condition: emit the conditional variant, suppress the fallback.
            if let Some(&fb_id) = typed_ast.fallback_for_conditional.get(&cond_def_id) {
                skipped_def_ids.insert(fb_id);
                active_for_fallback.entry(fb_id).or_default().push(cond_def_id);
            }
        } else {
            // Inactive condition: suppress the conditional variant, emit the fallback.
            skipped_def_ids.insert(cond_def_id);
        }
    }

    // E0010: ambiguous active conditions — multiple active conditionals sharing the same fallback.
    for (fb_id, active_conds) in &active_for_fallback {
        if active_conds.len() > 1 {
            let fb_entry = def_map.get_entry(*fb_id);
            let cond_names: Vec<&str> = active_conds
                .iter()
                .map(|id| typed_ast.conditional_fns[id].as_str())
                .collect();
            diags.push(
                writ_diagnostics::Diagnostic::error(
                    writ_diagnostics::code::E0010,
                    format!(
                        "multiple active conditions match function '{}': {}",
                        fb_entry.name,
                        cond_names.join(", ")
                    ),
                )
                .build(),
            );
        }
    }

    // 3. Walk TypedDecl list and emit rows.
    // We need to track TypeDefHandles for linking children.
    let mut typedef_handles: FxHashMap<DefId, TypeDefHandle> = FxHashMap::default();
    let mut methoddef_handles: FxHashMap<DefId, MethodDefHandle> = FxHashMap::default();
    // Track ContractDefHandles so collect_impl can look up contract tokens before finalize.
    let mut contractdef_handles: FxHashMap<DefId, ContractDefHandle> = FxHashMap::default();
    // Collect Reflectable auto-impl info for post-finalize fixup and body emission.
    let mut reflectable_infos: Vec<ReflectableInfo> = Vec::new();

    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Struct { def_id } => {
                collect_struct(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
                if let Some(&handle) = typedef_handles.get(def_id) {
                    emit_reflectable_auto_impl(handle, *def_id, builder);
                    reflectable_infos.push(ReflectableInfo { def_id: *def_id });
                }
            }
            TypedDecl::Class { def_id } => {
                collect_class(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
                if let Some(&handle) = typedef_handles.get(def_id) {
                    emit_reflectable_auto_impl(handle, *def_id, builder);
                    reflectable_infos.push(ReflectableInfo { def_id: *def_id });
                }
            }
            TypedDecl::Entity { def_id } => {
                collect_entity(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
                if let Some(&handle) = typedef_handles.get(def_id) {
                    emit_reflectable_auto_impl(handle, *def_id, builder);
                    reflectable_infos.push(ReflectableInfo { def_id: *def_id });
                }
            }
            TypedDecl::Enum { def_id } => {
                collect_enum(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
                if let Some(&handle) = typedef_handles.get(def_id) {
                    emit_reflectable_auto_impl(handle, *def_id, builder);
                    reflectable_infos.push(ReflectableInfo { def_id: *def_id });
                }
            }
            TypedDecl::Fn { def_id, .. } => {
                if skipped_def_ids.contains(def_id) {
                    continue;
                }
                collect_fn(*def_id, def_map, asts, interner, builder, &mut methoddef_handles, diags);
            }
            TypedDecl::Contract { def_id } => {
                let handle = collect_contract(*def_id, def_map, asts, interner, builder, diags);
                contractdef_handles.insert(*def_id, handle);
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
                    &contractdef_handles,
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
            TypedDecl::ExternComponent { def_id } => {
                collect_extern_component(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Const { def_id, .. } => {
                collect_const(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::Global { def_id, .. } => {
                collect_global(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::AttributeDef { .. } => {
                // No IL emission for attribute declarations in this phase.
                // Plan 02 will add AttributeDef table rows here.
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

    reflectable_infos
}

fn register_provisional_named_tokens(typed_ast: &TypedAst, builder: &mut ModuleBuilder) {
    let mut type_row = 0u32;
    let mut contract_row = 0u32;

    for decl in &typed_ast.decls {
        let (def_id, token) = match decl {
            TypedDecl::Struct { def_id }
            | TypedDecl::Class { def_id }
            | TypedDecl::Entity { def_id }
            | TypedDecl::Enum { def_id }
            | TypedDecl::Component { def_id }
            | TypedDecl::ExternComponent { def_id } => {
                type_row += 1;
                (*def_id, MetadataToken::new(TableId::TypeDef, type_row))
            }
            TypedDecl::Contract { def_id } => {
                contract_row += 1;
                (*def_id, MetadataToken::new(TableId::ContractDef, contract_row))
            }
            _ => continue,
        };
        builder.def_token_map.insert(def_id, token);
    }
}

fn register_library_type_refs(
    library_modules: &[&writ_module::Module],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    for module in library_modules {
        let module_name = writ_module::heap::read_string(
            &module.string_heap,
            module.header.module_name,
        )
        .unwrap_or("library");
        let module_version = writ_module::heap::read_string(
            &module.string_heap,
            module.header.module_version,
        )
        .unwrap_or("0.0.0");
        let module_ref = builder.add_module_ref(module_name, module_version);

        for type_def in &module.type_defs {
            let name = writ_module::heap::read_string(&module.string_heap, type_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(&module.string_heap, type_def.namespace)
                .unwrap_or("");
            register_library_type_ref(module_ref, name, namespace, def_map, builder);
        }
        for contract_def in &module.contract_defs {
            let name = writ_module::heap::read_string(&module.string_heap, contract_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(
                &module.string_heap,
                contract_def.namespace,
            )
            .unwrap_or("");
            register_library_type_ref(module_ref, name, namespace, def_map, builder);
        }
    }
}

fn register_library_type_ref(
    module_ref: usize,
    name: &str,
    namespace: &str,
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    if name.is_empty() {
        return;
    }
    let row = builder.add_type_ref(module_ref, name, namespace);
    let fqn = if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", namespace, name)
    };
    if let Some(def_id) = def_map.get(&fqn) {
        let token = MetadataToken::new(TableId::TypeRef, (row + 1) as u32);
        builder.def_token_map.insert(def_id, token);
    }
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

    // Attributes: walk all decls and emit AttributeDef rows (applications).
    collect_attributes(typed_ast, asts, builder);

    // Attribute declarations: emit AttributeDef rows with owner_kind=3.
    collect_attribute_decl_defs(typed_ast, asts, builder);

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
