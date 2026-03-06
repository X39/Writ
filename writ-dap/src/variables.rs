use writ_module::heap::{read_blob, read_string};
use writ_module::module::Module;
use writ_runtime::gc::GcHeap;
use writ_runtime::heap::HeapObject;
use writ_runtime::value::Value;

/// Pack (task_idx, frame_idx) into a single DAP `variablesReference` i64.
///
/// The high 32 bits hold task_idx, the low 32 bits hold frame_idx.
/// Adds +1 to ensure the result is never 0, since DAP reserves
/// variablesReference=0 for "no children / not expandable". Use
/// `unpack_variables_ref` to reverse this offset.
pub fn make_variables_ref(task_idx: u32, frame_idx: u32) -> i64 {
    (((task_idx as i64) << 32) | (frame_idx as i64)) + 1
}

/// Unpack a variablesReference back into (task_idx, frame_idx).
///
/// Subtracts the +1 offset added by `make_variables_ref` before unpacking.
pub fn unpack_variables_ref(r: i64) -> (u32, u32) {
    let r = r - 1;
    ((r >> 32) as u32, (r & 0xFFFF_FFFF) as u32)
}

/// Format a runtime `Value` as a human-readable string for the DAP Variables response.
///
/// Heap objects require a `GcHeap` to dereference. If a `Ref` points to an invalid
/// heap slot the function returns `"<invalid ref>"` instead of panicking.
#[allow(clippy::only_used_in_recursion)] // module reserved for future TypeDef name resolution
pub fn format_value(val: &Value, module: &Module, heap: &dyn GcHeap) -> String {
    match val {
        Value::Void => "(void)".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format!("{}", f),
        Value::Bool(b) => b.to_string(),
        Value::Ref(href) => match heap.get_object(*href) {
            Ok(obj) => match obj {
                HeapObject::String(s) => format!("{:?}", s),
                HeapObject::Struct { fields } => format!("struct({})", fields.len()),
                HeapObject::Array { elements, .. } => format!("[{} elements]", elements.len()),
                HeapObject::Delegate { method_idx, .. } => format!("fn@{}", method_idx),
                HeapObject::Enum { tag, .. } => format!("enum(tag={})", tag),
                HeapObject::Boxed(inner) => {
                    format!("box({})", format_value(inner, module, heap))
                }
            },
            Err(_) => "<invalid ref>".to_string(),
        },
        Value::Entity(eid) => format!("entity#{}", eid.index),
        Value::InlineStruct { type_idx, fields } => {
            format!("struct{}({})", type_idx, fields.len())
        }
    }
}

/// Decode a type-blob at `type_ref_offset` in the module's blob heap to a readable name.
///
/// Returns `"?"` for an offset of 0, unknown tags, or any decode error.
/// Only primitive tags (0x00-0x04), TypeDef (0x10), Array (0x20), and fn (0x30) are decoded;
/// nested type parameters are not expanded (e.g., `Array<T>` returns `"Array<?>"` for now).
pub fn decode_type_blob(module: &Module, type_ref_offset: u32) -> String {
    if type_ref_offset == 0 {
        return "?".to_string();
    }
    match read_blob(&module.blob_heap, type_ref_offset) {
        Err(_) => "?".to_string(),
        Ok(bytes) => {
            if bytes.is_empty() {
                return "?".to_string();
            }
            match bytes[0] {
                0x00 => "void".to_string(),
                0x01 => "int".to_string(),
                0x02 => "float".to_string(),
                0x03 => "bool".to_string(),
                0x04 => "string".to_string(),
                0x10 if bytes.len() >= 5 => {
                    let row_1based =
                        u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
                    let idx = row_1based.saturating_sub(1) as usize;
                    module
                        .type_defs
                        .get(idx)
                        .and_then(|td| read_string(&module.string_heap, td.name).ok())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Type".to_string())
                }
                0x20 => "Array<?>".to_string(),
                0x30 => "fn".to_string(),
                _ => "?".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::heap::{intern_string, write_blob};
    use writ_module::module::Module;
    use writ_module::tables::{TypeDefKind, TypeDefRow};
    use writ_runtime::heap::BumpHeap;
    use writ_runtime::value::{EntityId, Value};

    // ── variablesReference roundtrip ─────────────────────────────────────────

    /// DAP reserves variablesReference=0 for "no children". make_variables_ref(0, 0)
    /// must return 1 (the +1 offset ensures this), and the roundtrip must recover (0, 0).
    #[test]
    fn test_variables_ref_roundtrip_zero() {
        let r = make_variables_ref(0, 0);
        assert_eq!(r, 1, "make_variables_ref(0, 0) must return 1 (not 0)");
        assert_eq!(unpack_variables_ref(r), (0, 0));
    }

    /// The +1 offset must not interfere with non-zero inputs.
    #[test]
    fn test_variables_ref_never_zero() {
        assert_ne!(make_variables_ref(0, 0), 0, "variablesReference must never be 0");
        assert_ne!(make_variables_ref(0, 1), 0);
        assert_ne!(make_variables_ref(1, 0), 0);
        assert_ne!(make_variables_ref(5, 9), 0);
    }

    #[test]
    fn test_variables_ref_roundtrip_nonzero() {
        let r = make_variables_ref(5, 9);
        assert_eq!(unpack_variables_ref(r), (5, 9));
    }

    #[test]
    fn test_variables_ref_roundtrip_large() {
        // Use (u32::MAX - 1, u32::MAX) to avoid overflow from the +1 offset
        // when task_idx=u32::MAX and frame_idx=u32::MAX.
        let r = make_variables_ref(u32::MAX - 1, u32::MAX);
        assert_eq!(unpack_variables_ref(r), (u32::MAX - 1, u32::MAX));
    }

    #[test]
    fn test_variables_ref_roundtrip_mixed() {
        let r = make_variables_ref(1, 5);
        assert_eq!(unpack_variables_ref(r), (1, 5));

        let r2 = make_variables_ref(100, 999);
        assert_eq!(unpack_variables_ref(r2), (100, 999));
    }

    // ── format_value ─────────────────────────────────────────────────────────

    fn empty_module() -> Module {
        Module::new()
    }

    #[test]
    fn test_format_value_void() {
        let m = empty_module();
        let heap = BumpHeap::new();
        assert_eq!(format_value(&Value::Void, &m, &heap), "(void)");
    }

    #[test]
    fn test_format_value_int() {
        let m = empty_module();
        let heap = BumpHeap::new();
        assert_eq!(format_value(&Value::Int(42), &m, &heap), "42");
        assert_eq!(format_value(&Value::Int(-1), &m, &heap), "-1");
    }

    #[test]
    fn test_format_value_float() {
        let m = empty_module();
        let heap = BumpHeap::new();
        let s = format_value(&Value::Float(3.14), &m, &heap);
        assert!(s.contains("3.14"), "got: {}", s);
    }

    #[test]
    fn test_format_value_bool() {
        let m = empty_module();
        let heap = BumpHeap::new();
        assert_eq!(format_value(&Value::Bool(true), &m, &heap), "true");
        assert_eq!(format_value(&Value::Bool(false), &m, &heap), "false");
    }

    #[test]
    fn test_format_value_ref_string() {
        let m = empty_module();
        let mut heap = BumpHeap::new();
        let href = heap.alloc_string("hello");
        let result = format_value(&Value::Ref(href), &m, &heap);
        assert!(result.contains("hello"), "got: {}", result);
        // Should be quoted (uses {:?})
        assert!(result.starts_with('"'), "got: {}", result);
    }

    #[test]
    fn test_format_value_ref_struct() {
        let m = empty_module();
        let mut heap = BumpHeap::new();
        let href = heap.alloc_struct(3);
        let result = format_value(&Value::Ref(href), &m, &heap);
        assert_eq!(result, "struct(3)");
    }

    #[test]
    fn test_format_value_ref_array() {
        let m = empty_module();
        let mut heap = BumpHeap::new();
        let href = heap.alloc_array(1);
        let result = format_value(&Value::Ref(href), &m, &heap);
        assert_eq!(result, "[0 elements]");
    }

    #[test]
    fn test_format_value_ref_invalid() {
        let m = empty_module();
        // Allocate a string on heap_a to get a valid HeapRef, then use that ref
        // against an empty heap_b — the ref is now out of range on heap_b.
        let mut heap_a = BumpHeap::new();
        let href = heap_a.alloc_string("dangling");
        let heap_b = BumpHeap::new(); // empty — href is out of range here
        let result = format_value(&Value::Ref(href), &m, &heap_b);
        assert_eq!(result, "<invalid ref>");
    }

    #[test]
    fn test_format_value_entity() {
        let m = empty_module();
        let heap = BumpHeap::new();
        let eid = EntityId::new(7, 0);
        let result = format_value(&Value::Entity(eid), &m, &heap);
        assert_eq!(result, "entity#7");
    }

    #[test]
    fn test_format_value_inline_struct() {
        let m = empty_module();
        let heap = BumpHeap::new();
        let val = Value::InlineStruct {
            type_idx: 2,
            fields: vec![Value::Int(1), Value::Int(2)],
        };
        let result = format_value(&val, &m, &heap);
        assert_eq!(result, "struct2(2)");
    }

    // ── decode_type_blob ──────────────────────────────────────────────────────

    /// Build a module whose blob heap contains a single blob at offset 4 (just after the
    /// initial null-blob at offset 0).
    fn module_with_blob(tag_bytes: &[u8]) -> (Module, u32) {
        let mut m = Module::new();
        let offset = write_blob(&mut m.blob_heap, tag_bytes);
        (m, offset)
    }

    #[test]
    fn test_decode_type_blob_zero_offset_returns_question() {
        let m = Module::new();
        assert_eq!(decode_type_blob(&m, 0), "?");
    }

    #[test]
    fn test_decode_type_blob_void() {
        let (m, off) = module_with_blob(&[0x00]);
        assert_eq!(decode_type_blob(&m, off), "void");
    }

    #[test]
    fn test_decode_type_blob_int() {
        let (m, off) = module_with_blob(&[0x01]);
        assert_eq!(decode_type_blob(&m, off), "int");
    }

    #[test]
    fn test_decode_type_blob_float() {
        let (m, off) = module_with_blob(&[0x02]);
        assert_eq!(decode_type_blob(&m, off), "float");
    }

    #[test]
    fn test_decode_type_blob_bool() {
        let (m, off) = module_with_blob(&[0x03]);
        assert_eq!(decode_type_blob(&m, off), "bool");
    }

    #[test]
    fn test_decode_type_blob_string() {
        let (m, off) = module_with_blob(&[0x04]);
        assert_eq!(decode_type_blob(&m, off), "string");
    }

    #[test]
    fn test_decode_type_blob_array() {
        let (m, off) = module_with_blob(&[0x20, 0x01]);
        assert_eq!(decode_type_blob(&m, off), "Array<?>");
    }

    #[test]
    fn test_decode_type_blob_fn() {
        let (m, off) = module_with_blob(&[0x30, 0x00, 0x00]);
        assert_eq!(decode_type_blob(&m, off), "fn");
    }

    #[test]
    fn test_decode_type_blob_unknown_tag() {
        let (m, off) = module_with_blob(&[0xFF]);
        assert_eq!(decode_type_blob(&m, off), "?");
    }

    #[test]
    fn test_decode_type_blob_typedef_lookup() {
        let mut m = Module::new();
        // Intern a type name into the string heap
        let name_off = intern_string(&mut m.string_heap, "MyStruct");
        let ns_off = intern_string(&mut m.string_heap, "");
        // Add a TypeDefRow at index 0
        m.type_defs.push(TypeDefRow {
            name: name_off,
            namespace: ns_off,
            kind: TypeDefKind::Struct.as_u8(),
            flags: 0,
            field_list: 0,
            method_list: 0,
        });
        // Blob: tag 0x10 + u32 LE row=1 (1-based index into TypeDef table)
        let blob: Vec<u8> = {
            let mut b = vec![0x10u8];
            b.extend_from_slice(&1u32.to_le_bytes());
            b
        };
        let off = write_blob(&mut m.blob_heap, &blob);
        assert_eq!(decode_type_blob(&m, off), "MyStruct");
    }

    #[test]
    fn test_decode_type_blob_typedef_out_of_range() {
        let mut m = Module::new();
        // Blob: tag 0x10 + u32 LE row=99 (no type_defs)
        let blob: Vec<u8> = {
            let mut b = vec![0x10u8];
            b.extend_from_slice(&99u32.to_le_bytes());
            b
        };
        let off = write_blob(&mut m.blob_heap, &blob);
        assert_eq!(decode_type_blob(&m, off), "Type");
    }
}
