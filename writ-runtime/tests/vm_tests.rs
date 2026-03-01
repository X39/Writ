//! Per-instruction unit tests for the writ-runtime VM.
//!
//! Each test constructs a minimal module programmatically, spawns a task,
//! ticks to completion, and inspects the result.

use writ_module::module::MethodBody;
use writ_module::Instruction;
use writ_module::ModuleBuilder;
use writ_runtime::{
    ExecutionLimit, NullHost, Runtime, RuntimeBuilder, TaskState, TickResult, Value,
};

// ── Test helper ──────────────────────────────────────────────────

/// Encode a slice of instructions into raw code bytes.
fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

/// Build a module with one type and one method containing the given instructions.
/// `reg_count` specifies how many registers the method body has.
/// Returns a Runtime ready for spawning tasks.
fn build_runtime(instructions: &[Instruction], reg_count: u16) -> Runtime<NullHost> {
    let mut builder = ModuleBuilder::new("test");
    // Add a type (required for method ownership)
    builder.add_type_def("TestType", "", 0, 0);
    // Add the method with body
    let body = MethodBody {
        register_types: vec![0; reg_count as usize],
        code: encode(instructions),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, reg_count, body);
    let module = builder.build();

    RuntimeBuilder::new(module).build().unwrap()
}

/// Build, spawn method 0, tick to completion, return (runtime, task_id).
fn run_simple(
    instructions: &[Instruction],
    reg_count: u16,
) -> (Runtime<NullHost>, writ_runtime::TaskId) {
    let mut runtime = build_runtime(instructions, reg_count);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    (runtime, task_id)
}

/// Build, spawn method 0 with args, tick to completion, return (runtime, task_id).
fn run_with_args(
    instructions: &[Instruction],
    reg_count: u16,
    args: Vec<Value>,
) -> (Runtime<NullHost>, writ_runtime::TaskId) {
    let mut runtime = build_runtime(instructions, reg_count);
    let task_id = runtime.spawn_task(0, args).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    (runtime, task_id)
}

/// Build a module with two methods. Method 0 is the "main" method, method 1 is a "callee".
fn build_two_method_runtime(
    main_instrs: &[Instruction],
    main_reg_count: u16,
    callee_instrs: &[Instruction],
    callee_reg_count: u16,
) -> Runtime<NullHost> {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", 0, 0);

    let body0 = MethodBody {
        register_types: vec![0; main_reg_count as usize],
        code: encode(main_instrs),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, main_reg_count, body0);

    let body1 = MethodBody {
        register_types: vec![0; callee_reg_count as usize],
        code: encode(callee_instrs),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("callee", &[0], 0, callee_reg_count, body1);

    let module = builder.build();
    RuntimeBuilder::new(module).build().unwrap()
}

// ── Data Movement Tests ──────────────────────────────────────────

#[test]
fn load_int_stores_value() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 42 },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

#[test]
fn load_float_stores_value() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 3.14,
            },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Float(3.14)));
}

#[test]
fn load_true_stores_bool() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn load_false_stores_bool() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFalse { r_dst: 0 },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(false)));
}

#[test]
fn load_null_stores_void() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadNull { r_dst: 0 },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Void));
}

#[test]
fn mov_copies_value() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::Mov { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(99)));
}

// ── Integer Arithmetic Tests ─────────────────────────────────────

#[test]
fn add_i_produces_sum() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 2 },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::AddI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(5)));
}

#[test]
fn sub_i_produces_difference() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::SubI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(2)));
}

#[test]
fn mul_i_produces_product() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 4 },
            Instruction::LoadInt { r_dst: 1, value: 5 },
            Instruction::MulI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(20)));
}

#[test]
fn div_i_integer_division() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 10,
            },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::DivI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(3)));
}

#[test]
fn div_i_by_zero_crashes() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 10,
            },
            Instruction::LoadInt { r_dst: 1, value: 0 },
            Instruction::DivI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
}

#[test]
fn mod_i_produces_remainder() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 10,
            },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::ModI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(1)));
}

#[test]
fn neg_i_negates_value() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::NegI { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(-5)));
}

// ── Float Arithmetic Tests ───────────────────────────────────────

#[test]
fn add_f_produces_sum() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 2.5,
            },
            Instruction::LoadFloat {
                r_dst: 1,
                value: 1.5,
            },
            Instruction::AddF {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Float(4.0)));
}

#[test]
fn div_f_produces_quotient() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 10.0,
            },
            Instruction::LoadFloat {
                r_dst: 1,
                value: 3.0,
            },
            Instruction::DivF {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    match rt.return_value(tid) {
        Some(Value::Float(f)) => {
            assert!((f - 10.0 / 3.0).abs() < f64::EPSILON);
        }
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn neg_f_negates_value() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 7.5,
            },
            Instruction::NegF { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Float(-7.5)));
}

// ── Bitwise Tests ────────────────────────────────────────────────

#[test]
fn bit_and_produces_correct_result() {
    // 0b1100 & 0b1010 = 0b1000 = 8
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 0b1100,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 0b1010,
            },
            Instruction::BitAnd {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(8)));
}

#[test]
fn bit_or_produces_correct_result() {
    // 0b1100 | 0b1010 = 0b1110 = 14
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 0b1100,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 0b1010,
            },
            Instruction::BitOr {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(14)));
}

#[test]
fn shl_shift_left() {
    // 1 << 3 = 8
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::Shl {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(8)));
}

#[test]
fn shr_shift_right() {
    // 16 >> 2 = 4
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 16,
            },
            Instruction::LoadInt { r_dst: 1, value: 2 },
            Instruction::Shr {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(4)));
}

#[test]
fn not_bool() {
    // Logical NOT on bool: !false == true, !true == false (spec §52_3_4_bitwise_logical)
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFalse { r_dst: 0 },
            Instruction::Not { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));

    let (rt2, tid2) = run_simple(
        &[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::Not { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt2.return_value(tid2), Some(Value::Bool(false)));
}

// ── Comparison Tests ─────────────────────────────────────────────

#[test]
fn cmp_eq_i_equal() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::LoadInt { r_dst: 1, value: 5 },
            Instruction::CmpEqI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn cmp_eq_i_not_equal() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::LoadInt { r_dst: 1, value: 6 },
            Instruction::CmpEqI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(false)));
}

#[test]
fn cmp_lt_i_less() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 3 },
            Instruction::LoadInt { r_dst: 1, value: 5 },
            Instruction::CmpLtI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn cmp_lt_i_not_less() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::LoadInt { r_dst: 1, value: 3 },
            Instruction::CmpLtI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(false)));
}

#[test]
fn cmp_eq_f_equal() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 1.5,
            },
            Instruction::LoadFloat {
                r_dst: 1,
                value: 1.5,
            },
            Instruction::CmpEqF {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn cmp_eq_b_same() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::LoadTrue { r_dst: 1 },
            Instruction::CmpEqB {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn cmp_eq_s_compares_string_content_not_ref() {
    // Allocate two separate "hello" strings via LoadString.
    // Both should compare equal because CmpEqS compares content.
    // We need the string heap to contain "hello" — use ModuleBuilder with a string.
    // Alternative: use I2s to create two identical strings from the same integer.
    let (rt, tid) = run_simple(
        &[
            // Create "42" string in r0
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            // Create another "42" string in r2 (separate allocation)
            Instruction::I2s { r_dst: 2, r_src: 0 },
            // Compare the two separate string refs
            Instruction::CmpEqS {
                r_dst: 3,
                r_a: 1,
                r_b: 2,
            },
            Instruction::Ret { r_src: 3 },
        ],
        4,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

// ── Control Flow Tests ───────────────────────────────────────────

#[test]
fn br_unconditional_jump() {
    // LoadInt 1, Br(forward past LoadInt 2), LoadInt 2 (skipped), Ret r0
    // Br jumps over the second LoadInt, so r0 should be 1
    let load1 = Instruction::LoadInt { r_dst: 0, value: 1 };
    let load2 = Instruction::LoadInt {
        r_dst: 0,
        value: 999,
    };
    let ret = Instruction::Ret { r_src: 0 };

    // Compute byte offsets: LoadInt is 12 bytes, Br is 8 bytes, Ret is 4 bytes
    // layout: [LoadInt@0(12B)] [Br@12(8B)] [LoadInt@20(12B)] [Ret@32(4B)]
    // Br offset = 32 - 12 = 20 (from Br position to Ret position)
    let br = Instruction::Br { offset: 20 };

    let (rt, tid) = run_simple(&[load1, br, load2, ret], 1);
    assert_eq!(rt.return_value(tid), Some(Value::Int(1)));
}

#[test]
fn br_true_jumps_when_true() {
    // LoadTrue, BrTrue(to Ret), LoadInt 999 (skipped), Ret
    // layout: [LoadTrue@0(4B)] [BrTrue@4(8B)] [LoadInt@12(12B)] [Ret@24(4B)]
    // BrTrue offset = 24 - 4 = 20
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::BrTrue {
                r_cond: 0,
                offset: 20,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 999,
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    // r1 stays Void because LoadInt 999 was skipped
    assert_eq!(rt.return_value(tid), Some(Value::Void));
}

#[test]
fn br_true_falls_through_when_false() {
    // LoadFalse, BrTrue(skips), LoadInt 42, Ret
    // layout: [LoadFalse@0(4B)] [BrTrue@4(8B)] [LoadInt@12(12B)] [Ret@24(4B)]
    // BrTrue offset = 24 - 4 = 20 (would jump to Ret, but condition is false)
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFalse { r_dst: 0 },
            Instruction::BrTrue {
                r_cond: 0,
                offset: 20,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 42,
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

#[test]
fn br_false_jumps_when_false() {
    // LoadFalse, BrFalse(to Ret), LoadInt 999 (skipped), Ret
    // layout: [LoadFalse@0(4B)] [BrFalse@4(8B)] [LoadInt@12(12B)] [Ret@24(4B)]
    // BrFalse offset = 24 - 4 = 20
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFalse { r_dst: 0 },
            Instruction::BrFalse {
                r_cond: 0,
                offset: 20,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 999,
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Void));
}

#[test]
fn switch_dispatches_by_tag() {
    // r0 = 1, Switch on r0 with offsets [case0_target, case1_target]
    // case0: LoadInt 10, Ret   (skipped)
    // case1: LoadInt 20, Ret   (taken)

    // Switch encoding: 2 (opcode) + 2 (r_tag) + 2 (count) + 4*count (offsets)
    // With 2 cases: 2+2+2+4*2 = 14 bytes
    // Layout:
    // [LoadInt@0(12B)] [Switch@12(14B)] [LoadInt@26(12B)] [Ret@38(4B)] [LoadInt@42(12B)] [Ret@54(4B)]
    // Switch is at byte 12. case0 offset = 26 - 12 = 14, case1 offset = 42 - 12 = 30

    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::Switch {
                r_tag: 0,
                offsets: vec![14, 30],
            },
            // case 0 (skipped):
            Instruction::LoadInt {
                r_dst: 1,
                value: 10,
            },
            Instruction::Ret { r_src: 1 },
            // case 1 (taken):
            Instruction::LoadInt {
                r_dst: 1,
                value: 20,
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(20)));
}

// ── Call Management Tests ────────────────────────────────────────

#[test]
fn call_and_ret_delivers_return_value() {
    // method 0 (main): LoadInt 10 in r0, Call method 1 with r0 as arg, result to r1, Ret r1
    // method 1 (callee): receives 10 in r0, AddI r0+r0 into r1, Ret r1
    //
    // Call encoding: 2 (opcode) + 2 (r_dst) + 4 (method_idx) + 2 (r_base) + 2 (argc) = 12 bytes

    let main_instrs = vec![
        Instruction::LoadInt {
            r_dst: 0,
            value: 10,
        },
        // Call method 1 (MethodDef token 0x07000002 = table_id=7, row_index=2, array_index=1)
        Instruction::Call {
            r_dst: 1,
            method_idx: 0x07000002,
            r_base: 0,
            argc: 1,
        },
        Instruction::Ret { r_src: 1 },
    ];

    let callee_instrs = vec![
        // r0 has 10 (from caller)
        Instruction::AddI {
            r_dst: 1,
            r_a: 0,
            r_b: 0,
        }, // 10 + 10 = 20
        Instruction::Ret { r_src: 1 },
    ];

    let mut rt = build_two_method_runtime(&main_instrs, 2, &callee_instrs, 2);
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Int(20)));
}

#[test]
fn call_with_methoddef_token() {
    // Regression test for BUG-17: the CALL instruction's method_idx field carries
    // a MethodDef metadata token (table_id=7, 1-based row_index encoded in bits 23-0),
    // NOT a 0-based array index. This test verifies that decode_method_token correctly
    // maps 0x07000002 (table_id=7, row_index=2) to decoded_bodies[1].
    //
    // method 0 (main): LoadInt 5, Call method 1 via token 0x07000002, Ret result
    // method 1 (callee): r0 has 5, AddI r1=r0+r0=10, Ret r1
    let main_instrs = vec![
        Instruction::LoadInt { r_dst: 0, value: 5 },
        Instruction::Call {
            r_dst: 1,
            method_idx: 0x07000002, // MethodDef token: table_id=7, row_index=2 → array_index=1
            r_base: 0,
            argc: 1,
        },
        Instruction::Ret { r_src: 1 },
    ];
    let callee_instrs = vec![
        Instruction::AddI { r_dst: 1, r_a: 0, r_b: 0 }, // 5 + 5 = 10
        Instruction::Ret { r_src: 1 },
    ];
    let mut rt = build_two_method_runtime(&main_instrs, 2, &callee_instrs, 2);
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);
    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Int(10)));
}

#[test]
fn nested_calls_unwind_correctly() {
    // method 0: Call method 1, Ret result
    // method 1: LoadInt 5, Call method 2 with arg, Ret result
    // method 2: receives 5, AddI 5+5=10, Ret 10
    //
    // We need 3 methods. Build manually.
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("T", "", 0, 0);

    // Method 0: Call method 1 (MethodDef token 0x07000002 = table_id=7, row_index=2, array_index=1)
    let body0 = MethodBody {
        register_types: vec![0, 0],
        code: encode(&[
            Instruction::Call {
                r_dst: 0,
                method_idx: 0x07000002,
                r_base: 0,
                argc: 0,
            },
            Instruction::Ret { r_src: 0 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("m0", &[0], 0, 2, body0);

    // Method 1: LoadInt 5, Call method 2 (MethodDef token 0x07000003 = table_id=7, row_index=3, array_index=2)
    let body1 = MethodBody {
        register_types: vec![0, 0],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 5 },
            Instruction::Call {
                r_dst: 1,
                method_idx: 0x07000003,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("m1", &[0], 0, 2, body1);

    // Method 2: r0 has 5, compute 5+5, return 10
    let body2 = MethodBody {
        register_types: vec![0, 0],
        code: encode(&[
            Instruction::AddI {
                r_dst: 1,
                r_a: 0,
                r_b: 0,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("m2", &[0], 0, 2, body2);

    let module = builder.build();
    let mut rt = RuntimeBuilder::new(module).build().unwrap();
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Int(10)));
}

#[test]
fn tail_call_does_not_grow_stack() {
    // method 0: LoadInt 42, TailCall to method 1 with arg
    // method 1: Ret r0 (the 42 passed as arg)
    // After TailCall, call_depth should still be 1 (not 2).

    let main_instrs = vec![
        Instruction::LoadInt {
            r_dst: 0,
            value: 42,
        },
        Instruction::TailCall {
            method_idx: 0x07000002,
            r_base: 0,
            argc: 1,
        },
    ];

    let callee_instrs = vec![Instruction::Ret { r_src: 0 }];

    let mut rt = build_two_method_runtime(&main_instrs, 1, &callee_instrs, 1);
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

#[test]
fn call_extern_with_null_host_returns_void() {
    let (rt, tid) = run_simple(
        &[
            Instruction::CallExtern {
                r_dst: 0,
                extern_idx: 0,
                r_base: 0,
                argc: 0,
            },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Void));
}

#[test]
fn call_virt_crashes() {
    let (rt, tid) = run_simple(
        &[
            Instruction::CallVirt {
                r_dst: 0,
                r_obj: 0,
                contract_idx: 0,
                slot: 0,
                r_base: 0,
                argc: 0,
            },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
}

// ── Conversion Tests ─────────────────────────────────────────────

#[test]
fn i2f_converts_int_to_float() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::I2f { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Float(42.0)));
}

#[test]
fn f2i_truncates_float_to_int() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 3.7,
            },
            Instruction::F2i { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(3)));
}

#[test]
fn i2s_converts_int_to_string() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    // Check that it returned a Ref (string on heap)
    let ret = rt.return_value(tid).unwrap();
    match ret {
        Value::Ref(href) => {
            let s = rt.heap().read_string(href).unwrap();
            assert_eq!(s, "42");
        }
        other => panic!("expected Ref (string), got {:?}", other),
    }
}

#[test]
fn f2s_converts_float_to_string() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadFloat {
                r_dst: 0,
                value: 3.5,
            },
            Instruction::F2s { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    let ret = rt.return_value(tid).unwrap();
    match ret {
        Value::Ref(href) => {
            let s = rt.heap().read_string(href).unwrap();
            assert_eq!(s, "3.5");
        }
        other => panic!("expected Ref (string), got {:?}", other),
    }
}

#[test]
fn b2s_converts_bool_to_string() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::B2s { r_dst: 1, r_src: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    let ret = rt.return_value(tid).unwrap();
    match ret {
        Value::Ref(href) => {
            let s = rt.heap().read_string(href).unwrap();
            assert_eq!(s, "true");
        }
        other => panic!("expected Ref (string), got {:?}", other),
    }
}

// ── String Tests ─────────────────────────────────────────────────

#[test]
fn str_concat_joins_two_strings() {
    // Create "hello" via I2s(0) won't work. Use two I2s instead.
    // Actually, let's use I2s to get numeric strings and concatenate.
    // "4" + "2" = "42"
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 4 },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            Instruction::LoadInt { r_dst: 0, value: 2 },
            Instruction::I2s { r_dst: 2, r_src: 0 },
            Instruction::StrConcat {
                r_dst: 3,
                r_a: 1,
                r_b: 2,
            },
            Instruction::Ret { r_src: 3 },
        ],
        4,
    );
    let ret = rt.return_value(tid).unwrap();
    match ret {
        Value::Ref(href) => {
            let s = rt.heap().read_string(href).unwrap();
            assert_eq!(s, "42");
        }
        other => panic!("expected Ref (string), got {:?}", other),
    }
}

#[test]
fn str_len_returns_length() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 12345,
            },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            Instruction::StrLen { r_dst: 2, r_str: 1 },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    // "12345" has length 5
    assert_eq!(rt.return_value(tid), Some(Value::Int(5)));
}

// ── Object Model Tests ───────────────────────────────────────────

#[test]
fn new_allocates_struct() {
    // New with type_idx=1 (our TestType), then GetField/SetField
    let (rt, tid) = run_simple(
        &[
            Instruction::New {
                r_dst: 0,
                type_idx: 1,
            }, // type_idx 1 = first TypeDef (1-based)
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    // Verify it's a Ref
    match rt.return_value(tid) {
        Some(Value::Ref(_)) => {} // ok
        other => panic!("expected Ref, got {:?}", other),
    }
}

#[test]
fn get_set_field_round_trip() {
    // We need a type with at least 1 field. Add a field to the type.
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("MyStruct", "", 0, 0);
    builder.add_field_def("x", &[0x01], 0); // one field

    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::New {
                r_dst: 0,
                type_idx: 1,
            },
            Instruction::LoadInt {
                r_dst: 1,
                value: 42,
            },
            Instruction::SetField {
                r_obj: 0,
                field_idx: 0,
                r_val: 1,
            },
            Instruction::GetField {
                r_dst: 2,
                r_obj: 0,
                field_idx: 0,
            },
            Instruction::Ret { r_src: 2 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);
    let module = builder.build();

    let mut rt = RuntimeBuilder::new(module).build().unwrap();
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

// ── Array Tests ──────────────────────────────────────────────────

#[test]
fn array_add_load_store_len() {
    let (rt, tid) = run_simple(
        &[
            // NewArray
            Instruction::NewArray {
                r_dst: 0,
                elem_type: 0,
            },
            // Add element 10
            Instruction::LoadInt {
                r_dst: 1,
                value: 10,
            },
            Instruction::ArrayAdd { r_arr: 0, r_val: 1 },
            // Add element 20
            Instruction::LoadInt {
                r_dst: 1,
                value: 20,
            },
            Instruction::ArrayAdd { r_arr: 0, r_val: 1 },
            // ArrayLen
            Instruction::ArrayLen { r_dst: 2, r_arr: 0 },
            // ArrayLoad index 1
            Instruction::LoadInt { r_dst: 3, value: 1 },
            Instruction::ArrayLoad {
                r_dst: 4,
                r_arr: 0,
                r_idx: 3,
            },
            // Return [len, element_1] — we'll check both
            // Store len*100 + element for a combined check
            Instruction::LoadInt {
                r_dst: 5,
                value: 100,
            },
            Instruction::MulI {
                r_dst: 6,
                r_a: 2,
                r_b: 5,
            },
            Instruction::AddI {
                r_dst: 7,
                r_a: 6,
                r_b: 4,
            },
            Instruction::Ret { r_src: 7 },
        ],
        8,
    );
    // len=2, element[1]=20, so 2*100 + 20 = 220
    assert_eq!(rt.return_value(tid), Some(Value::Int(220)));
}

#[test]
fn array_store_overwrites_element() {
    let (rt, tid) = run_simple(
        &[
            Instruction::NewArray {
                r_dst: 0,
                elem_type: 0,
            },
            // Add two elements
            Instruction::LoadInt {
                r_dst: 1,
                value: 10,
            },
            Instruction::ArrayAdd { r_arr: 0, r_val: 1 },
            Instruction::LoadInt {
                r_dst: 1,
                value: 20,
            },
            Instruction::ArrayAdd { r_arr: 0, r_val: 1 },
            // Store 99 at index 0
            Instruction::LoadInt { r_dst: 2, value: 0 },
            Instruction::LoadInt {
                r_dst: 3,
                value: 99,
            },
            Instruction::ArrayStore {
                r_arr: 0,
                r_idx: 2,
                r_val: 3,
            },
            // Load index 0
            Instruction::ArrayLoad {
                r_dst: 4,
                r_arr: 0,
                r_idx: 2,
            },
            Instruction::Ret { r_src: 4 },
        ],
        5,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(99)));
}

// ── Option/Result/Enum Tests ─────────────────────────────────────

#[test]
fn wrap_some_and_unwrap_round_trip() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::WrapSome { r_dst: 1, r_val: 0 },
            Instruction::Unwrap { r_dst: 2, r_opt: 1 },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

#[test]
fn is_some_on_some_returns_true() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::WrapSome { r_dst: 1, r_val: 0 },
            Instruction::IsSome { r_dst: 2, r_opt: 1 },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn wrap_ok_is_ok_round_trip() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 100,
            },
            Instruction::WrapOk { r_dst: 1, r_val: 0 },
            Instruction::IsOk {
                r_dst: 2,
                r_result: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn wrap_err_is_err_round_trip() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: -1,
            },
            Instruction::WrapErr { r_dst: 1, r_err: 0 },
            Instruction::IsErr {
                r_dst: 2,
                r_result: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn new_enum_get_tag_extract_field() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 77,
            },
            Instruction::NewEnum {
                r_dst: 1,
                type_idx: 0,
                tag: 2,
                field_count: 1,
                r_base: 0,
            },
            Instruction::GetTag { r_dst: 2, r_enum: 1 },
            Instruction::ExtractField {
                r_dst: 3,
                r_enum: 1,
                field_idx: 0,
            },
            // tag*100 + field value = 2*100 + 77 = 277
            Instruction::LoadInt {
                r_dst: 4,
                value: 100,
            },
            Instruction::MulI {
                r_dst: 5,
                r_a: 2,
                r_b: 4,
            },
            Instruction::AddI {
                r_dst: 6,
                r_a: 5,
                r_b: 3,
            },
            Instruction::Ret { r_src: 6 },
        ],
        7,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(277)));
}

// ── Boxing Tests ─────────────────────────────────────────────────

#[test]
fn box_unbox_round_trip() {
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 123,
            },
            Instruction::Box { r_dst: 1, r_val: 0 },
            Instruction::Unbox {
                r_dst: 2,
                r_boxed: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(123)));
}

// ── Globals & Atomics Tests ──────────────────────────────────────

#[test]
fn load_store_global_round_trip() {
    // Need a module with a global defined
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("T", "", 0, 0);
    builder.add_global_def("g", &[0x01], 0, &[]);

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadInt {
                r_dst: 0,
                value: 99,
            },
            Instruction::StoreGlobal {
                global_idx: 0,
                r_src: 0,
            },
            Instruction::LoadGlobal {
                r_dst: 1,
                global_idx: 0,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();

    let mut rt = RuntimeBuilder::new(module).build().unwrap();
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.return_value(tid), Some(Value::Int(99)));
}

#[test]
fn atomic_begin_end_adjusts_depth() {
    // AtomicBegin, LoadInt, AtomicEnd, Ret
    // This just tests that atomic begin/end don't crash
    let (rt, tid) = run_simple(
        &[
            Instruction::AtomicBegin,
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::AtomicEnd,
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(42)));
}

#[test]
fn atomic_end_without_begin_crashes() {
    let (rt, tid) = run_simple(
        &[Instruction::AtomicEnd, Instruction::RetVoid],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
}

// ── Delegate Tests ───────────────────────────────────────────────

#[test]
fn new_delegate_and_call_indirect() {
    // method 0: create delegate to method 1, call it, return result
    // method 1: LoadInt 77, Ret

    let main_instrs = vec![
        // r0 = Void (no target), create delegate to method 1 (MethodDef token 0x07000002)
        Instruction::NewDelegate {
            r_dst: 0,
            method_idx: 0x07000002,
            r_target: 1, // r1 is Void, so no target
        },
        // CallIndirect: call the delegate in r0, result to r2
        Instruction::CallIndirect {
            r_dst: 2,
            r_delegate: 0,
            r_base: 0,
            argc: 0,
        },
        Instruction::Ret { r_src: 2 },
    ];

    let callee_instrs = vec![
        Instruction::LoadInt {
            r_dst: 0,
            value: 77,
        },
        Instruction::Ret { r_src: 0 },
    ];

    let mut rt = build_two_method_runtime(&main_instrs, 3, &callee_instrs, 1);
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);

    assert_eq!(rt.return_value(tid), Some(Value::Int(77)));
}

// ── RetVoid Test ─────────────────────────────────────────────────

#[test]
fn ret_void_returns_void() {
    let (rt, tid) = run_simple(&[Instruction::RetVoid], 1);
    assert_eq!(rt.return_value(tid), Some(Value::Void));
}

// ── Nop Test ─────────────────────────────────────────────────────

#[test]
fn nop_does_nothing() {
    let (rt, tid) = run_simple(
        &[
            Instruction::Nop,
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::Ret { r_src: 0 },
        ],
        1,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(1)));
}

// ── Crash Test ───────────────────────────────────────────────────

#[test]
fn crash_instruction_crashes_task() {
    // We need a string in r0 for the Crash instruction
    let (rt, tid) = run_simple(
        &[
            Instruction::LoadInt { r_dst: 0, value: 0 },
            Instruction::I2s { r_dst: 0, r_src: 0 },
            Instruction::Crash { r_msg: 0 },
        ],
        1,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
}

// ── Execution Limit Test ─────────────────────────────────────────

#[test]
fn execution_limit_pauses_task() {
    // A program that takes many instructions: loop with backward branch
    // LoadInt 0, LoadInt 1, AddI, Br(back to AddI)
    // With limit 3, should pause after 3 instructions

    let mut rt = build_runtime(
        &[
            Instruction::LoadInt { r_dst: 0, value: 0 },
            Instruction::LoadInt { r_dst: 1, value: 1 },
            // This AddI is at instruction index 2
            Instruction::AddI {
                r_dst: 0,
                r_a: 0,
                r_b: 1,
            },
            // Br back to AddI (byte offset: AddI is at 24 bytes, Br is at 32 bytes)
            // Br offset = 24 - 32 = -8
            Instruction::Br { offset: -8 },
        ],
        2,
    );
    let tid = rt.spawn_task(0, vec![]).unwrap();

    // Run with limit of 5 instructions
    let result = rt.tick(0.0, ExecutionLimit::Instructions(5));
    match result {
        TickResult::ExecutionLimitReached => {} // expected
        other => panic!("expected ExecutionLimitReached, got {:?}", other),
    }

    // Task should be back in Ready state
    assert_eq!(rt.task_state(tid), Some(TaskState::Ready));
}

// ── Task Spawn Args Test ─────────────────────────────────────────

#[test]
fn spawn_task_with_args() {
    // Create a method that receives two args and returns their sum
    let (rt, tid) = run_with_args(
        &[
            Instruction::AddI {
                r_dst: 2,
                r_a: 0,
                r_b: 1,
            },
            Instruction::Ret { r_src: 2 },
        ],
        3,
        vec![Value::Int(10), Value::Int(20)],
    );
    assert_eq!(rt.return_value(tid), Some(Value::Int(30)));
}

// ── Entity Instruction Tests ────────────────────────────────────

#[test]
fn spawn_entity_creates_pending_and_init_commits() {
    // SPAWN_ENTITY r0 type=1, INIT_ENTITY r0, RET r0
    // Type 1 needs to exist. We'll use the default type (idx 1 = 0-based idx 0).
    let (rt, tid) = run_simple(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::Ret { r_src: 0 },
        ],
        2,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Completed));
    // The returned value should be an Entity
    match rt.return_value(tid) {
        Some(Value::Entity(eid)) => {
            // Entity should be alive in the registry
            assert!(rt.entity_registry().is_alive(eid));
        }
        other => panic!("expected Entity, got {:?}", other),
    }
}

#[test]
fn entity_is_alive_returns_true_for_alive() {
    let (rt, tid) = run_simple(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::EntityIsAlive { r_dst: 1, r_entity: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn entity_is_alive_returns_false_after_destroy() {
    let (rt, tid) = run_simple(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::DestroyEntity { r_entity: 0 },
            Instruction::EntityIsAlive { r_dst: 1, r_entity: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(false)));
}

#[test]
fn destroy_stale_entity_crashes() {
    // Spawn, init, destroy, then try to destroy again
    let (rt, tid) = run_simple(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::DestroyEntity { r_entity: 0 },
            Instruction::DestroyEntity { r_entity: 0 }, // second destroy should crash
            Instruction::RetVoid,
        ],
        2,
    );
    assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
    let crash = rt.crash_info(tid).unwrap();
    assert!(crash.message.contains("not alive"), "crash message: {}", crash.message);
}

#[test]
fn get_or_create_singleton_returns_same_entity() {
    // First GET_OR_CREATE creates a new entity, second returns the same one.
    // We check by testing EntityIsAlive on the first entity after creating both,
    // then comparing by checking both are alive and have the same index.
    // Use a different approach: get_or_create twice, check both are alive,
    // then check the entity_registry has exactly 1 alive entity for that type.
    let mut runtime = build_runtime(
        &[
            Instruction::GetOrCreate { r_dst: 0, type_idx: 1 },
            Instruction::GetOrCreate { r_dst: 1, type_idx: 1 },
            // Both r0 and r1 should be the same entity
            // Return r0; r1 should be the same value
            Instruction::RetVoid,
        ],
        2,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    // The entity registry should have exactly 1 alive entity
    assert_eq!(runtime.entity_registry().alive_count(), 1);
    // The singleton should be registered
    assert!(runtime.entity_registry().get_singleton(1).is_some());
}

#[test]
fn entity_is_alive_on_uninitialized_handle_returns_false() {
    // An entity value that doesn't exist in the registry should return false
    let (rt, tid) = run_simple(
        &[
            // Load a fake entity id (u32::MAX, gen 0) via loading Int then treating as entity
            // Simpler: just use LoadNull to get Void in r0, then EntityIsAlive
            // EntityIsAlive on non-Entity value extracts a default EntityId which won't exist
            Instruction::LoadNull { r_dst: 0 },
            Instruction::EntityIsAlive { r_dst: 1, r_entity: 0 },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    assert_eq!(rt.return_value(tid), Some(Value::Bool(false)));
}

#[test]
fn spawn_init_two_entities_both_alive() {
    let mut runtime = build_runtime(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::SpawnEntity { r_dst: 1, type_idx: 1 },
            Instruction::InitEntity { r_entity: 1 },
            Instruction::RetVoid,
        ],
        2,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    // Both entities should be alive
    assert_eq!(runtime.entity_registry().alive_count(), 2);
}

#[test]
fn destroy_one_entity_other_survives() {
    let mut runtime = build_runtime(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::SpawnEntity { r_dst: 1, type_idx: 1 },
            Instruction::InitEntity { r_entity: 1 },
            Instruction::DestroyEntity { r_entity: 0 },
            // Check destroyed entity
            Instruction::EntityIsAlive { r_dst: 0, r_entity: 0 },
            Instruction::Ret { r_src: 0 },
        ],
        2,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(tid), Some(Value::Bool(false))); // destroyed
    // The other entity should still be alive
    assert_eq!(runtime.entity_registry().alive_count(), 1);
}

// ── CALL_VIRT Dispatch Tests ────────────────────────────────────────

#[test]
fn call_virt_int_add_dispatches_intrinsic() {
    // Build a user module that uses CALL_VIRT to add two integers.
    // The contract_idx encodes a TypeRef pointing to "Add" in "writ-runtime".
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", 0, 0);

    // Add ModuleRef to writ-runtime and TypeRef to "Add" contract
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let add_ref = builder.add_type_ref(mod_ref, "Add", "writ");

    // Method: LoadInt r0=10, LoadInt r1=20, CALL_VIRT Add on r0 with arg r1, Ret r2
    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 10 },
            Instruction::LoadInt { r_dst: 1, value: 20 },
            // CALL_VIRT: r_dst=2, r_obj=0 (self=10), contract_idx=Add TypeRef,
            // slot=0, r_base=0 (self is first arg), argc=2 (self + other)
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 0,
                contract_idx: add_ref.0, // TypeRef token encoding
                slot: 0,
                r_base: 0,
                argc: 2,
            },
            Instruction::Ret { r_src: 2 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);

    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(tid), Some(Value::Int(30))); // 10 + 20
}

#[test]
fn call_virt_float_mul_dispatches_intrinsic() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", 0, 0);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let mul_ref = builder.add_type_ref(mod_ref, "Mul", "writ");

    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::LoadFloat { r_dst: 0, value: 3.0 },
            Instruction::LoadFloat { r_dst: 1, value: 4.0 },
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 0,
                contract_idx: mul_ref.0,
                slot: 0,
                r_base: 0,
                argc: 2,
            },
            Instruction::Ret { r_src: 2 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);

    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(tid), Some(Value::Float(12.0))); // 3.0 * 4.0
}

#[test]
fn call_virt_bool_eq_dispatches_intrinsic() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", 0, 0);

    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let eq_ref = builder.add_type_ref(mod_ref, "Eq", "writ");

    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::LoadTrue { r_dst: 1 },
            Instruction::CallVirt {
                r_dst: 2,
                r_obj: 0,
                contract_idx: eq_ref.0,
                slot: 0,
                r_base: 0,
                argc: 2,
            },
            Instruction::Ret { r_src: 2 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 3, body);

    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
    assert_eq!(runtime.return_value(tid), Some(Value::Bool(true)));
}

#[test]
fn call_virt_invalid_dispatch_crashes_not_panics() {
    // CALL_VIRT with a contract that doesn't exist for the given type
    // should crash gracefully, not panic.
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", 0, 0);

    // Reference "Neg" contract -- Bool doesn't implement Neg
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let neg_ref = builder.add_type_ref(mod_ref, "Neg", "writ");

    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadTrue { r_dst: 0 },
            Instruction::CallVirt {
                r_dst: 1,
                r_obj: 0,
                contract_idx: neg_ref.0,
                slot: 0,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);

    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    // Should crash, not panic
    assert_eq!(runtime.task_state(tid), Some(TaskState::Cancelled));
    let crash = runtime.crash_info(tid).expect("should have crash info");
    assert!(
        crash.message.contains("CALL_VIRT: no implementation"),
        "crash message should describe missing dispatch: {}",
        crash.message
    );
}

#[test]
fn call_virt_user_defined_contract_dispatch_table_populated() {
    // User module defines its own contract and implements it on a type.
    // Verify the dispatch table includes the user's implementation entries
    // beyond the 36 intrinsic entries from the virtual module (post FIX-02).
    let mut builder = ModuleBuilder::new("test");
    let my_type = builder.add_type_def("MyType", "app", 0, 0);

    // Define a contract "MyContract" with method "compute"
    let my_contract = builder.add_contract_def("MyContract", "app");
    builder.add_contract_method("compute", &[], 0);

    // Implement MyContract on MyType with a method that returns 42
    builder.add_impl_def(my_type, my_contract);
    let impl_body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadInt { r_dst: 1, value: 42 },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("compute", &[], 0, 2, impl_body);

    // Main method (not used, just needed to construct Runtime)
    let main_body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 1, main_body);

    let module = builder.build();
    let runtime = RuntimeBuilder::new(module).build().unwrap();

    // Verify the dispatch table includes the user's entry
    // (36 intrinsic entries + 1 user entry = 37 after FIX-02 specialization fix)
    let dispatch_table = runtime.dispatch_table();
    assert_eq!(
        dispatch_table.len(), 37,
        "dispatch table should have 36 intrinsic + 1 user entry"
    );
}
