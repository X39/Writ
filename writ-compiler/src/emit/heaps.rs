//! String heap and blob heap builders with deduplication.
//!
//! Both heaps use length-prefixed encoding: `u32(byte_length)` followed by the bytes.
//! Offset 0 is reserved as the empty/null entry.

use rustc_hash::FxHashMap;

/// A string heap with deduplication.
///
/// Each entry is `u32(byte_length)` followed by UTF-8 bytes.
/// Offset 0 is the empty string (4 bytes: length=0).
pub struct StringHeap {
    data: Vec<u8>,
    dedup: FxHashMap<String, u32>,
}

impl StringHeap {
    /// Create a new string heap with the empty string at offset 0.
    pub fn new() -> Self {
        let mut data = Vec::new();
        // Reserve offset 0 for the empty/null string: length=0
        data.extend_from_slice(&0u32.to_le_bytes());
        let mut dedup = FxHashMap::default();
        dedup.insert(String::new(), 0);
        Self { data, dedup }
    }

    /// Intern a string, returning its heap offset.
    ///
    /// Duplicate strings return the same offset.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&offset) = self.dedup.get(s) {
            return offset;
        }
        let offset = self.data.len() as u32;
        let bytes = s.as_bytes();
        self.data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.data.extend_from_slice(bytes);
        self.dedup.insert(s.to_string(), offset);
        offset
    }

    /// Look up the string at a given heap offset.
    ///
    /// The heap stores: u32(byte_length) followed by UTF-8 bytes.
    /// Returns the string if the offset is valid.
    pub fn get_str(&self, offset: u32) -> &str {
        let pos = offset as usize;
        if pos + 4 > self.data.len() {
            return "";
        }
        let len = u32::from_le_bytes([
            self.data[pos],
            self.data[pos + 1],
            self.data[pos + 2],
            self.data[pos + 3],
        ]) as usize;
        let start = pos + 4;
        let end = start + len;
        if end > self.data.len() {
            return "";
        }
        std::str::from_utf8(&self.data[start..end]).unwrap_or("")
    }

    /// Get the raw heap data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the total size of the heap.
    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }
}

impl Default for StringHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StringHeap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StringHeap")
            .field("size", &self.data.len())
            .field("entries", &self.dedup.len())
            .finish()
    }
}

/// A blob heap with deduplication.
///
/// Each entry is `u32(byte_length)` followed by the raw bytes.
/// Offset 0 is the empty blob (4 bytes: length=0).
pub struct BlobHeap {
    data: Vec<u8>,
    dedup: FxHashMap<Vec<u8>, u32>,
}

impl BlobHeap {
    /// Create a new blob heap with the empty blob at offset 0.
    pub fn new() -> Self {
        let mut data = Vec::new();
        // Reserve offset 0 for the empty/null blob: length=0
        data.extend_from_slice(&0u32.to_le_bytes());
        let mut dedup = FxHashMap::default();
        dedup.insert(Vec::new(), 0);
        Self { data, dedup }
    }

    /// Intern a blob, returning its heap offset.
    ///
    /// Duplicate blobs return the same offset.
    pub fn intern(&mut self, blob: &[u8]) -> u32 {
        if let Some(&offset) = self.dedup.get(blob) {
            return offset;
        }
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        self.data.extend_from_slice(blob);
        self.dedup.insert(blob.to_vec(), offset);
        offset
    }

    /// Get the raw heap data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the total size of the heap.
    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }
}

impl Default for BlobHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BlobHeap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobHeap")
            .field("size", &self.data.len())
            .field("entries", &self.dedup.len())
            .finish()
    }
}
