//! Label allocator with fixup-pass for branch target resolution.
//!
//! Symbolic labels are created before the target instruction is emitted.
//! After all instructions are emitted, apply_fixups() patches all branch
//! offset fields with the correct relative byte offsets.
//!
//! Branch offset semantics (per spec):
//!   offset = target_byte_pos - branch_instruction_start_byte_pos
//!
//! The offset field is located at bytes [4..8) of the branch instruction
//! (after opcode u16 + r_cond/pad u16).

use rustc_hash::FxHashMap;

/// A symbolic label referencing a byte position in the instruction stream.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

/// Manages symbolic labels and resolves them to byte offsets via a fixup pass.
pub struct LabelAllocator {
    next: u32,
    /// Resolved labels: label id -> byte position in the code buffer.
    resolved: FxHashMap<u32, usize>,
    /// Pending fixups: (branch_instruction_start_byte_pos, label).
    fixups: Vec<(usize, Label)>,
}

impl LabelAllocator {
    /// Create a new label allocator.
    pub fn new() -> Self {
        Self {
            next: 0,
            resolved: FxHashMap::default(),
            fixups: Vec::new(),
        }
    }

    /// Allocate a new unique label.
    pub fn new_label(&mut self) -> Label {
        let id = self.next;
        self.next += 1;
        Label(id)
    }

    /// Mark a label as resolved to a specific byte position in the code stream.
    pub fn mark(&mut self, label: Label, byte_pos: usize) {
        self.resolved.insert(label.0, byte_pos);
    }

    /// Record that the branch instruction starting at `instr_byte_pos` needs
    /// its offset field patched to point to `label`.
    pub fn add_fixup(&mut self, instr_byte_pos: usize, label: Label) {
        self.fixups.push((instr_byte_pos, label));
    }

    /// Resolve a label to its instruction-index position, if it has been marked.
    ///
    /// Returns `Some(pos)` where pos is the instruction index passed to `mark()`,
    /// or `None` if the label has not been marked yet.
    pub fn resolve(&self, label: Label) -> Option<usize> {
        self.resolved.get(&label.0).copied()
    }

    /// Iterate resolved labels: yields (label_id, instruction_index) pairs.
    pub fn resolved_iter(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.resolved.iter().map(|(&k, &v)| (k, v))
    }

    /// Iterate pending fixups: yields (branch_instruction_index, label) pairs.
    pub fn fixups_iter(&self) -> &[(usize, Label)] {
        &self.fixups
    }

    /// Apply all recorded fixups to the code buffer.
    ///
    /// For each fixup at `branch_start`, computes:
    ///   offset = resolved[label] - branch_start
    /// and writes it as an i32 little-endian at `code[branch_start + 4]`.
    ///
    /// The offset field is at bytes 4..8 of the branch instruction because:
    ///   byte 0..2: opcode (u16)
    ///   byte 2..4: r_cond or pad (u16)
    ///   byte 4..8: offset (i32)
    pub fn apply_fixups(&self, code: &mut Vec<u8>) {
        for &(branch_start, label) in &self.fixups {
            let target = self.resolved[&label.0];
            let offset = (target as i64 - branch_start as i64) as i32;
            let patch_pos = branch_start + 4;
            let bytes = offset.to_le_bytes();
            code[patch_pos] = bytes[0];
            code[patch_pos + 1] = bytes[1];
            code[patch_pos + 2] = bytes[2];
            code[patch_pos + 3] = bytes[3];
        }
    }
}

impl Default for LabelAllocator {
    fn default() -> Self {
        Self::new()
    }
}
