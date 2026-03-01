use byteorder::{LittleEndian, WriteBytesExt};

use crate::error::EncodeError;
use crate::module::Module;
use crate::reader::row_size;
use crate::tables::*;

/// Serialize a Module to spec-compliant bytes.
///
/// Layout:
/// ```text
/// Offset 0:     Header (200 bytes)
/// Offset 200:   Tables (in order 0-20, each row_count * aligned_row_size)
/// After tables: String heap (verbatim)
/// After string: Blob heap (verbatim)
/// After blob:   Method bodies (each 4-byte aligned)
/// ```
pub fn to_bytes(module: &Module) -> Result<Vec<u8>, EncodeError> {
    // Phase 1: Compute sizes and offsets
    let table_counts: [usize; 21] = [
        module.module_defs.len(),
        module.module_refs.len(),
        module.type_defs.len(),
        module.type_refs.len(),
        module.type_specs.len(),
        module.field_defs.len(),
        module.field_refs.len(),
        module.method_defs.len(),
        module.method_refs.len(),
        module.param_defs.len(),
        module.contract_defs.len(),
        module.contract_methods.len(),
        module.impl_defs.len(),
        module.generic_params.len(),
        module.generic_constraints.len(),
        module.global_defs.len(),
        module.extern_defs.len(),
        module.component_slots.len(),
        module.locale_defs.len(),
        module.export_defs.len(),
        module.attribute_defs.len(),
    ];

    let mut table_sizes = [0usize; 21];
    for i in 0..21 {
        table_sizes[i] = table_counts[i] * row_size(i as u8);
    }

    let tables_total: usize = table_sizes.iter().sum();
    let tables_start = 200usize; // after header
    let string_heap_offset = tables_start + tables_total;
    let string_heap_size = module.string_heap.len();
    let blob_heap_offset = string_heap_offset + string_heap_size;
    let blob_heap_size = module.blob_heap.len();
    let bodies_start = blob_heap_offset + blob_heap_size;

    // Compute method body offsets and total body section size
    let mut body_offsets: Vec<(u32, u32)> = Vec::new(); // (offset, size) for each body
    let mut current_body_offset = bodies_start;
    let mut body_idx = 0usize;
    for method in &module.method_defs {
        if method.body_size > 0 || body_idx < module.method_bodies.len() {
            if body_idx < module.method_bodies.len() {
                let body = &module.method_bodies[body_idx];
                // Align to 4 bytes
                let align_pad = (4 - (current_body_offset % 4)) % 4;
                current_body_offset += align_pad;

                let body_size = compute_body_size(body, module.header.flags);
                body_offsets.push((current_body_offset as u32, body_size as u32));
                current_body_offset += body_size;
                body_idx += 1;
            }
        }
    }

    let total_size = current_body_offset;

    // Compute table directory (absolute offsets)
    let mut table_directory = [(0u32, 0u32); 21];
    let mut offset = tables_start;
    for i in 0..21 {
        table_directory[i] = (offset as u32, table_counts[i] as u32);
        offset += table_sizes[i];
    }

    // Phase 2: Write everything
    let mut out = Vec::with_capacity(total_size);

    // Write header
    out.extend_from_slice(b"WRIT");
    out.write_u16::<LittleEndian>(module.header.format_version)?;
    out.write_u16::<LittleEndian>(module.header.flags)?;
    out.write_u32::<LittleEndian>(module.header.module_name)?;
    out.write_u32::<LittleEndian>(module.header.module_version)?;
    out.write_u32::<LittleEndian>(string_heap_offset as u32)?;
    out.write_u32::<LittleEndian>(string_heap_size as u32)?;
    out.write_u32::<LittleEndian>(blob_heap_offset as u32)?;
    out.write_u32::<LittleEndian>(blob_heap_size as u32)?;
    for entry in &table_directory {
        out.write_u32::<LittleEndian>(entry.0)?;
        out.write_u32::<LittleEndian>(entry.1)?;
    }
    debug_assert_eq!(out.len(), 200);

    // Phase 3: Write tables
    for row in &module.module_defs {
        write_module_def(&mut out, row)?;
    }
    for row in &module.module_refs {
        write_module_ref(&mut out, row)?;
    }
    for row in &module.type_defs {
        write_type_def(&mut out, row)?;
    }
    for row in &module.type_refs {
        write_type_ref(&mut out, row)?;
    }
    for row in &module.type_specs {
        write_type_spec(&mut out, row)?;
    }
    for row in &module.field_defs {
        write_field_def(&mut out, row)?;
    }
    for row in &module.field_refs {
        write_field_ref(&mut out, row)?;
    }

    // Write method defs — patch body_offset/body_size from computed values
    let mut body_idx = 0usize;
    for row in &module.method_defs {
        let (body_off, body_sz) = if body_idx < body_offsets.len() && row.body_size > 0 {
            let v = body_offsets[body_idx];
            body_idx += 1;
            v
        } else if body_idx < body_offsets.len() && body_idx < module.method_bodies.len() {
            let v = body_offsets[body_idx];
            body_idx += 1;
            v
        } else {
            (0, 0)
        };
        write_method_def_with_offset(&mut out, row, body_off, body_sz)?;
    }

    for row in &module.method_refs {
        write_method_ref(&mut out, row)?;
    }
    for row in &module.param_defs {
        write_param_def(&mut out, row)?;
    }
    for row in &module.contract_defs {
        write_contract_def(&mut out, row)?;
    }
    for row in &module.contract_methods {
        write_contract_method(&mut out, row)?;
    }
    for row in &module.impl_defs {
        write_impl_def(&mut out, row)?;
    }
    for row in &module.generic_params {
        write_generic_param(&mut out, row)?;
    }
    for row in &module.generic_constraints {
        write_generic_constraint(&mut out, row)?;
    }
    for row in &module.global_defs {
        write_global_def(&mut out, row)?;
    }
    for row in &module.extern_defs {
        write_extern_def(&mut out, row)?;
    }
    for row in &module.component_slots {
        write_component_slot(&mut out, row)?;
    }
    for row in &module.locale_defs {
        write_locale_def(&mut out, row)?;
    }
    for row in &module.export_defs {
        write_export_def(&mut out, row)?;
    }
    for row in &module.attribute_defs {
        write_attribute_def(&mut out, row)?;
    }

    // Phase 4: Write heaps verbatim
    out.extend_from_slice(&module.string_heap);
    out.extend_from_slice(&module.blob_heap);

    // Phase 5: Write method bodies
    let has_debug = (module.header.flags & 1) != 0;
    for body in &module.method_bodies {
        // Pad to 4-byte alignment
        while out.len() % 4 != 0 {
            out.push(0);
        }

        // Register types
        for &reg_type in &body.register_types {
            out.write_u32::<LittleEndian>(reg_type)?;
        }

        // Code
        out.write_u32::<LittleEndian>(body.code.len() as u32)?;
        out.extend_from_slice(&body.code);

        // Debug info
        if has_debug {
            out.write_u16::<LittleEndian>(body.debug_locals.len() as u16)?;
            for local in &body.debug_locals {
                out.write_u16::<LittleEndian>(local.register)?;
                out.write_u32::<LittleEndian>(local.name)?;
                out.write_u32::<LittleEndian>(local.start_pc)?;
                out.write_u32::<LittleEndian>(local.end_pc)?;
            }

            out.write_u32::<LittleEndian>(body.source_spans.len() as u32)?;
            for span in &body.source_spans {
                out.write_u32::<LittleEndian>(span.pc)?;
                out.write_u32::<LittleEndian>(span.line)?;
                out.write_u16::<LittleEndian>(span.column)?;
            }
        }
    }

    Ok(out)
}

fn compute_body_size(body: &crate::module::MethodBody, flags: u16) -> usize {
    let has_debug = (flags & 1) != 0;
    let mut size = 0;
    // register types
    size += body.register_types.len() * 4;
    // code_size (u32) + code bytes
    size += 4 + body.code.len();
    // debug info
    if has_debug {
        size += 2; // debug_local_count
        size += body.debug_locals.len() * 14; // u16 + u32 + u32 + u32 = 14 bytes each
        size += 4; // source_span_count
        size += body.source_spans.len() * 10; // u32 + u32 + u16 = 10 bytes each
    }
    size
}

// ── Row Writers ────────────────────────────────────────────────

fn write_module_def(out: &mut Vec<u8>, row: &ModuleDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.version)?;
    out.write_u32::<LittleEndian>(row.flags)?;
    Ok(())
}

fn write_module_ref(out: &mut Vec<u8>, row: &ModuleRefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.min_version)?;
    Ok(())
}

fn write_type_def(out: &mut Vec<u8>, row: &TypeDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.namespace)?;
    out.push(row.kind);
    out.write_u16::<LittleEndian>(row.flags)?;
    out.write_u32::<LittleEndian>(row.field_list)?;
    out.write_u32::<LittleEndian>(row.method_list)?;
    out.push(0); // padding to 20 bytes
    Ok(())
}

fn write_type_ref(out: &mut Vec<u8>, row: &TypeRefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.scope.0)?;
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.namespace)?;
    Ok(())
}

fn write_type_spec(out: &mut Vec<u8>, row: &TypeSpecRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.signature)?;
    Ok(())
}

fn write_field_def(out: &mut Vec<u8>, row: &FieldDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.type_sig)?;
    out.write_u16::<LittleEndian>(row.flags)?;
    out.write_u16::<LittleEndian>(0)?; // padding to 12 bytes
    Ok(())
}

fn write_field_ref(out: &mut Vec<u8>, row: &FieldRefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.parent.0)?;
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.type_sig)?;
    Ok(())
}

fn write_method_def_with_offset(out: &mut Vec<u8>, row: &MethodDefRow, body_offset: u32, body_size: u32) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.signature)?;
    out.write_u16::<LittleEndian>(row.flags)?;
    out.write_u32::<LittleEndian>(body_offset)?;
    out.write_u32::<LittleEndian>(body_size)?;
    out.write_u16::<LittleEndian>(row.reg_count)?;
    out.write_u16::<LittleEndian>(row.param_count)?;
    out.write_u16::<LittleEndian>(0)?; // 2-byte alignment pad
    Ok(())
}

fn write_method_ref(out: &mut Vec<u8>, row: &MethodRefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.parent.0)?;
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.signature)?;
    Ok(())
}

fn write_param_def(out: &mut Vec<u8>, row: &ParamDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.type_sig)?;
    out.write_u16::<LittleEndian>(row.sequence)?;
    out.write_u16::<LittleEndian>(0)?; // padding to 12
    Ok(())
}

fn write_contract_def(out: &mut Vec<u8>, row: &ContractDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.namespace)?;
    out.write_u32::<LittleEndian>(row.method_list)?;
    out.write_u32::<LittleEndian>(row.generic_param_list)?;
    Ok(())
}

fn write_contract_method(out: &mut Vec<u8>, row: &ContractMethodRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.signature)?;
    out.write_u16::<LittleEndian>(row.slot)?;
    out.write_u16::<LittleEndian>(0)?; // padding to 12
    Ok(())
}

fn write_impl_def(out: &mut Vec<u8>, row: &ImplDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.type_token.0)?;
    out.write_u32::<LittleEndian>(row.contract.0)?;
    out.write_u32::<LittleEndian>(row.method_list)?;
    Ok(())
}

fn write_generic_param(out: &mut Vec<u8>, row: &GenericParamRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.owner.0)?;
    out.push(row.owner_kind);
    out.write_u16::<LittleEndian>(row.ordinal)?;
    out.write_u32::<LittleEndian>(row.name)?;
    out.push(0); // padding to 12
    Ok(())
}

fn write_generic_constraint(out: &mut Vec<u8>, row: &GenericConstraintRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.param)?;
    out.write_u32::<LittleEndian>(row.constraint.0)?;
    Ok(())
}

fn write_global_def(out: &mut Vec<u8>, row: &GlobalDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.type_sig)?;
    out.write_u16::<LittleEndian>(row.flags)?;
    out.write_u32::<LittleEndian>(row.init_value)?;
    out.write_u16::<LittleEndian>(0)?; // padding to 16
    Ok(())
}

fn write_extern_def(out: &mut Vec<u8>, row: &ExternDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.signature)?;
    out.write_u32::<LittleEndian>(row.import_name)?;
    out.write_u16::<LittleEndian>(row.flags)?;
    out.write_u16::<LittleEndian>(0)?; // padding to 16
    Ok(())
}

fn write_component_slot(out: &mut Vec<u8>, row: &ComponentSlotRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.owner_entity.0)?;
    out.write_u32::<LittleEndian>(row.component_type.0)?;
    Ok(())
}

fn write_locale_def(out: &mut Vec<u8>, row: &LocaleDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.dlg_method.0)?;
    out.write_u32::<LittleEndian>(row.locale)?;
    out.write_u32::<LittleEndian>(row.loc_method.0)?;
    Ok(())
}

fn write_export_def(out: &mut Vec<u8>, row: &ExportDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.name)?;
    out.push(row.item_kind);
    out.write_u32::<LittleEndian>(row.item.0)?;
    out.push(0); // 1 byte padding
    out.write_u16::<LittleEndian>(0)?; // 2 bytes padding -> total 12
    Ok(())
}

fn write_attribute_def(out: &mut Vec<u8>, row: &AttributeDefRow) -> Result<(), EncodeError> {
    out.write_u32::<LittleEndian>(row.owner.0)?;
    out.push(row.owner_kind);
    out.write_u32::<LittleEndian>(row.name)?;
    out.write_u32::<LittleEndian>(row.value)?;
    out.push(0); // 1 byte padding
    out.write_u16::<LittleEndian>(0)?; // 2 bytes padding -> total 16
    Ok(())
}
