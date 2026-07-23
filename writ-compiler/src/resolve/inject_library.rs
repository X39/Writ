//! Inject type definitions from pre-compiled library modules into the DefMap.
//!
//! Called at the start of `resolve()`, before `collect_declarations` (Pass 1),
//! so that library types are visible to all resolution passes.

use chumsky::span::SimpleSpan;
use writ_diagnostics::FileId;

use super::def_map::{DefEntry, DefKind, DefMap, DefVis};

/// Give an imported MethodDef a stable declaration identity without pretending
/// that it came from source text. The synthetic file id identifies the library,
/// while this span identifies the MethodDef row inside that library.
pub(crate) fn library_method_span(method_idx: usize) -> SimpleSpan {
    SimpleSpan {
        start: method_idx,
        end: method_idx.saturating_add(1),
        context: (),
    }
}

/// Find the DefId allocated for one exact imported top-level MethodDef row.
///
/// Names are insufficient here because a library may export overloads. Pairing
/// the library's synthetic file id with its MethodDef-row span makes this lookup
/// stable across resolution, type checking, and emission.
pub(crate) fn library_top_level_method_def_id(
    def_map: &DefMap,
    lib_file_id: FileId,
    method_idx: usize,
    method_name: &str,
) -> Option<super::def_map::DefId> {
    def_map.get_fn_by_span(
        method_name,
        lib_file_id,
        method_name,
        library_method_span(method_idx),
    )
}

/// Inject type and contract definitions from pre-compiled library modules into the DefMap.
///
/// For each library module, creates synthetic `DefEntry` records for all types,
/// contracts, and top-level functions and inserts them into `def_map`.
///
/// Library entries use synthetic `FileId(u32::MAX - 1 - lib_index)` to avoid
/// collision with user-source FileIds and the existing `FileId(u32::MAX)` sentinel
/// used by log/dialogue synthetics.
///
/// This must be called BEFORE `collect_declarations` so that library types are
/// in the DefMap when user-source Pass 1 and Pass 2 run. User code re-declaring
/// a library type will produce a duplicate-definition error (expected behavior).
pub fn inject_module_types(library_modules: &[&writ_module::Module], def_map: &mut DefMap) {
    let synthetic_span = SimpleSpan {
        start: 0,
        end: 0,
        context: (),
    };

    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);

        // --- Build generic param lookup for this module ---
        // Maps (owner_kind, 0-based owner index) -> sorted Vec<String> of param names
        // owner_kind 0 = TypeDef, owner_kind 2 = ContractDef

        // Collect generic params for type defs (owner_kind == 0)
        let type_generics: rustc_hash::FxHashMap<u32, Vec<String>> = {
            let mut map: rustc_hash::FxHashMap<u32, Vec<(u16, String)>> =
                rustc_hash::FxHashMap::default();
            for param in &module.generic_params {
                if param.owner_kind != 0 {
                    continue;
                }
                let row_1based = param.owner.row_index().unwrap_or(0);
                if row_1based == 0 {
                    continue;
                }
                let type_idx = row_1based - 1; // convert to 0-based
                let param_name = writ_module::heap::read_string(&module.string_heap, param.name)
                    .unwrap_or("")
                    .to_string();
                map.entry(type_idx)
                    .or_default()
                    .push((param.ordinal, param_name));
            }
            map.into_iter()
                .map(|(k, mut v)| {
                    v.sort_by_key(|(ord, _)| *ord);
                    (k, v.into_iter().map(|(_, name)| name).collect())
                })
                .collect()
        };

        // Collect generic params for contract defs (owner_kind == 2)
        let contract_generics: rustc_hash::FxHashMap<u32, Vec<String>> = {
            let mut map: rustc_hash::FxHashMap<u32, Vec<(u16, String)>> =
                rustc_hash::FxHashMap::default();
            for param in &module.generic_params {
                if param.owner_kind != 2 {
                    continue;
                }
                let row_1based = param.owner.row_index().unwrap_or(0);
                if row_1based == 0 {
                    continue;
                }
                let contract_idx = row_1based - 1; // convert to 0-based
                let param_name = writ_module::heap::read_string(&module.string_heap, param.name)
                    .unwrap_or("")
                    .to_string();
                map.entry(contract_idx)
                    .or_default()
                    .push((param.ordinal, param_name));
            }
            map.into_iter()
                .map(|(k, mut v)| {
                    v.sort_by_key(|(ord, _)| *ord);
                    (k, v.into_iter().map(|(_, name)| name).collect())
                })
                .collect()
        };

        // --- Inject type defs (structs, enums, entities, components, classes) ---
        for (type_idx, type_def) in module.type_defs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, type_def.name)
                .unwrap_or("")
                .to_string();
            let namespace = writ_module::heap::read_string(&module.string_heap, type_def.namespace)
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                continue;
            }

            let fqn = if namespace.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };

            // Skip if already present (guard against duplicates from multiple library loads)
            if def_map.by_fqn.contains_key(&fqn) {
                continue;
            }

            let kind = match writ_module::tables::TypeDefKind::from_u8(type_def.kind) {
                Some(writ_module::tables::TypeDefKind::Struct) => DefKind::Struct,
                Some(writ_module::tables::TypeDefKind::Enum) => DefKind::Enum,
                Some(writ_module::tables::TypeDefKind::Entity) => DefKind::Entity,
                Some(writ_module::tables::TypeDefKind::Component) => DefKind::Component,
                Some(writ_module::tables::TypeDefKind::Class) => DefKind::Class,
                None => continue, // unknown kind, skip
            };

            let generics = type_generics
                .get(&(type_idx as u32))
                .cloned()
                .unwrap_or_default();

            let entry = DefEntry {
                id: None,
                kind,
                vis: DefVis::Pub,
                file_id: lib_file_id,
                namespace: namespace.clone(),
                name: name.clone(),
                name_span: synthetic_span,
                generics,
                span: synthetic_span,
            };

            let id = def_map.arena.alloc(entry);
            def_map.by_fqn.insert(fqn, id);
            def_map
                .namespace_members
                .entry(namespace)
                .or_default()
                .push(id);
        }

        // --- Inject contract defs ---
        for (contract_idx, contract_def) in module.contract_defs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, contract_def.name)
                .unwrap_or("")
                .to_string();
            let namespace =
                writ_module::heap::read_string(&module.string_heap, contract_def.namespace)
                    .unwrap_or("")
                    .to_string();

            if name.is_empty() {
                continue;
            }

            let fqn = if namespace.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };

            if def_map.by_fqn.contains_key(&fqn) {
                continue;
            }

            let generics = contract_generics
                .get(&(contract_idx as u32))
                .cloned()
                .unwrap_or_default();

            let entry = DefEntry {
                id: None,
                kind: DefKind::Contract,
                vis: DefVis::Pub,
                file_id: lib_file_id,
                namespace: namespace.clone(),
                name: name.clone(),
                name_span: synthetic_span,
                generics,
                span: synthetic_span,
            };

            let id = def_map.arena.alloc(entry);
            def_map.by_fqn.insert(fqn, id);
            def_map
                .namespace_members
                .entry(namespace)
                .or_default()
                .push(id);
        }

        // --- Inject top-level functions using authoritative MethodDef.owner tokens. ---
        for method_idx in module.top_level_method_indices() {
            let method_def = &module.method_defs[method_idx];

            if method_def.flags & writ_module::tables::METHOD_FLAG_PUBLIC == 0 {
                continue;
            }

            let name = writ_module::heap::read_string(&module.string_heap, method_def.name)
                .unwrap_or("")
                .to_string();

            if name.is_empty() {
                continue;
            }

            // Top-level functions have no namespace in this module
            let fqn = name.clone();
            let method_span = library_method_span(method_idx);

            // `typecheck` defensively injects libraries again at its public
            // boundary. Keep that second pass idempotent at MethodDef-row
            // granularity rather than collapsing overloads by name.
            if library_top_level_method_def_id(def_map, lib_file_id, method_idx, &name).is_some() {
                continue;
            }

            let existing_fn = def_map.by_fqn.get(&fqn).copied().filter(|id| {
                matches!(def_map.get_entry(*id).kind, DefKind::Fn | DefKind::ExternFn)
            });
            if def_map.by_fqn.contains_key(&fqn) && existing_fn.is_none() {
                continue;
            }

            let entry = DefEntry {
                id: None,
                kind: DefKind::Fn,
                vis: DefVis::Pub,
                file_id: lib_file_id,
                namespace: String::new(),
                name: name.clone(),
                name_span: method_span,
                generics: Vec::new(),
                span: method_span,
            };

            let id = def_map.arena.alloc(entry);
            if method_def.flags & writ_module::tables::METHOD_FLAG_DIALOGUE != 0 {
                def_map.dialogue_defs.insert(id);
            }
            if let Some(existing_id) = existing_fn {
                def_map
                    .fn_overloads
                    .entry(fqn)
                    .or_insert_with(|| vec![existing_id])
                    .push(id);
            } else {
                def_map.by_fqn.insert(fqn, id);
            }
            def_map
                .namespace_members
                .entry(String::new())
                .or_default()
                .push(id);
        }
    }
}
