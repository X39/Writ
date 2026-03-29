use rustc_hash::FxHashMap;
use std::io::Cursor;

use writ_module::{Instruction, Module};

use crate::domain::ResolvedRefs;
use crate::error::RuntimeError;

/// A module with decoded instruction bodies and branch targets reindexed
/// from byte offsets to instruction indices.
pub struct LoadedModule {
    pub module: Module,
    /// For each method body (parallel to module.method_bodies),
    /// the decoded `Vec<Instruction>` with branch targets rewritten to instruction indices.
    pub decoded_bodies: Vec<Vec<Instruction>>,
    /// For each method body (parallel to decoded_bodies),
    /// maps instruction index -> byte offset in the original code stream.
    /// Used to translate VM instruction indices back to byte offsets for
    /// SourceSpan/breakpoint lookups.
    pub byte_offsets: Vec<Vec<u32>>,
    /// Cross-module reference resolution results. Populated by Domain::resolve_refs().
    pub resolved_refs: ResolvedRefs,
}

impl LoadedModule {
    /// Decode all method bodies from raw code bytes into `Vec<Instruction>`,
    /// converting branch byte offsets to instruction indices.
    pub fn from_module(module: Module) -> Result<Self, RuntimeError> {
        let mut decoded_bodies = Vec::with_capacity(module.method_bodies.len());
        let mut byte_offsets_all = Vec::with_capacity(module.method_bodies.len());
        for (method_idx, body) in module.method_bodies.iter().enumerate() {
            let (instructions, byte_offsets) = decode_and_reindex(&body.code, method_idx)?;
            decoded_bodies.push(instructions);
            byte_offsets_all.push(byte_offsets);
        }
        Ok(Self {
            module,
            decoded_bodies,
            byte_offsets: byte_offsets_all,
            resolved_refs: ResolvedRefs::new(),
        })
    }
}

/// Decode raw instruction bytes and reindex branch targets.
///
/// Pass 1: Decode all instructions, recording the byte offset of each instruction start.
/// Pass 2: Rewrite branch targets (Br, BrTrue, BrFalse, Switch, DeferPush)
///          from byte offsets to instruction indices.
///
/// Returns `(instructions, byte_offsets)` where `byte_offsets[i]` is the byte offset
/// in `raw_code` where instruction `i` starts. Used by `LoadedModule` to map instruction
/// indices back to byte offsets for SourceSpan and breakpoint lookups.
fn decode_and_reindex(raw_code: &[u8], method_idx: usize) -> Result<(Vec<Instruction>, Vec<u32>), RuntimeError> {
    if raw_code.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Pass 1: Decode all instructions and record byte offsets
    let mut instructions = Vec::new();
    let mut byte_offsets: Vec<u32> = Vec::new();
    let mut cursor = Cursor::new(raw_code);

    loop {
        let pos = cursor.position() as u32;
        if pos as usize >= raw_code.len() {
            break;
        }
        byte_offsets.push(pos);
        let instr = Instruction::decode(&mut cursor).map_err(|e| {
            RuntimeError::DecodeError {
                method_idx,
                offset: pos as usize,
                detail: format!("{}", e),
            }
        })?;
        instructions.push(instr);
    }

    // Build offset map: byte_offset -> instruction_index
    // Also include the byte position after the last instruction (for forward jumps to end)
    let mut offset_map: FxHashMap<u32, usize> = FxHashMap::default();
    for (idx, &byte_off) in byte_offsets.iter().enumerate() {
        offset_map.insert(byte_off, idx);
    }
    // Add the end position (one past the last instruction)
    let end_pos = cursor.position() as u32;
    offset_map.insert(end_pos, instructions.len());

    // Pass 2: Rewrite branch targets
    for (instr_idx, instr) in instructions.iter_mut().enumerate() {
        let current_byte_offset = byte_offsets[instr_idx];
        match instr {
            Instruction::Br { offset } => {
                let target_byte = (current_byte_offset as i64 + *offset as i64) as u32;
                let target_idx = offset_map.get(&target_byte).ok_or_else(|| {
                    RuntimeError::DecodeError {
                        method_idx,
                        offset: current_byte_offset as usize,
                        detail: format!(
                            "Br target byte offset {} not found in offset map",
                            target_byte
                        ),
                    }
                })?;
                *offset = *target_idx as i32;
            }
            Instruction::BrTrue { offset, .. } => {
                let target_byte = (current_byte_offset as i64 + *offset as i64) as u32;
                let target_idx = offset_map.get(&target_byte).ok_or_else(|| {
                    RuntimeError::DecodeError {
                        method_idx,
                        offset: current_byte_offset as usize,
                        detail: format!(
                            "BrTrue target byte offset {} not found in offset map",
                            target_byte
                        ),
                    }
                })?;
                *offset = *target_idx as i32;
            }
            Instruction::BrFalse { offset, .. } => {
                let target_byte = (current_byte_offset as i64 + *offset as i64) as u32;
                let target_idx = offset_map.get(&target_byte).ok_or_else(|| {
                    RuntimeError::DecodeError {
                        method_idx,
                        offset: current_byte_offset as usize,
                        detail: format!(
                            "BrFalse target byte offset {} not found in offset map",
                            target_byte
                        ),
                    }
                })?;
                *offset = *target_idx as i32;
            }
            Instruction::Switch { offsets, .. } => {
                for off in offsets.iter_mut() {
                    let target_byte = (current_byte_offset as i64 + *off as i64) as u32;
                    let target_idx = offset_map.get(&target_byte).ok_or_else(|| {
                        RuntimeError::DecodeError {
                            method_idx,
                            offset: current_byte_offset as usize,
                            detail: format!(
                                "Switch target byte offset {} not found in offset map",
                                target_byte
                            ),
                        }
                    })?;
                    *off = *target_idx as i32;
                }
            }
            Instruction::DeferPush { method_idx: handler_offset, .. } => {
                // DeferPush's method_idx field is actually a byte offset from method start
                // (per spec §3.11). Convert to instruction index.
                let byte_off = *handler_offset;
                let target_idx = offset_map.get(&byte_off).ok_or_else(|| {
                    RuntimeError::DecodeError {
                        method_idx,
                        offset: current_byte_offset as usize,
                        detail: format!(
                            "DeferPush handler byte offset {} not found in offset map",
                            byte_off
                        ),
                    }
                })?;
                *handler_offset = *target_idx as u32;
            }
            _ => {}
        }
    }

    Ok((instructions, byte_offsets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::Instruction;

    fn encode_instructions(instrs: &[Instruction]) -> Vec<u8> {
        let mut code = Vec::new();
        for instr in instrs {
            instr.encode(&mut code).unwrap();
        }
        code
    }

    #[test]
    fn empty_body() {
        let (result, offsets) = decode_and_reindex(&[], 0).unwrap();
        assert!(result.is_empty());
        assert!(offsets.is_empty());
    }

    #[test]
    fn linear_body_no_branches() {
        let instrs = vec![
            Instruction::LoadInt { r_dst: 0, value: 42 },
            Instruction::Ret { r_src: 0 },
        ];
        let code = encode_instructions(&instrs);
        let (decoded, offsets) = decode_and_reindex(&code, 0).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], Instruction::LoadInt { r_dst: 0, value: 42 });
        assert_eq!(decoded[1], Instruction::Ret { r_src: 0 });
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0], 0, "first instruction starts at byte 0");
    }

    #[test]
    fn forward_branch() {
        // Layout: [LoadInt (12B)] [Br(forward to Ret)] [LoadInt (12B)] [Ret (4B)]
        // Br offset should point to instruction index 3 (the Ret)
        let load1 = Instruction::LoadInt { r_dst: 0, value: 1 };
        let load2 = Instruction::LoadInt { r_dst: 1, value: 2 };
        let ret = Instruction::Ret { r_src: 0 };

        // First, compute the byte sizes
        let mut buf = Vec::new();
        load1.encode(&mut buf).unwrap(); // 12 bytes
        // Br is 8 bytes: 2 (opcode) + 2 (pad) + 4 (offset)
        // After Br at position br_pos, the next instruction (load2) is at br_pos+8
        // load2 is at br_pos+8, is 12 bytes, so ret is at br_pos+8+12 = br_pos+20
        // Br offset is relative to the Br instruction's byte position
        // target = br_pos + offset, so offset = (br_pos + 20) - br_pos = 20
        let br_offset = 20i32; // forward past Br(8) + LoadInt(12)
        let br_instr = Instruction::Br { offset: br_offset };
        br_instr.encode(&mut buf).unwrap();
        load2.encode(&mut buf).unwrap();
        ret.encode(&mut buf).unwrap();

        let (decoded, offsets) = decode_and_reindex(&buf, 0).unwrap();
        assert_eq!(decoded.len(), 4);
        // Br should now point to instruction index 3 (the Ret)
        match &decoded[1] {
            Instruction::Br { offset } => assert_eq!(*offset, 3),
            other => panic!("expected Br, got {:?}", other),
        }
        // byte_offsets should record the start position of each instruction
        assert_eq!(offsets.len(), 4);
        assert_eq!(offsets[0], 0); // LoadInt at byte 0
    }

    #[test]
    fn backward_branch() {
        // Layout: [LoadInt(12B)] [Br(backward to LoadInt)]
        // Br at position 12, offset should target position 0 (instruction 0)
        // offset = 0 - 12 = -12
        let load = Instruction::LoadInt { r_dst: 0, value: 1 };
        let br = Instruction::Br { offset: -12 };

        let mut buf = Vec::new();
        load.encode(&mut buf).unwrap(); // position 0, 12 bytes
        br.encode(&mut buf).unwrap();   // position 12

        let (decoded, offsets) = decode_and_reindex(&buf, 0).unwrap();
        assert_eq!(decoded.len(), 2);
        match &decoded[1] {
            Instruction::Br { offset } => assert_eq!(*offset, 0), // targets instruction 0
            other => panic!("expected Br, got {:?}", other),
        }
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 12); // Br starts at byte 12
    }

    #[test]
    fn conditional_branch_forward() {
        // [LoadTrue(4B)] [BrTrue(8B) forward past LoadInt to Ret] [LoadInt(12B)] [Ret(4B)]
        let load_true = Instruction::LoadTrue { r_dst: 0 };
        let load_int = Instruction::LoadInt { r_dst: 1, value: 99 };
        let ret = Instruction::Ret { r_src: 0 };

        let mut buf = Vec::new();
        load_true.encode(&mut buf).unwrap(); // 4 bytes
        // BrTrue is 8 bytes
        // LoadInt is at 4+8=12, 12 bytes, so Ret is at 24
        // offset = 24 - 4 = 20
        let br_true = Instruction::BrTrue { r_cond: 0, offset: 20 };
        br_true.encode(&mut buf).unwrap();
        load_int.encode(&mut buf).unwrap();
        ret.encode(&mut buf).unwrap();

        let (decoded, _offsets) = decode_and_reindex(&buf, 0).unwrap();
        assert_eq!(decoded.len(), 4);
        match &decoded[1] {
            Instruction::BrTrue { r_cond, offset } => {
                assert_eq!(*r_cond, 0);
                assert_eq!(*offset, 3); // targets Ret at instruction index 3
            }
            other => panic!("expected BrTrue, got {:?}", other),
        }
    }

    #[test]
    fn defer_push_reindexing() {
        // [DeferPush(8B) pointing to handler at byte 20]
        // [LoadInt(12B)]
        // [Ret(4B)] -- position 20
        // handler position 20 should map to instruction index 2

        // Actually: DeferPush is at byte 0 (8 bytes), LoadInt at byte 8 (12 bytes), Ret at byte 20 (4 bytes)
        let defer_push = Instruction::DeferPush { r_dst: 0, method_idx: 20 }; // byte offset 20
        let load_int = Instruction::LoadInt { r_dst: 0, value: 42 };
        let ret = Instruction::Ret { r_src: 0 };

        let mut buf = Vec::new();
        defer_push.encode(&mut buf).unwrap(); // 8 bytes
        load_int.encode(&mut buf).unwrap();   // 12 bytes, at position 8
        ret.encode(&mut buf).unwrap();        // 4 bytes, at position 20

        let (decoded, offsets) = decode_and_reindex(&buf, 0).unwrap();
        assert_eq!(decoded.len(), 3);
        match &decoded[0] {
            Instruction::DeferPush { r_dst, method_idx } => {
                assert_eq!(*r_dst, 0);
                assert_eq!(*method_idx, 2); // instruction index of Ret
            }
            other => panic!("expected DeferPush, got {:?}", other),
        }
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 8);
        assert_eq!(offsets[2], 20);
    }

    #[test]
    fn multiple_bodies() {
        // Test that LoadedModule handles multiple method bodies independently
        let instrs1 = vec![
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::Ret { r_src: 0 },
        ];
        let instrs2 = vec![
            Instruction::LoadInt { r_dst: 0, value: 2 },
            Instruction::RetVoid,
        ];

        use writ_module::module::MethodBody;

        let mut module = Module::new();
        module.method_bodies.push(MethodBody {
            register_types: vec![0],
            code: encode_instructions(&instrs1),
            debug_locals: vec![],
            source_spans: vec![],
        });
        module.method_bodies.push(MethodBody {
            register_types: vec![0],
            code: encode_instructions(&instrs2),
            debug_locals: vec![],
            source_spans: vec![],
        });

        let loaded = LoadedModule::from_module(module).unwrap();
        assert_eq!(loaded.decoded_bodies.len(), 2);
        assert_eq!(loaded.decoded_bodies[0].len(), 2);
        assert_eq!(loaded.decoded_bodies[1].len(), 2);
        assert_eq!(
            loaded.decoded_bodies[0][0],
            Instruction::LoadInt { r_dst: 0, value: 1 }
        );
        assert_eq!(
            loaded.decoded_bodies[1][0],
            Instruction::LoadInt { r_dst: 0, value: 2 }
        );
        // byte_offsets should be populated for both bodies
        assert_eq!(loaded.byte_offsets.len(), 2);
        assert_eq!(loaded.byte_offsets[0].len(), 2);
        assert_eq!(loaded.byte_offsets[1].len(), 2);
        assert_eq!(loaded.byte_offsets[0][0], 0, "first instruction of body 0 starts at byte 0");
        assert_eq!(loaded.byte_offsets[1][0], 0, "first instruction of body 1 starts at byte 0");
    }
}
