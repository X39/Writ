//! Attribute argument encoding and decoding for the blob heap.
//!
//! Attribute arguments are stored in the module blob heap using a tagged binary
//! format. Each argument is prefixed with a 1-byte tag identifying its type,
//! followed by its payload. A multi-argument blob is a sequential concatenation
//! of individually encoded arguments.
//!
//! ## Wire format
//!
//! ```text
//! ATTR_TAG_STRING (0x01): u32(byte_len, LE) + UTF-8 bytes
//! ATTR_TAG_INT    (0x02): i64 (little-endian, 8 bytes)
//! ATTR_TAG_BOOL   (0x03): u8 (0x00 = false, any other = true)
//! ATTR_TAG_NAMED  (0x04): u32(name_byte_len, LE) + name_bytes + [inner arg encoding]
//! ```
//!
//! An empty argument list encodes to an empty `Vec<u8>` (the null blob, offset 0).

use crate::error::DecodeError;

/// Tag byte for a string attribute argument.
pub const ATTR_TAG_STRING: u8 = 0x01;

/// Tag byte for an integer attribute argument (i64 little-endian).
pub const ATTR_TAG_INT: u8 = 0x02;

/// Tag byte for a boolean attribute argument (u8, 0x00 = false).
pub const ATTR_TAG_BOOL: u8 = 0x03;

/// Tag byte for a named attribute argument (name prefix + inner value).
pub const ATTR_TAG_NAMED: u8 = 0x04;

/// A decoded attribute argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// A UTF-8 string argument.
    String(std::string::String),
    /// A 64-bit signed integer argument.
    Int(i64),
    /// A boolean argument.
    Bool(bool),
    /// A named argument wrapping an inner value (e.g. `msg: "text"`).
    Named {
        name: std::string::String,
        value: Box<AttrValue>,
    },
}

/// Encode a slice of attribute arguments into a binary blob.
///
/// Returns an empty `Vec<u8>` for an empty slice — the caller should use
/// blob offset 0 (the null blob) in that case rather than interning empty bytes.
pub fn encode_attr_args(args: &[AttrValue]) -> Vec<u8> {
    let mut buf = Vec::new();
    for arg in args {
        encode_attr_value(arg, &mut buf);
    }
    buf
}

fn encode_attr_value(val: &AttrValue, buf: &mut Vec<u8>) {
    match val {
        AttrValue::String(s) => {
            buf.push(ATTR_TAG_STRING);
            let bytes = s.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
        AttrValue::Int(n) => {
            buf.push(ATTR_TAG_INT);
            buf.extend_from_slice(&n.to_le_bytes());
        }
        AttrValue::Bool(b) => {
            buf.push(ATTR_TAG_BOOL);
            buf.push(if *b { 1u8 } else { 0u8 });
        }
        AttrValue::Named { name, value } => {
            buf.push(ATTR_TAG_NAMED);
            let name_bytes = name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            encode_attr_value(value, buf);
        }
    }
}

/// Decode a binary blob back to a list of attribute argument values.
///
/// Returns `Ok(vec![])` for an empty slice.
/// Returns `Err(DecodeError::InvalidAttrTag)` for an unknown tag byte.
/// Returns `Err(DecodeError::BufferTooSmall)` for a truncated blob.
pub fn decode_attr_args(blob: &[u8]) -> Result<Vec<AttrValue>, DecodeError> {
    let mut cursor = 0usize;
    let mut out = Vec::new();
    while cursor < blob.len() {
        let (val, consumed) = decode_one(blob, cursor)?;
        out.push(val);
        cursor += consumed;
    }
    Ok(out)
}

fn decode_one(blob: &[u8], pos: usize) -> Result<(AttrValue, usize), DecodeError> {
    if pos >= blob.len() {
        return Err(DecodeError::BufferTooSmall);
    }
    let tag = blob[pos];
    match tag {
        ATTR_TAG_STRING => {
            // 1 (tag) + 4 (u32 len) + len (bytes)
            if pos + 5 > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let len = u32::from_le_bytes([
                blob[pos + 1],
                blob[pos + 2],
                blob[pos + 3],
                blob[pos + 4],
            ]) as usize;
            let start = pos + 5;
            if start + len > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let s = std::str::from_utf8(&blob[start..start + len])
                .map_err(DecodeError::BadUtf8)?
                .to_owned();
            Ok((AttrValue::String(s), 1 + 4 + len))
        }
        ATTR_TAG_INT => {
            // 1 (tag) + 8 (i64 LE)
            if pos + 9 > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let n = i64::from_le_bytes([
                blob[pos + 1],
                blob[pos + 2],
                blob[pos + 3],
                blob[pos + 4],
                blob[pos + 5],
                blob[pos + 6],
                blob[pos + 7],
                blob[pos + 8],
            ]);
            Ok((AttrValue::Int(n), 9))
        }
        ATTR_TAG_BOOL => {
            // 1 (tag) + 1 (u8)
            if pos + 2 > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let b = blob[pos + 1] != 0;
            Ok((AttrValue::Bool(b), 2))
        }
        ATTR_TAG_NAMED => {
            // 1 (tag) + 4 (u32 name_len) + name_len (bytes) + [inner]
            if pos + 5 > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let name_len = u32::from_le_bytes([
                blob[pos + 1],
                blob[pos + 2],
                blob[pos + 3],
                blob[pos + 4],
            ]) as usize;
            let name_start = pos + 5;
            if name_start + name_len > blob.len() {
                return Err(DecodeError::BufferTooSmall);
            }
            let name = std::str::from_utf8(&blob[name_start..name_start + name_len])
                .map_err(DecodeError::BadUtf8)?
                .to_owned();
            let inner_pos = name_start + name_len;
            let (inner_val, inner_consumed) = decode_one(blob, inner_pos)?;
            let total = 1 + 4 + name_len + inner_consumed;
            Ok((
                AttrValue::Named {
                    name,
                    value: Box::new(inner_val),
                },
                total,
            ))
        }
        unknown => Err(DecodeError::InvalidAttrTag(unknown)),
    }
}
