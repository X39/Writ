/// A metadata token encodes a table ID and row index in a single u32 value.
///
/// Layout:
/// - Bits 31-24: table ID (0-20)
/// - Bits 23-0:  row index (1-based; 0 = null token)
///
/// See spec section 2.16.4 for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetadataToken(pub u32);

impl MetadataToken {
    /// The null token. Represents "no reference".
    pub const NULL: MetadataToken = MetadataToken(0);

    /// Create a new metadata token with the given table ID and 1-based row index.
    ///
    /// # Panics
    ///
    /// Panics if `row_index` exceeds 24 bits (> 0x00FF_FFFF).
    pub fn new(table_id: u8, row_index: u32) -> Self {
        assert!(
            row_index <= 0x00FF_FFFF,
            "row_index {row_index} exceeds 24-bit maximum (0x00FFFFFF)"
        );
        MetadataToken(((table_id as u32) << 24) | row_index)
    }

    /// Returns the table ID (bits 31-24).
    pub fn table_id(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Returns the 1-based row index (bits 23-0), or `None` if this is the null token
    /// or the row index portion is 0.
    pub fn row_index(self) -> Option<u32> {
        let idx = self.0 & 0x00FF_FFFF;
        if idx == 0 { None } else { Some(idx) }
    }

    /// Returns `true` if this is the null token (all bits zero).
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}
