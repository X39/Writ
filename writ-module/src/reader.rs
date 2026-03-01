use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::error::DecodeError;
use crate::module::{DebugLocal, MethodBody, Module, ModuleHeader, SourceSpan};
use crate::tables::*;
use crate::token::MetadataToken;

/// Header size in bytes: 8 (magic/version/flags) + 8 (name/version) + 16 (heaps) + 168 (21 * 8 table dir) = 200
const HEADER_SIZE: usize = 200;

/// Aligned row sizes for each table (4-byte aligned per spec section 2.16.1).
const ROW_SIZES: [usize; 21] = [
    12, // 0  ModuleDef: u32+u32+u32 = 12
    8,  // 1  ModuleRef: u32+u32 = 8
    20, // 2  TypeDef: u32+u32+u8+u16+u32+u32 = 19 -> pad to 20
    12, // 3  TypeRef: u32+u32+u32 = 12
    4,  // 4  TypeSpec: u32 = 4
    12, // 5  FieldDef: u32+u32+u16 = 10 -> pad to 12
    12, // 6  FieldRef: u32+u32+u32 = 12
    24, // 7  MethodDef: u32+u32+u16+u32+u32+u16+u16 = 22 -> pad to 24
    12, // 8  MethodRef: u32+u32+u32 = 12
    12, // 9  ParamDef: u32+u32+u16 = 10 -> pad to 12
    16, // 10 ContractDef: u32+u32+u32+u32 = 16
    12, // 11 ContractMethod: u32+u32+u16 = 10 -> pad to 12
    12, // 12 ImplDef: u32+u32+u32 = 12
    12, // 13 GenericParam: u32+u8+u16+u32 = 11 -> pad to 12
    8,  // 14 GenericConstraint: u32+u32 = 8
    16, // 15 GlobalDef: u32+u32+u16+u32 = 14 -> pad to 16
    16, // 16 ExternDef: u32+u32+u32+u16 = 14 -> pad to 16
    8,  // 17 ComponentSlot: u32+u32 = 8
    12, // 18 LocaleDef: u32+u32+u32 = 12
    12, // 19 ExportDef: u32+u8+u32 = 9 -> pad to 12
    16, // 20 AttributeDef: u32+u8+u32+u32 = 13 -> pad to 16
];

pub fn row_size(table_id: u8) -> usize {
    ROW_SIZES[table_id as usize]
}

pub fn from_bytes(bytes: &[u8]) -> Result<Module, DecodeError> {
    if bytes.len() < HEADER_SIZE {
        return Err(DecodeError::UnexpectedEof);
    }

    let mut cur = Cursor::new(bytes);

    // Magic
    let mut magic = [0u8; 4];
    std::io::Read::read_exact(&mut cur, &mut magic)?;
    if magic != *b"WRIT" {
        return Err(DecodeError::BadMagic(magic));
    }

    // Version and flags
    let format_version = cur.read_u16::<LittleEndian>()?;
    let flags = cur.read_u16::<LittleEndian>()?;

    // Module name/version offsets
    let module_name = cur.read_u32::<LittleEndian>()?;
    let module_version = cur.read_u32::<LittleEndian>()?;

    // Heap offsets/sizes
    let string_heap_offset = cur.read_u32::<LittleEndian>()?;
    let string_heap_size = cur.read_u32::<LittleEndian>()?;
    let blob_heap_offset = cur.read_u32::<LittleEndian>()?;
    let blob_heap_size = cur.read_u32::<LittleEndian>()?;

    // Table directory: 21 entries of (offset, row_count)
    let mut table_directory = [(0u32, 0u32); 21];
    for entry in table_directory.iter_mut() {
        entry.0 = cur.read_u32::<LittleEndian>()?;
        entry.1 = cur.read_u32::<LittleEndian>()?;
    }

    let header = ModuleHeader {
        format_version,
        flags,
        module_name,
        module_version,
        string_heap_offset,
        string_heap_size,
        blob_heap_offset,
        blob_heap_size,
        table_directory,
    };

    // Read heaps
    let string_heap = read_slice(bytes, string_heap_offset as usize, string_heap_size as usize)?;
    let blob_heap = read_slice(bytes, blob_heap_offset as usize, blob_heap_size as usize)?;

    // Read tables
    let mut module = Module {
        header,
        string_heap,
        blob_heap,
        module_defs: Vec::new(),
        module_refs: Vec::new(),
        type_defs: Vec::new(),
        type_refs: Vec::new(),
        type_specs: Vec::new(),
        field_defs: Vec::new(),
        field_refs: Vec::new(),
        method_defs: Vec::new(),
        method_refs: Vec::new(),
        param_defs: Vec::new(),
        contract_defs: Vec::new(),
        contract_methods: Vec::new(),
        impl_defs: Vec::new(),
        generic_params: Vec::new(),
        generic_constraints: Vec::new(),
        global_defs: Vec::new(),
        extern_defs: Vec::new(),
        component_slots: Vec::new(),
        locale_defs: Vec::new(),
        export_defs: Vec::new(),
        attribute_defs: Vec::new(),
        method_bodies: Vec::new(),
    };

    for table_id in 0..21u8 {
        let (offset, row_count) = module.header.table_directory[table_id as usize];
        if row_count == 0 {
            continue;
        }
        let offset = offset as usize;
        let rs = ROW_SIZES[table_id as usize];

        for i in 0..row_count {
            let row_start = offset + (i as usize) * rs;
            if row_start + rs > bytes.len() {
                return Err(DecodeError::UnexpectedEof);
            }
            let row_bytes = &bytes[row_start..row_start + rs];
            let mut c = Cursor::new(row_bytes);

            match table_id {
                0 => module.module_defs.push(read_module_def(&mut c)?),
                1 => module.module_refs.push(read_module_ref(&mut c)?),
                2 => module.type_defs.push(read_type_def(&mut c)?),
                3 => module.type_refs.push(read_type_ref(&mut c)?),
                4 => module.type_specs.push(read_type_spec(&mut c)?),
                5 => module.field_defs.push(read_field_def(&mut c)?),
                6 => module.field_refs.push(read_field_ref(&mut c)?),
                7 => module.method_defs.push(read_method_def(&mut c)?),
                8 => module.method_refs.push(read_method_ref(&mut c)?),
                9 => module.param_defs.push(read_param_def(&mut c)?),
                10 => module.contract_defs.push(read_contract_def(&mut c)?),
                11 => module.contract_methods.push(read_contract_method(&mut c)?),
                12 => module.impl_defs.push(read_impl_def(&mut c)?),
                13 => module.generic_params.push(read_generic_param(&mut c)?),
                14 => module.generic_constraints.push(read_generic_constraint(&mut c)?),
                15 => module.global_defs.push(read_global_def(&mut c)?),
                16 => module.extern_defs.push(read_extern_def(&mut c)?),
                17 => module.component_slots.push(read_component_slot(&mut c)?),
                18 => module.locale_defs.push(read_locale_def(&mut c)?),
                19 => module.export_defs.push(read_export_def(&mut c)?),
                20 => module.attribute_defs.push(read_attribute_def(&mut c)?),
                _ => return Err(DecodeError::InvalidTableId(table_id)),
            }
        }
    }

    // Read method bodies
    let has_debug = (flags & 1) != 0;
    for method in &module.method_defs {
        if method.body_size == 0 {
            continue;
        }
        let body_start = method.body_offset as usize;
        let body_end = body_start + method.body_size as usize;
        if body_end > bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let body_bytes = &bytes[body_start..body_end];
        let mut c = Cursor::new(body_bytes);

        // Read register types
        let reg_count = method.reg_count as usize;
        let mut register_types = Vec::with_capacity(reg_count);
        for _ in 0..reg_count {
            register_types.push(c.read_u32::<LittleEndian>()?);
        }

        // Read code
        let code_size = c.read_u32::<LittleEndian>()? as usize;
        let code_start = c.position() as usize;
        if code_start + code_size > body_bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let code = body_bytes[code_start..code_start + code_size].to_vec();
        c.set_position((code_start + code_size) as u64);

        // Read debug info if present
        let mut debug_locals = Vec::new();
        let mut source_spans = Vec::new();

        if has_debug && (c.position() as usize) < body_bytes.len() {
            let debug_local_count = c.read_u16::<LittleEndian>()? as usize;
            for _ in 0..debug_local_count {
                debug_locals.push(DebugLocal {
                    register: c.read_u16::<LittleEndian>()?,
                    name: c.read_u32::<LittleEndian>()?,
                    start_pc: c.read_u32::<LittleEndian>()?,
                    end_pc: c.read_u32::<LittleEndian>()?,
                });
            }

            let source_span_count = c.read_u32::<LittleEndian>()? as usize;
            for _ in 0..source_span_count {
                source_spans.push(SourceSpan {
                    pc: c.read_u32::<LittleEndian>()?,
                    line: c.read_u32::<LittleEndian>()?,
                    column: c.read_u16::<LittleEndian>()?,
                });
            }
        }

        module.method_bodies.push(MethodBody {
            register_types,
            code,
            debug_locals,
            source_spans,
        });
    }

    Ok(module)
}

fn read_slice(bytes: &[u8], offset: usize, size: usize) -> Result<Vec<u8>, DecodeError> {
    if size == 0 {
        return Ok(Vec::new());
    }
    if offset + size > bytes.len() {
        return Err(DecodeError::UnexpectedEof);
    }
    Ok(bytes[offset..offset + size].to_vec())
}

// ── Row Readers ────────────────────────────────────────────────

fn read_module_def(c: &mut Cursor<&[u8]>) -> Result<ModuleDefRow, DecodeError> {
    Ok(ModuleDefRow {
        name: c.read_u32::<LittleEndian>()?,
        version: c.read_u32::<LittleEndian>()?,
        flags: c.read_u32::<LittleEndian>()?,
    })
}

fn read_module_ref(c: &mut Cursor<&[u8]>) -> Result<ModuleRefRow, DecodeError> {
    Ok(ModuleRefRow {
        name: c.read_u32::<LittleEndian>()?,
        min_version: c.read_u32::<LittleEndian>()?,
    })
}

fn read_type_def(c: &mut Cursor<&[u8]>) -> Result<TypeDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let namespace = c.read_u32::<LittleEndian>()?;
    let kind = c.read_u8()?;
    let flags = c.read_u16::<LittleEndian>()?;
    let field_list = c.read_u32::<LittleEndian>()?;
    let method_list = c.read_u32::<LittleEndian>()?;
    // 1 byte padding to reach 20 bytes
    let _ = c.read_u8()?;
    Ok(TypeDefRow { name, namespace, kind, flags, field_list, method_list })
}

fn read_type_ref(c: &mut Cursor<&[u8]>) -> Result<TypeRefRow, DecodeError> {
    Ok(TypeRefRow {
        scope: MetadataToken(c.read_u32::<LittleEndian>()?),
        name: c.read_u32::<LittleEndian>()?,
        namespace: c.read_u32::<LittleEndian>()?,
    })
}

fn read_type_spec(c: &mut Cursor<&[u8]>) -> Result<TypeSpecRow, DecodeError> {
    Ok(TypeSpecRow { signature: c.read_u32::<LittleEndian>()? })
}

fn read_field_def(c: &mut Cursor<&[u8]>) -> Result<FieldDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let type_sig = c.read_u32::<LittleEndian>()?;
    let flags = c.read_u16::<LittleEndian>()?;
    // 2 bytes padding to reach 12 bytes
    let _ = c.read_u16::<LittleEndian>()?;
    Ok(FieldDefRow { name, type_sig, flags })
}

fn read_field_ref(c: &mut Cursor<&[u8]>) -> Result<FieldRefRow, DecodeError> {
    Ok(FieldRefRow {
        parent: MetadataToken(c.read_u32::<LittleEndian>()?),
        name: c.read_u32::<LittleEndian>()?,
        type_sig: c.read_u32::<LittleEndian>()?,
    })
}

fn read_method_def(c: &mut Cursor<&[u8]>) -> Result<MethodDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let signature = c.read_u32::<LittleEndian>()?;
    let flags = c.read_u16::<LittleEndian>()?;
    let body_offset = c.read_u32::<LittleEndian>()?;
    let body_size = c.read_u32::<LittleEndian>()?;
    let reg_count = c.read_u16::<LittleEndian>()?;
    let param_count = c.read_u16::<LittleEndian>()?;
    let _ = c.read_u16::<LittleEndian>()?; // 2-byte alignment pad
    Ok(MethodDefRow { name, signature, flags, body_offset, body_size, reg_count, param_count })
}

fn read_method_ref(c: &mut Cursor<&[u8]>) -> Result<MethodRefRow, DecodeError> {
    Ok(MethodRefRow {
        parent: MetadataToken(c.read_u32::<LittleEndian>()?),
        name: c.read_u32::<LittleEndian>()?,
        signature: c.read_u32::<LittleEndian>()?,
    })
}

fn read_param_def(c: &mut Cursor<&[u8]>) -> Result<ParamDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let type_sig = c.read_u32::<LittleEndian>()?;
    let sequence = c.read_u16::<LittleEndian>()?;
    let _ = c.read_u16::<LittleEndian>()?; // padding
    Ok(ParamDefRow { name, type_sig, sequence })
}

fn read_contract_def(c: &mut Cursor<&[u8]>) -> Result<ContractDefRow, DecodeError> {
    Ok(ContractDefRow {
        name: c.read_u32::<LittleEndian>()?,
        namespace: c.read_u32::<LittleEndian>()?,
        method_list: c.read_u32::<LittleEndian>()?,
        generic_param_list: c.read_u32::<LittleEndian>()?,
    })
}

fn read_contract_method(c: &mut Cursor<&[u8]>) -> Result<ContractMethodRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let signature = c.read_u32::<LittleEndian>()?;
    let slot = c.read_u16::<LittleEndian>()?;
    let _ = c.read_u16::<LittleEndian>()?; // padding
    Ok(ContractMethodRow { name, signature, slot })
}

fn read_impl_def(c: &mut Cursor<&[u8]>) -> Result<ImplDefRow, DecodeError> {
    Ok(ImplDefRow {
        type_token: MetadataToken(c.read_u32::<LittleEndian>()?),
        contract: MetadataToken(c.read_u32::<LittleEndian>()?),
        method_list: c.read_u32::<LittleEndian>()?,
    })
}

fn read_generic_param(c: &mut Cursor<&[u8]>) -> Result<GenericParamRow, DecodeError> {
    let owner = MetadataToken(c.read_u32::<LittleEndian>()?);
    let owner_kind = c.read_u8()?;
    let ordinal = c.read_u16::<LittleEndian>()?;
    let name = c.read_u32::<LittleEndian>()?;
    let _ = c.read_u8()?; // padding
    Ok(GenericParamRow { owner, owner_kind, ordinal, name })
}

fn read_generic_constraint(c: &mut Cursor<&[u8]>) -> Result<GenericConstraintRow, DecodeError> {
    Ok(GenericConstraintRow {
        param: c.read_u32::<LittleEndian>()?,
        constraint: MetadataToken(c.read_u32::<LittleEndian>()?),
    })
}

fn read_global_def(c: &mut Cursor<&[u8]>) -> Result<GlobalDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let type_sig = c.read_u32::<LittleEndian>()?;
    let flags = c.read_u16::<LittleEndian>()?;
    let init_value = c.read_u32::<LittleEndian>()?;
    let _ = c.read_u16::<LittleEndian>()?; // padding
    Ok(GlobalDefRow { name, type_sig, flags, init_value })
}

fn read_extern_def(c: &mut Cursor<&[u8]>) -> Result<ExternDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let signature = c.read_u32::<LittleEndian>()?;
    let import_name = c.read_u32::<LittleEndian>()?;
    let flags = c.read_u16::<LittleEndian>()?;
    let _ = c.read_u16::<LittleEndian>()?; // padding
    Ok(ExternDefRow { name, signature, import_name, flags })
}

fn read_component_slot(c: &mut Cursor<&[u8]>) -> Result<ComponentSlotRow, DecodeError> {
    Ok(ComponentSlotRow {
        owner_entity: MetadataToken(c.read_u32::<LittleEndian>()?),
        component_type: MetadataToken(c.read_u32::<LittleEndian>()?),
    })
}

fn read_locale_def(c: &mut Cursor<&[u8]>) -> Result<LocaleDefRow, DecodeError> {
    Ok(LocaleDefRow {
        dlg_method: MetadataToken(c.read_u32::<LittleEndian>()?),
        locale: c.read_u32::<LittleEndian>()?,
        loc_method: MetadataToken(c.read_u32::<LittleEndian>()?),
    })
}

fn read_export_def(c: &mut Cursor<&[u8]>) -> Result<ExportDefRow, DecodeError> {
    let name = c.read_u32::<LittleEndian>()?;
    let item_kind = c.read_u8()?;
    let item = MetadataToken(c.read_u32::<LittleEndian>()?);
    let _ = c.read_u8()?; // padding (1 byte)
    let _ = c.read_u16::<LittleEndian>()?; // padding (2 bytes) -> total 12
    Ok(ExportDefRow { name, item_kind, item })
}

fn read_attribute_def(c: &mut Cursor<&[u8]>) -> Result<AttributeDefRow, DecodeError> {
    let owner = MetadataToken(c.read_u32::<LittleEndian>()?);
    let owner_kind = c.read_u8()?;
    let name = c.read_u32::<LittleEndian>()?;
    let value = c.read_u32::<LittleEndian>()?;
    let _ = c.read_u8()?; // padding (1 byte)
    let _ = c.read_u16::<LittleEndian>()?; // padding (2 bytes) -> total 16
    Ok(AttributeDefRow { owner, owner_kind, name, value })
}
