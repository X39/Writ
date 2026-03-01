use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::error::{DecodeError, EncodeError};

/// All 91 IL opcodes, grouped by category.
///
/// Operand field names match the spec. Shape comments reference section 4.1.
#[derive(Debug, Clone)]
pub enum Instruction {
    // ── 0x00 Meta ──────────────────────────────────────────────
    /// 0x0000 — Shape N (2B)
    Nop,
    /// 0x0001 — Shape R (4B)
    Crash { r_msg: u16 },

    // ── 0x01 Data Movement ─────────────────────────────────────
    /// 0x0100 — Shape RR (6B)
    Mov { r_dst: u16, r_src: u16 },
    /// 0x0101 — Shape RI64 (12B)
    LoadInt { r_dst: u16, value: i64 },
    /// 0x0102 — Shape RI64 (12B)
    LoadFloat { r_dst: u16, value: f64 },
    /// 0x0103 — Shape R (4B)
    LoadTrue { r_dst: u16 },
    /// 0x0104 — Shape R (4B)
    LoadFalse { r_dst: u16 },
    /// 0x0105 — Shape RI32 (8B)
    LoadString { r_dst: u16, string_idx: u32 },
    /// 0x0106 — Shape R (4B)
    LoadNull { r_dst: u16 },

    // ── 0x02 Integer Arithmetic ────────────────────────────────
    /// 0x0200 — Shape RRR (8B)
    AddI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0201 — Shape RRR (8B)
    SubI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0202 — Shape RRR (8B)
    MulI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0203 — Shape RRR (8B)
    DivI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0204 — Shape RRR (8B)
    ModI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0205 — Shape RR (6B)
    NegI { r_dst: u16, r_src: u16 },

    // ── 0x03 Float Arithmetic ──────────────────────────────────
    /// 0x0300 — Shape RRR (8B)
    AddF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0301 — Shape RRR (8B)
    SubF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0302 — Shape RRR (8B)
    MulF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0303 — Shape RRR (8B)
    DivF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0304 — Shape RRR (8B)
    ModF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0305 — Shape RR (6B)
    NegF { r_dst: u16, r_src: u16 },

    // ── 0x04 Bitwise & Logical ─────────────────────────────────
    /// 0x0400 — Shape RRR (8B)
    BitAnd { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0401 — Shape RRR (8B)
    BitOr { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0402 — Shape RRR (8B)
    Shl { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0403 — Shape RRR (8B)
    Shr { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0404 — Shape RR (6B)
    Not { r_dst: u16, r_src: u16 },

    // ── 0x05 Comparison ────────────────────────────────────────
    /// 0x0500 — Shape RRR (8B)
    CmpEqI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0501 — Shape RRR (8B)
    CmpEqF { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0502 — Shape RRR (8B)
    CmpEqB { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0503 — Shape RRR (8B)
    CmpEqS { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0504 — Shape RRR (8B)
    CmpLtI { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0505 — Shape RRR (8B)
    CmpLtF { r_dst: u16, r_a: u16, r_b: u16 },

    // ── 0x06 Control Flow ──────────────────────────────────────
    /// 0x0600 — Shape I32 (8B): u16(op) u16(pad=0) i32(offset)
    Br { offset: i32 },
    /// 0x0601 — Shape RI32 (8B): u16(op) u16(r_cond) i32(offset)
    BrTrue { r_cond: u16, offset: i32 },
    /// 0x0602 — Shape RI32 (8B): u16(op) u16(r_cond) i32(offset)
    BrFalse { r_cond: u16, offset: i32 },
    /// 0x0603 — var (6 + 4n B): u16(op) u16(r_tag) u16(n) i32[n]
    Switch { r_tag: u16, offsets: Vec<i32> },
    /// 0x0604 — Shape R (4B)
    Ret { r_src: u16 },
    /// 0x0605 — Shape N (2B)
    RetVoid,

    // ── 0x07 Calls & Delegates ─────────────────────────────────
    /// 0x0700 — Shape CALL (12B)
    Call { r_dst: u16, method_idx: u32, r_base: u16, argc: u16 },
    /// 0x0701 — var (14B): u16(op) u16(r_dst) u16(r_obj) u32(contract_idx) u16(slot) u16(r_base) u16(argc)
    CallVirt { r_dst: u16, r_obj: u16, contract_idx: u32, slot: u16, r_base: u16, argc: u16 },
    /// 0x0702 — Shape CALL (12B)
    CallExtern { r_dst: u16, extern_idx: u32, r_base: u16, argc: u16 },
    /// 0x0703 — var (10B): u16(op) u16(r_dst) u32(method_idx) u16(r_target)
    NewDelegate { r_dst: u16, method_idx: u32, r_target: u16 },
    /// 0x0704 — var (10B): u16(op) u16(r_dst) u16(r_delegate) u16(r_base) u16(argc)
    CallIndirect { r_dst: u16, r_delegate: u16, r_base: u16, argc: u16 },
    /// 0x0705 — var (10B): u16(op) u32(method_idx) u16(r_base) u16(argc)
    TailCall { method_idx: u32, r_base: u16, argc: u16 },

    // ── 0x08 Object Model ──────────────────────────────────────
    /// 0x0800 — Shape RI32 (8B)
    New { r_dst: u16, type_idx: u32 },
    /// 0x0801 — var (10B): u16(op) u16(r_dst) u16(r_obj) u32(field_idx)
    GetField { r_dst: u16, r_obj: u16, field_idx: u32 },
    /// 0x0802 — var (10B): u16(op) u16(r_obj) u32(field_idx) u16(r_val)
    SetField { r_obj: u16, field_idx: u32, r_val: u16 },
    /// 0x0803 — Shape RI32 (8B)
    SpawnEntity { r_dst: u16, type_idx: u32 },
    /// 0x0804 — Shape R (4B)
    InitEntity { r_entity: u16 },
    /// 0x0805 — var (10B): u16(op) u16(r_dst) u16(r_entity) u32(comp_type_idx)
    GetComponent { r_dst: u16, r_entity: u16, comp_type_idx: u32 },
    /// 0x0806 — Shape RI32 (8B)
    GetOrCreate { r_dst: u16, type_idx: u32 },
    /// 0x0807 — Shape RI32 (8B)
    FindAll { r_dst: u16, type_idx: u32 },
    /// 0x0808 — Shape R (4B)
    DestroyEntity { r_entity: u16 },
    /// 0x0809 — Shape RR (6B)
    EntityIsAlive { r_dst: u16, r_entity: u16 },

    // ── 0x09 Arrays ────────────────────────────────────────────
    /// 0x0900 — Shape RI32 (8B)
    NewArray { r_dst: u16, elem_type: u32 },
    /// 0x0901 — var (12B): u16(op) u16(r_dst) u32(elem_type) u16(count) u16(r_base)
    ArrayInit { r_dst: u16, elem_type: u32, count: u16, r_base: u16 },
    /// 0x0902 — Shape RRR (8B)
    ArrayLoad { r_dst: u16, r_arr: u16, r_idx: u16 },
    /// 0x0903 — Shape RRR (8B)
    ArrayStore { r_arr: u16, r_idx: u16, r_val: u16 },
    /// 0x0904 — Shape RR (6B)
    ArrayLen { r_dst: u16, r_arr: u16 },
    /// 0x0905 — Shape RR (6B)
    ArrayAdd { r_arr: u16, r_val: u16 },
    /// 0x0906 — Shape RR (6B)
    ArrayRemove { r_arr: u16, r_idx: u16 },
    /// 0x0907 — Shape RRR (8B)
    ArrayInsert { r_arr: u16, r_idx: u16, r_val: u16 },
    /// 0x0908 — var (10B): u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end)
    ArraySlice { r_dst: u16, r_arr: u16, r_start: u16, r_end: u16 },

    // ── 0x0A Type Operations ───────────────────────────────────
    // Option
    /// 0x0A00 — Shape RR (6B)
    WrapSome { r_dst: u16, r_val: u16 },
    /// 0x0A01 — Shape RR (6B)
    Unwrap { r_dst: u16, r_opt: u16 },
    /// 0x0A02 — Shape RR (6B)
    IsSome { r_dst: u16, r_opt: u16 },
    /// 0x0A03 — Shape RR (6B)
    IsNone { r_dst: u16, r_opt: u16 },

    // Result
    /// 0x0A10 — Shape RR (6B)
    WrapOk { r_dst: u16, r_val: u16 },
    /// 0x0A11 — Shape RR (6B)
    WrapErr { r_dst: u16, r_err: u16 },
    /// 0x0A12 — Shape RR (6B)
    UnwrapOk { r_dst: u16, r_result: u16 },
    /// 0x0A13 — Shape RR (6B)
    IsOk { r_dst: u16, r_result: u16 },
    /// 0x0A14 — Shape RR (6B)
    IsErr { r_dst: u16, r_result: u16 },
    /// 0x0A15 — Shape RR (6B)
    ExtractErr { r_dst: u16, r_result: u16 },

    // Enum
    /// 0x0A20 — var (14B): u16(op) u16(r_dst) u32(type_idx) u16(tag) u16(field_count) u16(r_base)
    NewEnum { r_dst: u16, type_idx: u32, tag: u16, field_count: u16, r_base: u16 },
    /// 0x0A21 — Shape RR (6B)
    GetTag { r_dst: u16, r_enum: u16 },
    /// 0x0A22 — var (8B): u16(op) u16(r_dst) u16(r_enum) u16(field_idx)
    ExtractField { r_dst: u16, r_enum: u16, field_idx: u16 },

    // ── 0x0B Concurrency ───────────────────────────────────────
    /// 0x0B00 — Shape CALL (12B)
    SpawnTask { r_dst: u16, method_idx: u32, r_base: u16, argc: u16 },
    /// 0x0B01 — Shape CALL (12B)
    SpawnDetached { r_dst: u16, method_idx: u32, r_base: u16, argc: u16 },
    /// 0x0B02 — Shape RR (6B)
    Join { r_dst: u16, r_task: u16 },
    /// 0x0B03 — Shape R (4B)
    Cancel { r_task: u16 },
    /// 0x0B04 — Shape RI32 (8B)
    DeferPush { r_dst: u16, method_idx: u32 },
    /// 0x0B05 — Shape N (2B)
    DeferPop,
    /// 0x0B06 — Shape N (2B)
    DeferEnd,

    // ── 0x0C Globals & Atomics ─────────────────────────────────
    /// 0x0C00 — Shape RI32 (8B)
    LoadGlobal { r_dst: u16, global_idx: u32 },
    /// 0x0C01 — var (8B): u16(op) u32(global_idx) u16(r_src)
    StoreGlobal { global_idx: u32, r_src: u16 },
    /// 0x0C02 — Shape N (2B)
    AtomicBegin,
    /// 0x0C03 — Shape N (2B)
    AtomicEnd,

    // ── 0x0D Conversion ────────────────────────────────────────
    /// 0x0D00 — Shape RR (6B)
    I2f { r_dst: u16, r_src: u16 },
    /// 0x0D01 — Shape RR (6B)
    F2i { r_dst: u16, r_src: u16 },
    /// 0x0D02 — Shape RR (6B)
    I2s { r_dst: u16, r_src: u16 },
    /// 0x0D03 — Shape RR (6B)
    F2s { r_dst: u16, r_src: u16 },
    /// 0x0D04 — Shape RR (6B)
    B2s { r_dst: u16, r_src: u16 },
    /// 0x0D05 — var (10B): u16(op) u16(r_dst) u16(r_src) u32(target_type)
    Convert { r_dst: u16, r_src: u16, target_type: u32 },

    // ── 0x0E Strings ───────────────────────────────────────────
    /// 0x0E00 — Shape RRR (8B)
    StrConcat { r_dst: u16, r_a: u16, r_b: u16 },
    /// 0x0E01 — var (8B): u16(op) u16(r_dst) u16(count) u16(r_base)
    StrBuild { r_dst: u16, count: u16, r_base: u16 },
    /// 0x0E02 — Shape RR (6B)
    StrLen { r_dst: u16, r_str: u16 },

    // ── 0x0F Boxing ────────────────────────────────────────────
    /// 0x0F00 — Shape RR (6B)
    Box { r_dst: u16, r_val: u16 },
    /// 0x0F01 — Shape RR (6B)
    Unbox { r_dst: u16, r_boxed: u16 },
}

impl Instruction {
    /// Returns the u16 opcode for this instruction.
    pub fn opcode(&self) -> u16 {
        match self {
            // 0x00 Meta
            Instruction::Nop => 0x0000,
            Instruction::Crash { .. } => 0x0001,
            // 0x01 Data Movement
            Instruction::Mov { .. } => 0x0100,
            Instruction::LoadInt { .. } => 0x0101,
            Instruction::LoadFloat { .. } => 0x0102,
            Instruction::LoadTrue { .. } => 0x0103,
            Instruction::LoadFalse { .. } => 0x0104,
            Instruction::LoadString { .. } => 0x0105,
            Instruction::LoadNull { .. } => 0x0106,
            // 0x02 Integer Arithmetic
            Instruction::AddI { .. } => 0x0200,
            Instruction::SubI { .. } => 0x0201,
            Instruction::MulI { .. } => 0x0202,
            Instruction::DivI { .. } => 0x0203,
            Instruction::ModI { .. } => 0x0204,
            Instruction::NegI { .. } => 0x0205,
            // 0x03 Float Arithmetic
            Instruction::AddF { .. } => 0x0300,
            Instruction::SubF { .. } => 0x0301,
            Instruction::MulF { .. } => 0x0302,
            Instruction::DivF { .. } => 0x0303,
            Instruction::ModF { .. } => 0x0304,
            Instruction::NegF { .. } => 0x0305,
            // 0x04 Bitwise & Logical
            Instruction::BitAnd { .. } => 0x0400,
            Instruction::BitOr { .. } => 0x0401,
            Instruction::Shl { .. } => 0x0402,
            Instruction::Shr { .. } => 0x0403,
            Instruction::Not { .. } => 0x0404,
            // 0x05 Comparison
            Instruction::CmpEqI { .. } => 0x0500,
            Instruction::CmpEqF { .. } => 0x0501,
            Instruction::CmpEqB { .. } => 0x0502,
            Instruction::CmpEqS { .. } => 0x0503,
            Instruction::CmpLtI { .. } => 0x0504,
            Instruction::CmpLtF { .. } => 0x0505,
            // 0x06 Control Flow
            Instruction::Br { .. } => 0x0600,
            Instruction::BrTrue { .. } => 0x0601,
            Instruction::BrFalse { .. } => 0x0602,
            Instruction::Switch { .. } => 0x0603,
            Instruction::Ret { .. } => 0x0604,
            Instruction::RetVoid => 0x0605,
            // 0x07 Calls & Delegates
            Instruction::Call { .. } => 0x0700,
            Instruction::CallVirt { .. } => 0x0701,
            Instruction::CallExtern { .. } => 0x0702,
            Instruction::NewDelegate { .. } => 0x0703,
            Instruction::CallIndirect { .. } => 0x0704,
            Instruction::TailCall { .. } => 0x0705,
            // 0x08 Object Model
            Instruction::New { .. } => 0x0800,
            Instruction::GetField { .. } => 0x0801,
            Instruction::SetField { .. } => 0x0802,
            Instruction::SpawnEntity { .. } => 0x0803,
            Instruction::InitEntity { .. } => 0x0804,
            Instruction::GetComponent { .. } => 0x0805,
            Instruction::GetOrCreate { .. } => 0x0806,
            Instruction::FindAll { .. } => 0x0807,
            Instruction::DestroyEntity { .. } => 0x0808,
            Instruction::EntityIsAlive { .. } => 0x0809,
            // 0x09 Arrays
            Instruction::NewArray { .. } => 0x0900,
            Instruction::ArrayInit { .. } => 0x0901,
            Instruction::ArrayLoad { .. } => 0x0902,
            Instruction::ArrayStore { .. } => 0x0903,
            Instruction::ArrayLen { .. } => 0x0904,
            Instruction::ArrayAdd { .. } => 0x0905,
            Instruction::ArrayRemove { .. } => 0x0906,
            Instruction::ArrayInsert { .. } => 0x0907,
            Instruction::ArraySlice { .. } => 0x0908,
            // 0x0A Type Operations — Option
            Instruction::WrapSome { .. } => 0x0A00,
            Instruction::Unwrap { .. } => 0x0A01,
            Instruction::IsSome { .. } => 0x0A02,
            Instruction::IsNone { .. } => 0x0A03,
            // 0x0A Type Operations — Result
            Instruction::WrapOk { .. } => 0x0A10,
            Instruction::WrapErr { .. } => 0x0A11,
            Instruction::UnwrapOk { .. } => 0x0A12,
            Instruction::IsOk { .. } => 0x0A13,
            Instruction::IsErr { .. } => 0x0A14,
            Instruction::ExtractErr { .. } => 0x0A15,
            // 0x0A Type Operations — Enum
            Instruction::NewEnum { .. } => 0x0A20,
            Instruction::GetTag { .. } => 0x0A21,
            Instruction::ExtractField { .. } => 0x0A22,
            // 0x0B Concurrency
            Instruction::SpawnTask { .. } => 0x0B00,
            Instruction::SpawnDetached { .. } => 0x0B01,
            Instruction::Join { .. } => 0x0B02,
            Instruction::Cancel { .. } => 0x0B03,
            Instruction::DeferPush { .. } => 0x0B04,
            Instruction::DeferPop => 0x0B05,
            Instruction::DeferEnd => 0x0B06,
            // 0x0C Globals & Atomics
            Instruction::LoadGlobal { .. } => 0x0C00,
            Instruction::StoreGlobal { .. } => 0x0C01,
            Instruction::AtomicBegin => 0x0C02,
            Instruction::AtomicEnd => 0x0C03,
            // 0x0D Conversion
            Instruction::I2f { .. } => 0x0D00,
            Instruction::F2i { .. } => 0x0D01,
            Instruction::I2s { .. } => 0x0D02,
            Instruction::F2s { .. } => 0x0D03,
            Instruction::B2s { .. } => 0x0D04,
            Instruction::Convert { .. } => 0x0D05,
            // 0x0E Strings
            Instruction::StrConcat { .. } => 0x0E00,
            Instruction::StrBuild { .. } => 0x0E01,
            Instruction::StrLen { .. } => 0x0E02,
            // 0x0F Boxing
            Instruction::Box { .. } => 0x0F00,
            Instruction::Unbox { .. } => 0x0F01,
        }
    }

    /// Encode this instruction to the given writer.
    pub fn encode<W: Write>(&self, w: &mut W) -> Result<(), EncodeError> {
        w.write_u16::<LittleEndian>(self.opcode())?;

        match self {
            // ── Shape N (no operands) ──────────────────────────
            Instruction::Nop
            | Instruction::RetVoid
            | Instruction::DeferPop
            | Instruction::DeferEnd
            | Instruction::AtomicBegin
            | Instruction::AtomicEnd => {}

            // ── Shape R (u16 reg) ──────────────────────────────
            Instruction::Crash { r_msg } => w.write_u16::<LittleEndian>(*r_msg)?,
            Instruction::LoadTrue { r_dst } => w.write_u16::<LittleEndian>(*r_dst)?,
            Instruction::LoadFalse { r_dst } => w.write_u16::<LittleEndian>(*r_dst)?,
            Instruction::LoadNull { r_dst } => w.write_u16::<LittleEndian>(*r_dst)?,
            Instruction::Ret { r_src } => w.write_u16::<LittleEndian>(*r_src)?,
            Instruction::InitEntity { r_entity } => w.write_u16::<LittleEndian>(*r_entity)?,
            Instruction::DestroyEntity { r_entity } => w.write_u16::<LittleEndian>(*r_entity)?,
            Instruction::Cancel { r_task } => w.write_u16::<LittleEndian>(*r_task)?,

            // ── Shape RR (u16, u16) ────────────────────────────
            Instruction::Mov { r_dst, r_src }
            | Instruction::NegI { r_dst, r_src }
            | Instruction::NegF { r_dst, r_src }
            | Instruction::Not { r_dst, r_src }
            | Instruction::I2f { r_dst, r_src }
            | Instruction::F2i { r_dst, r_src }
            | Instruction::I2s { r_dst, r_src }
            | Instruction::F2s { r_dst, r_src }
            | Instruction::B2s { r_dst, r_src } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_src)?;
            }
            Instruction::ArrayLen { r_dst, r_arr } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_arr)?;
            }
            Instruction::ArrayAdd { r_arr, r_val } => {
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::ArrayRemove { r_arr, r_idx } => {
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_idx)?;
            }
            Instruction::WrapSome { r_dst, r_val } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::Unwrap { r_dst, r_opt } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_opt)?;
            }
            Instruction::IsSome { r_dst, r_opt } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_opt)?;
            }
            Instruction::IsNone { r_dst, r_opt } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_opt)?;
            }
            Instruction::WrapOk { r_dst, r_val } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::WrapErr { r_dst, r_err } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_err)?;
            }
            Instruction::UnwrapOk { r_dst, r_result } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_result)?;
            }
            Instruction::IsOk { r_dst, r_result } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_result)?;
            }
            Instruction::IsErr { r_dst, r_result } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_result)?;
            }
            Instruction::ExtractErr { r_dst, r_result } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_result)?;
            }
            Instruction::GetTag { r_dst, r_enum } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_enum)?;
            }
            Instruction::Join { r_dst, r_task } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_task)?;
            }
            Instruction::StrLen { r_dst, r_str } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_str)?;
            }
            Instruction::Box { r_dst, r_val } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::Unbox { r_dst, r_boxed } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_boxed)?;
            }
            Instruction::EntityIsAlive { r_dst, r_entity } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_entity)?;
            }

            // ── Shape RRR (u16, u16, u16) ──────────────────────
            Instruction::AddI { r_dst, r_a, r_b }
            | Instruction::SubI { r_dst, r_a, r_b }
            | Instruction::MulI { r_dst, r_a, r_b }
            | Instruction::DivI { r_dst, r_a, r_b }
            | Instruction::ModI { r_dst, r_a, r_b }
            | Instruction::AddF { r_dst, r_a, r_b }
            | Instruction::SubF { r_dst, r_a, r_b }
            | Instruction::MulF { r_dst, r_a, r_b }
            | Instruction::DivF { r_dst, r_a, r_b }
            | Instruction::ModF { r_dst, r_a, r_b }
            | Instruction::BitAnd { r_dst, r_a, r_b }
            | Instruction::BitOr { r_dst, r_a, r_b }
            | Instruction::Shl { r_dst, r_a, r_b }
            | Instruction::Shr { r_dst, r_a, r_b }
            | Instruction::CmpEqI { r_dst, r_a, r_b }
            | Instruction::CmpEqF { r_dst, r_a, r_b }
            | Instruction::CmpEqB { r_dst, r_a, r_b }
            | Instruction::CmpEqS { r_dst, r_a, r_b }
            | Instruction::CmpLtI { r_dst, r_a, r_b }
            | Instruction::CmpLtF { r_dst, r_a, r_b }
            | Instruction::StrConcat { r_dst, r_a, r_b } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_a)?;
                w.write_u16::<LittleEndian>(*r_b)?;
            }
            Instruction::ArrayLoad { r_dst, r_arr, r_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_idx)?;
            }
            Instruction::ArrayStore { r_arr, r_idx, r_val } => {
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_idx)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::ArrayInsert { r_arr, r_idx, r_val } => {
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_idx)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }

            // ── Shape RI32 (u16, u32) ──────────────────────────
            Instruction::LoadString { r_dst, string_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*string_idx)?;
            }
            Instruction::New { r_dst, type_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*type_idx)?;
            }
            Instruction::SpawnEntity { r_dst, type_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*type_idx)?;
            }
            Instruction::GetOrCreate { r_dst, type_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*type_idx)?;
            }
            Instruction::FindAll { r_dst, type_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*type_idx)?;
            }
            Instruction::NewArray { r_dst, elem_type } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*elem_type)?;
            }
            Instruction::DeferPush { r_dst, method_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*method_idx)?;
            }
            Instruction::LoadGlobal { r_dst, global_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*global_idx)?;
            }
            Instruction::BrTrue { r_cond, offset } => {
                w.write_u16::<LittleEndian>(*r_cond)?;
                w.write_u32::<LittleEndian>(*offset as u32)?;
            }
            Instruction::BrFalse { r_cond, offset } => {
                w.write_u16::<LittleEndian>(*r_cond)?;
                w.write_u32::<LittleEndian>(*offset as u32)?;
            }

            // ── Shape RI64 (u16, u64) ──────────────────────────
            Instruction::LoadInt { r_dst, value } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u64::<LittleEndian>(*value as u64)?;
            }
            Instruction::LoadFloat { r_dst, value } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u64::<LittleEndian>(value.to_bits())?;
            }

            // ── Shape I32 (pad + i32) ──────────────────────────
            Instruction::Br { offset } => {
                w.write_u16::<LittleEndian>(0)?; // padding
                w.write_u32::<LittleEndian>(*offset as u32)?;
            }

            // ── Shape CALL (u16, u32, u16, u16) ────────────────
            Instruction::Call { r_dst, method_idx, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*method_idx)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::CallExtern { r_dst, extern_idx, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*extern_idx)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::SpawnTask { r_dst, method_idx, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*method_idx)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::SpawnDetached { r_dst, method_idx, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*method_idx)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }

            // ── Variable-layout instructions ───────────────────
            Instruction::Switch { r_tag, offsets } => {
                w.write_u16::<LittleEndian>(*r_tag)?;
                w.write_u16::<LittleEndian>(offsets.len() as u16)?;
                for off in offsets {
                    w.write_i32::<LittleEndian>(*off)?;
                }
            }
            Instruction::CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_obj)?;
                w.write_u32::<LittleEndian>(*contract_idx)?;
                w.write_u16::<LittleEndian>(*slot)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::NewDelegate { r_dst, method_idx, r_target } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*method_idx)?;
                w.write_u16::<LittleEndian>(*r_target)?;
            }
            Instruction::CallIndirect { r_dst, r_delegate, r_base, argc } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_delegate)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::TailCall { method_idx, r_base, argc } => {
                w.write_u32::<LittleEndian>(*method_idx)?;
                w.write_u16::<LittleEndian>(*r_base)?;
                w.write_u16::<LittleEndian>(*argc)?;
            }
            Instruction::GetField { r_dst, r_obj, field_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_obj)?;
                w.write_u32::<LittleEndian>(*field_idx)?;
            }
            Instruction::SetField { r_obj, field_idx, r_val } => {
                w.write_u16::<LittleEndian>(*r_obj)?;
                w.write_u32::<LittleEndian>(*field_idx)?;
                w.write_u16::<LittleEndian>(*r_val)?;
            }
            Instruction::GetComponent { r_dst, r_entity, comp_type_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_entity)?;
                w.write_u32::<LittleEndian>(*comp_type_idx)?;
            }
            Instruction::ArrayInit { r_dst, elem_type, count, r_base } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*elem_type)?;
                w.write_u16::<LittleEndian>(*count)?;
                w.write_u16::<LittleEndian>(*r_base)?;
            }
            Instruction::ArraySlice { r_dst, r_arr, r_start, r_end } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_arr)?;
                w.write_u16::<LittleEndian>(*r_start)?;
                w.write_u16::<LittleEndian>(*r_end)?;
            }
            Instruction::NewEnum { r_dst, type_idx, tag, field_count, r_base } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u32::<LittleEndian>(*type_idx)?;
                w.write_u16::<LittleEndian>(*tag)?;
                w.write_u16::<LittleEndian>(*field_count)?;
                w.write_u16::<LittleEndian>(*r_base)?;
            }
            Instruction::ExtractField { r_dst, r_enum, field_idx } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_enum)?;
                w.write_u16::<LittleEndian>(*field_idx)?;
            }
            Instruction::StoreGlobal { global_idx, r_src } => {
                w.write_u32::<LittleEndian>(*global_idx)?;
                w.write_u16::<LittleEndian>(*r_src)?;
            }
            Instruction::Convert { r_dst, r_src, target_type } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*r_src)?;
                w.write_u32::<LittleEndian>(*target_type)?;
            }
            Instruction::StrBuild { r_dst, count, r_base } => {
                w.write_u16::<LittleEndian>(*r_dst)?;
                w.write_u16::<LittleEndian>(*count)?;
                w.write_u16::<LittleEndian>(*r_base)?;
            }
        }

        Ok(())
    }

    /// Decode a single instruction from the given reader.
    pub fn decode<R: Read>(r: &mut R) -> Result<Self, DecodeError> {
        let opcode = r.read_u16::<LittleEndian>()?;

        match opcode {
            // ── 0x00 Meta ──────────────────────────────────────
            0x0000 => Ok(Instruction::Nop),
            0x0001 => Ok(Instruction::Crash { r_msg: r.read_u16::<LittleEndian>()? }),

            // ── 0x01 Data Movement ─────────────────────────────
            0x0100 => Ok(Instruction::Mov {
                r_dst: r.read_u16::<LittleEndian>()?,
                r_src: r.read_u16::<LittleEndian>()?,
            }),
            0x0101 => Ok(Instruction::LoadInt {
                r_dst: r.read_u16::<LittleEndian>()?,
                value: r.read_u64::<LittleEndian>()? as i64,
            }),
            0x0102 => Ok(Instruction::LoadFloat {
                r_dst: r.read_u16::<LittleEndian>()?,
                value: f64::from_bits(r.read_u64::<LittleEndian>()?),
            }),
            0x0103 => Ok(Instruction::LoadTrue { r_dst: r.read_u16::<LittleEndian>()? }),
            0x0104 => Ok(Instruction::LoadFalse { r_dst: r.read_u16::<LittleEndian>()? }),
            0x0105 => Ok(Instruction::LoadString {
                r_dst: r.read_u16::<LittleEndian>()?,
                string_idx: r.read_u32::<LittleEndian>()?,
            }),
            0x0106 => Ok(Instruction::LoadNull { r_dst: r.read_u16::<LittleEndian>()? }),

            // ── 0x02 Integer Arithmetic ────────────────────────
            0x0200 => read_rrr(r).map(|(d, a, b)| Instruction::AddI { r_dst: d, r_a: a, r_b: b }),
            0x0201 => read_rrr(r).map(|(d, a, b)| Instruction::SubI { r_dst: d, r_a: a, r_b: b }),
            0x0202 => read_rrr(r).map(|(d, a, b)| Instruction::MulI { r_dst: d, r_a: a, r_b: b }),
            0x0203 => read_rrr(r).map(|(d, a, b)| Instruction::DivI { r_dst: d, r_a: a, r_b: b }),
            0x0204 => read_rrr(r).map(|(d, a, b)| Instruction::ModI { r_dst: d, r_a: a, r_b: b }),
            0x0205 => read_rr(r).map(|(d, s)| Instruction::NegI { r_dst: d, r_src: s }),

            // ── 0x03 Float Arithmetic ──────────────────────────
            0x0300 => read_rrr(r).map(|(d, a, b)| Instruction::AddF { r_dst: d, r_a: a, r_b: b }),
            0x0301 => read_rrr(r).map(|(d, a, b)| Instruction::SubF { r_dst: d, r_a: a, r_b: b }),
            0x0302 => read_rrr(r).map(|(d, a, b)| Instruction::MulF { r_dst: d, r_a: a, r_b: b }),
            0x0303 => read_rrr(r).map(|(d, a, b)| Instruction::DivF { r_dst: d, r_a: a, r_b: b }),
            0x0304 => read_rrr(r).map(|(d, a, b)| Instruction::ModF { r_dst: d, r_a: a, r_b: b }),
            0x0305 => read_rr(r).map(|(d, s)| Instruction::NegF { r_dst: d, r_src: s }),

            // ── 0x04 Bitwise & Logical ─────────────────────────
            0x0400 => read_rrr(r).map(|(d, a, b)| Instruction::BitAnd { r_dst: d, r_a: a, r_b: b }),
            0x0401 => read_rrr(r).map(|(d, a, b)| Instruction::BitOr { r_dst: d, r_a: a, r_b: b }),
            0x0402 => read_rrr(r).map(|(d, a, b)| Instruction::Shl { r_dst: d, r_a: a, r_b: b }),
            0x0403 => read_rrr(r).map(|(d, a, b)| Instruction::Shr { r_dst: d, r_a: a, r_b: b }),
            0x0404 => read_rr(r).map(|(d, s)| Instruction::Not { r_dst: d, r_src: s }),

            // ── 0x05 Comparison ────────────────────────────────
            0x0500 => read_rrr(r).map(|(d, a, b)| Instruction::CmpEqI { r_dst: d, r_a: a, r_b: b }),
            0x0501 => read_rrr(r).map(|(d, a, b)| Instruction::CmpEqF { r_dst: d, r_a: a, r_b: b }),
            0x0502 => read_rrr(r).map(|(d, a, b)| Instruction::CmpEqB { r_dst: d, r_a: a, r_b: b }),
            0x0503 => read_rrr(r).map(|(d, a, b)| Instruction::CmpEqS { r_dst: d, r_a: a, r_b: b }),
            0x0504 => read_rrr(r).map(|(d, a, b)| Instruction::CmpLtI { r_dst: d, r_a: a, r_b: b }),
            0x0505 => read_rrr(r).map(|(d, a, b)| Instruction::CmpLtF { r_dst: d, r_a: a, r_b: b }),

            // ── 0x06 Control Flow ──────────────────────────────
            0x0600 => {
                let _pad = r.read_u16::<LittleEndian>()?;
                let offset = r.read_u32::<LittleEndian>()? as i32;
                Ok(Instruction::Br { offset })
            }
            0x0601 => {
                let r_cond = r.read_u16::<LittleEndian>()?;
                let offset = r.read_u32::<LittleEndian>()? as i32;
                Ok(Instruction::BrTrue { r_cond, offset })
            }
            0x0602 => {
                let r_cond = r.read_u16::<LittleEndian>()?;
                let offset = r.read_u32::<LittleEndian>()? as i32;
                Ok(Instruction::BrFalse { r_cond, offset })
            }
            0x0603 => {
                let r_tag = r.read_u16::<LittleEndian>()?;
                let n = r.read_u16::<LittleEndian>()? as usize;
                let mut offsets = Vec::with_capacity(n);
                for _ in 0..n {
                    offsets.push(r.read_i32::<LittleEndian>()?);
                }
                Ok(Instruction::Switch { r_tag, offsets })
            }
            0x0604 => Ok(Instruction::Ret { r_src: r.read_u16::<LittleEndian>()? }),
            0x0605 => Ok(Instruction::RetVoid),

            // ── 0x07 Calls & Delegates ─────────────────────────
            0x0700 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let method_idx = r.read_u32::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::Call { r_dst, method_idx, r_base, argc })
            }
            0x0701 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_obj = r.read_u16::<LittleEndian>()?;
                let contract_idx = r.read_u32::<LittleEndian>()?;
                let slot = r.read_u16::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc })
            }
            0x0702 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let extern_idx = r.read_u32::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::CallExtern { r_dst, extern_idx, r_base, argc })
            }
            0x0703 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let method_idx = r.read_u32::<LittleEndian>()?;
                let r_target = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::NewDelegate { r_dst, method_idx, r_target })
            }
            0x0704 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_delegate = r.read_u16::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::CallIndirect { r_dst, r_delegate, r_base, argc })
            }
            0x0705 => {
                let method_idx = r.read_u32::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::TailCall { method_idx, r_base, argc })
            }

            // ── 0x08 Object Model ──────────────────────────────
            0x0800 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let type_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::New { r_dst, type_idx })
            }
            0x0801 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_obj = r.read_u16::<LittleEndian>()?;
                let field_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::GetField { r_dst, r_obj, field_idx })
            }
            0x0802 => {
                let r_obj = r.read_u16::<LittleEndian>()?;
                let field_idx = r.read_u32::<LittleEndian>()?;
                let r_val = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::SetField { r_obj, field_idx, r_val })
            }
            0x0803 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let type_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::SpawnEntity { r_dst, type_idx })
            }
            0x0804 => Ok(Instruction::InitEntity { r_entity: r.read_u16::<LittleEndian>()? }),
            0x0805 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_entity = r.read_u16::<LittleEndian>()?;
                let comp_type_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::GetComponent { r_dst, r_entity, comp_type_idx })
            }
            0x0806 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let type_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::GetOrCreate { r_dst, type_idx })
            }
            0x0807 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let type_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::FindAll { r_dst, type_idx })
            }
            0x0808 => Ok(Instruction::DestroyEntity { r_entity: r.read_u16::<LittleEndian>()? }),
            0x0809 => read_rr(r).map(|(d, e)| Instruction::EntityIsAlive { r_dst: d, r_entity: e }),

            // ── 0x09 Arrays ────────────────────────────────────
            0x0900 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let elem_type = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::NewArray { r_dst, elem_type })
            }
            0x0901 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let elem_type = r.read_u32::<LittleEndian>()?;
                let count = r.read_u16::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::ArrayInit { r_dst, elem_type, count, r_base })
            }
            0x0902 => read_rrr(r).map(|(d, a, i)| Instruction::ArrayLoad { r_dst: d, r_arr: a, r_idx: i }),
            0x0903 => read_rrr(r).map(|(a, i, v)| Instruction::ArrayStore { r_arr: a, r_idx: i, r_val: v }),
            0x0904 => read_rr(r).map(|(d, a)| Instruction::ArrayLen { r_dst: d, r_arr: a }),
            0x0905 => read_rr(r).map(|(a, v)| Instruction::ArrayAdd { r_arr: a, r_val: v }),
            0x0906 => read_rr(r).map(|(a, i)| Instruction::ArrayRemove { r_arr: a, r_idx: i }),
            0x0907 => read_rrr(r).map(|(a, i, v)| Instruction::ArrayInsert { r_arr: a, r_idx: i, r_val: v }),
            0x0908 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_arr = r.read_u16::<LittleEndian>()?;
                let r_start = r.read_u16::<LittleEndian>()?;
                let r_end = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::ArraySlice { r_dst, r_arr, r_start, r_end })
            }

            // ── 0x0A Type Operations — Option ──────────────────
            0x0A00 => read_rr(r).map(|(d, v)| Instruction::WrapSome { r_dst: d, r_val: v }),
            0x0A01 => read_rr(r).map(|(d, o)| Instruction::Unwrap { r_dst: d, r_opt: o }),
            0x0A02 => read_rr(r).map(|(d, o)| Instruction::IsSome { r_dst: d, r_opt: o }),
            0x0A03 => read_rr(r).map(|(d, o)| Instruction::IsNone { r_dst: d, r_opt: o }),

            // ── 0x0A Type Operations — Result ──────────────────
            0x0A10 => read_rr(r).map(|(d, v)| Instruction::WrapOk { r_dst: d, r_val: v }),
            0x0A11 => read_rr(r).map(|(d, e)| Instruction::WrapErr { r_dst: d, r_err: e }),
            0x0A12 => read_rr(r).map(|(d, res)| Instruction::UnwrapOk { r_dst: d, r_result: res }),
            0x0A13 => read_rr(r).map(|(d, res)| Instruction::IsOk { r_dst: d, r_result: res }),
            0x0A14 => read_rr(r).map(|(d, res)| Instruction::IsErr { r_dst: d, r_result: res }),
            0x0A15 => read_rr(r).map(|(d, res)| Instruction::ExtractErr { r_dst: d, r_result: res }),

            // ── 0x0A Type Operations — Enum ────────────────────
            0x0A20 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let type_idx = r.read_u32::<LittleEndian>()?;
                let tag = r.read_u16::<LittleEndian>()?;
                let field_count = r.read_u16::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::NewEnum { r_dst, type_idx, tag, field_count, r_base })
            }
            0x0A21 => read_rr(r).map(|(d, e)| Instruction::GetTag { r_dst: d, r_enum: e }),
            0x0A22 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_enum = r.read_u16::<LittleEndian>()?;
                let field_idx = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::ExtractField { r_dst, r_enum, field_idx })
            }

            // ── 0x0B Concurrency ───────────────────────────────
            0x0B00 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let method_idx = r.read_u32::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::SpawnTask { r_dst, method_idx, r_base, argc })
            }
            0x0B01 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let method_idx = r.read_u32::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                let argc = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::SpawnDetached { r_dst, method_idx, r_base, argc })
            }
            0x0B02 => read_rr(r).map(|(d, t)| Instruction::Join { r_dst: d, r_task: t }),
            0x0B03 => Ok(Instruction::Cancel { r_task: r.read_u16::<LittleEndian>()? }),
            0x0B04 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let method_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::DeferPush { r_dst, method_idx })
            }
            0x0B05 => Ok(Instruction::DeferPop),
            0x0B06 => Ok(Instruction::DeferEnd),

            // ── 0x0C Globals & Atomics ─────────────────────────
            0x0C00 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let global_idx = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::LoadGlobal { r_dst, global_idx })
            }
            0x0C01 => {
                let global_idx = r.read_u32::<LittleEndian>()?;
                let r_src = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::StoreGlobal { global_idx, r_src })
            }
            0x0C02 => Ok(Instruction::AtomicBegin),
            0x0C03 => Ok(Instruction::AtomicEnd),

            // ── 0x0D Conversion ────────────────────────────────
            0x0D00 => read_rr(r).map(|(d, s)| Instruction::I2f { r_dst: d, r_src: s }),
            0x0D01 => read_rr(r).map(|(d, s)| Instruction::F2i { r_dst: d, r_src: s }),
            0x0D02 => read_rr(r).map(|(d, s)| Instruction::I2s { r_dst: d, r_src: s }),
            0x0D03 => read_rr(r).map(|(d, s)| Instruction::F2s { r_dst: d, r_src: s }),
            0x0D04 => read_rr(r).map(|(d, s)| Instruction::B2s { r_dst: d, r_src: s }),
            0x0D05 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let r_src = r.read_u16::<LittleEndian>()?;
                let target_type = r.read_u32::<LittleEndian>()?;
                Ok(Instruction::Convert { r_dst, r_src, target_type })
            }

            // ── 0x0E Strings ───────────────────────────────────
            0x0E00 => read_rrr(r).map(|(d, a, b)| Instruction::StrConcat { r_dst: d, r_a: a, r_b: b }),
            0x0E01 => {
                let r_dst = r.read_u16::<LittleEndian>()?;
                let count = r.read_u16::<LittleEndian>()?;
                let r_base = r.read_u16::<LittleEndian>()?;
                Ok(Instruction::StrBuild { r_dst, count, r_base })
            }
            0x0E02 => read_rr(r).map(|(d, s)| Instruction::StrLen { r_dst: d, r_str: s }),

            // ── 0x0F Boxing ────────────────────────────────────
            0x0F00 => read_rr(r).map(|(d, v)| Instruction::Box { r_dst: d, r_val: v }),
            0x0F01 => read_rr(r).map(|(d, b)| Instruction::Unbox { r_dst: d, r_boxed: b }),

            _ => Err(DecodeError::InvalidOpcode(opcode)),
        }
    }
}

/// Implement PartialEq manually to handle f64 comparison via to_bits().
impl PartialEq for Instruction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Instruction::LoadFloat { r_dst: d1, value: v1 }, Instruction::LoadFloat { r_dst: d2, value: v2 }) => {
                d1 == d2 && v1.to_bits() == v2.to_bits()
            }
            // For all other variants, use structural comparison
            _ => {
                // Encode both and compare bytes (reliable for all variants)
                let mut buf1 = Vec::new();
                let mut buf2 = Vec::new();
                if self.encode(&mut buf1).is_ok() && other.encode(&mut buf2).is_ok() {
                    buf1 == buf2
                } else {
                    false
                }
            }
        }
    }
}

impl Eq for Instruction {}

// ── Helpers ────────────────────────────────────────────────────────

fn read_rr<R: Read>(r: &mut R) -> Result<(u16, u16), DecodeError> {
    let a = r.read_u16::<LittleEndian>()?;
    let b = r.read_u16::<LittleEndian>()?;
    Ok((a, b))
}

fn read_rrr<R: Read>(r: &mut R) -> Result<(u16, u16, u16), DecodeError> {
    let a = r.read_u16::<LittleEndian>()?;
    let b = r.read_u16::<LittleEndian>()?;
    let c = r.read_u16::<LittleEndian>()?;
    Ok((a, b, c))
}
