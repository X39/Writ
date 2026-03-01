use crate::token::MetadataToken;

/// TypeDef kind discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeDefKind {
    Struct = 0,
    Enum = 1,
    Entity = 2,
    Component = 3,
}

impl TypeDefKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Struct),
            1 => Some(Self::Enum),
            2 => Some(Self::Entity),
            3 => Some(Self::Component),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Table ID enum with explicit discriminants matching the spec table directory order (0-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Number of metadata tables defined by the spec.
    pub const COUNT: usize = 21;

    pub fn from_u8(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::ModuleDef),
            1 => Some(Self::ModuleRef),
            2 => Some(Self::TypeDef),
            3 => Some(Self::TypeRef),
            4 => Some(Self::TypeSpec),
            5 => Some(Self::FieldDef),
            6 => Some(Self::FieldRef),
            7 => Some(Self::MethodDef),
            8 => Some(Self::MethodRef),
            9 => Some(Self::ParamDef),
            10 => Some(Self::ContractDef),
            11 => Some(Self::ContractMethod),
            12 => Some(Self::ImplDef),
            13 => Some(Self::GenericParam),
            14 => Some(Self::GenericConstraint),
            15 => Some(Self::GlobalDef),
            16 => Some(Self::ExternDef),
            17 => Some(Self::ComponentSlot),
            18 => Some(Self::LocaleDef),
            19 => Some(Self::ExportDef),
            20 => Some(Self::AttributeDef),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ──── Table Row Structs ────────────────────────────────────────────────

/// Table 0: Module identity (always 1 row).
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefRow {
    pub name: u32,    // string heap offset
    pub version: u32, // string heap offset
    pub flags: u32,
}

/// Table 1: Dependencies on other modules.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleRefRow {
    pub name: u32,        // string heap offset
    pub min_version: u32, // string heap offset
}

/// Table 2: Types defined in this module.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDefRow {
    pub name: u32,        // string heap offset
    pub namespace: u32,   // string heap offset
    pub kind: u8,         // TypeDefKind discriminant
    pub flags: u16,
    pub field_list: u32,  // index of first FieldDef row (1-based)
    pub method_list: u32, // index of first MethodDef row (1-based)
}

/// Table 3: Types in other modules (resolved at load time).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRefRow {
    pub scope: MetadataToken, // ModuleRef token
    pub name: u32,            // string heap offset
    pub namespace: u32,       // string heap offset
}

/// Table 4: Instantiated generic types.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeSpecRow {
    pub signature: u32, // blob heap offset
}

/// Table 5: Fields on types defined here.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefRow {
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
    pub flags: u16,
}

/// Table 6: Fields in other modules (resolved at load time).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldRefRow {
    pub parent: MetadataToken,
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
}

/// Table 7: Methods/functions defined here.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDefRow {
    pub name: u32,        // string heap offset
    pub signature: u32,   // blob heap offset
    pub flags: u16,
    pub body_offset: u32,
    pub body_size: u32,
    pub reg_count: u16,
    pub param_count: u16, // count of parameter registers r0..r(param_count-1)
}

/// Table 8: Methods in other modules (resolved at load time).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodRefRow {
    pub parent: MetadataToken,
    pub name: u32,      // string heap offset
    pub signature: u32, // blob heap offset
}

/// Table 9: Method parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDefRow {
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
    pub sequence: u16,
}

/// Table 10: Contract declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDefRow {
    pub name: u32,              // string heap offset
    pub namespace: u32,         // string heap offset
    pub method_list: u32,       // index of first ContractMethod row
    pub generic_param_list: u32, // index of first GenericParam row
}

/// Table 11: Method slots within a contract.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractMethodRow {
    pub name: u32,      // string heap offset
    pub signature: u32, // blob heap offset
    pub slot: u16,
}

/// Table 12: Contract implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDefRow {
    pub type_token: MetadataToken,
    pub contract: MetadataToken,
    pub method_list: u32, // index of first MethodDef row for this impl
}

/// Table 13: Type parameters on types/methods.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParamRow {
    pub owner: MetadataToken,
    pub owner_kind: u8,
    pub ordinal: u16,
    pub name: u32, // string heap offset
}

/// Table 14: Bounds on type parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericConstraintRow {
    pub param: u32, // GenericParam row index
    pub constraint: MetadataToken,
}

/// Table 15: Constants and `global mut` variables.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDefRow {
    pub name: u32,       // string heap offset
    pub type_sig: u32,   // blob heap offset
    pub flags: u16,
    pub init_value: u32, // blob heap offset
}

/// Table 16: Extern function/type declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternDefRow {
    pub name: u32,        // string heap offset
    pub signature: u32,   // blob heap offset
    pub import_name: u32, // string heap offset
    pub flags: u16,
}

/// Table 17: Entity -> component bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentSlotRow {
    pub owner_entity: MetadataToken,
    pub component_type: MetadataToken,
}

/// Table 18: Dialogue locale dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct LocaleDefRow {
    pub dlg_method: MetadataToken,
    pub locale: u32, // string heap offset
    pub loc_method: MetadataToken,
}

/// Table 19: Convenience index of pub-visible items.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportDefRow {
    pub name: u32, // string heap offset
    pub item_kind: u8,
    pub item: MetadataToken,
}

/// Table 20: Metadata attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeDefRow {
    pub owner: MetadataToken,
    pub owner_kind: u8,
    pub name: u32,  // string heap offset
    pub value: u32, // blob heap offset
}
