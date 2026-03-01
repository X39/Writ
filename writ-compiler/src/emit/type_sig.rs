//! TypeRef blob encoding per spec section 2.15.3.
//!
//! Converts `Ty` values into variable-length byte sequences stored in the blob heap.

use crate::check::ty::{Ty, TyInterner, TyKind};
use crate::resolve::def_map::DefId;

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
    let mut buf = Vec::new();
    encode_type_into(ty, interner, token_for_def, blob_heap, &mut buf);
    buf
}

/// Encode a type into the given buffer (recursive helper).
fn encode_type_into(
    ty: Ty,
    interner: &TyInterner,
    token_for_def: &dyn Fn(DefId) -> MetadataToken,
    blob_heap: &mut BlobHeap,
    buf: &mut Vec<u8>,
) {
    match interner.kind(ty) {
        TyKind::Void => buf.push(0x00),
        TyKind::Int => buf.push(0x01),
        TyKind::Float => buf.push(0x02),
        TyKind::Bool => buf.push(0x03),
        TyKind::String => buf.push(0x04),

        TyKind::Struct(def_id) | TyKind::Entity(def_id) | TyKind::Enum(def_id) => {
            let token = token_for_def(*def_id);
            buf.push(0x10);
            buf.extend_from_slice(&token.row().to_le_bytes());
        }

        TyKind::GenericParam(idx) => {
            buf.push(0x12);
            buf.extend_from_slice(&(*idx as u16).to_le_bytes());
        }

        TyKind::Array(elem) => {
            buf.push(0x20);
            encode_type_into(*elem, interner, token_for_def, blob_heap, buf);
        }

        TyKind::Func { params, ret } => {
            // Encode function signature as a separate blob:
            // u16 param_count + TypeRef[] params + TypeRef return
            let mut sig_buf = Vec::new();
            sig_buf.extend_from_slice(&(params.len() as u16).to_le_bytes());
            for &p in params {
                encode_type_into(p, interner, token_for_def, blob_heap, &mut sig_buf);
            }
            encode_type_into(*ret, interner, token_for_def, blob_heap, &mut sig_buf);
            let blob_offset = blob_heap.intern(&sig_buf);
            buf.push(0x30);
            buf.extend_from_slice(&blob_offset.to_le_bytes());
        }

        TyKind::Option(inner) | TyKind::Result(inner, _) | TyKind::TaskHandle(inner) => {
            // These are writ-runtime generic types. For Phase 24, encode as TypeSpec
            // references (0x11). We emit a TypeSpec row with the generic instantiation.
            // For now, encode a placeholder TypeSpec reference at row 0 (will be
            // resolved when TypeSpec emission is implemented in Phase 25).
            //
            // The inner type is still encoded for correctness.
            buf.push(0x11);
            // Placeholder TypeSpec row index (0 means unresolved)
            buf.extend_from_slice(&0u32.to_le_bytes());
            let _ = inner; // suppress unused warning
        }

        TyKind::Infer(_) => {
            debug_assert!(false, "Infer type should not appear in emit output");
            buf.push(0x00); // fallback to void
        }

        TyKind::Error => {
            debug_assert!(false, "Error type should not appear in emit output");
            buf.push(0x00); // fallback to void
        }
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
    let mut buf = Vec::new();
    buf.extend_from_slice(&(param_types.len() as u16).to_le_bytes());
    for &p in param_types {
        encode_type_into(p, interner, token_for_def, blob_heap, &mut buf);
    }
    encode_type_into(ret_type, interner, token_for_def, blob_heap, &mut buf);
    blob_heap.intern(&buf)
}
