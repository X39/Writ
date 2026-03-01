/// Top-level module error type.
#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("encode error: {0}")]
    Encode(#[from] EncodeError),
}

/// Errors encountered while decoding (reading) a binary module.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bad magic bytes: expected WRIT, got {}", format_magic(.0))]
    BadMagic([u8; 4]),

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u16),

    #[error("bad UTF-8 in string heap: {0}")]
    BadUtf8(#[from] std::str::Utf8Error),

    #[error("invalid opcode: 0x{0:04X}")]
    InvalidOpcode(u16),

    #[error("invalid table ID: {0}")]
    InvalidTableId(u8),

    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("TypeRef nesting too deep")]
    TypeRefTooDeep,

    #[error("invalid TypeRef kind: 0x{0:02X}")]
    InvalidTypeRefKind(u8),

    #[error("buffer too small")]
    BufferTooSmall,
}

/// Errors encountered while encoding (writing) a binary module.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("string too large: {0} bytes")]
    StringTooLarge(usize),

    #[error("too many rows: {0}")]
    TooManyRows(usize),
}

fn format_magic(bytes: &[u8; 4]) -> String {
    format!(
        "[0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}]",
        bytes[0], bytes[1], bytes[2], bytes[3]
    )
}
