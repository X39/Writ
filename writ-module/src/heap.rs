use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

use crate::error::DecodeError;

/// Initialize a string heap with a null/empty-string record at offset 0.
///
/// The first 4 bytes are a u32(0) length prefix, meaning offset 0 reads as "".
pub fn init_string_heap() -> Vec<u8> {
    let mut heap = Vec::new();
    heap.write_u32::<LittleEndian>(0).unwrap();
    heap
}

/// Intern a string into the heap, returning its offset.
///
/// Format: u32(byte_length) followed by UTF-8 bytes (no null terminator).
pub fn intern_string(heap: &mut Vec<u8>, s: &str) -> u32 {
    let offset = heap.len() as u32;
    heap.write_u32::<LittleEndian>(s.len() as u32).unwrap();
    heap.extend_from_slice(s.as_bytes());
    offset
}

/// Read a string from the heap at the given offset.
///
/// Offset 0 returns "" (the empty/null string). Otherwise reads u32(len) + bytes.
pub fn read_string(heap: &[u8], offset: u32) -> Result<&str, DecodeError> {
    let offset = offset as usize;
    if offset + 4 > heap.len() {
        return Err(DecodeError::BufferTooSmall);
    }
    let mut cursor = Cursor::new(&heap[offset..]);
    let len = cursor.read_u32::<LittleEndian>()? as usize;
    let start = offset + 4;
    if start + len > heap.len() {
        return Err(DecodeError::BufferTooSmall);
    }
    let bytes = &heap[start..start + len];
    std::str::from_utf8(bytes).map_err(DecodeError::BadUtf8)
}

/// Initialize a blob heap with a null/empty-blob record at offset 0.
pub fn init_blob_heap() -> Vec<u8> {
    let mut heap = Vec::new();
    heap.write_u32::<LittleEndian>(0).unwrap();
    heap
}

/// Write a blob into the heap, returning its offset.
///
/// Format: u32(byte_length) followed by the raw bytes.
pub fn write_blob(heap: &mut Vec<u8>, data: &[u8]) -> u32 {
    let offset = heap.len() as u32;
    heap.write_u32::<LittleEndian>(data.len() as u32).unwrap();
    heap.extend_from_slice(data);
    offset
}

/// Read a blob from the heap at the given offset.
///
/// Offset 0 returns &[] (the empty/null blob). Otherwise reads u32(len) + bytes.
pub fn read_blob(heap: &[u8], offset: u32) -> Result<&[u8], DecodeError> {
    let offset = offset as usize;
    if offset + 4 > heap.len() {
        return Err(DecodeError::BufferTooSmall);
    }
    let mut cursor = Cursor::new(&heap[offset..]);
    let len = cursor.read_u32::<LittleEndian>()? as usize;
    let start = offset + 4;
    if start + len > heap.len() {
        return Err(DecodeError::BufferTooSmall);
    }
    Ok(&heap[start..start + len])
}
