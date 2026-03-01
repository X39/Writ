/// ASM-02 tests: forward and backward label resolution.
use writ_module::Instruction;

fn assemble_and_decode_method(src: &str) -> Vec<Instruction> {
    let module = writ_assembler::assemble(src).expect("should assemble");
    assert!(!module.method_bodies.is_empty(), "expected at least one method body");
    let code = &module.method_bodies[0].code;
    let mut cursor = std::io::Cursor::new(code.as_slice());
    let mut instructions = Vec::new();
    while (cursor.position() as usize) < code.len() {
        match Instruction::decode(&mut cursor) {
            Ok(instr) => instructions.push(instr),
            Err(e) => panic!("decode error at offset {}: {:?}", cursor.position(), e),
        }
    }
    instructions
}

#[test]
fn forward_label_resolution() {
    // BR .end should jump forward over the NOP to RET_VOID
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        BR .end
        NOP
        .end:
        RET_VOID
    }
}
"#;
    let instrs = assemble_and_decode_method(src);
    assert_eq!(instrs.len(), 3, "BR + NOP + RET_VOID");

    // BR is 8 bytes (opcode u16 + pad u16 + offset i32)
    // NOP is 2 bytes
    // BR offset should be: target(.end) - (BR_offset + BR_size)
    //   .end is at byte 10 (8 + 2), BR is at byte 0, BR size is 8
    //   offset = 10 - (0 + 8) = 2
    if let Instruction::Br { offset } = &instrs[0] {
        assert_eq!(*offset, 2, "forward branch offset: skip NOP (2 bytes)");
    } else {
        panic!("expected BR instruction");
    }
}

#[test]
fn backward_label_resolution() {
    // .top: NOP, then BR .top should produce a negative offset
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .top:
        NOP
        BR .top
        RET_VOID
    }
}
"#;
    let instrs = assemble_and_decode_method(src);
    assert_eq!(instrs.len(), 3, "NOP + BR + RET_VOID");

    // NOP is at byte 0, size 2
    // BR is at byte 2, size 8
    // .top is at byte 0
    // offset = 0 - (2 + 8) = -10
    if let Instruction::Br { offset } = &instrs[1] {
        assert_eq!(*offset, -10, "backward branch offset: jump back over BR(8) + NOP(2)");
    } else {
        panic!("expected BR instruction");
    }
}

#[test]
fn forward_and_backward_labels() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 bool
        .top:
        NOP
        BR_FALSE r0, .done
        BR .top
        .done:
        RET_VOID
    }
}
"#;
    let instrs = assemble_and_decode_method(src);
    assert_eq!(instrs.len(), 4, "NOP + BR_FALSE + BR + RET_VOID");

    // NOP at byte 0, size 2
    // BR_FALSE at byte 2, size 8
    // BR at byte 10, size 8
    // .done at byte 18
    // .top at byte 0

    // BR_FALSE offset: .done(18) - (2 + 8) = 8
    if let Instruction::BrFalse { r_cond, offset } = &instrs[1] {
        assert_eq!(*r_cond, 0);
        assert_eq!(*offset, 8, "BR_FALSE forward to .done");
    } else {
        panic!("expected BR_FALSE");
    }

    // BR offset: .top(0) - (10 + 8) = -18
    if let Instruction::Br { offset } = &instrs[2] {
        assert_eq!(*offset, -18, "BR backward to .top");
    } else {
        panic!("expected BR");
    }
}

#[test]
fn multiple_labels_in_method() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 bool
        BR .middle
        .start:
        NOP
        .middle:
        NOP
        BR_FALSE r0, .start
        .end:
        RET_VOID
    }
}
"#;
    let instrs = assemble_and_decode_method(src);
    assert_eq!(instrs.len(), 5, "BR + NOP + NOP + BR_FALSE + RET_VOID");

    // BR at byte 0 (size 8): jumps to .middle at byte 10
    // NOP at byte 8 (size 2) (.start at byte 8)
    // NOP at byte 10 (size 2) (.middle at byte 10)
    // BR_FALSE at byte 12 (size 8): jumps to .start at byte 8
    // RET_VOID at byte 20 (.end at byte 20)

    if let Instruction::Br { offset } = &instrs[0] {
        // .middle(10) - (0 + 8) = 2
        assert_eq!(*offset, 2, "BR to .middle");
    } else {
        panic!("expected BR");
    }

    if let Instruction::BrFalse { offset, .. } = &instrs[3] {
        // .start(8) - (12 + 8) = -12
        assert_eq!(*offset, -12, "BR_FALSE to .start");
    } else {
        panic!("expected BR_FALSE");
    }
}
