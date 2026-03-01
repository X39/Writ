//! Debug info emission for IL method bodies.
//!
//! Emits DebugLocal entries for ALL registers (params, locals, temps)
//! and SourceSpan entries per statement/expression.

use writ_module::module::{DebugLocal, SourceSpan};

use super::BodyEmitter;

/// Emit DebugLocal entries for all registers in the emitter.
///
/// - Parameters (tracked in locals with param names) use their source names.
/// - Let bindings (registered in emitter.locals) use their source names.
/// - Temporaries that are not in locals get synthetic names: `$tmp_{n}`.
///
/// `string_heap_out` is used to intern the names; returns the offset for each.
pub fn emit_debug_locals(emitter: &BodyEmitter<'_>, total_code_size: u32) -> Vec<DebugLocal> {
    let reg_count = emitter.regs.reg_count() as u16;

    // Build reverse map: register -> name (from emitter.locals)
    let mut reg_to_name: rustc_hash::FxHashMap<u16, &str> = rustc_hash::FxHashMap::default();
    for (name, &reg) in &emitter.locals {
        reg_to_name.entry(reg).or_insert(name.as_str());
    }

    // Also check debug_locals for param names recorded during emission
    for &(reg, ref name, _start, _end) in &emitter.debug_locals {
        reg_to_name.entry(reg).or_insert(name.as_str());
    }

    let mut result = Vec::with_capacity(reg_count as usize);

    for r in 0..reg_count {
        // Name: from locals map if available, otherwise synthetic
        // We store the name as a string heap offset.
        // For Phase 25 (before CLI integration), we use 0 as a placeholder offset.
        // The actual string heap is in the ModuleBuilder; full wiring is in serialize.rs.
        let _name = reg_to_name.get(&r).copied().unwrap_or("$tmp");

        result.push(DebugLocal {
            register: r,
            name: 0, // placeholder; real offset set during serialization
            start_pc: 0,
            end_pc: total_code_size,
        });
    }

    result
}

/// Emit SourceSpan entries for a method body.
///
/// Each entry maps a PC (byte offset in the instruction stream) to a source location.
/// For Phase 25, we store `span.start` as the `line` field and `0` for column,
/// since line:column resolution requires the source text (available in Phase 26 CLI).
pub fn emit_source_spans(emitter: &BodyEmitter<'_>) -> Vec<SourceSpan> {
    emitter
        .source_spans
        .iter()
        .map(|(instr_idx, span)| SourceSpan {
            pc: *instr_idx,
            line: span.start as u32,
            column: 0,
        })
        .collect()
}
