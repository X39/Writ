use std::io::Cursor;
use writ_module::Instruction;
use writ_module::error::DecodeError;

/// Encode an instruction to bytes, then decode it back, and assert equality.
fn round_trip(instr: &Instruction) -> Instruction {
    let mut buf = Vec::new();
    instr.encode(&mut buf).expect("encode should succeed");
    let mut cursor = Cursor::new(&buf[..]);
    let decoded = Instruction::decode(&mut cursor).expect("decode should succeed");
    assert_eq!(
        *instr, decoded,
        "round-trip failed for opcode 0x{:04X}",
        instr.opcode()
    );
    decoded
}

// ── Shape N ────────────────────────────────────────────────────

#[test]
fn test_nop_round_trip() {
    round_trip(&Instruction::Nop);
}

#[test]
fn test_ret_void_round_trip() {
    round_trip(&Instruction::RetVoid);
}

#[test]
fn test_defer_pop_round_trip() {
    round_trip(&Instruction::DeferPop);
}

#[test]
fn test_defer_end_round_trip() {
    round_trip(&Instruction::DeferEnd);
}

#[test]
fn test_atomic_begin_round_trip() {
    round_trip(&Instruction::AtomicBegin);
}

#[test]
fn test_atomic_end_round_trip() {
    round_trip(&Instruction::AtomicEnd);
}

// ── Shape R ────────────────────────────────────────────────────

#[test]
fn test_crash_round_trip() {
    round_trip(&Instruction::Crash { r_msg: 7 });
}

#[test]
fn test_load_true_round_trip() {
    round_trip(&Instruction::LoadTrue { r_dst: 0 });
}

#[test]
fn test_load_false_round_trip() {
    round_trip(&Instruction::LoadFalse { r_dst: 3 });
}

#[test]
fn test_load_null_round_trip() {
    round_trip(&Instruction::LoadNull { r_dst: 10 });
}

#[test]
fn test_ret_round_trip() {
    round_trip(&Instruction::Ret { r_src: 5 });
}

#[test]
fn test_init_entity_round_trip() {
    round_trip(&Instruction::InitEntity { r_entity: 2 });
}

#[test]
fn test_destroy_entity_round_trip() {
    round_trip(&Instruction::DestroyEntity { r_entity: 4 });
}

#[test]
fn test_cancel_round_trip() {
    round_trip(&Instruction::Cancel { r_task: 9 });
}

// ── Shape RR ───────────────────────────────────────────────────

#[test]
fn test_mov_round_trip() {
    round_trip(&Instruction::Mov { r_dst: 1, r_src: 2 });
}

#[test]
fn test_neg_i_round_trip() {
    round_trip(&Instruction::NegI { r_dst: 0, r_src: 1 });
}

#[test]
fn test_neg_f_round_trip() {
    round_trip(&Instruction::NegF { r_dst: 3, r_src: 4 });
}

#[test]
fn test_not_round_trip() {
    round_trip(&Instruction::Not { r_dst: 5, r_src: 6 });
}

#[test]
fn test_array_len_round_trip() {
    round_trip(&Instruction::ArrayLen { r_dst: 0, r_arr: 1 });
}

#[test]
fn test_array_add_round_trip() {
    round_trip(&Instruction::ArrayAdd { r_arr: 2, r_val: 3 });
}

#[test]
fn test_array_remove_round_trip() {
    round_trip(&Instruction::ArrayRemove { r_arr: 4, r_idx: 5 });
}

// ── Shape RRR ──────────────────────────────────────────────────

#[test]
fn test_add_i_round_trip() {
    round_trip(&Instruction::AddI { r_dst: 0, r_a: 1, r_b: 2 });
}

#[test]
fn test_str_concat_round_trip() {
    round_trip(&Instruction::StrConcat { r_dst: 7, r_a: 8, r_b: 9 });
}

#[test]
fn test_array_load_round_trip() {
    round_trip(&Instruction::ArrayLoad { r_dst: 0, r_arr: 1, r_idx: 2 });
}

#[test]
fn test_array_store_round_trip() {
    round_trip(&Instruction::ArrayStore { r_arr: 3, r_idx: 4, r_val: 5 });
}

#[test]
fn test_array_insert_round_trip() {
    round_trip(&Instruction::ArrayInsert { r_arr: 6, r_idx: 7, r_val: 8 });
}

// ── Shape RI32 ─────────────────────────────────────────────────

#[test]
fn test_load_string_round_trip() {
    round_trip(&Instruction::LoadString { r_dst: 0, string_idx: 100 });
}

#[test]
fn test_new_round_trip() {
    round_trip(&Instruction::New { r_dst: 1, type_idx: 0x02_000001 });
}

#[test]
fn test_br_true_round_trip() {
    round_trip(&Instruction::BrTrue { r_cond: 0, offset: 42 });
}

#[test]
fn test_br_false_negative_offset_round_trip() {
    round_trip(&Instruction::BrFalse { r_cond: 3, offset: -10 });
}

// ── Shape RI64 ─────────────────────────────────────────────────

#[test]
fn test_load_int_round_trip() {
    round_trip(&Instruction::LoadInt { r_dst: 0, value: 42 });
}

#[test]
fn test_load_int_max_round_trip() {
    round_trip(&Instruction::LoadInt { r_dst: 1, value: i64::MAX });
}

#[test]
fn test_load_int_min_round_trip() {
    round_trip(&Instruction::LoadInt { r_dst: 2, value: i64::MIN });
}

#[test]
fn test_load_float_round_trip() {
    round_trip(&Instruction::LoadFloat { r_dst: 0, value: std::f64::consts::PI });
}

#[test]
fn test_load_float_neg_zero_round_trip() {
    round_trip(&Instruction::LoadFloat { r_dst: 1, value: -0.0f64 });
}

#[test]
fn test_load_float_nan_round_trip() {
    let instr = Instruction::LoadFloat { r_dst: 2, value: f64::NAN };
    let mut buf = Vec::new();
    instr.encode(&mut buf).unwrap();
    let decoded = Instruction::decode(&mut Cursor::new(&buf[..])).unwrap();
    // NaN != NaN, so check bits
    if let Instruction::LoadFloat { r_dst, value } = decoded {
        assert_eq!(r_dst, 2);
        assert!(value.is_nan());
    } else {
        panic!("Expected LoadFloat");
    }
}

// ── Shape I32 ──────────────────────────────────────────────────

#[test]
fn test_br_round_trip() {
    round_trip(&Instruction::Br { offset: 100 });
}

#[test]
fn test_br_negative_offset_round_trip() {
    round_trip(&Instruction::Br { offset: -50 });
}

// ── Shape CALL ─────────────────────────────────────────────────

#[test]
fn test_call_round_trip() {
    round_trip(&Instruction::Call { r_dst: 0, method_idx: 0x07_000001, r_base: 1, argc: 3 });
}

#[test]
fn test_call_extern_round_trip() {
    round_trip(&Instruction::CallExtern { r_dst: 5, extern_idx: 0x10_000002, r_base: 6, argc: 1 });
}

#[test]
fn test_spawn_task_round_trip() {
    round_trip(&Instruction::SpawnTask { r_dst: 0, method_idx: 100, r_base: 1, argc: 2 });
}

#[test]
fn test_spawn_detached_round_trip() {
    round_trip(&Instruction::SpawnDetached { r_dst: 3, method_idx: 200, r_base: 4, argc: 0 });
}

// ── Variable-layout ────────────────────────────────────────────

#[test]
fn test_switch_empty_round_trip() {
    round_trip(&Instruction::Switch { r_tag: 0, offsets: vec![] });
}

#[test]
fn test_switch_one_round_trip() {
    round_trip(&Instruction::Switch { r_tag: 1, offsets: vec![42] });
}

#[test]
fn test_switch_five_round_trip() {
    round_trip(&Instruction::Switch { r_tag: 2, offsets: vec![10, -20, 30, -40, 50] });
}

#[test]
fn test_call_virt_round_trip() {
    round_trip(&Instruction::CallVirt {
        r_dst: 0, r_obj: 1, contract_idx: 0x0A_000003, slot: 2, r_base: 3, argc: 1,
    });
}

#[test]
fn test_new_delegate_round_trip() {
    round_trip(&Instruction::NewDelegate { r_dst: 0, method_idx: 0x07_000005, r_target: 1 });
}

#[test]
fn test_call_indirect_round_trip() {
    round_trip(&Instruction::CallIndirect { r_dst: 0, r_delegate: 1, r_base: 2, argc: 3 });
}

#[test]
fn test_tail_call_round_trip() {
    round_trip(&Instruction::TailCall { method_idx: 0x07_000010, r_base: 0, argc: 2 });
}

#[test]
fn test_get_field_round_trip() {
    round_trip(&Instruction::GetField { r_dst: 0, r_obj: 1, field_idx: 0x05_000001 });
}

#[test]
fn test_set_field_round_trip() {
    round_trip(&Instruction::SetField { r_obj: 1, field_idx: 0x05_000002, r_val: 2 });
}

#[test]
fn test_get_component_round_trip() {
    round_trip(&Instruction::GetComponent { r_dst: 0, r_entity: 1, comp_type_idx: 0x02_000003 });
}

#[test]
fn test_array_init_round_trip() {
    round_trip(&Instruction::ArrayInit { r_dst: 0, elem_type: 0x04_000001, count: 5, r_base: 1 });
}

#[test]
fn test_array_slice_round_trip() {
    round_trip(&Instruction::ArraySlice { r_dst: 0, r_arr: 1, r_start: 2, r_end: 3 });
}

#[test]
fn test_new_enum_round_trip() {
    round_trip(&Instruction::NewEnum { r_dst: 0, type_idx: 0x02_000005, tag: 3, field_count: 2, r_base: 1 });
}

#[test]
fn test_extract_field_round_trip() {
    round_trip(&Instruction::ExtractField { r_dst: 0, r_enum: 1, field_idx: 2 });
}

#[test]
fn test_store_global_round_trip() {
    round_trip(&Instruction::StoreGlobal { global_idx: 0x0F_000001, r_src: 5 });
}

#[test]
fn test_convert_round_trip() {
    round_trip(&Instruction::Convert { r_dst: 0, r_src: 1, target_type: 0x04_000002 });
}

#[test]
fn test_str_build_round_trip() {
    round_trip(&Instruction::StrBuild { r_dst: 0, count: 3, r_base: 1 });
}

// ── Comprehensive all-91 test ──────────────────────────────────

#[test]
fn test_all_91_opcodes_round_trip() {
    let instructions: Vec<Instruction> = vec![
        // 0x00 Meta (2)
        Instruction::Nop,
        Instruction::Crash { r_msg: 1 },
        // 0x01 Data Movement (7)
        Instruction::Mov { r_dst: 0, r_src: 1 },
        Instruction::LoadInt { r_dst: 0, value: 999 },
        Instruction::LoadFloat { r_dst: 0, value: 3.14 },
        Instruction::LoadTrue { r_dst: 0 },
        Instruction::LoadFalse { r_dst: 0 },
        Instruction::LoadString { r_dst: 0, string_idx: 42 },
        Instruction::LoadNull { r_dst: 0 },
        // 0x02 Integer Arithmetic (6)
        Instruction::AddI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::SubI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::MulI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::DivI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::ModI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::NegI { r_dst: 0, r_src: 1 },
        // 0x03 Float Arithmetic (6)
        Instruction::AddF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::SubF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::MulF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::DivF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::ModF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::NegF { r_dst: 0, r_src: 1 },
        // 0x04 Bitwise & Logical (5)
        Instruction::BitAnd { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::BitOr { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::Shl { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::Shr { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::Not { r_dst: 0, r_src: 1 },
        // 0x05 Comparison (6)
        Instruction::CmpEqI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::CmpEqF { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::CmpEqB { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::CmpEqS { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::CmpLtI { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::CmpLtF { r_dst: 0, r_a: 1, r_b: 2 },
        // 0x06 Control Flow (6)
        Instruction::Br { offset: 10 },
        Instruction::BrTrue { r_cond: 0, offset: 20 },
        Instruction::BrFalse { r_cond: 0, offset: -5 },
        Instruction::Switch { r_tag: 0, offsets: vec![1, 2, 3] },
        Instruction::Ret { r_src: 0 },
        Instruction::RetVoid,
        // 0x07 Calls & Delegates (6)
        Instruction::Call { r_dst: 0, method_idx: 100, r_base: 1, argc: 2 },
        Instruction::CallVirt { r_dst: 0, r_obj: 1, contract_idx: 200, slot: 0, r_base: 2, argc: 1 },
        Instruction::CallExtern { r_dst: 0, extern_idx: 300, r_base: 1, argc: 0 },
        Instruction::NewDelegate { r_dst: 0, method_idx: 400, r_target: 1 },
        Instruction::CallIndirect { r_dst: 0, r_delegate: 1, r_base: 2, argc: 3 },
        Instruction::TailCall { method_idx: 500, r_base: 0, argc: 1 },
        // 0x08 Object Model (10)
        Instruction::New { r_dst: 0, type_idx: 100 },
        Instruction::GetField { r_dst: 0, r_obj: 1, field_idx: 200 },
        Instruction::SetField { r_obj: 1, field_idx: 300, r_val: 2 },
        Instruction::SpawnEntity { r_dst: 0, type_idx: 400 },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::GetComponent { r_dst: 0, r_entity: 1, comp_type_idx: 500 },
        Instruction::GetOrCreate { r_dst: 0, type_idx: 600 },
        Instruction::FindAll { r_dst: 0, type_idx: 700 },
        Instruction::DestroyEntity { r_entity: 0 },
        Instruction::EntityIsAlive { r_dst: 0, r_entity: 1 },
        // 0x09 Arrays (9)
        Instruction::NewArray { r_dst: 0, elem_type: 100 },
        Instruction::ArrayInit { r_dst: 0, elem_type: 200, count: 5, r_base: 1 },
        Instruction::ArrayLoad { r_dst: 0, r_arr: 1, r_idx: 2 },
        Instruction::ArrayStore { r_arr: 0, r_idx: 1, r_val: 2 },
        Instruction::ArrayLen { r_dst: 0, r_arr: 1 },
        Instruction::ArrayAdd { r_arr: 0, r_val: 1 },
        Instruction::ArrayRemove { r_arr: 0, r_idx: 1 },
        Instruction::ArrayInsert { r_arr: 0, r_idx: 1, r_val: 2 },
        Instruction::ArraySlice { r_dst: 0, r_arr: 1, r_start: 2, r_end: 3 },
        // 0x0A Option (4)
        Instruction::WrapSome { r_dst: 0, r_val: 1 },
        Instruction::Unwrap { r_dst: 0, r_opt: 1 },
        Instruction::IsSome { r_dst: 0, r_opt: 1 },
        Instruction::IsNone { r_dst: 0, r_opt: 1 },
        // 0x0A Result (6)
        Instruction::WrapOk { r_dst: 0, r_val: 1 },
        Instruction::WrapErr { r_dst: 0, r_err: 1 },
        Instruction::UnwrapOk { r_dst: 0, r_result: 1 },
        Instruction::IsOk { r_dst: 0, r_result: 1 },
        Instruction::IsErr { r_dst: 0, r_result: 1 },
        Instruction::ExtractErr { r_dst: 0, r_result: 1 },
        // 0x0A Enum (3)
        Instruction::NewEnum { r_dst: 0, type_idx: 100, tag: 1, field_count: 2, r_base: 1 },
        Instruction::GetTag { r_dst: 0, r_enum: 1 },
        Instruction::ExtractField { r_dst: 0, r_enum: 1, field_idx: 0 },
        // 0x0B Concurrency (7)
        Instruction::SpawnTask { r_dst: 0, method_idx: 100, r_base: 1, argc: 2 },
        Instruction::SpawnDetached { r_dst: 0, method_idx: 200, r_base: 1, argc: 0 },
        Instruction::Join { r_dst: 0, r_task: 1 },
        Instruction::Cancel { r_task: 0 },
        Instruction::DeferPush { r_dst: 0, method_idx: 300 },
        Instruction::DeferPop,
        Instruction::DeferEnd,
        // 0x0C Globals & Atomics (4)
        Instruction::LoadGlobal { r_dst: 0, global_idx: 100 },
        Instruction::StoreGlobal { global_idx: 200, r_src: 1 },
        Instruction::AtomicBegin,
        Instruction::AtomicEnd,
        // 0x0D Conversion (6)
        Instruction::I2f { r_dst: 0, r_src: 1 },
        Instruction::F2i { r_dst: 0, r_src: 1 },
        Instruction::I2s { r_dst: 0, r_src: 1 },
        Instruction::F2s { r_dst: 0, r_src: 1 },
        Instruction::B2s { r_dst: 0, r_src: 1 },
        Instruction::Convert { r_dst: 0, r_src: 1, target_type: 300 },
        // 0x0E Strings (3)
        Instruction::StrConcat { r_dst: 0, r_a: 1, r_b: 2 },
        Instruction::StrBuild { r_dst: 0, count: 3, r_base: 1 },
        Instruction::StrLen { r_dst: 0, r_str: 1 },
        // 0x0F Boxing (2)
        Instruction::Box { r_dst: 0, r_val: 1 },
        Instruction::Unbox { r_dst: 0, r_boxed: 1 },
    ];

    // The plan references "91 opcodes" but the actual opcode assignment table (spec section 4.2)
    // defines 98 distinct opcodes when fully counted across all categories.
    assert_eq!(instructions.len(), 98, "expected exactly 98 instructions (all opcodes from spec section 4.2)");

    for instr in &instructions {
        round_trip(instr);
    }
}

// ── Error cases ────────────────────────────────────────────────

#[test]
fn test_invalid_opcode_returns_error() {
    let bytes: [u8; 4] = [0xFF, 0xFF, 0x00, 0x00]; // opcode 0xFFFF
    let mut cursor = Cursor::new(&bytes[..]);
    let result = Instruction::decode(&mut cursor);
    assert!(result.is_err());
    match result.unwrap_err() {
        DecodeError::InvalidOpcode(op) => assert_eq!(op, 0xFFFF),
        other => panic!("Expected InvalidOpcode, got {other:?}"),
    }
}
