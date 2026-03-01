// Writ IL Module Format - ImHex Pattern
// Spec reference: language-spec/spec/44-45 (type system + module format)
// Format: little-endian throughout, 200-byte header, 21 metadata tables,
//         string/blob heaps, method bodies with optional debug info.

#pragma endian little
#pragma pattern_limit 1000000

import std.io;
import std.mem;
import std.string;

// ============================================================================
// Constants
// ============================================================================

#define HEADER_SIZE 200
#define TABLE_COUNT 21

// ============================================================================
// Enums
// ============================================================================

enum TypeDefKind : u8 {
    Struct    = 0,
    Enum      = 1,
    Entity    = 2,
    Component = 3,
};

enum HookKind : u8 {
    None        = 0,
    Create      = 1,
    Destroy     = 2,
    Finalize    = 3,
    Serialize   = 4,
    Deserialize = 5,
    Interact    = 6,
};

enum TypeRefKind : u8 {
    Void      = 0x00,
    Int       = 0x01,
    Float     = 0x02,
    Bool      = 0x03,
    String    = 0x04,
    TypeDef   = 0x10,
    TypeSpec  = 0x11,
    GenParam  = 0x12,
    Array     = 0x20,
    FuncType  = 0x30,
};

enum ExportItemKind : u8 {
    Type   = 0,
    Method = 1,
    Field  = 2,
};

enum OwnerKind : u8 {
    Type   = 0,
    Method = 1,
    Field  = 2,
};

enum GenericParamOwnerKind : u8 {
    TypeDef     = 0,
    MethodDef   = 1,
    ContractDef = 2,
};

// Opcode enum (u16) - high byte = category, low byte = instruction
enum Opcode : u16 {
    // 0x00 Meta
    NOP           = 0x0000,
    CRASH         = 0x0001,
    // 0x01 Data Movement
    MOV           = 0x0100,
    LOAD_INT      = 0x0101,
    LOAD_FLOAT    = 0x0102,
    LOAD_TRUE     = 0x0103,
    LOAD_FALSE    = 0x0104,
    LOAD_STRING   = 0x0105,
    LOAD_NULL     = 0x0106,
    // 0x02 Integer Arithmetic
    ADD_I         = 0x0200,
    SUB_I         = 0x0201,
    MUL_I         = 0x0202,
    DIV_I         = 0x0203,
    MOD_I         = 0x0204,
    NEG_I         = 0x0205,
    // 0x03 Float Arithmetic
    ADD_F         = 0x0300,
    SUB_F         = 0x0301,
    MUL_F         = 0x0302,
    DIV_F         = 0x0303,
    MOD_F         = 0x0304,
    NEG_F         = 0x0305,
    // 0x04 Bitwise & Logical
    BIT_AND       = 0x0400,
    BIT_OR        = 0x0401,
    SHL           = 0x0402,
    SHR           = 0x0403,
    NOT           = 0x0404,
    // 0x05 Comparison
    CMP_EQ_I      = 0x0500,
    CMP_EQ_F      = 0x0501,
    CMP_EQ_B      = 0x0502,
    CMP_EQ_S      = 0x0503,
    CMP_LT_I      = 0x0504,
    CMP_LT_F      = 0x0505,
    // 0x06 Control Flow
    BR            = 0x0600,
    BR_TRUE       = 0x0601,
    BR_FALSE      = 0x0602,
    SWITCH        = 0x0603,
    RET           = 0x0604,
    RET_VOID      = 0x0605,
    // 0x07 Calls & Delegates
    CALL          = 0x0700,
    CALL_VIRT     = 0x0701,
    CALL_EXTERN   = 0x0702,
    NEW_DELEGATE  = 0x0703,
    CALL_INDIRECT = 0x0704,
    TAIL_CALL     = 0x0705,
    // 0x08 Object Model
    NEW           = 0x0800,
    GET_FIELD     = 0x0801,
    SET_FIELD     = 0x0802,
    SPAWN_ENTITY  = 0x0803,
    INIT_ENTITY   = 0x0804,
    GET_COMPONENT = 0x0805,
    GET_OR_CREATE = 0x0806,
    FIND_ALL      = 0x0807,
    DESTROY_ENTITY = 0x0808,
    ENTITY_IS_ALIVE = 0x0809,
    // 0x09 Arrays
    NEW_ARRAY     = 0x0900,
    ARRAY_INIT    = 0x0901,
    ARRAY_LOAD    = 0x0902,
    ARRAY_STORE   = 0x0903,
    ARRAY_LEN     = 0x0904,
    ARRAY_ADD     = 0x0905,
    ARRAY_REMOVE  = 0x0906,
    ARRAY_INSERT  = 0x0907,
    ARRAY_SLICE   = 0x0908,
    // 0x0A Type Operations - Option
    WRAP_SOME     = 0x0A00,
    UNWRAP        = 0x0A01,
    IS_SOME       = 0x0A02,
    IS_NONE       = 0x0A03,
    // 0x0A Type Operations - Result
    WRAP_OK       = 0x0A10,
    WRAP_ERR      = 0x0A11,
    UNWRAP_OK     = 0x0A12,
    IS_OK         = 0x0A13,
    IS_ERR        = 0x0A14,
    EXTRACT_ERR   = 0x0A15,
    // 0x0A Type Operations - Enum
    NEW_ENUM      = 0x0A20,
    GET_TAG       = 0x0A21,
    EXTRACT_FIELD = 0x0A22,
    // 0x0B Concurrency
    SPAWN_TASK    = 0x0B00,
    SPAWN_DETACHED = 0x0B01,
    JOIN          = 0x0B02,
    CANCEL        = 0x0B03,
    DEFER_PUSH    = 0x0B04,
    DEFER_POP     = 0x0B05,
    DEFER_END     = 0x0B06,
    // 0x0C Globals & Atomics
    LOAD_GLOBAL   = 0x0C00,
    STORE_GLOBAL  = 0x0C01,
    ATOMIC_BEGIN  = 0x0C02,
    ATOMIC_END    = 0x0C03,
    // 0x0D Conversion
    I2F           = 0x0D00,
    F2I           = 0x0D01,
    I2S           = 0x0D02,
    F2S           = 0x0D03,
    B2S           = 0x0D04,
    CONVERT       = 0x0D05,
    // 0x0E Strings
    STR_CONCAT    = 0x0E00,
    STR_BUILD     = 0x0E01,
    STR_LEN       = 0x0E02,
    // 0x0F Boxing
    BOX           = 0x0F00,
    UNBOX         = 0x0F01,
};

// ============================================================================
// Bitfield types
// ============================================================================

bitfield ModuleFlags {
    debug_info : 1;
    padding    : 15;
};

bitfield MethodDefFlags {
    is_pub      : 1;
    is_static   : 1;
    is_mut_self : 1;
    hook_kind   : 3;
    intrinsic   : 1;
    reserved    : 9;
};

bitfield FieldDefFlags {
    is_pub            : 1;
    has_default       : 1;
    is_component_field : 1;
    reserved          : 13;
};

// ============================================================================
// Metadata Token formatting
// ============================================================================

fn format_table_name(u8 id) {
    if (id == 0)  return "ModuleDef";
    if (id == 1)  return "ModuleRef";
    if (id == 2)  return "TypeDef";
    if (id == 3)  return "TypeRef";
    if (id == 4)  return "TypeSpec";
    if (id == 5)  return "FieldDef";
    if (id == 6)  return "FieldRef";
    if (id == 7)  return "MethodDef";
    if (id == 8)  return "MethodRef";
    if (id == 9)  return "ParamDef";
    if (id == 10) return "ContractDef";
    if (id == 11) return "ContractMethod";
    if (id == 12) return "ImplDef";
    if (id == 13) return "GenericParam";
    if (id == 14) return "GenericConstraint";
    if (id == 15) return "GlobalDef";
    if (id == 16) return "ExternDef";
    if (id == 17) return "ComponentSlot";
    if (id == 18) return "LocaleDef";
    if (id == 19) return "ExportDef";
    if (id == 20) return "AttributeDef";
    return "Unknown";
};

fn format_token(u32 token) {
    if (token == 0) return "null";
    u8 table_id = (token >> 24) & 0xFF;
    u32 row = token & 0x00FFFFFF;
    return std::string::to_string(format_table_name(table_id)) + "[" + std::string::to_string(row) + "]";
};

// ============================================================================
// String heap access
// ============================================================================

// Read a string from the string heap at a given offset.
// String heap entries are length-prefixed UTF-8: u32(byte_len) + bytes.
// Header layout: offset 16 = string_heap_offset (u32).
fn heap_string(u32 offset) {
    if (offset == 0) return "<null>";
    u32 heap_base = std::mem::read_unsigned(16, 4);
    u32 str_len = std::mem::read_unsigned(heap_base + offset, 4);
    if (str_len == 0) return "<empty>";
    return std::mem::read_string(heap_base + offset + 4, str_len);
};

// ============================================================================
// Table directory entry
// ============================================================================

struct TableDirEntry {
    u32 offset;
    u32 row_count;
} [[static]];

// ============================================================================
// Module Header (200 bytes)
// ============================================================================

struct ModuleHeader {
    char magic[4]          [[comment("Must be 'WRIT'")]];
    u16 format_version     [[comment("Format version (starts at 1)")]];
    ModuleFlags flags      [[comment("Bit 0 = debug info present")]];

    u32 module_name        [[comment("String heap offset"), format("heap_string")]];
    u32 module_version     [[comment("String heap offset (semver)"), format("heap_string")]];

    u32 string_heap_offset [[comment("File offset to string heap")]];
    u32 string_heap_size;
    u32 blob_heap_offset   [[comment("File offset to blob heap")]];
    u32 blob_heap_size;

    TableDirEntry table_directory[TABLE_COUNT] [[comment("21 table entries")]];
};

// ============================================================================
// Table 0: ModuleDef (12 bytes)
// ============================================================================

struct ModuleDefRow {
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    u32 version    [[comment("String heap offset"), format("heap_string")]];
    u32 flags;
} [[static]];

// ============================================================================
// Table 1: ModuleRef (8 bytes)
// ============================================================================

struct ModuleRefRow {
    u32 name        [[comment("String heap offset"), format("heap_string")]];
    u32 min_version [[comment("String heap offset"), format("heap_string")]];
} [[static]];

// ============================================================================
// Table 2: TypeDef (20 bytes, padded from 19)
// ============================================================================

struct TypeDefRow {
    u32 name         [[comment("String heap offset"), format("heap_string")]];
    u32 ns           [[comment("Namespace (string heap offset)"), format("heap_string")]];
    TypeDefKind kind;
    u16 flags;
    u32 field_list   [[comment("First FieldDef row (1-based)")]];
    u32 method_list  [[comment("First MethodDef row (1-based)")]];
    padding[1];
} [[static]];

// ============================================================================
// Table 3: TypeRef (12 bytes)
// ============================================================================

struct TypeRefRow {
    u32 scope      [[comment("ModuleRef token"), format("format_token")]];
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    u32 ns         [[comment("Namespace (string heap offset)"), format("heap_string")]];
} [[static]];

// ============================================================================
// Table 4: TypeSpec (4 bytes)
// ============================================================================

struct TypeSpecRow {
    u32 signature  [[comment("Blob heap offset")]];
} [[static]];

// ============================================================================
// Table 5: FieldDef (12 bytes, padded from 10)
// ============================================================================

struct FieldDefRow {
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    u32 type_sig   [[comment("Blob heap offset")]];
    FieldDefFlags flags;
    padding[2];
} [[static]];

// ============================================================================
// Table 6: FieldRef (12 bytes)
// ============================================================================

struct FieldRefRow {
    u32 parent_token [[comment("Parent token"), format("format_token")]];
    u32 name         [[comment("String heap offset"), format("heap_string")]];
    u32 type_sig     [[comment("Blob heap offset")]];
} [[static]];

// ============================================================================
// Table 7: MethodDef (20 bytes)
// ============================================================================

struct MethodDefRow {
    u32 name            [[comment("String heap offset"), format("heap_string")]];
    u32 signature       [[comment("Blob heap offset")]];
    MethodDefFlags flags;
    u32 body_offset     [[comment("File offset to method body")]];
    u32 body_size       [[comment("Total body size in bytes")]];
    u16 reg_count       [[comment("Number of typed registers")]];
} [[static]];

// ============================================================================
// Table 8: MethodRef (12 bytes)
// ============================================================================

struct MethodRefRow {
    u32 parent_token [[comment("Parent token"), format("format_token")]];
    u32 name         [[comment("String heap offset"), format("heap_string")]];
    u32 signature    [[comment("Blob heap offset")]];
} [[static]];

// ============================================================================
// Table 9: ParamDef (12 bytes, padded from 10)
// ============================================================================

struct ParamDefRow {
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    u32 type_sig   [[comment("Blob heap offset")]];
    u16 sequence;
    padding[2];
} [[static]];

// ============================================================================
// Table 10: ContractDef (16 bytes)
// ============================================================================

struct ContractDefRow {
    u32 name              [[comment("String heap offset"), format("heap_string")]];
    u32 ns                [[comment("Namespace (string heap offset)"), format("heap_string")]];
    u32 method_list       [[comment("First ContractMethod row")]];
    u32 generic_param_list [[comment("First GenericParam row")]];
} [[static]];

// ============================================================================
// Table 11: ContractMethod (12 bytes, padded from 10)
// ============================================================================

struct ContractMethodRow {
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    u32 signature  [[comment("Blob heap offset")]];
    u16 slot;
    padding[2];
} [[static]];

// ============================================================================
// Table 12: ImplDef (12 bytes)
// ============================================================================

struct ImplDefRow {
    u32 type_token    [[comment("TypeDef token"), format("format_token")]];
    u32 contract      [[comment("ContractDef token"), format("format_token")]];
    u32 method_list   [[comment("First MethodDef row for impl")]];
} [[static]];

// ============================================================================
// Table 13: GenericParam (12 bytes, padded from 11)
// ============================================================================

struct GenericParamRow {
    u32 owner           [[comment("Owner token"), format("format_token")]];
    GenericParamOwnerKind owner_kind;
    u16 ordinal;
    u32 name            [[comment("String heap offset"), format("heap_string")]];
    padding[1];
} [[static]];

// ============================================================================
// Table 14: GenericConstraint (8 bytes)
// ============================================================================

struct GenericConstraintRow {
    u32 param        [[comment("GenericParam row (1-based)")]];
    u32 constraint   [[comment("Contract token"), format("format_token")]];
} [[static]];

// ============================================================================
// Table 15: GlobalDef (16 bytes, padded from 14)
// ============================================================================

struct GlobalDefRow {
    u32 name         [[comment("String heap offset"), format("heap_string")]];
    u32 type_sig     [[comment("Blob heap offset")]];
    u16 flags;
    u32 init_value   [[comment("Blob heap offset")]];
    padding[2];
} [[static]];

// ============================================================================
// Table 16: ExternDef (16 bytes, padded from 14)
// ============================================================================

struct ExternDefRow {
    u32 name         [[comment("String heap offset"), format("heap_string")]];
    u32 signature    [[comment("Blob heap offset")]];
    u32 import_name  [[comment("String heap offset"), format("heap_string")]];
    u16 flags;
    padding[2];
} [[static]];

// ============================================================================
// Table 17: ComponentSlot (8 bytes)
// ============================================================================

struct ComponentSlotRow {
    u32 owner_entity   [[comment("Entity TypeDef token"), format("format_token")]];
    u32 component_type [[comment("Component type token"), format("format_token")]];
} [[static]];

// ============================================================================
// Table 18: LocaleDef (12 bytes)
// ============================================================================

struct LocaleDefRow {
    u32 dlg_method  [[comment("MethodDef token"), format("format_token")]];
    u32 locale      [[comment("String heap offset"), format("heap_string")]];
    u32 loc_method  [[comment("MethodDef token"), format("format_token")]];
} [[static]];

// ============================================================================
// Table 19: ExportDef (12 bytes, padded from 9)
// ============================================================================

struct ExportDefRow {
    u32 name       [[comment("String heap offset"), format("heap_string")]];
    ExportItemKind item_kind;
    u32 item       [[comment("Exported item token"), format("format_token")]];
    padding[3];
} [[static]];

// ============================================================================
// Table 20: AttributeDef (16 bytes, padded from 13)
// ============================================================================

struct AttributeDefRow {
    u32 owner       [[comment("Owner token"), format("format_token")]];
    OwnerKind owner_kind;
    u32 name        [[comment("String heap offset"), format("heap_string")]];
    u32 value       [[comment("Blob heap offset")]];
    padding[3];
} [[static]];

// ============================================================================
// String heap entry
// ============================================================================

struct HeapString {
    u32 byte_length;
    char data[byte_length];
};

struct HeapBlob {
    u32 byte_length;
    u8 data[byte_length];
};

// ============================================================================
// Debug info structures (within method bodies)
// ============================================================================

struct DebugLocal {
    u16 reg_idx;
    u32 name      [[comment("String heap offset"), format("heap_string")]];
    u32 start_pc;
    u32 end_pc;
};

struct SourceSpan {
    u32 pc;
    u32 line;
    u16 column;
};

// ============================================================================
// IL Instruction decoder
// ============================================================================

struct ILInstruction {
    Opcode op;

    // Operands vary by opcode shape
    if (op == Opcode::NOP || op == Opcode::RET_VOID ||
        op == Opcode::DEFER_POP || op == Opcode::DEFER_END ||
        op == Opcode::ATOMIC_BEGIN || op == Opcode::ATOMIC_END) {
        // Shape N: no operands (2B total)
    }
    else if (op == Opcode::CRASH || op == Opcode::LOAD_TRUE ||
             op == Opcode::LOAD_FALSE || op == Opcode::LOAD_NULL ||
             op == Opcode::RET || op == Opcode::INIT_ENTITY ||
             op == Opcode::DESTROY_ENTITY || op == Opcode::CANCEL) {
        // Shape R: 1 register (4B total)
        u16 r;
    }
    else if (op == Opcode::MOV || op == Opcode::NEG_I || op == Opcode::NEG_F ||
             op == Opcode::NOT || op == Opcode::I2F || op == Opcode::F2I ||
             op == Opcode::I2S || op == Opcode::F2S || op == Opcode::B2S ||
             op == Opcode::ARRAY_LEN || op == Opcode::ARRAY_ADD ||
             op == Opcode::ARRAY_REMOVE ||
             op == Opcode::WRAP_SOME || op == Opcode::UNWRAP ||
             op == Opcode::IS_SOME || op == Opcode::IS_NONE ||
             op == Opcode::WRAP_OK || op == Opcode::WRAP_ERR ||
             op == Opcode::UNWRAP_OK || op == Opcode::IS_OK ||
             op == Opcode::IS_ERR || op == Opcode::EXTRACT_ERR ||
             op == Opcode::GET_TAG || op == Opcode::JOIN ||
             op == Opcode::STR_LEN || op == Opcode::BOX ||
             op == Opcode::UNBOX || op == Opcode::ENTITY_IS_ALIVE) {
        // Shape RR: 2 registers (6B total)
        u16 r_dst;
        u16 r_src;
    }
    else if (op == Opcode::ADD_I || op == Opcode::SUB_I || op == Opcode::MUL_I ||
             op == Opcode::DIV_I || op == Opcode::MOD_I ||
             op == Opcode::ADD_F || op == Opcode::SUB_F || op == Opcode::MUL_F ||
             op == Opcode::DIV_F || op == Opcode::MOD_F ||
             op == Opcode::BIT_AND || op == Opcode::BIT_OR ||
             op == Opcode::SHL || op == Opcode::SHR ||
             op == Opcode::CMP_EQ_I || op == Opcode::CMP_EQ_F ||
             op == Opcode::CMP_EQ_B || op == Opcode::CMP_EQ_S ||
             op == Opcode::CMP_LT_I || op == Opcode::CMP_LT_F ||
             op == Opcode::STR_CONCAT ||
             op == Opcode::ARRAY_LOAD || op == Opcode::ARRAY_STORE ||
             op == Opcode::ARRAY_INSERT) {
        // Shape RRR: 3 registers (8B total)
        u16 r_dst;
        u16 r_a;
        u16 r_b;
    }
    else if (op == Opcode::LOAD_INT || op == Opcode::LOAD_FLOAT) {
        // Shape RI64: register + 64-bit immediate (12B total)
        u16 r_dst;
        u64 imm64;
    }
    else if (op == Opcode::LOAD_STRING || op == Opcode::NEW ||
             op == Opcode::SPAWN_ENTITY || op == Opcode::GET_OR_CREATE ||
             op == Opcode::FIND_ALL || op == Opcode::NEW_ARRAY ||
             op == Opcode::DEFER_PUSH || op == Opcode::LOAD_GLOBAL ||
             op == Opcode::BR_TRUE || op == Opcode::BR_FALSE) {
        // Shape RI32: register + 32-bit value (8B total)
        u16 r_dst;
        u32 imm32;
    }
    else if (op == Opcode::BR) {
        // Shape I32: pad + signed offset (8B total)
        u16 pad;
        s32 offset;
    }
    else if (op == Opcode::CALL || op == Opcode::CALL_EXTERN ||
             op == Opcode::SPAWN_TASK || op == Opcode::SPAWN_DETACHED) {
        // Shape CALL: r_dst, method_idx(u32), r_base, argc (12B total)
        u16 r_dst;
        u32 method_idx  [[format("format_token")]];
        u16 r_base;
        u16 argc;
    }
    else if (op == Opcode::CALL_VIRT) {
        // 14B: r_dst, r_obj, contract_idx(u32), slot, r_base, argc
        u16 r_dst;
        u16 r_obj;
        u32 contract_idx [[format("format_token")]];
        u16 slot;
        u16 r_base;
        u16 argc;
    }
    else if (op == Opcode::NEW_DELEGATE) {
        // 10B: r_dst, method_idx(u32), r_target
        u16 r_dst;
        u32 method_idx  [[format("format_token")]];
        u16 r_target;
    }
    else if (op == Opcode::CALL_INDIRECT) {
        // 10B: r_dst, r_delegate, r_base, argc
        u16 r_dst;
        u16 r_delegate;
        u16 r_base;
        u16 argc;
    }
    else if (op == Opcode::TAIL_CALL) {
        // 10B: method_idx(u32), r_base, argc
        u32 method_idx  [[format("format_token")]];
        u16 r_base;
        u16 argc;
    }
    else if (op == Opcode::SWITCH) {
        // Variable: r_tag, n(u16), offsets[n] (6 + 4n B)
        u16 r_tag;
        u16 n;
        s32 offsets[n];
    }
    else if (op == Opcode::GET_FIELD || op == Opcode::GET_COMPONENT) {
        // 10B: r_dst, r_obj, field_idx(u32)
        u16 r_dst;
        u16 r_obj;
        u32 field_idx   [[format("format_token")]];
    }
    else if (op == Opcode::SET_FIELD) {
        // 10B: r_obj, field_idx(u32), r_val
        u16 r_obj;
        u32 field_idx   [[format("format_token")]];
        u16 r_val;
    }
    else if (op == Opcode::ARRAY_INIT) {
        // 12B: r_dst, elem_type(u32), count, r_base
        u16 r_dst;
        u32 elem_type;
        u16 count;
        u16 r_base;
    }
    else if (op == Opcode::ARRAY_SLICE) {
        // 10B: r_dst, r_arr, r_start, r_end
        u16 r_dst;
        u16 r_arr;
        u16 r_start;
        u16 r_end;
    }
    else if (op == Opcode::NEW_ENUM) {
        // 14B: r_dst, type_idx(u32), tag, field_count, r_base
        u16 r_dst;
        u32 type_idx    [[format("format_token")]];
        u16 tag;
        u16 field_count;
        u16 r_base;
    }
    else if (op == Opcode::EXTRACT_FIELD) {
        // 8B: r_dst, r_enum, field_idx(u16)
        u16 r_dst;
        u16 r_enum;
        u16 field_idx;
    }
    else if (op == Opcode::STORE_GLOBAL) {
        // 8B: global_idx(u32), r_src
        u32 global_idx  [[format("format_token")]];
        u16 r_src;
    }
    else if (op == Opcode::CONVERT) {
        // 10B: r_dst, r_src, target_type(u32)
        u16 r_dst;
        u16 r_src;
        u32 target_type;
    }
    else if (op == Opcode::STR_BUILD) {
        // 8B: r_dst, count, r_base
        u16 r_dst;
        u16 count;
        u16 r_base;
    }
    else {
        // Unknown opcode - read as raw u16
        u16 unknown_operand;
    }
};

// ============================================================================
// Method body helpers
// ============================================================================

// Look up the reg_count for a method body at the given file offset.
// Scans the MethodDef table (table 7) for a row whose body_offset matches.
// Header layout: table_directory[7] at offset 88 (offset field) / 92 (row_count).
// MethodDef row (20B): name(4) sig(4) flags(2) body_offset(4) body_size(4) reg_count(2).
fn lookup_reg_count(u128 body_pos) {
    u32 mdef_count = std::mem::read_unsigned(92, 4);
    u32 mdef_table = std::mem::read_unsigned(88, 4);
    u32 result = 0;
    for (u32 i = 0, i < mdef_count, i = i + 1) {
        if (std::mem::read_unsigned(mdef_table + i * 20 + 10, 4) == body_pos) {
            result = std::mem::read_unsigned(mdef_table + i * 20 + 18, 2);
            break;
        }
    }
    return result;
};

// ============================================================================
// Method body (placed at MethodDef.body_offset)
// ============================================================================

struct MethodBody<auto reg_count> {
    u32 register_types[reg_count] [[comment("Blob heap offsets for register TypeRefs")]];
    u32 code_size;
    ILInstruction code[while($ < addressof(code_size) + 4 + code_size)] [[comment("IL instructions")]];

    // Debug info: present when module header flags bit 0 is set
    // Header offset 6 = flags (u16), bit 0 = debug_info
    if ((std::mem::read_unsigned(6, 2) & 1) != 0) {
        u16 debug_local_count;
        DebugLocal debug_locals[debug_local_count] [[comment("Debug local variable mappings")]];
        u32 source_span_count;
        SourceSpan source_spans[source_span_count] [[comment("Source location mappings")]];
    }
};

// Wrapper that handles 4-byte alignment then parses one MethodBody.
// Used as a sequential array element; determines reg_count by looking up
// the current file position in the MethodDef table.
struct MethodBodyEntry {
    if ($ % 4 == 1) { padding[3]; }
    if ($ % 4 == 2) { padding[2]; }
    if ($ % 4 == 3) { padding[1]; }
    MethodBody<lookup_reg_count($)> body;
};

// ============================================================================
// Main module structure
// ============================================================================

struct WritModule {
    ModuleHeader header [[comment("200-byte fixed header")]];

    // ── Table 0: ModuleDef ────────────────────────────────────────
    if (header.table_directory[0].row_count > 0) {
        ModuleDefRow module_defs[header.table_directory[0].row_count]
            @ header.table_directory[0].offset
            [[comment("Table 0: Module identity")]];
    }

    // ── Table 1: ModuleRef ────────────────────────────────────────
    if (header.table_directory[1].row_count > 0) {
        ModuleRefRow module_refs[header.table_directory[1].row_count]
            @ header.table_directory[1].offset
            [[comment("Table 1: Module dependencies")]];
    }

    // ── Table 2: TypeDef ──────────────────────────────────────────
    if (header.table_directory[2].row_count > 0) {
        TypeDefRow type_defs[header.table_directory[2].row_count]
            @ header.table_directory[2].offset
            [[comment("Table 2: Type definitions")]];
    }

    // ── Table 3: TypeRef ──────────────────────────────────────────
    if (header.table_directory[3].row_count > 0) {
        TypeRefRow type_refs[header.table_directory[3].row_count]
            @ header.table_directory[3].offset
            [[comment("Table 3: Cross-module type references")]];
    }

    // ── Table 4: TypeSpec ─────────────────────────────────────────
    if (header.table_directory[4].row_count > 0) {
        TypeSpecRow type_specs[header.table_directory[4].row_count]
            @ header.table_directory[4].offset
            [[comment("Table 4: Instantiated generic types")]];
    }

    // ── Table 5: FieldDef ─────────────────────────────────────────
    if (header.table_directory[5].row_count > 0) {
        FieldDefRow field_defs[header.table_directory[5].row_count]
            @ header.table_directory[5].offset
            [[comment("Table 5: Field definitions")]];
    }

    // ── Table 6: FieldRef ─────────────────────────────────────────
    if (header.table_directory[6].row_count > 0) {
        FieldRefRow field_refs[header.table_directory[6].row_count]
            @ header.table_directory[6].offset
            [[comment("Table 6: Cross-module field references")]];
    }

    // ── Table 7: MethodDef ────────────────────────────────────────
    if (header.table_directory[7].row_count > 0) {
        MethodDefRow method_defs[header.table_directory[7].row_count]
            @ header.table_directory[7].offset
            [[comment("Table 7: Method definitions")]];
    }

    // ── Table 8: MethodRef ────────────────────────────────────────
    if (header.table_directory[8].row_count > 0) {
        MethodRefRow method_refs[header.table_directory[8].row_count]
            @ header.table_directory[8].offset
            [[comment("Table 8: Cross-module method references")]];
    }

    // ── Table 9: ParamDef ─────────────────────────────────────────
    if (header.table_directory[9].row_count > 0) {
        ParamDefRow param_defs[header.table_directory[9].row_count]
            @ header.table_directory[9].offset
            [[comment("Table 9: Method parameters")]];
    }

    // ── Table 10: ContractDef ─────────────────────────────────────
    if (header.table_directory[10].row_count > 0) {
        ContractDefRow contract_defs[header.table_directory[10].row_count]
            @ header.table_directory[10].offset
            [[comment("Table 10: Contract declarations")]];
    }

    // ── Table 11: ContractMethod ──────────────────────────────────
    if (header.table_directory[11].row_count > 0) {
        ContractMethodRow contract_methods[header.table_directory[11].row_count]
            @ header.table_directory[11].offset
            [[comment("Table 11: Contract method slots")]];
    }

    // ── Table 12: ImplDef ─────────────────────────────────────────
    if (header.table_directory[12].row_count > 0) {
        ImplDefRow impl_defs[header.table_directory[12].row_count]
            @ header.table_directory[12].offset
            [[comment("Table 12: Contract implementations")]];
    }

    // ── Table 13: GenericParam ────────────────────────────────────
    if (header.table_directory[13].row_count > 0) {
        GenericParamRow generic_params[header.table_directory[13].row_count]
            @ header.table_directory[13].offset
            [[comment("Table 13: Generic type parameters")]];
    }

    // ── Table 14: GenericConstraint ───────────────────────────────
    if (header.table_directory[14].row_count > 0) {
        GenericConstraintRow generic_constraints[header.table_directory[14].row_count]
            @ header.table_directory[14].offset
            [[comment("Table 14: Generic parameter constraints")]];
    }

    // ── Table 15: GlobalDef ───────────────────────────────────────
    if (header.table_directory[15].row_count > 0) {
        GlobalDefRow global_defs[header.table_directory[15].row_count]
            @ header.table_directory[15].offset
            [[comment("Table 15: Global constants/variables")]];
    }

    // ── Table 16: ExternDef ───────────────────────────────────────
    if (header.table_directory[16].row_count > 0) {
        ExternDefRow extern_defs[header.table_directory[16].row_count]
            @ header.table_directory[16].offset
            [[comment("Table 16: Extern declarations")]];
    }

    // ── Table 17: ComponentSlot ───────────────────────────────────
    if (header.table_directory[17].row_count > 0) {
        ComponentSlotRow component_slots[header.table_directory[17].row_count]
            @ header.table_directory[17].offset
            [[comment("Table 17: Entity-component bindings")]];
    }

    // ── Table 18: LocaleDef ───────────────────────────────────────
    if (header.table_directory[18].row_count > 0) {
        LocaleDefRow locale_defs[header.table_directory[18].row_count]
            @ header.table_directory[18].offset
            [[comment("Table 18: Dialogue locale dispatch")]];
    }

    // ── Table 19: ExportDef ───────────────────────────────────────
    if (header.table_directory[19].row_count > 0) {
        ExportDefRow export_defs[header.table_directory[19].row_count]
            @ header.table_directory[19].offset
            [[comment("Table 19: Public export index")]];
    }

    // ── Table 20: AttributeDef ────────────────────────────────────
    if (header.table_directory[20].row_count > 0) {
        AttributeDefRow attribute_defs[header.table_directory[20].row_count]
            @ header.table_directory[20].offset
            [[comment("Table 20: Metadata attributes")]];
    }

    // ── String Heap ───────────────────────────────────────────────
    if (header.string_heap_size > 0) {
        HeapString string_heap[while($ < header.string_heap_offset + header.string_heap_size)]
            @ header.string_heap_offset
            [[comment("String heap (length-prefixed UTF-8 entries)")]];
    }

    // ── Blob Heap ─────────────────────────────────────────────────
    if (header.blob_heap_size > 0) {
        HeapBlob blob_heap[while($ < header.blob_heap_offset + header.blob_heap_size)]
            @ header.blob_heap_offset
            [[comment("Blob heap (TypeRef encodings, signatures, constants)")]];
    }

    // ── Method Bodies ────────────────────────────────────────────
    // Bodies are stored contiguously after the blob heap, 4-byte aligned.
    // Each MethodBodyEntry self-discovers its reg_count by looking up
    // the current file offset in the MethodDef table.
    if (header.blob_heap_offset + header.blob_heap_size < std::mem::size()) {
        MethodBodyEntry method_bodies[while($ < std::mem::size())]
            @ (header.blob_heap_offset + header.blob_heap_size)
            [[comment("Method bodies (register types + IL code + debug info)")]];
    }
};

// ============================================================================
// Root placement
// ============================================================================

WritModule module @ 0x00;
