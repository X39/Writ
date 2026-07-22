//! Collection pass: walk TypedAst + DefMap + original ASTs, populate ModuleBuilder.
//!
//! This mirrors the pattern in `check/env.rs` — we need the original ASTs for
//! field/param/hook details that TypedDecl doesn't carry.

use std::collections::HashSet;

use rustc_hash::FxHashMap;
use writ_diagnostics::FileId;

use crate::check::ir::{TypedAst, TypedDecl};
use crate::check::ty::{Ty, TyInterner};
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
use globals::{collect_const, collect_global};
use encoding::{collect_exports, collect_attributes, collect_attribute_decl_defs, collect_locale_defs, collect_component_slots};
use walker::{collect_addressable_generic_types, collect_called_def_ids};

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
) -> (Vec<ReflectableInfo>, HashSet<DefId>) {
    let def_map = &typed_ast.def_map;

    // 1. ModuleDef: always exactly 1 row.
    let module_name = find_module_name(def_map, asts);
    builder.set_module_def(&module_name, "0.1.0", 0);

    // 2. ModuleRef: preserve normalized dependency order. Public emit entry
    // points guarantee that exactly one of these entries is writ-runtime.
    let library_module_refs = register_library_module_refs(library_modules, builder);
    let runtime_mod_idx = library_modules
        .iter()
        .zip(&library_module_refs)
        .find_map(|(module, &module_ref)| {
            crate::core_library::is_core_module(module).then_some(module_ref)
        })
        .unwrap_or_else(|| builder.add_module_ref("writ-runtime", "1.0.0"));

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

    register_library_type_refs(library_modules, &library_module_refs, def_map, builder);
    register_library_field_refs(library_modules, def_map, builder);
    register_library_method_refs(
        library_modules,
        &library_module_refs,
        def_map,
        builder,
    );
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

    for ty in collect_addressable_generic_types(typed_ast, interner, &skipped_def_ids) {
        intern_type_spec_for_ty(ty, interner, builder);
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
    let called_ids = collect_called_def_ids(typed_ast, &skipped_def_ids);
    inject_log_extern_defs(def_map, builder, &called_ids);
    inject_dialogue_extern_defs(def_map, builder, &called_ids);

    (reflectable_infos, skipped_def_ids)
}

/// Redirect calls resolved to a suppressed fallback to the active conditional body.
///
/// The checker deliberately resolves every call to the non-conditional fallback so
/// argument checking is independent of the active build conditions.  Metadata
/// collection, however, emits the conditional definition instead when its condition
/// is active.  Install the alias only after exports and attributes have been
/// collected, so the fallback does not appear as a duplicate exported definition.
pub(super) fn bind_active_conditional_call_targets(
    typed_ast: &TypedAst,
    active_conditions: &HashSet<String>,
    builder: &mut ModuleBuilder,
) {
    for (&conditional_id, condition) in &typed_ast.conditional_fns {
        if !active_conditions.contains(condition.as_str()) {
            continue;
        }
        let Some(&fallback_id) = typed_ast.fallback_for_conditional.get(&conditional_id) else {
            continue;
        };
        let Some(conditional_token) = builder.token_for_def(conditional_id) else {
            continue;
        };
        builder.def_token_map.insert(fallback_id, conditional_token);
    }
}

/// Add a TypeSpec for a checked structural generic type, or reuse its token.
pub(super) fn intern_type_spec_for_ty(
    ty: Ty,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
) -> MetadataToken {
    let ty = interner.resolve_infer(ty);
    if let Some(token) = builder.type_spec_token_for_ty(ty) {
        return token;
    }

    let def_tokens = builder.def_token_map.clone();
    let token_for_def = |def_id: DefId| {
        def_tokens
            .get(&def_id)
            .copied()
            .unwrap_or(MetadataToken::NULL)
    };
    let signature = crate::emit::type_sig::encode_type_bytes(ty, interner, &token_for_def);
    let signature = builder.blob_heap.intern(&signature);
    builder.add_type_spec(ty, signature)
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

fn register_library_module_refs(
    library_modules: &[&writ_module::Module],
    builder: &mut ModuleBuilder,
) -> Vec<usize> {
    library_modules
        .iter()
        .map(|module| {
            let module_def = module.module_defs.first();
            let module_name = module_def
                .and_then(|row| {
                    writ_module::heap::read_string(&module.string_heap, row.name).ok()
                })
                .or_else(|| {
                    writ_module::heap::read_string(
                        &module.string_heap,
                        module.header.module_name,
                    )
                    .ok()
                })
                .unwrap_or("library");
            let module_version = module_def
                .and_then(|row| {
                    writ_module::heap::read_string(&module.string_heap, row.version).ok()
                })
                .or_else(|| {
                    writ_module::heap::read_string(
                        &module.string_heap,
                        module.header.module_version,
                    )
                    .ok()
                })
                .unwrap_or("0.0.0");
            builder.add_module_ref(module_name, module_version)
        })
        .collect()
}

fn register_library_type_refs(
    library_modules: &[&writ_module::Module],
    library_module_refs: &[usize],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);
        let module_ref = library_module_refs[lib_index];

        for type_def in &module.type_defs {
            let name = writ_module::heap::read_string(&module.string_heap, type_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(&module.string_heap, type_def.namespace)
                .unwrap_or("");
            register_library_type_ref(
                module_ref,
                name,
                namespace,
                lib_file_id,
                def_map,
                builder,
            );
        }
        for contract_def in &module.contract_defs {
            let name = writ_module::heap::read_string(&module.string_heap, contract_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(
                &module.string_heap,
                contract_def.namespace,
            )
            .unwrap_or("");
            register_library_type_ref(
                module_ref,
                name,
                namespace,
                lib_file_id,
                def_map,
                builder,
            );
        }
    }
}

fn register_library_type_ref(
    module_ref: usize,
    name: &str,
    namespace: &str,
    lib_file_id: FileId,
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    if name.is_empty() {
        return;
    }
    let scope = MetadataToken::new(TableId::ModuleRef, (module_ref + 1) as u32);
    let existing_row = builder
        .finalized_type_refs()
        .iter()
        .position(|type_ref| {
            type_ref.scope == scope
                && builder.string_heap.get_str(type_ref.name) == name
                && builder.string_heap.get_str(type_ref.namespace) == namespace
        });
    let row = match existing_row {
        Some(index) => index,
        None => builder.add_type_ref(module_ref, name, namespace),
    };
    let fqn = if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", namespace, name)
    };
    if let Some(def_id) = def_map.get(&fqn) {
        // DefMap injection is first-wins. Only the library that supplied this
        // DefId may bind its token; a later module with the same FQN must not
        // redirect references away from the authoritative dependency.
        if def_map.get_entry(def_id).file_id == lib_file_id {
            let token = MetadataToken::new(TableId::TypeRef, (row + 1) as u32);
            builder.def_token_map.insert(def_id, token);
        }
    }
}

fn register_library_method_refs(
    library_modules: &[&writ_module::Module],
    library_module_refs: &[usize],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);

        // Direct TypeDef methods keep a bare imported parent.
        for (type_index, type_def) in module.type_defs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, type_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(
                &module.string_heap,
                type_def.namespace,
            )
            .unwrap_or("");
            let fqn = if namespace.is_empty() {
                name.to_string()
            } else {
                format!("{}::{}", namespace, name)
            };
            let Some(def_id) = def_map.get(&fqn) else {
                continue;
            };
            if def_map.get_entry(def_id).file_id != lib_file_id {
                continue;
            }
            let Some(parent) = builder.token_for_def(def_id) else {
                continue;
            };
            register_library_method_rows(
                module,
                module.type_method_indices(type_index),
                parent,
                true,
                def_map,
                builder,
            );
        }

        // ImplDefs may target imported types or exact specializations, so walk
        // them independently rather than flattening them under a local TypeDef.
        for (impl_index, implementation) in module.impl_defs.iter().enumerate() {
            let Some(parent) = remap_library_method_parent(
                module,
                implementation.type_token,
                def_map,
                builder,
            ) else {
                continue;
            };
            register_library_method_rows(
                module,
                module.impl_method_indices(impl_index),
                parent,
                implementation.contract.is_null(),
                def_map,
                builder,
            );
        }

        let module_parent = MetadataToken::new(
            TableId::ModuleRef,
            (library_module_refs[lib_index] + 1) as u32,
        );
        for method_index in module.top_level_method_indices() {
            let method = &module.method_defs[method_index];
            let method_name = writ_module::heap::read_string(&module.string_heap, method.name)
                .unwrap_or("");
            let Some(def_id) = crate::resolve::inject_library::library_top_level_method_def_id(
                def_map,
                lib_file_id,
                method_index,
                method_name,
            ) else { continue };
            let Ok(signature) = writ_module::heap::read_blob(
                &module.blob_heap, method.signature
            ) else { continue };
            let Some(signature) = remap_library_method_signature(
                module, signature, def_map, builder
            ) else { continue };
            let row = builder.add_method_ref_with_origin(
                module_parent, method_name, &signature, true, false
            );
            builder.def_token_map.insert(
                def_id,
                MetadataToken::new(TableId::MethodRef, (row + 1) as u32),
            );
        }
    }
}

fn register_library_field_refs(
    library_modules: &[&writ_module::Module],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);

        for (type_index, type_def) in module.type_defs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, type_def.name)
                .unwrap_or("");
            let namespace = writ_module::heap::read_string(&module.string_heap, type_def.namespace)
                .unwrap_or("");
            let fqn = if namespace.is_empty() {
                name.to_owned()
            } else {
                format!("{namespace}::{name}")
            };
            let Some(owner_def_id) = def_map.get(&fqn) else {
                continue;
            };
            if def_map.get_entry(owner_def_id).file_id != lib_file_id {
                continue;
            }
            let Some(parent) = builder.token_for_def(owner_def_id) else {
                continue;
            };
            if !matches!(parent.table(), TableId::TypeDef | TableId::TypeRef) {
                panic!(
                    "unsupported imported FieldRef parent table {} for `{fqn}`",
                    parent.table() as u8
                );
            }

            let field_start = type_def.field_list.saturating_sub(1) as usize;
            let field_end = module.type_defs.get(type_index + 1)
                .map(|next| next.field_list.saturating_sub(1) as usize)
                .unwrap_or(module.field_defs.len())
                .min(module.field_defs.len());
            if field_start > field_end {
                continue;
            }

            for field in &module.field_defs[field_start..field_end] {
                let field_name = writ_module::heap::read_string(&module.string_heap, field.name)
                    .unwrap_or("");
                if field_name.is_empty() {
                    continue;
                }
                let Ok(signature) = writ_module::heap::read_blob(&module.blob_heap, field.type_sig)
                else {
                    continue;
                };
                let Ok(signature) = writ_module::signature::decode_type_signature(signature) else {
                    continue;
                };
                let Some(signature) = remap_library_type_signature(
                    module, &signature, def_map, builder
                ) else {
                    continue;
                };
                let Ok(signature) = writ_module::signature::encode_type_signature(&signature)
                else {
                    continue;
                };
                builder.add_field_ref(owner_def_id, parent, field_name, &signature);
            }
        }
    }
}

fn register_library_method_rows(
    module: &writ_module::Module,
    method_indices: Vec<usize>,
    parent: MetadataToken,
    inherent: bool,
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) {
    for method_index in method_indices {
        let method = &module.method_defs[method_index];
        let method_name = writ_module::heap::read_string(&module.string_heap, method.name)
            .unwrap_or("");
        if method_name.is_empty() {
            continue;
        }
        let Ok(signature) = writ_module::heap::read_blob(&module.blob_heap, method.signature)
        else {
            continue;
        };
        let Some(signature) = remap_library_method_signature(module, signature, def_map, builder)
        else {
            continue;
        };
        let has_receiver = !method.owner.is_null() && method.flags & (1 << 1) == 0;
        builder.add_method_ref_with_origin(
            parent,
            method_name,
            &signature,
            inherent,
            has_receiver,
        );
    }
}

fn remap_library_method_parent(
    module: &writ_module::Module,
    parent: writ_module::MetadataToken,
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) -> Option<MetadataToken> {
    if parent.table_id() != writ_module::tables::TableId::TypeSpec.as_u8() {
        return consumer_token_for_library_named(module, parent, def_map, builder);
    }
    let row = parent.row_index()?.checked_sub(1)? as usize;
    let type_spec = module.type_specs.get(row)?;
    let blob = writ_module::heap::read_blob(&module.blob_heap, type_spec.signature).ok()?;
    let signature = writ_module::signature::decode_type_signature(blob).ok()?;
    let signature = remap_library_type_signature(module, &signature, def_map, builder)?;
    let encoded = writ_module::signature::encode_type_signature(&signature).ok()?;
    let signature = builder.blob_heap.intern(&encoded);
    Some(builder.add_type_spec_signature(signature))
}

fn remap_library_method_signature(
    module: &writ_module::Module,
    signature: &[u8],
    def_map: &DefMap,
    builder: &ModuleBuilder,
) -> Option<Vec<u8>> {
    let (params, ret) = writ_module::signature::decode_method_signature(signature).ok()?;
    let params = params
        .iter()
        .map(|param| remap_library_type_signature(module, param, def_map, builder))
        .collect::<Option<Vec<_>>>()?;
    let ret = remap_library_type_signature(module, &ret, def_map, builder)?;
    writ_module::signature::encode_method_signature(&params, &ret).ok()
}

fn remap_library_type_signature(
    module: &writ_module::Module,
    signature: &writ_module::signature::TypeSignature,
    def_map: &DefMap,
    builder: &ModuleBuilder,
) -> Option<writ_module::signature::TypeSignature> {
    use writ_module::signature::TypeSignature;
    Some(match signature {
        TypeSignature::Named(token) => TypeSignature::Named(writ_module::MetadataToken(
            consumer_token_for_library_named(module, *token, def_map, builder)?.0,
        )),
        TypeSignature::Generic { namespace, name, args } => TypeSignature::Generic {
            namespace: namespace.clone(),
            name: name.clone(),
            args: args.iter()
                .map(|arg| remap_library_type_signature(module, arg, def_map, builder))
                .collect::<Option<Vec<_>>>()?,
        },
        TypeSignature::Array(element) => TypeSignature::Array(Box::new(
            remap_library_type_signature(module, element, def_map, builder)?,
        )),
        TypeSignature::Function { params, ret } => TypeSignature::Function {
            params: params.iter()
                .map(|param| remap_library_type_signature(module, param, def_map, builder))
                .collect::<Option<Vec<_>>>()?,
            ret: Box::new(remap_library_type_signature(module, ret, def_map, builder)?),
        },
        other => other.clone(),
    })
}

fn consumer_token_for_library_named(
    module: &writ_module::Module,
    token: writ_module::MetadataToken,
    def_map: &DefMap,
    builder: &ModuleBuilder,
) -> Option<MetadataToken> {
    let row = token.row_index()?.checked_sub(1)? as usize;
    let (name, namespace) = match token.table_id() {
        id if id == writ_module::tables::TableId::TypeDef.as_u8() => {
            let definition = module.type_defs.get(row)?;
            (
                writ_module::heap::read_string(&module.string_heap, definition.name).ok()?,
                writ_module::heap::read_string(&module.string_heap, definition.namespace).ok()?,
            )
        }
        id if id == writ_module::tables::TableId::TypeRef.as_u8() => {
            let reference = module.type_refs.get(row)?;
            (
                writ_module::heap::read_string(&module.string_heap, reference.name).ok()?,
                writ_module::heap::read_string(&module.string_heap, reference.namespace).ok()?,
            )
        }
        id if id == writ_module::tables::TableId::ContractDef.as_u8() => {
            let definition = module.contract_defs.get(row)?;
            (
                writ_module::heap::read_string(&module.string_heap, definition.name).ok()?,
                writ_module::heap::read_string(&module.string_heap, definition.namespace).ok()?,
            )
        }
        _ => return None,
    };
    let fqn = if namespace.is_empty() { name.to_string() } else { format!("{}::{}", namespace, name) };
    builder.token_for_def(def_map.get(&fqn)?)
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

fn find_module_name(def_map: &DefMap, asts: &[(FileId, &crate::ast::Ast)]) -> String {
    let source_files: rustc_hash::FxHashSet<FileId> =
        asts.iter().map(|(file_id, _)| *file_id).collect();
    // Use the first namespace declared by this compilation unit, or "main".
    // Dependency entries, including the injected core module, also live in the
    // DefMap and must never determine the emitted module identity.
    for entry in def_map.arena.iter() {
        if !source_files.contains(&entry.1.file_id) {
            continue;
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
