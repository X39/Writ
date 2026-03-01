use crate::value::{EntityId, HeapRef, Value};

pub(super) fn extract_int(val: &Value) -> i64 {
    match val {
        Value::Int(n) => *n,
        _ => 0,
    }
}

pub(super) fn extract_float(val: &Value) -> f64 {
    match val {
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

pub(super) fn extract_bool(val: &Value) -> bool {
    match val {
        Value::Bool(b) => *b,
        _ => false,
    }
}

pub(super) fn extract_ref(val: &Value) -> HeapRef {
    match val {
        Value::Ref(href) => *href,
        _ => HeapRef(u32::MAX), // Will cause error on use
    }
}

pub(super) fn extract_entity(val: &Value) -> EntityId {
    match val {
        Value::Entity(eid) => *eid,
        _ => EntityId::new(u32::MAX, 0),
    }
}

/// Get the number of fields for a type from its TypeDef.
pub(super) fn get_type_field_count(module: &writ_module::Module, type_idx: u32) -> usize {
    // type_idx is a 1-based MetadataToken index
    let idx = type_idx.saturating_sub(1) as usize;
    if idx >= module.type_defs.len() {
        return 4; // default field count for unknown types
    }
    let type_def = &module.type_defs[idx];
    let field_start = type_def.field_list.saturating_sub(1) as usize;
    let field_end = if idx + 1 < module.type_defs.len() {
        module.type_defs[idx + 1].field_list.saturating_sub(1) as usize
    } else {
        module.field_defs.len()
    };
    field_end.saturating_sub(field_start)
}
