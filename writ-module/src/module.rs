use crate::error::{DecodeError, EncodeError};
use crate::heap;
use crate::tables::*;

/// The 200-byte header of a binary module file.
#[derive(Debug, Clone)]
pub struct ModuleHeader {
    pub format_version: u16,
    pub flags: u16,
    pub module_name: u32,    // string heap offset
    pub module_version: u32, // string heap offset
    pub string_heap_offset: u32,
    pub string_heap_size: u32,
    pub blob_heap_offset: u32,
    pub blob_heap_size: u32,
    /// Table directory: 21 entries of (offset, row_count).
    pub table_directory: [(u32, u32); 21],
}

/// A decoded method body.
#[derive(Debug, Clone)]
pub struct MethodBody {
    /// Blob heap offsets for per-register TypeRef encodings.
    pub register_types: Vec<u32>,
    /// Raw instruction bytes (used for byte-exact round-trip).
    pub code: Vec<u8>,
    /// Debug local variable info (present only if debug flag is set).
    pub debug_locals: Vec<DebugLocal>,
    /// Source span mappings (present only if debug flag is set).
    pub source_spans: Vec<SourceSpan>,
}

/// Debug info: maps a register to a named local variable within a PC range.
#[derive(Debug, Clone, PartialEq)]
pub struct DebugLocal {
    pub register: u16,
    pub name: u32, // string heap offset
    pub start_pc: u32,
    pub end_pc: u32,
}

/// Debug info: maps a PC offset to a source location.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceSpan {
    pub pc: u32,
    pub line: u32,
    pub column: u16,
}

/// Complete in-memory representation of an IL module.
#[derive(Debug, Clone)]
pub struct Module {
    pub header: ModuleHeader,

    // Heaps
    pub string_heap: Vec<u8>,
    pub blob_heap: Vec<u8>,

    // All 21 metadata tables
    pub module_defs: Vec<ModuleDefRow>,
    pub module_refs: Vec<ModuleRefRow>,
    pub type_defs: Vec<TypeDefRow>,
    pub type_refs: Vec<TypeRefRow>,
    pub type_specs: Vec<TypeSpecRow>,
    pub field_defs: Vec<FieldDefRow>,
    pub field_refs: Vec<FieldRefRow>,
    pub method_defs: Vec<MethodDefRow>,
    pub method_refs: Vec<MethodRefRow>,
    pub param_defs: Vec<ParamDefRow>,
    pub contract_defs: Vec<ContractDefRow>,
    pub contract_methods: Vec<ContractMethodRow>,
    pub impl_defs: Vec<ImplDefRow>,
    pub generic_params: Vec<GenericParamRow>,
    pub generic_constraints: Vec<GenericConstraintRow>,
    pub global_defs: Vec<GlobalDefRow>,
    pub extern_defs: Vec<ExternDefRow>,
    pub component_slots: Vec<ComponentSlotRow>,
    pub locale_defs: Vec<LocaleDefRow>,
    pub export_defs: Vec<ExportDefRow>,
    pub attribute_defs: Vec<AttributeDefRow>,

    // Method bodies (one per MethodDef with body_size > 0)
    pub method_bodies: Vec<MethodBody>,
}

impl Module {
    /// Create a new empty module with initialized heaps and format_version = 1.
    pub fn new() -> Self {
        Module {
            header: ModuleHeader {
                format_version: 2,
                flags: 0,
                module_name: 0,
                module_version: 0,
                string_heap_offset: 0,
                string_heap_size: 0,
                blob_heap_offset: 0,
                blob_heap_size: 0,
                table_directory: [(0, 0); 21],
            },
            string_heap: heap::init_string_heap(),
            blob_heap: heap::init_blob_heap(),
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
        }
    }

    /// Deserialize a module from spec-compliant bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Module, DecodeError> {
        crate::reader::from_bytes(bytes)
    }

    /// Serialize this module to spec-compliant bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        crate::writer::to_bytes(self)
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}
