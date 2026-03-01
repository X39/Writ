//! Metadata types for the IL module format.
//!
//! Defines `MetadataToken`, `TableId`, and row structs for all 21 metadata tables
//! per spec section 2.16.

/// Table identifiers matching the table directory order in spec 2.16.5.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TableId {
    ModuleDef = 0,
    ModuleRef = 1,
    TypeDef = 2,
    TypeRef = 3,
    TypeSpec = 4,
    FieldDef = 5,
    FieldRef = 6,
    MethodDef = 7,
    MethodRef = 8,
    ParamDef = 9,
    ContractDef = 10,
    ContractMethod = 11,
    ImplDef = 12,
    GenericParam = 13,
    GenericConstraint = 14,
    GlobalDef = 15,
    ExternDef = 16,
    ComponentSlot = 17,
    LocaleDef = 18,
    ExportDef = 19,
    AttributeDef = 20,
}

impl TableId {
    /// Convert from raw u8 value. Returns None if out of range.
    pub fn from_u8(v: u8) -> Option<TableId> {
        match v {
            0 => Some(TableId::ModuleDef),
            1 => Some(TableId::ModuleRef),
            2 => Some(TableId::TypeDef),
            3 => Some(TableId::TypeRef),
            4 => Some(TableId::TypeSpec),
            5 => Some(TableId::FieldDef),
            6 => Some(TableId::FieldRef),
            7 => Some(TableId::MethodDef),
            8 => Some(TableId::MethodRef),
            9 => Some(TableId::ParamDef),
            10 => Some(TableId::ContractDef),
            11 => Some(TableId::ContractMethod),
            12 => Some(TableId::ImplDef),
            13 => Some(TableId::GenericParam),
            14 => Some(TableId::GenericConstraint),
            15 => Some(TableId::GlobalDef),
            16 => Some(TableId::ExternDef),
            17 => Some(TableId::ComponentSlot),
            18 => Some(TableId::LocaleDef),
            19 => Some(TableId::ExportDef),
            20 => Some(TableId::AttributeDef),
            _ => None,
        }
    }
}

/// A metadata token encoding both table ID and row index.
///
/// Spec 2.16.4: Bits 31-24 = table ID (0-20), Bits 23-0 = row index (1-based; 0 = null).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetadataToken(pub u32);

impl MetadataToken {
    /// The null token (row index 0).
    pub const NULL: MetadataToken = MetadataToken(0);

    /// Create a new token from a table ID and 1-based row index.
    pub fn new(table: TableId, row: u32) -> Self {
        debug_assert!(row > 0, "MetadataToken row must be 1-based (got 0)");
        debug_assert!(
            row <= 0x00FF_FFFF,
            "MetadataToken row exceeds 24-bit maximum"
        );
        MetadataToken((table as u32) << 24 | row)
    }

    /// Extract the table ID.
    pub fn table(self) -> TableId {
        let id = (self.0 >> 24) as u8;
        TableId::from_u8(id).expect("invalid table ID in token")
    }

    /// Extract the 1-based row index (0 for null token).
    pub fn row(self) -> u32 {
        self.0 & 0x00FF_FFFF
    }

    /// Check if this is the null token.
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

// =============================================================================
// TypeDef kind and hook kind enums
// =============================================================================

/// TypeDef.kind values per spec 2.16.5.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeDefKind {
    Struct = 0,
    Enum = 1,
    Entity = 2,
    Component = 3,
}

/// MethodDef hook_kind values per spec 2.16.5.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HookKind {
    None = 0,
    Create = 1,
    Destroy = 2,
    Finalize = 3,
    Serialize = 4,
    Deserialize = 5,
    Interact = 6,
}

impl HookKind {
    /// Parse a hook event name to a HookKind.
    pub fn from_event_name(name: &str) -> HookKind {
        match name {
            "create" => HookKind::Create,
            "destroy" => HookKind::Destroy,
            "finalize" => HookKind::Finalize,
            "serialize" => HookKind::Serialize,
            "deserialize" => HookKind::Deserialize,
            "interact" => HookKind::Interact,
            _ => HookKind::None,
        }
    }
}

// =============================================================================
// Flag encoding helpers
// =============================================================================

/// Pack MethodDef flags into a u16.
///
/// Layout: bit 0 = is_pub, bit 1 = is_static, bit 2 = is_mut_self,
///         bits 3-5 = hook_kind (0-6), bit 6 = intrinsic.
pub fn method_flags(is_pub: bool, is_static: bool, is_mut_self: bool, hook: HookKind) -> u16 {
    let mut flags: u16 = 0;
    if is_pub {
        flags |= 1 << 0;
    }
    if is_static {
        flags |= 1 << 1;
    }
    if is_mut_self {
        flags |= 1 << 2;
    }
    flags |= (hook as u16) << 3;
    flags
}

/// Extract hook_kind from MethodDef flags.
pub fn extract_hook_kind(flags: u16) -> HookKind {
    let raw = ((flags >> 3) & 0x07) as u8;
    match raw {
        0 => HookKind::None,
        1 => HookKind::Create,
        2 => HookKind::Destroy,
        3 => HookKind::Finalize,
        4 => HookKind::Serialize,
        5 => HookKind::Deserialize,
        6 => HookKind::Interact,
        _ => HookKind::None,
    }
}

/// Pack FieldDef flags into a u16.
///
/// Layout: bit 0 = is_pub, bit 1 = has_default, bit 2 = is_component_field.
pub fn field_flags(is_pub: bool, has_default: bool, is_component_field: bool) -> u16 {
    let mut flags: u16 = 0;
    if is_pub {
        flags |= 1 << 0;
    }
    if has_default {
        flags |= 1 << 1;
    }
    if is_component_field {
        flags |= 1 << 2;
    }
    flags
}

// =============================================================================
// Row structs for all 21 metadata tables
// =============================================================================

/// Table 0: ModuleDef — Module identity (always 1 row).
#[derive(Debug, Clone)]
pub struct ModuleDefRow {
    pub name: u32,    // string heap offset
    pub version: u32, // string heap offset
    pub flags: u32,
}

/// Table 1: ModuleRef — Dependencies on other modules.
#[derive(Debug, Clone)]
pub struct ModuleRefRow {
    pub name: u32,        // string heap offset
    pub min_version: u32, // string heap offset
}

/// Table 2: TypeDef — Types defined in this module.
#[derive(Debug, Clone)]
pub struct TypeDefRow {
    pub name: u32,      // string heap offset
    pub namespace: u32,  // string heap offset
    pub kind: u8,        // TypeDefKind
    pub flags: u16,
    pub field_list: u32, // first FieldDef row index (1-based)
    pub method_list: u32, // first MethodDef row index (1-based)
}

/// Table 3: TypeRef — Types in other modules.
#[derive(Debug, Clone)]
pub struct TypeRefRow {
    pub scope: MetadataToken, // ModuleRef token
    pub name: u32,            // string heap offset
    pub namespace: u32,       // string heap offset
}

/// Table 4: TypeSpec — Instantiated generic types.
#[derive(Debug, Clone)]
pub struct TypeSpecRow {
    pub signature: u32, // blob heap offset
}

/// Table 5: FieldDef — Fields on types defined here.
#[derive(Debug, Clone)]
pub struct FieldDefRow {
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
    pub flags: u16,
}

/// Table 6: FieldRef — Fields in other modules.
#[derive(Debug, Clone)]
pub struct FieldRefRow {
    pub parent: MetadataToken,
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
}

/// Table 7: MethodDef — Methods/functions defined here.
#[derive(Debug, Clone)]
pub struct MethodDefRow {
    pub name: u32,        // string heap offset
    pub signature: u32,   // blob heap offset
    pub flags: u16,
    pub body_offset: u32,
    pub body_size: u32,
    pub reg_count: u16,
    pub param_count: u16, // count of parameter registers r0..r(param_count-1)
}

/// Table 8: MethodRef — Methods in other modules.
#[derive(Debug, Clone)]
pub struct MethodRefRow {
    pub parent: MetadataToken,
    pub name: u32,      // string heap offset
    pub signature: u32, // blob heap offset
}

/// Table 9: ParamDef — Method parameters.
#[derive(Debug, Clone)]
pub struct ParamDefRow {
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
    pub sequence: u16,
}

/// Table 10: ContractDef — Contract declarations.
#[derive(Debug, Clone)]
pub struct ContractDefRow {
    pub name: u32,             // string heap offset
    pub namespace: u32,        // string heap offset
    pub method_list: u32,      // first ContractMethod row index
    pub generic_param_list: u32, // first GenericParam row index
}

/// Table 11: ContractMethod — Method slots within a contract.
#[derive(Debug, Clone)]
pub struct ContractMethodRow {
    pub name: u32,      // string heap offset
    pub signature: u32, // blob heap offset
    pub slot: u16,
}

/// Table 12: ImplDef — Contract implementations.
#[derive(Debug, Clone)]
pub struct ImplDefRow {
    pub type_token: MetadataToken,
    pub contract_token: MetadataToken,
    pub method_list: u32, // first impl MethodDef row index
}

/// Table 13: GenericParam — Type parameters on types/methods.
#[derive(Debug, Clone)]
pub struct GenericParamRow {
    pub owner: MetadataToken,
    pub owner_kind: u8, // 0=TypeDef, 1=MethodDef, 2=ContractDef
    pub ordinal: u16,
    pub name: u32, // string heap offset
}

/// Table 14: GenericConstraint — Bounds on type parameters.
#[derive(Debug, Clone)]
pub struct GenericConstraintRow {
    pub param_row: u32,           // GenericParam row index (1-based)
    pub constraint: MetadataToken, // Contract token
}

/// Table 15: GlobalDef — Constants and `global mut` variables.
#[derive(Debug, Clone)]
pub struct GlobalDefRow {
    pub name: u32,       // string heap offset
    pub type_sig: u32,   // blob heap offset
    pub flags: u16,
    pub init_value: u32, // blob heap offset
}

/// Table 16: ExternDef — Extern function/type declarations.
#[derive(Debug, Clone)]
pub struct ExternDefRow {
    pub name: u32,        // string heap offset
    pub signature: u32,   // blob heap offset
    pub import_name: u32, // string heap offset
    pub flags: u16,
}

/// Table 17: ComponentSlot — Entity to component bindings.
#[derive(Debug, Clone)]
pub struct ComponentSlotRow {
    pub owner_entity: MetadataToken, // TypeDef token
    pub component_type: MetadataToken, // TypeDef token
}

/// Table 18: LocaleDef — Dialogue locale dispatch.
#[derive(Debug, Clone)]
pub struct LocaleDefRow {
    pub dlg_method: MetadataToken,  // MethodDef token
    pub locale: u32,                // string heap offset
    pub loc_method: MetadataToken,  // MethodDef token
}

/// Table 19: ExportDef — Convenience index of pub-visible items.
#[derive(Debug, Clone)]
pub struct ExportDefRow {
    pub name: u32,         // string heap offset
    pub item_kind: u8,     // 0=type, 1=method, 2=field
    pub item: MetadataToken,
}

/// Table 20: AttributeDef — Metadata attributes ([Singleton], etc.).
#[derive(Debug, Clone)]
pub struct AttributeDefRow {
    pub owner: MetadataToken,
    pub owner_kind: u8,    // 0=type, 1=method, 2=field
    pub name: u32,         // string heap offset
    pub value: u32,        // blob heap offset
}
