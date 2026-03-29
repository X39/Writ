//! Inject type definitions from pre-compiled library modules into the DefMap.
//!
//! Called at the start of `resolve()`, before `collect_declarations` (Pass 1),
//! so that library types are visible to all resolution passes.

use chumsky::span::SimpleSpan;
use writ_diagnostics::FileId;

use super::def_map::{DefEntry, DefKind, DefMap, DefVis};

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
pub fn inject_module_types(
    library_modules: &[&writ_module::Module],
    def_map: &mut DefMap,
) {
    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };

    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);

        // --- Build generic param lookup for this module ---
        // Maps (owner_kind, 0-based owner index) -> sorted Vec<String> of param names
        // owner_kind 0 = TypeDef, owner_kind 2 = ContractDef

        // Collect generic params for type defs (owner_kind == 0)
        let type_generics: rustc_hash::FxHashMap<u32, Vec<String>> = {
            let mut map: rustc_hash::FxHashMap<u32, Vec<(u16, String)>> = rustc_hash::FxHashMap::default();
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
                map.entry(type_idx).or_default().push((param.ordinal, param_name));
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
            let mut map: rustc_hash::FxHashMap<u32, Vec<(u16, String)>> = rustc_hash::FxHashMap::default();
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
                map.entry(contract_idx).or_default().push((param.ordinal, param_name));
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
            def_map.namespace_members
                .entry(namespace)
                .or_default()
                .push(id);
        }

        // --- Inject contract defs ---
        for (contract_idx, contract_def) in module.contract_defs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, contract_def.name)
                .unwrap_or("")
                .to_string();
            let namespace = writ_module::heap::read_string(&module.string_heap, contract_def.namespace)
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
            def_map.namespace_members
                .entry(namespace)
                .or_default()
                .push(id);
        }

        // --- Inject top-level functions (methods not owned by any TypeDef or ImplDef) ---
        // A method is top-level if its 0-based index is NOT in any TypeDef's method range
        // AND NOT in any ImplDef's method range.

        // Build the set of method indices owned by TypeDefs
        let type_method_ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            for (i, type_def) in module.type_defs.iter().enumerate() {
                if type_def.method_list == 0 {
                    continue;
                }
                let start = (type_def.method_list - 1) as usize; // convert 1-based to 0-based
                // end = next type_def's method_list or total method count
                let end = if i + 1 < module.type_defs.len() {
                    let next_start = module.type_defs[i + 1].method_list;
                    if next_start == 0 {
                        module.method_defs.len()
                    } else {
                        (next_start - 1) as usize
                    }
                } else {
                    module.method_defs.len()
                };
                if start < end {
                    ranges.push((start, end));
                }
            }
            ranges
        };

        // Build the set of method indices owned by ImplDefs
        let impl_method_ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            for (i, impl_def) in module.impl_defs.iter().enumerate() {
                if impl_def.method_list == 0 {
                    continue;
                }
                let start = (impl_def.method_list - 1) as usize;
                let end = if i + 1 < module.impl_defs.len() {
                    let next_start = module.impl_defs[i + 1].method_list;
                    if next_start == 0 {
                        module.method_defs.len()
                    } else {
                        (next_start - 1) as usize
                    }
                } else {
                    module.method_defs.len()
                };
                if start < end {
                    ranges.push((start, end));
                }
            }
            ranges
        };

        let is_owned = |method_idx: usize| -> bool {
            for &(start, end) in &type_method_ranges {
                if method_idx >= start && method_idx < end {
                    return true;
                }
            }
            for &(start, end) in &impl_method_ranges {
                if method_idx >= start && method_idx < end {
                    return true;
                }
            }
            false
        };

        for (method_idx, method_def) in module.method_defs.iter().enumerate() {
            if is_owned(method_idx) {
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

            if def_map.by_fqn.contains_key(&fqn) {
                continue;
            }

            let entry = DefEntry {
                id: None,
                kind: DefKind::Fn,
                vis: DefVis::Pub,
                file_id: lib_file_id,
                namespace: String::new(),
                name: name.clone(),
                name_span: synthetic_span,
                generics: Vec::new(),
                span: synthetic_span,
            };

            let id = def_map.arena.alloc(entry);
            def_map.by_fqn.insert(fqn, id);
            def_map.namespace_members
                .entry(String::new())
                .or_default()
                .push(id);
        }
    }
}
