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
use writ_module::signature::TypeSignature;
use writ_module::tables::TypeDefKind;

use crate::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};

use super::env::{FnSig, ImplEntry, TypeEnv};
use super::ty::{Ty, TyInterner, TyKind};

// =============================================================================
// Type blob decoder
// =============================================================================

#[derive(Clone, Copy)]
enum LibraryTypeKind {
    Struct,
    Class,
    Entity,
    Enum,
    Contract,
    Component,
}

#[derive(Clone, Copy)]
struct LibraryType {
    def_id: DefId,
    kind: LibraryTypeKind,
}

impl LibraryType {
    fn from_type_def(def_id: DefId, kind: TypeDefKind) -> Self {
        let kind = match kind {
            TypeDefKind::Struct => LibraryTypeKind::Struct,
            TypeDefKind::Class => LibraryTypeKind::Class,
            TypeDefKind::Entity => LibraryTypeKind::Entity,
            TypeDefKind::Enum => LibraryTypeKind::Enum,
            TypeDefKind::Component => LibraryTypeKind::Component,
        };
        Self { def_id, kind }
    }
}

fn library_type_for_def(def_id: DefId, def_map: &DefMap) -> Option<LibraryType> {
    let kind = match def_map.get_entry(def_id).kind {
        DefKind::Struct => LibraryTypeKind::Struct,
        DefKind::Class => LibraryTypeKind::Class,
        DefKind::Entity => LibraryTypeKind::Entity,
        DefKind::Enum => LibraryTypeKind::Enum,
        DefKind::Contract => LibraryTypeKind::Contract,
        DefKind::Component | DefKind::ExternComponent => LibraryTypeKind::Component,
        _ => return None,
    };
    Some(LibraryType { def_id, kind })
}

fn lookup_constructor<'a, T>(
    types: &'a FxHashMap<String, T>,
    namespace: &str,
    name: &str,
) -> Option<&'a T> {
    if namespace.is_empty() {
        types.get(name)
    } else {
        types.get(&format!("{}::{}", namespace, name))
    }
}

/// Decode a type from a blob at `cursor`, advancing the cursor past the decoded bytes.
///
/// Mirrors the encoding in `writ-compiler/src/emit/type_sig.rs`.
fn type_signature_to_ty(
    signature: &TypeSignature,
    lib_type_token_map: &FxHashMap<u32, LibraryType>,
    lib_type_name_map: &FxHashMap<String, LibraryType>,
    interner: &mut TyInterner,
) -> Ty {
    match signature {
        TypeSignature::Void => interner.void(),
        TypeSignature::Int => interner.int(),
        TypeSignature::Float => interner.float(),
        TypeSignature::Bool => interner.bool_ty(),
        TypeSignature::String => interner.string_ty(),
        TypeSignature::Entity => interner.any_entity(),
        TypeSignature::Named(token) => lib_type_token_map
            .get(&token.0)
            .copied()
            .map(|named| nominal_ty(named, interner))
            .unwrap_or_else(|| interner.error()),
        TypeSignature::Generic {
            namespace,
            name,
            args,
        } => {
            let decoded_args: Vec<Ty> = args
                .iter()
                .map(|arg| {
                    type_signature_to_ty(arg, lib_type_token_map, lib_type_name_map, interner)
                })
                .collect();
            match (namespace.as_str(), name.as_str(), decoded_args.as_slice()) {
                ("writ" | "", "Option", [inner]) => interner.option(*inner),
                ("writ" | "", "Result", [ok, err]) => interner.result(*ok, *err),
                ("writ" | "", "TaskHandle", [inner]) => interner.task_handle(*inner),
                ("writ" | "", "Type", [inner]) => interner.reflection_type(*inner),
                _ => {
                    lookup_constructor(lib_type_name_map, namespace, name)
                        .copied()
                        .map(|named| {
                            let base = nominal_ty(named, interner);
                            interner.generic_instance(
                                base,
                                namespace.clone(),
                                name.clone(),
                                decoded_args,
                            )
                        })
                        .unwrap_or_else(|| interner.error())
                }
            }
        }
        TypeSignature::GenericParam(ordinal) => {
            interner.intern(TyKind::GenericParam(u32::from(*ordinal)))
        }
        TypeSignature::Array(element) => {
            let element =
                type_signature_to_ty(element, lib_type_token_map, lib_type_name_map, interner);
            interner.array(element)
        }
        TypeSignature::Function { params, ret } => {
            let params = params
                .iter()
                .map(|param| {
                    type_signature_to_ty(param, lib_type_token_map, lib_type_name_map, interner)
                })
                .collect();
            let ret = type_signature_to_ty(ret, lib_type_token_map, lib_type_name_map, interner);
            interner.func(params, ret)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{impl_generic_param_count, lookup_constructor};
    use rustc_hash::FxHashMap;
    use writ_module::module::MethodBody;
    use writ_module::{MetadataToken, ModuleBuilder};

    #[test]
    fn namespaced_constructor_lookup_never_falls_back_to_short_name() {
        let mut types = FxHashMap::default();
        types.insert("List".to_string(), 1);
        types.insert("alpha::List".to_string(), 2);

        assert_eq!(lookup_constructor(&types, "alpha", "List"), Some(&2));
        assert_eq!(lookup_constructor(&types, "beta", "List"), None);
        assert_eq!(lookup_constructor(&types, "", "List"), Some(&1));
    }

    #[test]
    fn impl_generic_prefix_uses_owned_method_generic_ordinals() {
        let mut builder = ModuleBuilder::new("impl-prefix");
        let implementation = builder.add_impl_def(MetadataToken::NULL, MetadataToken::NULL);
        let method = builder.add_impl_method(
            implementation,
            "pick",
            &[],
            0,
            0,
            MethodBody {
                register_types: vec![],
                code: vec![],
                debug_locals: vec![],
                source_spans: vec![],
            },
        );
        builder.add_generic_param(method, 1, 1, "U");
        let module = builder.build();

        assert_eq!(impl_generic_param_count(&module, [], &[0]), 1);
    }
}

fn nominal_ty(named: LibraryType, interner: &mut TyInterner) -> Ty {
    match named.kind {
        LibraryTypeKind::Struct => interner.intern(TyKind::Struct(named.def_id)),
        LibraryTypeKind::Class => interner.intern(TyKind::Class(named.def_id)),
        LibraryTypeKind::Entity => interner.intern(TyKind::Entity(named.def_id)),
        LibraryTypeKind::Enum => interner.intern(TyKind::Enum(named.def_id)),
        LibraryTypeKind::Contract => interner.intern(TyKind::Contract(named.def_id)),
        LibraryTypeKind::Component => interner.intern(TyKind::Struct(named.def_id)),
    }
}

fn decode_type_from_blob(
    blob: &[u8],
    cursor: &mut usize,
    lib_type_token_map: &FxHashMap<u32, LibraryType>,
    lib_type_name_map: &FxHashMap<String, LibraryType>,
    interner: &mut TyInterner,
) -> Ty {
    let bytes = blob.get(*cursor..).unwrap_or_default();
    match writ_module::signature::decode_type_signature(bytes) {
        Ok(signature) => {
            *cursor = blob.len();
            type_signature_to_ty(&signature, lib_type_token_map, lib_type_name_map, interner)
        }
        Err(_) => interner.error(),
    }
}

fn decode_impl_type_token(
    token: writ_module::MetadataToken,
    module: &Module,
    lib_type_token_map: &FxHashMap<u32, LibraryType>,
    lib_type_name_map: &FxHashMap<String, LibraryType>,
    interner: &mut TyInterner,
) -> Option<Ty> {
    match token.table_id() {
        2 | 3 | 10 => lib_type_token_map
            .get(&token.0)
            .copied()
            .map(|named| nominal_ty(named, interner)),
        4 => {
            let index = token.row_index()?.checked_sub(1)? as usize;
            let type_spec = module.type_specs.get(index)?;
            let blob = writ_module::heap::read_blob(&module.blob_heap, type_spec.signature).ok()?;
            let signature = writ_module::signature::decode_type_signature(blob).ok()?;
            Some(type_signature_to_ty(
                &signature,
                lib_type_token_map,
                lib_type_name_map,
                interner,
            ))
        }
        _ => None,
    }
}

fn impl_generic_param_count(
    module: &Module,
    tokens: impl IntoIterator<Item = writ_module::MetadataToken>,
    owned_method_indices: &[usize],
) -> u32 {
    let descriptor_count = tokens
        .into_iter()
        .filter(|token| token.table_id() == 4)
        .filter_map(|token| {
            let index = token.row_index()?.checked_sub(1)? as usize;
            let type_spec = module.type_specs.get(index)?;
            let blob = writ_module::heap::read_blob(&module.blob_heap, type_spec.signature).ok()?;
            writ_module::signature::decode_type_signature(blob).ok()
        })
        .filter_map(|signature| max_generic_param_ordinal(&signature))
        .max()
        .map_or(0, |ordinal| u32::from(ordinal) + 1);

    // Method-level generic ordinals are emitted after the impl-generic
    // prefix. This preserves the prefix even when an impl parameter does not
    // occur in either the target or contract descriptor.
    let method_prefix = module
        .generic_params
        .iter()
        .filter(|param| param.owner_kind == 1)
        .filter_map(|param| {
            let method_idx = param.owner.row_index()?.checked_sub(1)? as usize;
            owned_method_indices
                .contains(&method_idx)
                .then_some(u32::from(param.ordinal))
        })
        .min()
        .unwrap_or(0);

    descriptor_count.max(method_prefix)
}

fn max_generic_param_ordinal(signature: &TypeSignature) -> Option<u16> {
    match signature {
        TypeSignature::GenericParam(ordinal) => Some(*ordinal),
        TypeSignature::Generic { args, .. } => args
            .iter()
            .filter_map(max_generic_param_ordinal)
            .max(),
        TypeSignature::Array(element) => max_generic_param_ordinal(element),
        TypeSignature::Function { params, ret } => params
            .iter()
            .chain(std::iter::once(ret.as_ref()))
            .filter_map(max_generic_param_ordinal)
            .max(),
        _ => None,
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
    lib_type_token_map: &FxHashMap<u32, LibraryType>,
    lib_type_name_map: &FxHashMap<String, LibraryType>,
    interner: &mut TyInterner,
) -> (Vec<Ty>, Ty) {
    match writ_module::signature::decode_method_signature(blob) {
        Ok((params, ret)) => {
            let params = params
                .iter()
                .map(|param| {
                    type_signature_to_ty(param, lib_type_token_map, lib_type_name_map, interner)
                })
                .collect();
            let ret = type_signature_to_ty(&ret, lib_type_token_map, lib_type_name_map, interner);
            (params, ret)
        }
        Err(_) => (Vec::new(), interner.error()),
    }
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
    lib_type_token_map: &FxHashMap<u32, LibraryType>,
    lib_type_name_map: &FxHashMap<String, LibraryType>,
    interner: &mut TyInterner,
    lib_file_id: FileId,
    method_generics: Vec<String>,
) -> FnSig {
    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };

    // Decode the method signature blob
    let (param_tys, ret_ty) = match writ_module::heap::read_blob(&module.blob_heap, method.signature) {
        Ok(blob) => decode_method_sig(blob, lib_type_token_map, lib_type_name_map, interner),
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

        // Map full TypeDef/TypeRef tokens and namespace-qualified constructor
        // names back to the DefIds injected during resolution.
        let mut lib_type_token_map: FxHashMap<u32, LibraryType> = FxHashMap::default();
        let mut lib_type_name_map: FxHashMap<String, LibraryType> = FxHashMap::default();
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
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };

            if let Some(def_id) = def_map.get(&fqn) {
                let kind = TypeDefKind::from_u8(type_def.kind)
                    .unwrap_or(TypeDefKind::Struct);
                let named = LibraryType::from_type_def(def_id, kind);
                let token = writ_module::MetadataToken::new(2, row_1based);
                lib_type_token_map.insert(token.0, named);
                lib_type_name_map.insert(fqn, named);
                lib_type_name_map.entry(name).or_insert(named);
            }
        }

        for (type_ref_idx, type_ref) in module.type_refs.iter().enumerate() {
            let name = writ_module::heap::read_string(&module.string_heap, type_ref.name)
                .unwrap_or("")
                .to_string();
            let namespace = writ_module::heap::read_string(&module.string_heap, type_ref.namespace)
                .unwrap_or("")
                .to_string();
            let fqn = if namespace.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };
            let Some(def_id) = def_map.get(&fqn) else {
                continue;
            };
            let Some(named) = library_type_for_def(def_id, def_map) else {
                continue;
            };
            let token = writ_module::MetadataToken::new(3, (type_ref_idx + 1) as u32);
            lib_type_token_map.insert(token.0, named);
            lib_type_name_map.entry(fqn).or_insert(named);
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
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };

            if let Some(def_id) = def_map.get(&fqn) {
                lib_contract_def_id_map.insert(row_1based, def_id);
                let named = LibraryType {
                    def_id,
                    kind: LibraryTypeKind::Contract,
                };
                let token = writ_module::MetadataToken::new(10, row_1based);
                lib_type_token_map.insert(token.0, named);
                lib_type_name_map.insert(fqn, named);
                lib_type_name_map.entry(name).or_insert(named);
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

            let type_token = writ_module::MetadataToken::new(2, row_1based);
            let def_id = match lib_type_token_map.get(&type_token.0) {
                Some(named) => named.def_id,
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
                        decode_type_from_blob(
                            blob,
                            &mut cursor,
                            &lib_type_token_map,
                            &lib_type_name_map,
                            interner,
                        )
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
            let target_ty = match decode_impl_type_token(
                impl_def.type_token,
                module,
                &lib_type_token_map,
                &lib_type_name_map,
                interner,
            ) {
                Some(ty) => ty,
                None => continue,
            };
            let type_def_id = match interner.kind(target_ty) {
                TyKind::Struct(def_id)
                | TyKind::Class(def_id)
                | TyKind::Entity(def_id)
                | TyKind::Enum(def_id) => *def_id,
                _ => continue,
            };

            let contract_ty = (!impl_def.contract.is_null()).then(|| {
                decode_impl_type_token(
                    impl_def.contract,
                    module,
                    &lib_type_token_map,
                    &lib_type_name_map,
                    interner,
                )
            }).flatten();
            let contract_def_id = contract_ty.and_then(|ty| match interner.kind(ty) {
                TyKind::Contract(def_id) => Some(*def_id),
                _ => None,
            });

            let owned_methods = module.impl_method_indices(impl_idx);
            if owned_methods.is_empty() {
                continue;
            }
            let impl_generic_count = impl_generic_param_count(
                module,
                [impl_def.type_token, impl_def.contract],
                &owned_methods,
            );

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
            for method_idx in owned_methods.iter().copied() {
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
                    &lib_type_token_map,
                    &lib_type_name_map,
                    interner,
                    lib_file_id,
                    generics,
                );
                methods.push((method_name, sig));
            }

            let impl_entry = ImplEntry {
                impl_def_id: impl_entry_def_id,
                impl_generic_count,
                target_ty,
                contract_def_id,
                contract_ty,
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
        for method_idx in module.top_level_method_indices() {
            let method = &module.method_defs[method_idx];

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
                &lib_type_token_map,
                &lib_type_name_map,
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
                    Ok(blob) => decode_method_sig(
                        blob,
                        &lib_type_token_map,
                        &lib_type_name_map,
                        interner,
                    ),
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
