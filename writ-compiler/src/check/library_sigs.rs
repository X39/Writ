//! Reconstruct TypeEnv signatures from pre-compiled library module binaries.
//!
//! This module mirrors `writ-compiler/src/emit/type_sig.rs` encoding to decode
//! `FnSig` and `ImplEntry` values from blob heap data.
//!
//! Called after `TypeEnv::build` in `typecheck()` to populate method signatures
//! for library types that have no AST representation.

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;
use writ_diagnostics::FileId;
use writ_module::Module;
use writ_module::tables::TypeDefKind;

use crate::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};

use super::env::{FnSig, ImplEntry, TypeEnv};
use super::ty::{Ty, TyInterner, TyKind};

// =============================================================================
// Type blob decoder
// =============================================================================

/// Decode a type from a blob at `cursor`, advancing the cursor past the decoded bytes.
///
/// Mirrors the encoding in `writ-compiler/src/emit/type_sig.rs`.
fn decode_type_from_blob(
    blob: &[u8],
    cursor: &mut usize,
    lib_type_def_id_map: &FxHashMap<u32, (DefId, TypeDefKind)>,
    interner: &mut TyInterner,
) -> Ty {
    if *cursor >= blob.len() {
        return interner.error();
    }
    let tag = blob[*cursor];
    *cursor += 1;

    match tag {
        0x00 => interner.void(),
        0x01 => interner.int(),
        0x02 => interner.float(),
        0x03 => interner.bool_ty(),
        0x04 => interner.string_ty(),
        0x05 => interner.any_entity(),

        0x10 => {
            // Named type: u32 1-based TypeDef row index
            if *cursor + 4 > blob.len() {
                return interner.error();
            }
            let row = u32::from_le_bytes([
                blob[*cursor],
                blob[*cursor + 1],
                blob[*cursor + 2],
                blob[*cursor + 3],
            ]);
            *cursor += 4;
            if let Some(&(def_id, kind)) = lib_type_def_id_map.get(&row) {
                match kind {
                    TypeDefKind::Struct => interner.intern(TyKind::Struct(def_id)),
                    TypeDefKind::Class => interner.intern(TyKind::Class(def_id)),
                    TypeDefKind::Entity => interner.intern(TyKind::Entity(def_id)),
                    TypeDefKind::Enum => interner.intern(TyKind::Enum(def_id)),
                    TypeDefKind::Component => interner.intern(TyKind::Struct(def_id)), // components are struct-like
                }
            } else {
                interner.error()
            }
        }

        0x11 => {
            // TypeSpec placeholder (Option/Result/TaskHandle stub) — skip 4-byte row
            if *cursor + 4 <= blob.len() {
                *cursor += 4;
            }
            // Can't reconstruct these without more context — return Error (safe: suppresses errors)
            interner.error()
        }

        0x12 => {
            // Generic param: u16 index
            if *cursor + 2 > blob.len() {
                return interner.error();
            }
            let idx = u16::from_le_bytes([blob[*cursor], blob[*cursor + 1]]) as u32;
            *cursor += 2;
            interner.intern(TyKind::GenericParam(idx))
        }

        0x20 => {
            // Array<T>: recursive element type
            let elem = decode_type_from_blob(blob, cursor, lib_type_def_id_map, interner);
            interner.array(elem)
        }

        0x30 => {
            // Func: u32 blob_offset into the blob heap (in the outer module's blob_heap)
            // The function signature sub-blob format is: u16(param_count) + TypeRef[] + TypeRef(ret)
            // NOTE: The offset here is INTO the outer module's blob heap, but we are already
            // working within a sub-blob. Since we don't have the blob_heap here, we skip.
            // This is rare in practice (function-typed fields/params); return Func{[],void} as placeholder.
            if *cursor + 4 <= blob.len() {
                *cursor += 4;
            }
            let params: Vec<Ty> = Vec::new();
            let ret = interner.void();
            interner.intern(TyKind::Func { params, ret })
        }

        _ => {
            // Unknown tag — bail out (no way to skip unknown size)
            interner.error()
        }
    }
}

// =============================================================================
// Method signature decoder
// =============================================================================

/// Decode a method's parameter list and return type from its signature blob.
///
/// Blob format: u16(param_count) + TypeRef[param_count] + TypeRef(return_type).
fn decode_method_sig(
    blob: &[u8],
    lib_type_def_id_map: &FxHashMap<u32, (DefId, TypeDefKind)>,
    interner: &mut TyInterner,
) -> (Vec<Ty>, Ty) {
    if blob.len() < 2 {
        return (Vec::new(), interner.void());
    }
    let param_count = u16::from_le_bytes([blob[0], blob[1]]) as usize;
    let mut cursor = 2;

    let mut param_tys = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        let ty = decode_type_from_blob(blob, &mut cursor, lib_type_def_id_map, interner);
        param_tys.push(ty);
    }
    let ret = decode_type_from_blob(blob, &mut cursor, lib_type_def_id_map, interner);
    (param_tys, ret)
}

/// Build a `FnSig` from a `MethodDefRow` plus associated `ParamDefRow`s.
///
/// `param_start` is the 0-based index of the first `ParamDefRow` for this method,
/// `param_end` is one-past the last (exclusive).
fn build_fn_sig_from_binary(
    method: &writ_module::tables::MethodDefRow,
    method_name: &str,
    module: &Module,
    param_start: usize,
    param_end: usize,
    lib_type_def_id_map: &FxHashMap<u32, (DefId, TypeDefKind)>,
    interner: &mut TyInterner,
    lib_file_id: FileId,
    method_generics: Vec<String>,
) -> FnSig {
    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };

    // Decode the method signature blob
    let (param_tys, ret_ty) = match writ_module::heap::read_blob(&module.blob_heap, method.signature) {
        Ok(blob) => decode_method_sig(blob, lib_type_def_id_map, interner),
        Err(_) => (Vec::new(), interner.void()),
    };

    // Build (name, ty) pairs from ParamDef rows
    let mut params: Vec<(String, Ty)> = Vec::new();
    let mut self_param: Option<bool> = None;

    let param_rows = &module.param_defs[param_start..param_end.min(module.param_defs.len())];

    // The param_tys are in order: first comes self (if any), then regular params.
    // We need to match them up with param rows. But the method's param_count field
    // may differ from what's in param_tys (self is encoded separately in the param table).
    // Strategy: iterate param rows in sequence order; use param_tys index to decode.
    let mut param_ty_idx = 0;
    for param_row in param_rows {
        let param_name = writ_module::heap::read_string(&module.string_heap, param_row.name)
            .unwrap_or("_")
            .to_string();

        if param_name == "self" || param_name == "self_" {
            self_param = Some(false);
            // self is not in param_tys (it's implicit)
        } else if param_name == "mut_self" {
            self_param = Some(true);
        } else {
            // Regular param — use the next param_ty
            let ty = if param_ty_idx < param_tys.len() {
                param_tys[param_ty_idx]
            } else {
                interner.error()
            };
            param_ty_idx += 1;
            params.push((param_name, ty));
        }
    }

    // If no param rows but we have types, fall back to positional assignment
    if params.is_empty() && self_param.is_none() && !param_tys.is_empty() {
        for (i, ty) in param_tys.into_iter().enumerate() {
            params.push((format!("p{}", i), ty));
        }
    }

    let generic_count = method_generics.len();
    FnSig {
        name: method_name.to_string(),
        params,
        ret: ret_ty,
        generics: method_generics,
        self_param,
        bounds: vec![vec![]; generic_count],
        bound_decl_spans: vec![synthetic_span; generic_count],
        fn_file: lib_file_id,
    }
}

// =============================================================================
// Main injection entry point
// =============================================================================

/// Inject method signatures, struct fields, and impl entries from pre-compiled
/// library modules into the TypeEnv.
///
/// Called after `TypeEnv::build` in `typecheck()`, so user-source type info is
/// already present. Library types were previously injected into DefMap by
/// `inject_module_types` in the resolve stage.
///
/// `def_map` must have already been augmented by `inject_module_types`.
pub fn inject_library_sigs(
    library_modules: &[&Module],
    def_map: &mut DefMap,
    type_env: &mut TypeEnv,
    interner: &mut TyInterner,
) {
    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };

    for (lib_index, module) in library_modules.iter().enumerate() {
        let lib_file_id = FileId(u32::MAX - 1 - lib_index as u32);

        // Build lib_type_def_id_map: 1-based TypeDef row -> (DefId, TypeDefKind)
        // This maps binary type references back to DefIds in the DefMap.
        let mut lib_type_def_id_map: FxHashMap<u32, (DefId, TypeDefKind)> = FxHashMap::default();
        for (type_idx, type_def) in module.type_defs.iter().enumerate() {
            let row_1based = (type_idx + 1) as u32;
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
                name
            } else {
                format!("{}::{}", namespace, name)
            };

            if let Some(def_id) = def_map.get(&fqn) {
                let kind = TypeDefKind::from_u8(type_def.kind)
                    .unwrap_or(TypeDefKind::Struct);
                lib_type_def_id_map.insert(row_1based, (def_id, kind));
            }
        }

        // Build contract_def_id_map: 1-based ContractDef row -> DefId
        let mut lib_contract_def_id_map: FxHashMap<u32, DefId> = FxHashMap::default();
        for (contract_idx, contract_def) in module.contract_defs.iter().enumerate() {
            let row_1based = (contract_idx + 1) as u32;
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
                name
            } else {
                format!("{}::{}", namespace, name)
            };

            if let Some(def_id) = def_map.get(&fqn) {
                lib_contract_def_id_map.insert(row_1based, def_id);
            }
        }

        // Build method generic params: method 0-based index -> Vec<String>
        let method_generics: FxHashMap<u32, Vec<String>> = {
            let mut map: FxHashMap<u32, Vec<(u16, String)>> = FxHashMap::default();
            for param in &module.generic_params {
                if param.owner_kind != 1 {
                    // 1 = MethodDef
                    continue;
                }
                let row_1based = param.owner.row_index().unwrap_or(0);
                if row_1based == 0 {
                    continue;
                }
                let method_idx = row_1based - 1; // 0-based
                let param_name = writ_module::heap::read_string(&module.string_heap, param.name)
                    .unwrap_or("")
                    .to_string();
                map.entry(method_idx).or_default().push((param.ordinal, param_name));
            }
            map.into_iter()
                .map(|(k, mut v)| {
                    v.sort_by_key(|(ord, _)| *ord);
                    (k, v.into_iter().map(|(_, name)| name).collect())
                })
                .collect()
        };

        // Compute param_def ranges for each method:
        // method_param_ranges[method_idx] = (start, end) in module.param_defs (0-based)
        // The param_count field in MethodDefRow tells how many params, but we need the
        // start offset. We rely on the fact that params are ordered by method.
        // Actually, there's no direct "param_list" in MethodDefRow (unlike TypeDef.field_list).
        // We use param sequence numbers and match by method index ordering.
        // Simpler: params are laid out sequentially; method i's params follow method i-1's params.
        // Use method.param_count to build ranges.
        let method_param_ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::with_capacity(module.method_defs.len());
            let mut param_cursor = 0usize;
            for method in &module.method_defs {
                let start = param_cursor;
                let count = method.param_count as usize;
                let end = start + count;
                ranges.push((start, end));
                param_cursor = end;
            }
            ranges
        };

        // ---- Struct fields ----
        for (type_idx, type_def) in module.type_defs.iter().enumerate() {
            let row_1based = (type_idx + 1) as u32;
            let kind = TypeDefKind::from_u8(type_def.kind).unwrap_or(TypeDefKind::Struct);

            // Only Struct and Class have fields (Entity has properties but that's separate)
            if !matches!(kind, TypeDefKind::Struct | TypeDefKind::Class | TypeDefKind::Entity) {
                continue;
            }

            let def_id = match lib_type_def_id_map.get(&row_1based) {
                Some(&(def_id, _)) => def_id,
                None => continue,
            };

            // Already has fields from user AST? Skip (library overrides user is wrong)
            if type_env.struct_fields.contains_key(&def_id) || type_env.entity_fields.contains_key(&def_id) {
                continue;
            }

            // Compute field range for this type
            let field_start = if type_def.field_list == 0 {
                continue; // no fields
            } else {
                (type_def.field_list - 1) as usize // 1-based to 0-based
            };
            let field_end = if type_idx + 1 < module.type_defs.len() {
                let next_fl = module.type_defs[type_idx + 1].field_list;
                if next_fl == 0 {
                    module.field_defs.len()
                } else {
                    (next_fl - 1) as usize
                }
            } else {
                module.field_defs.len()
            };

            let mut fields: Vec<(String, Ty, SimpleSpan)> = Vec::new();
            for field_def in &module.field_defs[field_start..field_end.min(module.field_defs.len())] {
                let field_name = writ_module::heap::read_string(&module.string_heap, field_def.name)
                    .unwrap_or("_")
                    .to_string();

                let field_ty = match writ_module::heap::read_blob(&module.blob_heap, field_def.type_sig) {
                    Ok(blob) => {
                        let mut cursor = 0;
                        decode_type_from_blob(blob, &mut cursor, &lib_type_def_id_map, interner)
                    }
                    Err(_) => interner.error(),
                };

                fields.push((field_name, field_ty, synthetic_span));
            }

            if matches!(kind, TypeDefKind::Entity) {
                type_env.entity_fields.insert(def_id, fields);
            } else {
                type_env.struct_fields.insert(def_id, fields);
            }
        }

        // ---- Impl blocks (method signatures) ----
        for (impl_idx, impl_def) in module.impl_defs.iter().enumerate() {
            // Get the type DefId this impl is for
            let type_def_id = match impl_def.type_token.row_index() {
                Some(row_1based) => match lib_type_def_id_map.get(&row_1based) {
                    Some(&(def_id, _)) => def_id,
                    None => continue,
                },
                None => continue,
            };

            // Get the contract DefId (if any)
            let contract_def_id = impl_def.contract.row_index()
                .and_then(|row| lib_contract_def_id_map.get(&row).copied());

            // Compute method range
            let method_start = if impl_def.method_list == 0 {
                continue;
            } else {
                (impl_def.method_list - 1) as usize
            };
            let method_end = if impl_idx + 1 < module.impl_defs.len() {
                let next_ml = module.impl_defs[impl_idx + 1].method_list;
                if next_ml == 0 {
                    module.method_defs.len()
                } else {
                    (next_ml - 1) as usize
                }
            } else {
                module.method_defs.len()
            };

            // Create a synthetic DefId for this impl block by allocating a DefEntry
            let impl_entry_def_id = {
                let entry = DefEntry {
                    id: None,
                    kind: DefKind::Impl,
                    vis: DefVis::Pub,
                    file_id: lib_file_id,
                    namespace: String::new(),
                    name: format!("lib_impl#{}", lib_index * 1000 + impl_idx),
                    name_span: synthetic_span,
                    generics: Vec::new(),
                    span: synthetic_span,
                };
                def_map.arena.alloc(entry)
            };

            let mut methods: Vec<(String, FnSig)> = Vec::new();
            for method_idx in method_start..method_end.min(module.method_defs.len()) {
                let method = &module.method_defs[method_idx];
                let method_name = writ_module::heap::read_string(&module.string_heap, method.name)
                    .unwrap_or("_")
                    .to_string();

                let (param_start, param_end) = if method_idx < method_param_ranges.len() {
                    method_param_ranges[method_idx]
                } else {
                    (0, 0)
                };

                let generics = method_generics
                    .get(&(method_idx as u32))
                    .cloned()
                    .unwrap_or_default();

                let sig = build_fn_sig_from_binary(
                    method,
                    &method_name,
                    module,
                    param_start,
                    param_end,
                    &lib_type_def_id_map,
                    interner,
                    lib_file_id,
                    generics,
                );
                methods.push((method_name, sig));
            }

            let impl_entry = ImplEntry {
                impl_def_id: impl_entry_def_id,
                contract_def_id,
                methods,
            };

            type_env.impl_index
                .entry(type_def_id)
                .or_default()
                .push(impl_entry);
        }

        // ---- Top-level function signatures ----
        // Top-level functions were injected into DefMap by inject_module_types as DefKind::Fn.
        // Inject their FnSig into type_env.fn_sigs so call expressions type-check.
        //
        // A method is top-level if its 0-based index is NOT owned by any TypeDef or ImplDef.
        // We reuse the same ownership detection logic as inject_module_types.

        // Build type method ranges (same logic as inject_module_types)
        let type_method_ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            for (i, type_def) in module.type_defs.iter().enumerate() {
                if type_def.method_list == 0 { continue; }
                let start = (type_def.method_list - 1) as usize;
                let end = if i + 1 < module.type_defs.len() {
                    let next = module.type_defs[i + 1].method_list;
                    if next == 0 { module.method_defs.len() } else { (next - 1) as usize }
                } else { module.method_defs.len() };
                if start < end { ranges.push((start, end)); }
            }
            ranges
        };
        let impl_method_ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            for (i, impl_def) in module.impl_defs.iter().enumerate() {
                if impl_def.method_list == 0 { continue; }
                let start = (impl_def.method_list - 1) as usize;
                let end = if i + 1 < module.impl_defs.len() {
                    let next = module.impl_defs[i + 1].method_list;
                    if next == 0 { module.method_defs.len() } else { (next - 1) as usize }
                } else { module.method_defs.len() };
                if start < end { ranges.push((start, end)); }
            }
            ranges
        };
        let is_owned = |method_idx: usize| -> bool {
            for &(s, e) in &type_method_ranges { if method_idx >= s && method_idx < e { return true; } }
            for &(s, e) in &impl_method_ranges { if method_idx >= s && method_idx < e { return true; } }
            false
        };

        for (method_idx, method) in module.method_defs.iter().enumerate() {
            if is_owned(method_idx) { continue; }

            let method_name = writ_module::heap::read_string(&module.string_heap, method.name)
                .unwrap_or("")
                .to_string();
            if method_name.is_empty() { continue; }

            // Look up DefId in DefMap (was registered by inject_module_types)
            let def_id = match def_map.get(&method_name) {
                Some(id) => id,
                None => continue,
            };

            // Skip if we already have a sig (user code shadows or multiple loads)
            if type_env.fn_sigs.contains_key(&def_id) { continue; }

            let (param_start, param_end) = if method_idx < method_param_ranges.len() {
                method_param_ranges[method_idx]
            } else {
                (0, 0)
            };

            let generics = method_generics
                .get(&(method_idx as u32))
                .cloned()
                .unwrap_or_default();

            let sig = build_fn_sig_from_binary(
                method,
                &method_name,
                module,
                param_start,
                param_end,
                &lib_type_def_id_map,
                interner,
                lib_file_id,
                generics,
            );
            type_env.fn_sigs.insert(def_id, sig);
        }

        // ---- Contract methods ----
        for (contract_idx, contract_def) in module.contract_defs.iter().enumerate() {
            let row_1based = (contract_idx + 1) as u32;
            let contract_def_id = match lib_contract_def_id_map.get(&row_1based) {
                Some(&def_id) => def_id,
                None => continue,
            };

            if type_env.contract_methods.contains_key(&contract_def_id) {
                continue;
            }

            // Compute contract method range
            let cm_start = if contract_def.method_list == 0 {
                continue;
            } else {
                (contract_def.method_list - 1) as usize
            };
            let cm_end = if contract_idx + 1 < module.contract_defs.len() {
                let next_ml = module.contract_defs[contract_idx + 1].method_list;
                if next_ml == 0 {
                    module.contract_methods.len()
                } else {
                    (next_ml - 1) as usize
                }
            } else {
                module.contract_methods.len()
            };

            let mut sigs: Vec<FnSig> = Vec::new();
            for cm_idx in cm_start..cm_end.min(module.contract_methods.len()) {
                let cm = &module.contract_methods[cm_idx];
                let cm_name = writ_module::heap::read_string(&module.string_heap, cm.name)
                    .unwrap_or("_")
                    .to_string();
                let (param_tys, ret_ty) = match writ_module::heap::read_blob(&module.blob_heap, cm.signature) {
                    Ok(blob) => decode_method_sig(blob, &lib_type_def_id_map, interner),
                    Err(_) => (Vec::new(), interner.void()),
                };
                let params: Vec<(String, Ty)> = param_tys
                    .into_iter()
                    .enumerate()
                    .map(|(i, ty)| (format!("p{}", i), ty))
                    .collect();
                sigs.push(FnSig {
                    name: cm_name,
                    params,
                    ret: ret_ty,
                    generics: Vec::new(),
                    self_param: None,
                    bounds: Vec::new(),
                    bound_decl_spans: Vec::new(),
                    fn_file: lib_file_id,
                });
            }

            type_env.contract_methods.insert(contract_def_id, sigs);
        }
    }
}
