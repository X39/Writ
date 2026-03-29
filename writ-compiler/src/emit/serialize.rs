//! Binary serialization: converts compiler ModuleBuilder + EmittedBodies
//! to a writ_module::Module, then calls Module::to_bytes() for the final binary.
//!
//! This is the final stage of the IL codegen pipeline (Phase 25, Plan 04).
//! It translates the compiler's internal metadata representation into the
//! spec-compliant writ_module format and serializes it to bytes.

use writ_module::module::{DebugLocal, MethodBody, Module, SourceSpan};
use writ_module::token::MetadataToken as WmToken;
// Intentional wildcard: tables module exports 23 row-struct types — serialization
// consumes all table types during binary module encoding.
use writ_module::tables::*;

use writ_diagnostics::FileId;

use crate::check::ty::TyInterner;

use super::body::EmittedBody;
use super::body::labels::{Label, LabelAllocator};
use super::module_builder::ModuleBuilder;

/// Translate compiler ModuleBuilder + EmittedBodies to a writ_module::Module.
///
/// Field-by-field mapping from compiler internal row types to writ_module row types.
/// The two are structurally identical (both are spec-compliant row layouts).
///
/// Takes `builder` by `&mut` so that register type blobs can be interned into
/// the builder's blob heap during body translation.
///
/// `sources` provides per-file source text (parallel to `bodies` by FileId) for
/// computing real 1-based line/column numbers in SourceSpan entries (PREP-01).
pub fn translate(
    builder: &mut ModuleBuilder,
    bodies: &[EmittedBody],
    interner: &TyInterner,
    emit_debug_info: bool,
    sources: &[(FileId, &str)],
) -> Module {
    let mut module = Module::new();

    // ── Heaps ─────────────────────────────────────────────────────────────────
    // String heap is stable at this point (all strings interned during collection).
    // Blob heap is finalized after body translation (register type blobs are added below).
    module.string_heap = builder.string_heap.data().to_vec();

    // ── Table 0: ModuleDef ────────────────────────────────────────────────────
    if let Some(mdef) = &builder.module_def {
        module.module_defs.push(ModuleDefRow {
            name: mdef.name,
            version: mdef.version,
            flags: mdef.flags,
        });
        module.header.module_name = mdef.name;
        module.header.module_version = mdef.version;
    }

    // ── Table 1: ModuleRef ────────────────────────────────────────────────────
    for mref in &builder.module_refs {
        module.module_refs.push(ModuleRefRow {
            name: mref.name,
            min_version: mref.min_version,
        });
    }

    // ── Table 2: TypeDef ──────────────────────────────────────────────────────
    for td in builder.finalized_type_defs() {
        module.type_defs.push(TypeDefRow {
            name: td.name,
            namespace: td.namespace,
            kind: td.kind,
            flags: td.flags,
            field_list: td.field_list,
            method_list: td.method_list,
        });
    }

    // ── Table 3: TypeRef ──────────────────────────────────────────────────────
    for tr in builder.finalized_type_refs() {
        module.type_refs.push(TypeRefRow {
            scope: WmToken(tr.scope.0),
            name: tr.name,
            namespace: tr.namespace,
        });
    }

    // ── Table 4: TypeSpec ─────────────────────────────────────────────────────
    for ts in builder.finalized_type_specs() {
        module.type_specs.push(TypeSpecRow {
            signature: ts.signature,
        });
    }

    // ── Table 5: FieldDef ─────────────────────────────────────────────────────
    for fd in builder.finalized_field_defs() {
        module.field_defs.push(FieldDefRow {
            name: fd.name,
            type_sig: fd.type_sig,
            flags: fd.flags,
        });
    }

    // ── Table 6: FieldRef ─────────────────────────────────────────────────────
    for fr in builder.finalized_field_refs() {
        module.field_refs.push(FieldRefRow {
            parent: WmToken(fr.parent.0),
            name: fr.name,
            type_sig: fr.type_sig,
        });
    }

    // ── Table 7: MethodDef ────────────────────────────────────────────────────
    // We add placeholders; body_offset/body_size/reg_count filled in after body serialization.
    let mut method_def_body_indices: Vec<Option<usize>> = Vec::new();

    // Collect orphaned body indices (bodies with method_def_id == None, i.e. lambda bodies).
    // These are matched to orphaned MethodDefs (def_id == None) in discovery order.
    let orphaned_body_indices: Vec<usize> = bodies
        .iter()
        .enumerate()
        .filter(|(_, b)| b.method_def_id.is_none())
        .map(|(i, _)| i)
        .collect();
    let mut orphan_cursor = 0usize;

    // Track which body indices have been consumed to handle multiple methods
    // sharing the same DefId (all impl methods share the impl block's DefId).
    let mut consumed_body_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for (def_id, md) in builder.finalized_method_def_entries() {
        // Find the body for this method (by DefId for named methods,
        // by position order for lambda methods with def_id == None).
        // When multiple bodies share a DefId (impl methods), skip already-consumed indices.
        let body_idx = if let Some(did) = def_id {
            bodies.iter().enumerate()
                .position(|(i, b)| b.method_def_id == Some(did) && !consumed_body_indices.contains(&i))
        } else {
            // Lambda MethodDef: match to the next orphaned body in order.
            let idx = orphaned_body_indices.get(orphan_cursor).copied();
            if idx.is_some() {
                orphan_cursor += 1;
            }
            idx
        };
        if let Some(idx) = body_idx {
            consumed_body_indices.insert(idx);
        }
        method_def_body_indices.push(body_idx);

        module.method_defs.push(MethodDefRow {
            name: md.name,
            signature: md.signature,
            flags: md.flags,
            body_offset: 0, // filled after body serialization
            body_size: 0,
            reg_count: body_idx.map(|i| bodies[i].reg_count).unwrap_or(0),
            param_count: md.param_count,
        });
    }

    // ── Table 8: MethodRef ────────────────────────────────────────────────────
    for mr in builder.finalized_method_refs() {
        module.method_refs.push(MethodRefRow {
            parent: WmToken(mr.parent.0),
            name: mr.name,
            signature: mr.signature,
        });
    }

    // ── Table 9: ParamDef ─────────────────────────────────────────────────────
    for pd in builder.finalized_param_defs() {
        module.param_defs.push(ParamDefRow {
            name: pd.name,
            type_sig: pd.type_sig,
            sequence: pd.sequence,
        });
    }

    // ── Table 10: ContractDef ─────────────────────────────────────────────────
    for cd in builder.finalized_contract_defs() {
        module.contract_defs.push(ContractDefRow {
            name: cd.name,
            namespace: cd.namespace,
            method_list: cd.method_list,
            generic_param_list: cd.generic_param_list,
        });
    }

    // ── Table 11: ContractMethod ──────────────────────────────────────────────
    for cm in builder.finalized_contract_methods() {
        module.contract_methods.push(ContractMethodRow {
            name: cm.name,
            signature: cm.signature,
            slot: cm.slot,
        });
    }

    // ── Table 12: ImplDef ─────────────────────────────────────────────────────
    // Note: compiler's ImplDefRow has `contract_token`; writ-module has `contract`
    for id in builder.finalized_impl_defs() {
        module.impl_defs.push(ImplDefRow {
            type_token: WmToken(id.type_token.0),
            contract: WmToken(id.contract_token.0),
            method_list: id.method_list,
        });
    }

    // ── Table 13: GenericParam ────────────────────────────────────────────────
    for gp in builder.finalized_generic_params() {
        module.generic_params.push(GenericParamRow {
            owner: WmToken(gp.owner.0),
            owner_kind: gp.owner_kind,
            ordinal: gp.ordinal,
            name: gp.name,
        });
    }

    // ── Table 14: GenericConstraint ───────────────────────────────────────────
    // Note: compiler's GenericConstraintRow has `param_row`; writ-module has `param`
    for gc in builder.finalized_generic_constraints() {
        module.generic_constraints.push(GenericConstraintRow {
            param: gc.param_row,
            constraint: WmToken(gc.constraint.0),
        });
    }

    // ── Table 15: GlobalDef ───────────────────────────────────────────────────
    for gd in &builder.global_defs {
        module.global_defs.push(GlobalDefRow {
            name: gd.name,
            type_sig: gd.type_sig,
            flags: gd.flags,
            init_value: gd.init_value,
        });
    }

    // ── Table 16: ExternDef ───────────────────────────────────────────────────
    for ed in &builder.extern_defs {
        module.extern_defs.push(ExternDefRow {
            name: ed.name,
            signature: ed.signature,
            import_name: ed.import_name,
            flags: ed.flags,
        });
    }

    // ── Table 17: ComponentSlot ───────────────────────────────────────────────
    for cs in &builder.component_slots {
        module.component_slots.push(ComponentSlotRow {
            owner_entity: WmToken(cs.owner_entity.0),
            component_type: WmToken(cs.component_type.0),
        });
    }

    // ── Table 18: LocaleDef ───────────────────────────────────────────────────
    for ld in &builder.locale_defs {
        module.locale_defs.push(LocaleDefRow {
            dlg_method: WmToken(ld.dlg_method.0),
            locale: ld.locale,
            loc_method: WmToken(ld.loc_method.0),
        });
    }

    // ── Table 19: ExportDef ───────────────────────────────────────────────────
    for ed in &builder.export_defs {
        module.export_defs.push(ExportDefRow {
            name: ed.name,
            item_kind: ed.item_kind,
            item: WmToken(ed.item.0),
        });
    }

    // ── Table 20: AttributeDef ────────────────────────────────────────────────
    for ad in &builder.attribute_defs {
        module.attribute_defs.push(AttributeDefRow {
            owner: WmToken(ad.owner.0),
            owner_kind: ad.owner_kind,
            name: ad.name,
            value: ad.value,
        });
    }

    // ── Method bodies ──────────────────────────────────────────────────────────
    // Serialize each EmittedBody that has a matching MethodDef.
    // We add bodies in MethodDef order.
    //
    // Register type blobs: snapshot the def_token_map so we can pass an immutable
    // closure to encode_type while still mutating builder.blob_heap. This avoids a
    // split-borrow conflict on &mut ModuleBuilder.
    let def_token_map_snapshot = builder.def_token_map.clone();

    for (mdef_idx, body_idx_opt) in method_def_body_indices.iter().enumerate() {
        if let Some(body_idx) = body_idx_opt {
            let body = &bodies[*body_idx];
            let code = encode_instructions(&body.instructions, &body.label_allocator);
            let total_code_size = code.len() as u32;

            // Debug info
            let instr_byte_starts = compute_instr_byte_starts(&body.instructions);

            // Find source text for this body's file (for PREP-01 line/col computation).
            // If no source is available, fall back to line_starts = [0] which gives
            // line=1, col=offset+1 — acceptable for test-only paths without source text.
            let line_starts: Vec<u32> = if let Some(file_id) = body.method_def_id
                .and(None::<FileId>) // bodies don't carry FileId directly
                .or_else(|| sources.first().map(|(fid, _)| *fid))
            {
                sources
                    .iter()
                    .find(|(fid, _)| *fid == file_id)
                    .map(|(_, src)| build_line_starts(src))
                    .unwrap_or_else(|| vec![0u32])
            } else if !sources.is_empty() {
                // Use the first (and usually only) source file's line map
                build_line_starts(sources[0].1)
            } else {
                vec![0u32]
            };

            let mut debug_locals = if emit_debug_info {
                build_debug_locals(
                    body.reg_count,
                    &body.debug_locals,
                    total_code_size,
                    &mut builder.string_heap,
                    &instr_byte_starts,
                )
            } else {
                Vec::new()
            };
            let source_spans = build_source_spans(&body.source_spans, &instr_byte_starts, &line_starts);

            // Register type table: encode each register's Ty into a blob heap offset.
            //
            // The token_for_def closure borrows only def_token_map_snapshot (not builder),
            // so builder.blob_heap can be mutated for intern() without borrow conflicts.
            let token_for_def = |def_id: crate::resolve::def_map::DefId|
                -> crate::emit::metadata::MetadataToken
            {
                def_token_map_snapshot
                    .get(&def_id)
                    .copied()
                    .unwrap_or(crate::emit::metadata::MetadataToken::NULL)
            };

            // Clamp or pad reg_types to exactly reg_count entries.
            // In correct output reg_types.len() == reg_count; the pad is defensive only.
            let reg_types: Vec<crate::check::ty::Ty> =
                if body.reg_types.len() >= body.reg_count as usize {
                    body.reg_types[..body.reg_count as usize].to_vec()
                } else {
                    let mut types = body.reg_types.clone();
                    // Void is pre-interned at index 4 by TyInterner::new()
                    types.resize(body.reg_count as usize, crate::check::ty::Ty(4));
                    types
                };

            let register_types: Vec<u32> = reg_types
                .iter()
                .map(|ty| {
                    // Skip encoding for Error/Infer types — these indicate registers that
                    // survived body emission despite partial type inference (e.g. due to
                    // type errors in surrounding expressions). Use blob offset 0 (empty)
                    // to avoid triggering the debug_assert in encode_type.
                    use crate::check::ty::TyKind;
                    match interner.kind(*ty) {
                        TyKind::Error | TyKind::Infer(_) => 0u32,
                        _ => {
                            let bytes = crate::emit::type_sig::encode_type(
                                *ty,
                                interner,
                                &token_for_def,
                                &mut builder.blob_heap,
                            );
                            builder.blob_heap.intern(&bytes)
                        }
                    }
                })
                .collect();

            // Back-fill type_ref in debug_locals from the register_types blob offsets (PREP-05).
            // Each DebugLocal.register is an index into register_types.
            for dl in &mut debug_locals {
                if (dl.register as usize) < register_types.len() {
                    dl.type_ref = register_types[dl.register as usize];
                }
            }

            // Update the MethodDef row's reg_count (already set above)
            // body_offset and body_size are set by the writ-module writer from the body index
            module.method_bodies.push(MethodBody {
                register_types,
                code,
                debug_locals,
                source_spans,
            });

            // Set body_size so the writer knows there is a body
            if mdef_idx < module.method_defs.len() {
                let code_size = module.method_bodies.last().unwrap().code.len() as u32;
                module.method_defs[mdef_idx].body_size = code_size;
            }
        }
    }

    // ── Blob heap (finalized after register type encoding) ────────────────────
    // The blob heap may have grown during register type encoding above, so we
    // copy the final state now (after all body processing is complete).
    module.blob_heap = builder.blob_heap.data().to_vec();

    // ── String heap (finalized after debug local name interning) ──────────────
    // The string heap may have grown during build_debug_locals (which interns
    // variable names via string_heap.intern()). We copy the final state now,
    // overwriting the snapshot taken at the top of translate() which pre-dates
    // body processing. Without this update, DebugLocal.name offsets would point
    // past the end of module.string_heap and read_string would fail (BUG-15 fix).
    module.string_heap = builder.string_heap.data().to_vec();

    // Format version 5: array opcode overhaul (Phase 120 — ArrayResize/Copy/NewArraySized/NewArrayFilled)
    module.header.format_version = 5;
    module.header.flags = if emit_debug_info { 1 } else { 0 };

    module
}

/// Serialize a complete module to bytes.
pub fn serialize(
    builder: &mut ModuleBuilder,
    bodies: &[EmittedBody],
    interner: &TyInterner,
    emit_debug_info: bool,
    sources: &[(FileId, &str)],
) -> Result<Vec<u8>, String> {
    let module = translate(builder, bodies, interner, emit_debug_info, sources);
    module.to_bytes().map_err(|e| format!("{:?}", e))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Encode a list of instructions to bytes, applying branch offset fixups.
///
/// Uses a 4-pass approach:
/// 1. Compute the byte start position for each instruction index (for offset translation).
/// 2. Encode all instructions to a flat byte buffer (branch offsets start as 0).
/// 3. Translate instruction-index-keyed label positions/fixups to byte positions,
///    build a byte-keyed LabelAllocator, and apply fixups to patch branch offsets.
/// 4. Convert SWITCH instruction-index offsets to byte-position offsets.
///    (SWITCH bypasses the fixup pipeline due to variable-length offset arrays.)
fn encode_instructions(
    instructions: &[writ_module::instruction::Instruction],
    labels: &LabelAllocator,
) -> Vec<u8> {
    use writ_module::instruction::Instruction;

    // Pass 1: compute byte start position for each instruction index
    let mut instr_byte_starts: Vec<usize> = Vec::with_capacity(instructions.len() + 1);
    let mut pos = 0usize;
    for instr in instructions {
        instr_byte_starts.push(pos);
        let mut tmp = Vec::new();
        let _ = instr.encode(&mut tmp);
        pos += tmp.len();
    }
    instr_byte_starts.push(pos); // sentinel: byte position just past last instruction

    // Pass 2: encode all instructions to bytes
    let mut code = Vec::new();
    for instr in instructions {
        let _ = instr.encode(&mut code); // encode errors are non-fatal in Phase 25
    }

    // Pass 3: build a byte-position-keyed LabelAllocator and apply fixups
    let mut byte_labels = LabelAllocator::new();
    for (label_id, instr_idx) in labels.resolved_iter() {
        let byte_pos = instr_byte_starts.get(instr_idx).copied().unwrap_or(code.len());
        byte_labels.mark(Label(label_id), byte_pos);
    }
    for &(branch_instr_idx, label) in labels.fixups_iter() {
        let byte_pos = instr_byte_starts.get(branch_instr_idx).copied().unwrap_or(0);
        byte_labels.add_fixup(byte_pos, label);
    }
    byte_labels.apply_fixups(&mut code);

    // Pass 4: convert instruction-index-relative fields to byte-position values.
    //
    // Two instruction types store instruction indices that the runtime expects as byte offsets:
    //
    // a) SWITCH: emit_enum_match stores offsets as (target_instr_idx - switch_instr_idx).
    //    The runtime's decode_and_reindex expects (target_byte_start - switch_byte_start).
    //    Br/BrTrue/BrFalse go through the fixup pipeline (Pass 3) which handles this,
    //    but SWITCH bypasses it because it has variable-length offset arrays.
    //
    // b) DeferPush: emit_defer stores method_idx as a raw instruction index (handler start).
    //    The runtime's decode_and_reindex expects the handler's byte offset from method start.
    for (instr_idx, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::Switch { offsets, .. } => {
                let switch_byte_start = instr_byte_starts[instr_idx];
                // SWITCH binary layout: opcode(2) + r_tag(2) + count(2) + offsets(4 each)
                let offsets_patch_start = switch_byte_start + 6;
                for (slot_idx, &instr_offset) in offsets.iter().enumerate() {
                    // instr_offset = target_instr_idx - switch_instr_idx (instruction distance)
                    let target_instr_idx = (instr_idx as i64 + instr_offset as i64) as usize;
                    let target_byte_start = instr_byte_starts
                        .get(target_instr_idx)
                        .copied()
                        .unwrap_or(code.len());
                    let byte_offset = (target_byte_start as i64 - switch_byte_start as i64) as i32;
                    let patch_pos = offsets_patch_start + slot_idx * 4;
                    code[patch_pos..patch_pos + 4].copy_from_slice(&byte_offset.to_le_bytes());
                }
            }
            Instruction::DeferPush { method_idx: handler_instr_idx, .. } => {
                // DeferPush binary layout: opcode(2) + r_dst(2) + method_idx(4)
                // method_idx stores the instruction index of the handler body start.
                // The runtime expects the byte offset of the handler from method start.
                let defer_push_byte_start = instr_byte_starts[instr_idx];
                let handler_byte_start = instr_byte_starts
                    .get(*handler_instr_idx as usize)
                    .copied()
                    .unwrap_or(code.len());
                let patch_pos = defer_push_byte_start + 4;
                code[patch_pos..patch_pos + 4]
                    .copy_from_slice(&(handler_byte_start as u32).to_le_bytes());
            }
            _ => {}
        }
    }

    code
}

/// Compute the byte start offset for each instruction index.
///
/// Returns a Vec where result[i] = byte offset of instruction i in the encoded stream.
/// An extra sentinel entry at the end holds the total byte size.
fn compute_instr_byte_starts(instructions: &[writ_module::instruction::Instruction]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(instructions.len() + 1);
    let mut pos = 0usize;
    for instr in instructions {
        starts.push(pos);
        let mut tmp = Vec::new();
        let _ = instr.encode(&mut tmp);
        pos += tmp.len();
    }
    starts.push(pos); // sentinel: position just past the last instruction
    starts
}

/// Build DebugLocal entries from the body's recorded debug info.
///
/// Variable names are interned into the string heap so DebugLocal.name carries the
/// correct heap offset rather than the hardcoded 0 placeholder (BUG-13 fix).
///
/// `instr_byte_starts` is used to convert instruction-index PCs to byte-offset PCs
/// (BUG-15 fix), matching the format expected by the DAP server's collect_frame_variables.
fn build_debug_locals(
    reg_count: u16,
    debug_locals: &[(u16, String, u32, u32)],
    total_code_size: u32,
    string_heap: &mut super::heaps::StringHeap,
    instr_byte_starts: &[usize],
) -> Vec<DebugLocal> {
    // Build register -> (name, start_pc, end_pc) from the recorded debug locals.
    // If a register appears multiple times (shouldn't in practice), keep the first entry.
    let mut reg_info: rustc_hash::FxHashMap<u16, (&str, u32, u32)> = rustc_hash::FxHashMap::default();
    for (reg, name, start_pc, end_pc) in debug_locals {
        reg_info.entry(*reg).or_insert((name.as_str(), *start_pc, *end_pc));
    }

    (0..reg_count)
        .map(|r| {
            let (name_str, start_pc_instr, end_pc_instr) = reg_info
                .get(&r)
                .copied()
                .unwrap_or(("", 0, u32::MAX));

            // Convert instruction-index start_pc to byte-offset PC.
            let start_pc = instr_byte_starts
                .get(start_pc_instr as usize)
                .copied()
                .unwrap_or(0) as u32;

            // Convert instruction-index end_pc to byte-offset PC.
            // The u32::MAX sentinel means "end of method"; clamp to total_code_size.
            let end_pc = if end_pc_instr == u32::MAX {
                total_code_size
            } else {
                instr_byte_starts
                    .get(end_pc_instr as usize)
                    .copied()
                    .unwrap_or(total_code_size as usize) as u32
            };

            // Intern the name into the string heap. Unnamed registers (name="") get offset 0.
            let name_offset = if name_str.is_empty() {
                0u32
            } else {
                string_heap.intern(name_str)
            };

            DebugLocal {
                register: r,
                name: name_offset,
                type_ref: 0, // populated after register_types are encoded (see translate())
                start_pc,
                end_pc,
            }
        })
        .collect()
}

/// Build a sorted table of byte offsets for the start of each line.
///
/// `line_starts[0]` is always 0 (start of line 1).
/// `line_starts[n]` is the byte offset immediately after the nth newline (start of line n+1).
pub(crate) fn build_line_starts(src: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, b) in src.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            starts.push(i as u32 + 1);
        }
    }
    starts
}

/// Convert a byte offset into a source file to a 1-based (line, column) pair.
///
/// Both line and column are 1-based, matching LSP and DAP conventions.
pub(crate) fn byte_offset_to_line_col(offset: u32, line_starts: &[u32]) -> (u32, u16) {
    let line_idx = line_starts.partition_point(|&s| s <= offset).saturating_sub(1);
    let line = line_idx as u32 + 1;
    let col = offset.saturating_sub(line_starts[line_idx]) + 1;
    (line, col.min(u16::MAX as u32) as u16)
}

/// Build SourceSpan entries from the body's recorded span info.
///
/// Uses `instr_byte_starts` to convert instruction indices to byte offsets (BUG-14 fix).
/// Uses `line_starts` to convert source byte offsets to 1-based line/column numbers (PREP-01).
fn build_source_spans(
    source_spans: &[(u32, chumsky::span::SimpleSpan)],
    instr_byte_starts: &[usize],
    line_starts: &[u32],
) -> Vec<SourceSpan> {
    source_spans
        .iter()
        .map(|(instr_idx, span)| {
            let byte_offset = instr_byte_starts
                .get(*instr_idx as usize)
                .copied()
                .unwrap_or(0) as u32;
            let (line, column) = byte_offset_to_line_col(span.start as u32, line_starts);
            SourceSpan {
                pc: byte_offset,
                line,
                column,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_switch_byte_offsets() {
        use writ_module::instruction::Instruction;

        // Build a minimal instruction sequence that exercises SWITCH:
        // [0] LoadInt r0, 1          (some setup)
        // [1] Switch r_tag=0, offsets=[2, 3]  (2 variants: arm at instr 3, arm at instr 4)
        // [2] Nop                    (filler)
        // [3] Nop                    (variant 0 arm target)
        // [4] Nop                    (variant 1 arm target)
        let instructions = vec![
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::Switch { r_tag: 0, offsets: vec![2, 3] },
            Instruction::Nop,
            Instruction::Nop,
            Instruction::Nop,
        ];

        let labels = super::super::body::labels::LabelAllocator::new();
        let code = encode_instructions(&instructions, &labels);

        // Compute expected byte starts to verify the patch:
        let byte_starts = compute_instr_byte_starts(&instructions);
        // LoadInt: opcode(2) + r_dst(2) + value(8) = 12 bytes
        // Switch:  opcode(2) + r_tag(2) + count(2) + 2*4 = 14 bytes
        // Nop:     opcode(2) = 2 bytes each

        let switch_byte_start = byte_starts[1];
        // SWITCH layout: opcode(2) + r_tag(2) + count(2) = 6 bytes before offsets
        let offset0_pos = switch_byte_start + 6;
        let offset1_pos = switch_byte_start + 10;

        // Expected byte offsets:
        // Variant 0 target is instr 3 (switch is instr 1, so instr_offset=2)
        //   byte_offset = byte_starts[3] - byte_starts[1]
        let expected_off0 = (byte_starts[3] as i64 - switch_byte_start as i64) as i32;
        // Variant 1 target is instr 4 (switch is instr 1, so instr_offset=3)
        //   byte_offset = byte_starts[4] - byte_starts[1]
        let expected_off1 = (byte_starts[4] as i64 - switch_byte_start as i64) as i32;

        let actual_off0 = i32::from_le_bytes(code[offset0_pos..offset0_pos + 4].try_into().unwrap());
        let actual_off1 = i32::from_le_bytes(code[offset1_pos..offset1_pos + 4].try_into().unwrap());

        assert_eq!(actual_off0, expected_off0,
            "SWITCH offset[0] should be byte-relative, not instruction-index-relative");
        assert_eq!(actual_off1, expected_off1,
            "SWITCH offset[1] should be byte-relative, not instruction-index-relative");

        // Sanity: byte offsets should be > instruction-index offsets
        // because each instruction is at least 2 bytes
        assert!(expected_off0 > 2, "byte offset should be larger than instruction-index offset (2)");
        assert!(expected_off1 > 3, "byte offset should be larger than instruction-index offset (3)");
    }

    #[test]
    fn test_build_line_starts_empty() {
        assert_eq!(build_line_starts(""), vec![0]);
    }

    #[test]
    fn test_build_line_starts_single_line() {
        assert_eq!(build_line_starts("abc"), vec![0]);
    }

    #[test]
    fn test_build_line_starts_multiline() {
        // "abc\ndef\nghi": newlines at byte 3 and 7
        assert_eq!(build_line_starts("abc\ndef\nghi"), vec![0, 4, 8]);
    }

    #[test]
    fn test_byte_offset_to_line_col_start() {
        let starts = vec![0u32, 4, 8];
        assert_eq!(byte_offset_to_line_col(0, &starts), (1, 1));
    }

    #[test]
    fn test_byte_offset_to_line_col_end_of_first_line() {
        let starts = vec![0u32, 4, 8];
        // byte 3 = 'c' in "abc\n" = line 1, col 4
        assert_eq!(byte_offset_to_line_col(3, &starts), (1, 4));
    }

    #[test]
    fn test_byte_offset_to_line_col_start_of_second_line() {
        let starts = vec![0u32, 4, 8];
        // byte 4 = start of "def" = line 2, col 1
        assert_eq!(byte_offset_to_line_col(4, &starts), (2, 1));
    }

    #[test]
    fn test_byte_offset_to_line_col_middle_of_second_line() {
        let starts = vec![0u32, 4, 8];
        // byte 6 = 'f' in "def" (offset 2 from line start 4) = line 2, col 3
        assert_eq!(byte_offset_to_line_col(6, &starts), (2, 3));
    }

    #[test]
    fn test_build_source_spans_real_line_col() {
        use chumsky::span::SimpleSpan;
        // "abc\ndef": newlines at byte 3 -> line_starts = [0, 4]
        let line_starts = build_line_starts("abc\ndef");
        assert_eq!(line_starts, vec![0, 4]);

        // Instruction 0 at byte_start 0, span starting at byte 4 (start of "def")
        let source_spans = vec![(0u32, SimpleSpan { start: 4usize, end: 6usize, context: () })];
        let instr_byte_starts = vec![0usize, 10]; // instr 0 starts at byte 0

        let spans = build_source_spans(&source_spans, &instr_byte_starts, &line_starts);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].pc, 0);
        assert_eq!(spans[0].line, 2, "byte 4 should be line 2");
        assert_eq!(spans[0].column, 1, "byte 4 should be col 1");
    }
}
