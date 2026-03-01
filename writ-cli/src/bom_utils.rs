//! UTF-8 BOM handling utilities.
//!
//! Detects and handles Byte Order Marks (BOM) in text files:
//! - UTF-8 BOM (EF BB BF) → stripped on read
//! - UTF-16 LE/BE, UTF-32 → converted to UTF-8
//! - No BOM → used as-is

/// Detect and handle BOM, returning clean UTF-8 string.
///
/// Supports:
/// - UTF-8 BOM (0xEF 0xBB 0xBF) — stripped
/// - UTF-16 LE BOM (0xFF 0xFE) — converted to UTF-8
/// - UTF-16 BE BOM (0xFE 0xFF) — converted to UTF-8
/// - UTF-32 LE BOM (0xFF 0xFE 0x00 0x00) — converted to UTF-8
/// - UTF-32 BE BOM (0x00 0x00 0xFE 0xFF) — converted to UTF-8
/// - No BOM — returned as UTF-8 directly
///
/// # Errors
/// Returns an error if the input is not valid for any detected encoding.
pub fn strip_bom_and_decode(bytes: &[u8]) -> Result<String, String> {
    // UTF-32 LE BOM (check before UTF-16 LE since it shares first 2 bytes)
    if bytes.len() >= 4 && bytes[0..4] == [0xFF, 0xFE, 0x00, 0x00] {
        let text = FromUtf16Le::from_utf16le(&bytes[4..])
            .map_err(|_| "Invalid UTF-32 LE encoding".to_string())?;
        return Ok(text);
    }

    // UTF-32 BE BOM
    if bytes.len() >= 4 && bytes[0..4] == [0x00, 0x00, 0xFE, 0xFF] {
        let text = FromUtf16Be::from_utf16be(&bytes[4..])
            .map_err(|_| "Invalid UTF-32 BE encoding".to_string())?;
        return Ok(text);
    }

    // UTF-8 BOM
    if bytes.len() >= 3 && bytes[0..3] == [0xEF, 0xBB, 0xBF] {
        let text = String::from_utf8(bytes[3..].to_vec())
            .map_err(|_| "Invalid UTF-8 encoding".to_string())?;
        return Ok(text);
    }

    // UTF-16 LE BOM
    if bytes.len() >= 2 && bytes[0..2] == [0xFF, 0xFE] {
        let text = FromUtf16Le::from_utf16le(&bytes[2..])
            .map_err(|_| "Invalid UTF-16 LE encoding".to_string())?;
        return Ok(text);
    }

    // UTF-16 BE BOM
    if bytes.len() >= 2 && bytes[0..2] == [0xFE, 0xFF] {
        let text = FromUtf16Be::from_utf16be(&bytes[2..])
            .map_err(|_| "Invalid UTF-16 BE encoding".to_string())?;
        return Ok(text);
    }

    // No BOM detected — assume UTF-8
    String::from_utf8(bytes.to_vec())
        .map_err(|_| "Invalid UTF-8 encoding (no BOM detected)".to_string())
}

/// Add UTF-8 BOM to the beginning of a string.
pub fn add_utf8_bom(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 3);
    result.push_str("\u{FEFF}"); // UTF-8 BOM
    result.push_str(text);
    result
}

/// Helper trait to decode UTF-16 LE.
trait FromUtf16Le: Sized {
    fn from_utf16le(v: &[u8]) -> Result<Self, String>;
}

impl FromUtf16Le for String {
    fn from_utf16le(bytes: &[u8]) -> Result<String, String> {
        if bytes.len() % 2 != 0 {
            return Err("UTF-16 LE byte count not even".to_string());
        }
        let mut units = Vec::new();
        for chunk in bytes.chunks_exact(2) {
            let unit = u16::from_le_bytes([chunk[0], chunk[1]]);
            units.push(unit);
        }
        String::from_utf16(&units).map_err(|_| "Invalid UTF-16 LE sequence".to_string())
    }
}

/// Helper trait to decode UTF-16 BE.
trait FromUtf16Be: Sized {
    fn from_utf16be(v: &[u8]) -> Result<Self, String>;
}

impl FromUtf16Be for String {
    fn from_utf16be(bytes: &[u8]) -> Result<String, String> {
        if bytes.len() % 2 != 0 {
            return Err("UTF-16 BE byte count not even".to_string());
        }
        let mut units = Vec::new();
        for chunk in bytes.chunks_exact(2) {
            let unit = u16::from_be_bytes([chunk[0], chunk[1]]);
            units.push(unit);
        }
        String::from_utf16(&units).map_err(|_| "Invalid UTF-16 BE sequence".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_bom() {
        let text = "hello world";
        let result = strip_bom_and_decode(text.as_bytes()).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"hello");
        let result = strip_bom_and_decode(&bytes).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_utf16_le_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        // "hi" in UTF-16 LE
        bytes.extend_from_slice(&[0x68, 0x00, 0x69, 0x00]);
        let result = strip_bom_and_decode(&bytes).unwrap();
        assert_eq!(result, "hi");
    }

    #[test]
    fn test_utf16_be_bom() {
        let mut bytes = vec![0xFE, 0xFF];
        // "hi" in UTF-16 BE
        bytes.extend_from_slice(&[0x00, 0x68, 0x00, 0x69]);
        let result = strip_bom_and_decode(&bytes).unwrap();
        assert_eq!(result, "hi");
    }

    #[test]
    fn test_add_utf8_bom() {
        let text = "hello";
        let with_bom = add_utf8_bom(text);
        assert_eq!(with_bom, "\u{FEFF}hello");
    }

    #[test]
    fn test_add_bom_and_strip() {
        let text = "test";
        let with_bom = add_utf8_bom(text);
        let bytes = with_bom.as_bytes();
        let stripped = strip_bom_and_decode(bytes).unwrap();
        assert_eq!(stripped, "test");
    }
}
