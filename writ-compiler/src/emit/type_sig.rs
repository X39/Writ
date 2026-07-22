//! TypeRef blob encoding per spec section 2.15.3.
//!
//! Converts `Ty` values into variable-length byte sequences stored in the blob heap.

use crate::check::ty::{Ty, TyInterner, TyKind};
use crate::resolve::def_map::DefId;
use writ_module::signature::{TypeSignature, encode_method_signature, encode_type_signature};

use super::heaps::BlobHeap;
use super::metadata::MetadataToken;

/// Encode a `Ty` into a TypeRef blob per spec 2.15.3.
///
/// The `token_for_def` closure resolves a DefId to its MetadataToken.
/// Returns the encoded bytes.
pub fn encode_type(
    ty: Ty,
    interner: &TyInterner,
    token_for_def: &dyn Fn(DefId) -> MetadataToken,
    blob_heap: &mut BlobHeap,
) -> Vec<u8> {
    let _ = blob_heap;
    encode_type_bytes(ty, interner, token_for_def)
}

/// Encode a `Ty` without borrowing a blob heap.
///
/// Collection uses this form when it must create a TypeSpec row before body
/// serialization. The caller interns the returned descriptor in its own heap.
pub fn encode_type_bytes(
    ty: Ty,
    interner: &TyInterner,
    token_for_def: &dyn Fn(DefId) -> MetadataToken,
) -> Vec<u8> {
    encode_type_signature(&type_signature_for_ty(ty, interner, token_for_def))
        .expect("compiler type signature exceeds module format limits")
}

/// Convert a compiler type into the format's recursive type descriptor.
fn type_signature_for_ty(
    ty: Ty,
    interner: &TyInterner,
    token_for_def: &dyn Fn(DefId) -> MetadataToken,
) -> TypeSignature {
    let ty = interner.resolve_infer(ty);
    match interner.full_kind(ty) {
        TyKind::Void => TypeSignature::Void,
        TyKind::Int => TypeSignature::Int,
        TyKind::Float => TypeSignature::Float,
        TyKind::Bool => TypeSignature::Bool,
        TyKind::String => TypeSignature::String,

        TyKind::AnyEntity => TypeSignature::Entity,

        TyKind::Struct(def_id)
        | TyKind::Class(def_id)
        | TyKind::Entity(def_id)
        | TyKind::Enum(def_id)
        | TyKind::Contract(def_id) => {
            let token = token_for_def(*def_id);
            TypeSignature::Named(writ_module::MetadataToken(token.0))
        }

        TyKind::GenericInstance {
            namespace,
            name,
            args,
            ..
        } => TypeSignature::Generic {
            namespace: namespace.clone(),
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| type_signature_for_ty(*arg, interner, token_for_def))
                .collect(),
        },

        TyKind::GenericParam(idx) => TypeSignature::GenericParam(*idx as u16),

        TyKind::Array(elem) => {
            let element = type_signature_for_ty(*elem, interner, token_for_def);
            TypeSignature::Array(Box::new(element))
        }

        TyKind::Func { params, ret } => {
            let params = params
                .iter()
                .map(|&param| type_signature_for_ty(param, interner, token_for_def))
                .collect();
            let ret = Box::new(type_signature_for_ty(*ret, interner, token_for_def));
            TypeSignature::Function { params, ret }
        }

        TyKind::ReflectionType(inner) => TypeSignature::Generic {
            namespace: "writ".to_string(),
            name: "Type".to_string(),
            args: vec![type_signature_for_ty(*inner, interner, token_for_def)],
        },

        TyKind::Option(inner) => TypeSignature::Generic {
            namespace: "writ".to_string(),
            name: "Option".to_string(),
            args: vec![type_signature_for_ty(*inner, interner, token_for_def)],
        },

        TyKind::Result(ok, err) => TypeSignature::Generic {
            namespace: "writ".to_string(),
            name: "Result".to_string(),
            args: vec![
                type_signature_for_ty(*ok, interner, token_for_def),
                type_signature_for_ty(*err, interner, token_for_def),
            ],
        },

        TyKind::TaskHandle(inner) => TypeSignature::Generic {
            namespace: "writ".to_string(),
            name: "TaskHandle".to_string(),
            args: vec![type_signature_for_ty(*inner, interner, token_for_def)],
        },

        // A genuinely unbound inference variable or prior type error must not
        // panic serialization. Retained bindings were followed above.
        TyKind::Infer(_) | TyKind::Error => TypeSignature::Void,
    }
}

/// Encode a method signature blob for MethodDef/ContractMethod/ExternDef.
///
/// Format: u16 param_count + TypeRef[] params + TypeRef return_type.
pub fn encode_method_sig(
    param_types: &[Ty],
    ret_type: Ty,
    interner: &TyInterner,
    token_for_def: &dyn Fn(DefId) -> MetadataToken,
    blob_heap: &mut BlobHeap,
) -> u32 {
    let params: Vec<TypeSignature> = param_types
        .iter()
        .map(|&param| type_signature_for_ty(param, interner, token_for_def))
        .collect();
    let ret = type_signature_for_ty(ret_type, interner, token_for_def);
    let buf = encode_method_signature(&params, &ret)
        .expect("compiler method signature exceeds module format limits");
    blob_heap.intern(&buf)
}
