//! Safe TypeSpec decoding and canonical runtime identities.

use rustc_hash::FxHashMap;
use writ_module::MetadataToken;
use writ_module::signature::TypeSignature;

use crate::loader::LoadedModule;

const MAX_RESOLUTION_DEPTH: usize = 64;
const INVALID_KEY: u32 = u32::MAX;
type NominalKey = (usize, usize);

#[derive(Clone, Copy)]
pub(crate) enum NominalSpace {
    Type,
    Contract,
}

pub(crate) fn resolve_type_key(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> u32 {
    resolve_type_location(module_idx, token, modules)
        .map(|(owner, row)| ((owner as u32) << 16) | row as u32)
        .unwrap_or(INVALID_KEY)
}

pub(crate) fn resolve_contract_key(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> u32 {
    resolve_contract_location(module_idx, token, modules)
        .map(|(owner, row)| ((owner as u32) << 16) | row as u32)
        .unwrap_or(INVALID_KEY)
}

pub(crate) fn type_name(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> String {
    let Some((owner, row)) = resolve_type_location(module_idx, token, modules) else {
        return String::new();
    };
    let Some(module) = modules.get(owner).map(|loaded| &loaded.module) else {
        return String::new();
    };
    let Some(type_def) = module.type_defs.get(row) else {
        return String::new();
    };
    writ_module::heap::read_string(&module.string_heap, type_def.name)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn contract_method_count(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> usize {
    let Some((owner, row)) = resolve_contract_location(module_idx, token, modules) else {
        return 0;
    };
    let Some(module) = modules.get(owner).map(|loaded| &loaded.module) else {
        return 0;
    };
    let Some(contract) = module.contract_defs.get(row) else {
        return 0;
    };
    let start = contract.method_list.saturating_sub(1) as usize;
    let end = module
        .contract_defs
        .get(row + 1)
        .map(|next| next.method_list.saturating_sub(1) as usize)
        .unwrap_or(module.contract_methods.len());
    end.saturating_sub(start)
        .min(module.contract_methods.len().saturating_sub(start))
}

/// Return a stable legacy discriminator for a TypeSpec shape.
///
/// Structural dispatch does not depend on this hash. It remains available for
/// compatibility lookups and hashes open patterns (including GenericParam
/// ordinals) rather than collapsing every pattern to zero. Named constructors
/// must resolve to canonical global identities; unresolved descriptors fail
/// closed with zero.
pub(crate) fn specialization_hash(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> u32 {
    specialization_hash_for(module_idx, token, modules, NominalSpace::Contract)
}

pub(crate) fn specialization_hash_for(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
    space: NominalSpace,
) -> u32 {
    if token.table_id() != 4 {
        return 0;
    }
    let Some(signature) = type_spec_signature(module_idx, token, modules) else {
        return 0;
    };
    let mut hash = 0x811c_9dc5;
    if !hash_signature(&signature, module_idx, modules, space, &mut hash) {
        return 0;
    }
    if hash == 0 { 1 } else { hash }
}

pub(crate) fn type_spec_signature(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> Option<TypeSignature> {
    if token.table_id() != 4 {
        return None;
    }
    let module = &modules.get(module_idx)?.module;
    let row = token.row_index()?.checked_sub(1)? as usize;
    let type_spec = module.type_specs.get(row)?;
    let blob = writ_module::heap::read_blob(&module.blob_heap, type_spec.signature).ok()?;
    writ_module::signature::decode_type_signature(blob).ok()
}

pub(crate) fn resolve_type_location(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> Option<(usize, usize)> {
    resolve_type_location_inner(module_idx, token, modules, 0)
}

fn resolve_type_location_inner(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
    depth: usize,
) -> Option<(usize, usize)> {
    if depth >= MAX_RESOLUTION_DEPTH {
        return None;
    }
    let loaded = modules.get(module_idx)?;
    let row = token.row_index()?.checked_sub(1)? as usize;
    match token.table_id() {
        2 => loaded.module.type_defs.get(row).map(|_| (module_idx, row)),
        3 => loaded
            .resolved_refs
            .types
            .get(&(row as u32))
            .map(|resolved| (resolved.module_idx, resolved.typedef_idx)),
        4 => match type_spec_signature(module_idx, token, modules)? {
            TypeSignature::Named(named) => {
                resolve_type_location_inner(module_idx, named, modules, depth + 1)
            }
            TypeSignature::Generic {
                namespace, name, ..
            } => find_type_location(module_idx, &namespace, &name, modules),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn resolve_contract_location(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
) -> Option<(usize, usize)> {
    resolve_contract_location_inner(module_idx, token, modules, 0)
}

fn resolve_contract_location_inner(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
    depth: usize,
) -> Option<(usize, usize)> {
    if depth >= MAX_RESOLUTION_DEPTH {
        return None;
    }
    let loaded = modules.get(module_idx)?;
    let row = token.row_index()?.checked_sub(1)? as usize;
    match token.table_id() {
        10 => loaded
            .module
            .contract_defs
            .get(row)
            .map(|_| (module_idx, row)),
        3 => loaded
            .resolved_refs
            .contracts
            .get(&(row as u32))
            .map(|resolved| (resolved.module_idx, resolved.contractdef_idx)),
        4 => match type_spec_signature(module_idx, token, modules)? {
            TypeSignature::Named(named) => {
                resolve_contract_location_inner(module_idx, named, modules, depth + 1)
            }
            TypeSignature::Generic {
                namespace, name, ..
            } => find_contract_location(module_idx, &namespace, &name, modules),
            _ => None,
        },
        _ => None,
    }
}

fn find_type_location(
    module_idx: usize,
    namespace: &str,
    name: &str,
    modules: &[LoadedModule],
) -> Option<(usize, usize)> {
    let loaded = modules.get(module_idx)?;
    if let Some(row) = find_local_type(&loaded.module, namespace, name) {
        return Some((module_idx, row));
    }
    for (row, type_ref) in loaded.module.type_refs.iter().enumerate() {
        if token_name_matches(
            &loaded.module,
            type_ref.name,
            type_ref.namespace,
            namespace,
            name,
        ) {
            if let Some(resolved) = loaded.resolved_refs.types.get(&(row as u32)) {
                return Some((resolved.module_idx, resolved.typedef_idx));
            }
        }
    }
    None
}

fn find_contract_location(
    module_idx: usize,
    namespace: &str,
    name: &str,
    modules: &[LoadedModule],
) -> Option<(usize, usize)> {
    let loaded = modules.get(module_idx)?;
    if let Some(row) = find_local_contract(&loaded.module, namespace, name) {
        return Some((module_idx, row));
    }
    for (row, type_ref) in loaded.module.type_refs.iter().enumerate() {
        if token_name_matches(
            &loaded.module,
            type_ref.name,
            type_ref.namespace,
            namespace,
            name,
        ) {
            if let Some(resolved) = loaded.resolved_refs.contracts.get(&(row as u32)) {
                return Some((resolved.module_idx, resolved.contractdef_idx));
            }
        }
    }
    None
}

fn find_local_type(module: &writ_module::Module, namespace: &str, name: &str) -> Option<usize> {
    module.type_defs.iter().position(|type_def| {
        token_name_matches(module, type_def.name, type_def.namespace, namespace, name)
    })
}

fn find_local_contract(module: &writ_module::Module, namespace: &str, name: &str) -> Option<usize> {
    module.contract_defs.iter().position(|contract| {
        token_name_matches(module, contract.name, contract.namespace, namespace, name)
    })
}

fn token_name_matches(
    module: &writ_module::Module,
    name_offset: u32,
    namespace_offset: u32,
    namespace: &str,
    name: &str,
) -> bool {
    writ_module::heap::read_string(&module.string_heap, name_offset).ok() == Some(name)
        && writ_module::heap::read_string(&module.string_heap, namespace_offset).ok()
            == Some(namespace)
}

fn hash_signature(
    signature: &TypeSignature,
    module_idx: usize,
    modules: &[LoadedModule],
    space: NominalSpace,
    hash: &mut u32,
) -> bool {
    match signature {
        TypeSignature::Void => hash_byte(hash, 0x00),
        TypeSignature::Int => hash_byte(hash, 0x01),
        TypeSignature::Float => hash_byte(hash, 0x02),
        TypeSignature::Bool => hash_byte(hash, 0x03),
        TypeSignature::String => hash_byte(hash, 0x04),
        TypeSignature::Entity => hash_byte(hash, 0x05),
        TypeSignature::Named(token) => {
            hash_byte(hash, 0x10);
            let Some(key) = nominal_token_key(module_idx, *token, modules, space) else {
                return false;
            };
            hash_nominal_key(hash, key);
        }
        TypeSignature::Generic {
            namespace,
            name,
            args,
        } => {
            hash_byte(hash, 0x11);
            let Some(key) = constructor_key(module_idx, namespace, name, modules, space) else {
                return false;
            };
            hash_nominal_key(hash, key);
            hash_bytes(hash, &(args.len() as u32).to_le_bytes());
            for arg in args {
                if !hash_signature(arg, module_idx, modules, NominalSpace::Type, hash) {
                    return false;
                }
            }
        }
        TypeSignature::GenericParam(ordinal) => {
            hash_byte(hash, 0x12);
            hash_bytes(hash, &ordinal.to_le_bytes());
        }
        TypeSignature::Array(element) => {
            hash_byte(hash, 0x20);
            if !hash_signature(element, module_idx, modules, NominalSpace::Type, hash) {
                return false;
            }
        }
        TypeSignature::Function { params, ret } => {
            hash_byte(hash, 0x30);
            hash_bytes(hash, &(params.len() as u32).to_le_bytes());
            for param in params {
                if !hash_signature(param, module_idx, modules, NominalSpace::Type, hash) {
                    return false;
                }
            }
            if !hash_signature(ret, module_idx, modules, NominalSpace::Type, hash) {
                return false;
            }
        }
    }
    true
}

fn constructor_key(
    module_idx: usize,
    namespace: &str,
    name: &str,
    modules: &[LoadedModule],
    space: NominalSpace,
) -> Option<NominalKey> {
    match space {
        NominalSpace::Type => find_type_location(module_idx, namespace, name, modules),
        NominalSpace::Contract => find_contract_location(module_idx, namespace, name, modules),
    }
}

fn hash_nominal_key(hash: &mut u32, (module_idx, row): NominalKey) {
    hash_bytes(hash, &(module_idx as u64).to_le_bytes());
    hash_bytes(hash, &(row as u64).to_le_bytes());
}

/// Match an ImplDef's target and contract TypeSpec patterns against the
/// concrete receiver allocation and CALL_VIRT contract specialization.
/// GenericParam bindings are shared across both descriptors.
pub(crate) fn matches_impl_specialization(
    pattern_target: Option<(usize, &TypeSignature)>,
    actual_target: Option<(usize, &TypeSignature)>,
    pattern_contract: Option<(usize, &TypeSignature)>,
    actual_contract: Option<(usize, &TypeSignature)>,
    modules: &[LoadedModule],
) -> bool {
    let mut bindings = FxHashMap::default();
    optional_pattern_matches(
        pattern_target,
        actual_target,
        NominalSpace::Type,
        false,
        modules,
        &mut bindings,
    ) && optional_pattern_matches(
        pattern_contract,
        actual_contract,
        NominalSpace::Contract,
        true,
        modules,
        &mut bindings,
    )
}

fn optional_pattern_matches(
    pattern: Option<(usize, &TypeSignature)>,
    actual: Option<(usize, &TypeSignature)>,
    space: NominalSpace,
    allow_erased_actual: bool,
    modules: &[LoadedModule],
    bindings: &mut FxHashMap<u16, (usize, TypeSignature)>,
) -> bool {
    match (pattern, actual) {
        (None, None) => true,
        (Some((pattern_module, pattern)), Some((actual_module, actual))) => match_signature(
            pattern,
            pattern_module,
            actual,
            actual_module,
            space,
            modules,
            bindings,
        ),
        (Some((pattern_module, pattern)), None) if allow_erased_actual => {
            signature_is_resolvable(pattern, pattern_module, space, modules)
        }
        _ => false,
    }
}

fn signature_is_resolvable(
    signature: &TypeSignature,
    module_idx: usize,
    space: NominalSpace,
    modules: &[LoadedModule],
) -> bool {
    match signature {
        TypeSignature::Void
        | TypeSignature::Int
        | TypeSignature::Float
        | TypeSignature::Bool
        | TypeSignature::String
        | TypeSignature::Entity
        | TypeSignature::GenericParam(_) => true,
        TypeSignature::Named(token) => {
            nominal_token_key(module_idx, *token, modules, space).is_some()
        }
        TypeSignature::Generic {
            namespace,
            name,
            args,
        } => {
            constructor_key(module_idx, namespace, name, modules, space).is_some()
                && args.iter().all(|argument| {
                    signature_is_resolvable(argument, module_idx, NominalSpace::Type, modules)
                })
        }
        TypeSignature::Array(element) => {
            signature_is_resolvable(element, module_idx, NominalSpace::Type, modules)
        }
        TypeSignature::Function { params, ret } => {
            params.iter().all(|parameter| {
                signature_is_resolvable(parameter, module_idx, NominalSpace::Type, modules)
            }) && signature_is_resolvable(ret, module_idx, NominalSpace::Type, modules)
        }
    }
}

fn match_signature(
    pattern: &TypeSignature,
    pattern_module: usize,
    actual: &TypeSignature,
    actual_module: usize,
    space: NominalSpace,
    modules: &[LoadedModule],
    bindings: &mut FxHashMap<u16, (usize, TypeSignature)>,
) -> bool {
    if let TypeSignature::GenericParam(ordinal) = pattern {
        return match bindings.get(ordinal) {
            Some((bound_module, bound)) => signatures_equal(
                bound,
                *bound_module,
                actual,
                actual_module,
                NominalSpace::Type,
                modules,
            ),
            None => {
                bindings.insert(*ordinal, (actual_module, actual.clone()));
                true
            }
        };
    }

    match (pattern, actual) {
        (TypeSignature::Void, TypeSignature::Void)
        | (TypeSignature::Int, TypeSignature::Int)
        | (TypeSignature::Float, TypeSignature::Float)
        | (TypeSignature::Bool, TypeSignature::Bool)
        | (TypeSignature::String, TypeSignature::String)
        | (TypeSignature::Entity, TypeSignature::Entity) => true,
        (TypeSignature::Named(left), TypeSignature::Named(right)) => {
            nominal_token_key(pattern_module, *left, modules, space)
                .zip(nominal_token_key(actual_module, *right, modules, space))
                .is_some_and(|(left, right)| left == right)
        }
        (
            TypeSignature::Generic {
                namespace: left_namespace,
                name: left_name,
                args: left_args,
            },
            TypeSignature::Generic {
                namespace: right_namespace,
                name: right_name,
                args: right_args,
            },
        ) => {
            if !constructors_equal(
                pattern_module,
                left_namespace,
                left_name,
                actual_module,
                right_namespace,
                right_name,
                modules,
                space,
            ) || left_args.len() != right_args.len()
            {
                return false;
            }
            left_args.iter().zip(right_args).all(|(left, right)| {
                match_signature(
                    left,
                    pattern_module,
                    right,
                    actual_module,
                    NominalSpace::Type,
                    modules,
                    bindings,
                )
            })
        }
        (TypeSignature::Array(left), TypeSignature::Array(right)) => match_signature(
            left,
            pattern_module,
            right,
            actual_module,
            NominalSpace::Type,
            modules,
            bindings,
        ),
        (
            TypeSignature::Function {
                params: left_params,
                ret: left_ret,
            },
            TypeSignature::Function {
                params: right_params,
                ret: right_ret,
            },
        ) => {
            left_params.len() == right_params.len()
                && left_params.iter().zip(right_params).all(|(left, right)| {
                    match_signature(
                        left,
                        pattern_module,
                        right,
                        actual_module,
                        NominalSpace::Type,
                        modules,
                        bindings,
                    )
                })
                && match_signature(
                    left_ret,
                    pattern_module,
                    right_ret,
                    actual_module,
                    NominalSpace::Type,
                    modules,
                    bindings,
                )
        }
        _ => false,
    }
}

fn signatures_equal(
    left: &TypeSignature,
    left_module: usize,
    right: &TypeSignature,
    right_module: usize,
    space: NominalSpace,
    modules: &[LoadedModule],
) -> bool {
    match (left, right) {
        (TypeSignature::GenericParam(left), TypeSignature::GenericParam(right)) => {
            // GenericParam identity is scoped to the descriptor that contains
            // it. Imported descriptors may be copied into another module, so
            // module identity is deliberately irrelevant here.
            left == right
        }
        (TypeSignature::Void, TypeSignature::Void)
        | (TypeSignature::Int, TypeSignature::Int)
        | (TypeSignature::Float, TypeSignature::Float)
        | (TypeSignature::Bool, TypeSignature::Bool)
        | (TypeSignature::String, TypeSignature::String)
        | (TypeSignature::Entity, TypeSignature::Entity) => true,
        (TypeSignature::Named(left), TypeSignature::Named(right)) => {
            nominal_token_key(left_module, *left, modules, space)
                .zip(nominal_token_key(right_module, *right, modules, space))
                .is_some_and(|(left, right)| left == right)
        }
        (
            TypeSignature::Generic {
                namespace: left_namespace,
                name: left_name,
                args: left_args,
            },
            TypeSignature::Generic {
                namespace: right_namespace,
                name: right_name,
                args: right_args,
            },
        ) => {
            constructors_equal(
                left_module,
                left_namespace,
                left_name,
                right_module,
                right_namespace,
                right_name,
                modules,
                space,
            ) && left_args.len() == right_args.len()
                && left_args.iter().zip(right_args).all(|(left, right)| {
                    signatures_equal(
                        left,
                        left_module,
                        right,
                        right_module,
                        NominalSpace::Type,
                        modules,
                    )
                })
        }
        (TypeSignature::Array(left), TypeSignature::Array(right)) => signatures_equal(
            left,
            left_module,
            right,
            right_module,
            NominalSpace::Type,
            modules,
        ),
        (
            TypeSignature::Function {
                params: left_params,
                ret: left_ret,
            },
            TypeSignature::Function {
                params: right_params,
                ret: right_ret,
            },
        ) => {
            left_params.len() == right_params.len()
                && left_params.iter().zip(right_params).all(|(left, right)| {
                    signatures_equal(
                        left,
                        left_module,
                        right,
                        right_module,
                        NominalSpace::Type,
                        modules,
                    )
                })
                && signatures_equal(
                    left_ret,
                    left_module,
                    right_ret,
                    right_module,
                    NominalSpace::Type,
                    modules,
                )
        }
        _ => false,
    }
}

/// Compare two decoded descriptors using canonical identities across modules.
#[allow(dead_code)] // shared by direct-call specialization resolution
pub(crate) fn type_signatures_equal(
    left: &TypeSignature,
    left_module: usize,
    right: &TypeSignature,
    right_module: usize,
    space: NominalSpace,
    modules: &[LoadedModule],
) -> bool {
    signatures_equal(left, left_module, right, right_module, space, modules)
}

fn nominal_token_key(
    module_idx: usize,
    token: MetadataToken,
    modules: &[LoadedModule],
    space: NominalSpace,
) -> Option<NominalKey> {
    match space {
        NominalSpace::Type => resolve_type_location(module_idx, token, modules),
        NominalSpace::Contract => resolve_contract_location(module_idx, token, modules),
    }
}

#[allow(clippy::too_many_arguments)]
fn constructors_equal(
    left_module: usize,
    left_namespace: &str,
    left_name: &str,
    right_module: usize,
    right_namespace: &str,
    right_name: &str,
    modules: &[LoadedModule],
    space: NominalSpace,
) -> bool {
    match (
        constructor_key(left_module, left_namespace, left_name, modules, space),
        constructor_key(right_module, right_namespace, right_name, modules, space),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn hash_bytes(hash: &mut u32, bytes: &[u8]) {
    for byte in bytes {
        hash_byte(hash, *byte);
    }
}

fn hash_byte(hash: &mut u32, byte: u8) {
    *hash ^= u32::from(byte);
    *hash = hash.wrapping_mul(0x0100_0193);
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::ModuleBuilder;
    use writ_module::signature::{TypeSignature, encode_type_signature};

    #[test]
    fn malformed_and_out_of_range_type_specs_fail_closed() {
        let mut builder = ModuleBuilder::new("bad-typespec");
        let malformed = builder.add_type_spec(&[0xff]);
        let loaded = LoadedModule::from_module(builder.build()).unwrap();
        let modules = vec![loaded];

        for token in [malformed, MetadataToken::new(4, 999)] {
            assert_eq!(resolve_type_key(0, token, &modules), u32::MAX);
            assert_eq!(resolve_contract_key(0, token, &modules), u32::MAX);
            assert_eq!(type_name(0, token, &modules), "");
            assert_eq!(contract_method_count(0, token, &modules), 0);
            assert_eq!(specialization_hash(0, token, &modules), 0);
        }
    }

    #[test]
    fn open_generic_shapes_have_nonzero_distinct_legacy_hashes() {
        let mut builder = ModuleBuilder::new("open-typespec-hashes");
        builder.add_contract_def("Carries", "test");
        let direct = encode_type_signature(&TypeSignature::Generic {
            namespace: "test".to_string(),
            name: "Carries".to_string(),
            args: vec![TypeSignature::GenericParam(0)],
        })
        .unwrap();
        let array = encode_type_signature(&TypeSignature::Generic {
            namespace: "test".to_string(),
            name: "Carries".to_string(),
            args: vec![TypeSignature::Array(Box::new(TypeSignature::GenericParam(
                0,
            )))],
        })
        .unwrap();
        let direct = builder.add_type_spec(&direct);
        let array = builder.add_type_spec(&array);
        let modules = vec![LoadedModule::from_module(builder.build()).unwrap()];

        let direct_hash = specialization_hash(0, direct, &modules);
        let array_hash = specialization_hash(0, array, &modules);
        assert_ne!(direct_hash, 0);
        assert_ne!(array_hash, 0);
        assert_ne!(direct_hash, array_hash);
    }

    #[test]
    fn generic_constructor_does_not_resolve_through_unreferenced_sibling_module() {
        let mut consumer = ModuleBuilder::new("consumer");
        let signature = encode_type_signature(&TypeSignature::Generic {
            namespace: "sibling".to_string(),
            name: "Crate".to_string(),
            args: vec![TypeSignature::Int],
        })
        .unwrap();
        let type_spec = consumer.add_type_spec(&signature);

        let mut sibling = ModuleBuilder::new("sibling");
        sibling.add_type_def(
            "Crate",
            "sibling",
            writ_module::tables::TypeDefKind::Class,
            0,
        );

        let modules = vec![
            LoadedModule::from_module(consumer.build()).unwrap(),
            LoadedModule::from_module(sibling.build()).unwrap(),
        ];

        assert_eq!(
            specialization_hash_for(0, type_spec, &modules, NominalSpace::Type),
            0,
            "a TypeSpec must not bypass ModuleRef/TypeRef dependency boundaries"
        );
    }

    #[test]
    fn canonical_nominal_hash_does_not_truncate_metadata_rows() {
        let mut high_row = 0x811c_9dc5;
        hash_nominal_key(&mut high_row, (0, 1 << 16));
        let mut next_module = 0x811c_9dc5;
        hash_nominal_key(&mut next_module, (1, 0));

        assert_ne!(high_row, next_module);
    }

    #[test]
    fn copied_open_descriptors_are_alpha_equivalent_across_modules() {
        assert!(type_signatures_equal(
            &TypeSignature::GenericParam(3),
            1,
            &TypeSignature::GenericParam(3),
            9,
            NominalSpace::Type,
            &[],
        ));
    }
}
