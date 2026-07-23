use crate::value::{EntityId, HeapRef, Value};

#[inline(always)]
pub(super) fn extract_int(val: &Value) -> i64 {
    match val {
        Value::Int(n) => *n,
        _ => 0,
    }
}

#[inline(always)]
pub(super) fn extract_float(val: &Value) -> f64 {
    match val {
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

#[inline(always)]
pub(super) fn extract_bool(val: &Value) -> bool {
    match val {
        Value::Bool(b) => *b,
        _ => false,
    }
}

#[inline(always)]
pub(super) fn extract_ref(val: &Value) -> HeapRef {
    match val {
        Value::Ref(href) => *href,
        _ => HeapRef(u32::MAX), // Will cause error on use
    }
}

#[inline(always)]
pub(super) fn extract_entity(val: &Value) -> EntityId {
    match val {
        Value::Entity(eid) => *eid,
        _ => EntityId::new(u32::MAX, 0),
    }
}

/// Return the exact number of fields owned by a resolved, zero-based TypeDef.
///
/// Loaded modules normally satisfy these invariants already. Keeping the
/// calculation checked here prevents dispatch from inventing an object layout
/// if malformed metadata is ever supplied programmatically.
pub(super) fn get_type_field_count(
    module: &writ_module::Module,
    type_def_idx: usize,
) -> Result<usize, String> {
    let type_def = module
        .type_defs
        .get(type_def_idx)
        .ok_or_else(|| format!("TypeDef row {} is out of range", type_def_idx + 1))?;
    let field_start = type_def.field_list.checked_sub(1).ok_or_else(|| {
        format!(
            "TypeDef row {} has invalid zero field_list",
            type_def_idx + 1
        )
    })? as usize;
    let field_end = if type_def_idx + 1 < module.type_defs.len() {
        module.type_defs[type_def_idx + 1]
            .field_list
            .checked_sub(1)
            .ok_or_else(|| {
                format!(
                    "TypeDef row {} has invalid zero field_list",
                    type_def_idx + 2
                )
            })? as usize
    } else {
        module.field_defs.len()
    };
    if field_start > field_end || field_end > module.field_defs.len() {
        return Err(format!(
            "TypeDef row {} has invalid field range {field_start}..{field_end} for {} FieldDef row(s)",
            type_def_idx + 1,
            module.field_defs.len()
        ));
    }
    Ok(field_end - field_start)
}
