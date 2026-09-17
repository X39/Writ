//! Self-contained type and method signature encoding.
//!
//! Format version 7 stores every nested type inline. This keeps signatures
//! independently decodable when a module is consumed as a compilation
//! dependency; no blob-heap or TypeSpec side lookup is required.

use crate::error::{DecodeError, EncodeError};
use crate::token::MetadataToken;

const MAX_TYPE_DEPTH: usize = 64;

/// A type descriptor as represented in module metadata signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSignature {
    Void,
    Int,
    Float,
    Bool,
    String,
    Entity,
    Named(MetadataToken),
    Generic {
        /// Constructor namespace. Empty means the current module namespace.
        namespace: String,
        /// Constructor name, for example `Option` or `Result`.
        name: String,
        args: Vec<TypeSignature>,
    },
    GenericParam(u16),
    Array(Box<TypeSignature>),
    Function {
        params: Vec<TypeSignature>,
        ret: Box<TypeSignature>,
    },
}

/// Encode one complete TypeRef descriptor.
pub fn encode_type_signature(signature: &TypeSignature) -> Result<Vec<u8>, EncodeError> {
    let mut bytes = Vec::new();
    encode_type_into(signature, &mut bytes, 0)?;
    Ok(bytes)
}

/// Decode one complete TypeRef descriptor, rejecting trailing bytes.
pub fn decode_type_signature(bytes: &[u8]) -> Result<TypeSignature, DecodeError> {
    let (signature, cursor) = decode_type_signature_prefix(bytes)?;
    if cursor != bytes.len() {
        return Err(DecodeError::InvalidTypeSignature(
            "trailing bytes after type descriptor",
        ));
    }
    Ok(signature)
}

/// Decode the first TypeRef descriptor in `bytes` and return its byte length.
///
/// Method signatures concatenate several TypeRefs, so consumers that walk a
/// larger signature use this API while standalone blobs use
/// [`decode_type_signature`] for trailing-byte validation.
pub fn decode_type_signature_prefix(bytes: &[u8]) -> Result<(TypeSignature, usize), DecodeError> {
    let mut cursor = 0;
    let signature = decode_type_from(bytes, &mut cursor, 0)?;
    Ok((signature, cursor))
}

/// Encode a method signature as `param_count + params + return_type`.
pub fn encode_method_signature(
    params: &[TypeSignature],
    ret: &TypeSignature,
) -> Result<Vec<u8>, EncodeError> {
    let param_count = checked_u16(params.len(), "method parameter count")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&param_count.to_le_bytes());
    for param in params {
        encode_type_into(param, &mut bytes, 0)?;
    }
    encode_type_into(ret, &mut bytes, 0)?;
    Ok(bytes)
}

/// Decode a complete method signature, rejecting truncation and trailing bytes.
pub fn decode_method_signature(
    bytes: &[u8],
) -> Result<(Vec<TypeSignature>, TypeSignature), DecodeError> {
    let mut cursor = 0;
    let param_count = read_u16(bytes, &mut cursor)? as usize;
    let mut params = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        params.push(decode_type_from(bytes, &mut cursor, 0)?);
    }
    let ret = decode_type_from(bytes, &mut cursor, 0)?;
    if cursor != bytes.len() {
        return Err(DecodeError::InvalidTypeSignature(
            "trailing bytes after method signature",
        ));
    }
    Ok((params, ret))
}

fn encode_type_into(
    signature: &TypeSignature,
    bytes: &mut Vec<u8>,
    depth: usize,
) -> Result<(), EncodeError> {
    if depth >= MAX_TYPE_DEPTH {
        return Err(EncodeError::TypeSignatureTooDeep);
    }
    match signature {
        TypeSignature::Void => bytes.push(0x00),
        TypeSignature::Int => bytes.push(0x01),
        TypeSignature::Float => bytes.push(0x02),
        TypeSignature::Bool => bytes.push(0x03),
        TypeSignature::String => bytes.push(0x04),
        TypeSignature::Entity => bytes.push(0x05),
        TypeSignature::Named(token) => {
            bytes.push(0x10);
            bytes.extend_from_slice(&token.0.to_le_bytes());
        }
        TypeSignature::Generic {
            namespace,
            name,
            args,
        } => {
            bytes.push(0x11);
            let namespace_bytes = namespace.as_bytes();
            let namespace_len =
                checked_u16(namespace_bytes.len(), "generic constructor namespace")?;
            bytes.extend_from_slice(&namespace_len.to_le_bytes());
            bytes.extend_from_slice(namespace_bytes);
            let name_bytes = name.as_bytes();
            let name_len = checked_u16(name_bytes.len(), "generic constructor name")?;
            bytes.extend_from_slice(&name_len.to_le_bytes());
            bytes.extend_from_slice(name_bytes);
            let arg_count = checked_u16(args.len(), "generic argument count")?;
            bytes.extend_from_slice(&arg_count.to_le_bytes());
            for arg in args {
                encode_type_into(arg, bytes, depth + 1)?;
            }
        }
        TypeSignature::GenericParam(ordinal) => {
            bytes.push(0x12);
            bytes.extend_from_slice(&ordinal.to_le_bytes());
        }
        TypeSignature::Array(element) => {
            bytes.push(0x20);
            encode_type_into(element, bytes, depth + 1)?;
        }
        TypeSignature::Function { params, ret } => {
            bytes.push(0x30);
            let param_count = checked_u16(params.len(), "function parameter count")?;
            bytes.extend_from_slice(&param_count.to_le_bytes());
            for param in params {
                encode_type_into(param, bytes, depth + 1)?;
            }
            encode_type_into(ret, bytes, depth + 1)?;
        }
    }
    Ok(())
}

fn decode_type_from(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
) -> Result<TypeSignature, DecodeError> {
    if depth >= MAX_TYPE_DEPTH {
        return Err(DecodeError::TypeRefTooDeep);
    }
    let tag = read_u8(bytes, cursor)?;
    let signature = match tag {
        0x00 => TypeSignature::Void,
        0x01 => TypeSignature::Int,
        0x02 => TypeSignature::Float,
        0x03 => TypeSignature::Bool,
        0x04 => TypeSignature::String,
        0x05 => TypeSignature::Entity,
        0x10 => TypeSignature::Named(MetadataToken(read_u32(bytes, cursor)?)),
        0x11 => {
            let namespace_len = read_u16(bytes, cursor)? as usize;
            let namespace_bytes = read_bytes(bytes, cursor, namespace_len)?;
            let namespace = std::str::from_utf8(namespace_bytes)?.to_owned();
            let name_len = read_u16(bytes, cursor)? as usize;
            let name_bytes = read_bytes(bytes, cursor, name_len)?;
            let name = std::str::from_utf8(name_bytes)?.to_owned();
            if name.is_empty() {
                return Err(DecodeError::InvalidTypeSignature(
                    "generic constructor name is empty",
                ));
            }
            let arg_count = read_u16(bytes, cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(decode_type_from(bytes, cursor, depth + 1)?);
            }
            TypeSignature::Generic {
                namespace,
                name,
                args,
            }
        }
        0x12 => TypeSignature::GenericParam(read_u16(bytes, cursor)?),
        0x20 => TypeSignature::Array(Box::new(decode_type_from(bytes, cursor, depth + 1)?)),
        0x30 => {
            let param_count = read_u16(bytes, cursor)? as usize;
            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                params.push(decode_type_from(bytes, cursor, depth + 1)?);
            }
            let ret = Box::new(decode_type_from(bytes, cursor, depth + 1)?);
            TypeSignature::Function { params, ret }
        }
        other => return Err(DecodeError::InvalidTypeRefKind(other)),
    };
    Ok(signature)
}

fn checked_u16(value: usize, what: &'static str) -> Result<u16, EncodeError> {
    u16::try_from(value).map_err(|_| EncodeError::TypeSignatureTooLarge { what, value })
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, DecodeError> {
    let value = *bytes.get(*cursor).ok_or(DecodeError::UnexpectedEof)?;
    *cursor += 1;
    Ok(value)
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, DecodeError> {
    let raw = read_bytes(bytes, cursor, 2)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, DecodeError> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'a [u8], DecodeError> {
    let end = cursor.checked_add(len).ok_or(DecodeError::UnexpectedEof)?;
    let value = bytes.get(*cursor..end).ok_or(DecodeError::UnexpectedEof)?;
    *cursor = end;
    Ok(value)
}
