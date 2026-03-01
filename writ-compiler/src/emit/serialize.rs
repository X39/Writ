//! Binary serialization: converts compiler ModuleBuilder + EmittedBodies
//! to a writ_module::Module, then calls Module::to_bytes() for the final binary.
//!
//! This is the final stage of the IL codegen pipeline (Phase 25, Plan 04).
//! It translates the compiler's internal metadata representation into the
//! spec-compliant writ_module format and serializes it to bytes.

use writ_module::module::{DebugLocal, MethodBody, Module, SourceSpan};
use writ_module::token::MetadataToken as WmToken;
use writ_module::tables::*;

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
pub fn translate(
    builder: &mut ModuleBuilder,
    bodies: &[EmittedBody],
    interner: &TyInterner,
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

    for (def_id, md) in builder.finalized_method_def_entries() {
        // Find the body for this method (by DefId)
        let body_idx = if let Some(did) = def_id {
            bodies.iter().position(|b| b.method_def_id == Some(did))
        } else {
            None
        };
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
            let debug_locals = build_debug_locals(
                body.reg_count,
                &body.debug_locals,
                total_code_size,
                &mut builder.string_heap,
            );
            let source_spans = build_source_spans(&body.source_spans, &instr_byte_starts);

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

    // Set header format_version
    module.header.format_version = 2;
    module.header.flags = 1; // debug flag on

    module
}

/// Serialize a complete module to bytes.
pub fn serialize(
    builder: &mut ModuleBuilder,
    bodies: &[EmittedBody],
    interner: &TyInterner,
) -> Result<Vec<u8>, String> {
    let module = translate(builder, bodies, interner);
    module.to_bytes().map_err(|e| format!("{:?}", e))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Encode a list of instructions to bytes, applying branch offset fixups.
///
/// Uses a 3-pass approach:
/// 1. Compute the byte start position for each instruction index (for offset translation).
/// 2. Encode all instructions to a flat byte buffer (branch offsets start as 0).
/// 3. Translate instruction-index-keyed label positions/fixups to byte positions,
///    build a byte-keyed LabelAllocator, and apply fixups to patch branch offsets.
fn encode_instructions(
    instructions: &[writ_module::instruction::Instruction],
    labels: &LabelAllocator,
) -> Vec<u8> {
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
fn build_debug_locals(
    reg_count: u16,
    debug_locals: &[(u16, String, u32, u32)],
    total_code_size: u32,
    string_heap: &mut super::heaps::StringHeap,
) -> Vec<DebugLocal> {
    // Build register -> (name, start_pc, end_pc) from the recorded debug locals.
    // If a register appears multiple times (shouldn't in practice), keep the first entry.
    let mut reg_info: rustc_hash::FxHashMap<u16, (&str, u32, u32)> = rustc_hash::FxHashMap::default();
    for (reg, name, start_pc, end_pc) in debug_locals {
        reg_info.entry(*reg).or_insert((name.as_str(), *start_pc, *end_pc));
    }

    (0..reg_count)
        .map(|r| {
            let (name_str, start_pc, end_pc) = reg_info
                .get(&r)
                .copied()
                .unwrap_or(("", 0, total_code_size));

            // Intern the name into the string heap. Unnamed registers (name="") get offset 0.
            let name_offset = if name_str.is_empty() {
                0u32
            } else {
                string_heap.intern(name_str)
            };

            DebugLocal {
                register: r,
                name: name_offset,
                start_pc,
                end_pc: if end_pc == u32::MAX { total_code_size } else { end_pc },
            }
        })
        .collect()
}

/// Build SourceSpan entries from the body's recorded span info.
///
/// Uses `instr_byte_starts` to convert instruction indices to byte offsets (BUG-14 fix).
/// The source_spans vec is empty for now (body emitter doesn't yet push spans), so
/// this returns an empty Vec in practice. The fix ensures correctness when spans are added.
fn build_source_spans(
    source_spans: &[(u32, chumsky::span::SimpleSpan)],
    instr_byte_starts: &[usize],
) -> Vec<SourceSpan> {
    source_spans
        .iter()
        .map(|(instr_idx, _span)| {
            let byte_offset = instr_byte_starts
                .get(*instr_idx as usize)
                .copied()
                .unwrap_or(0) as u32;
            SourceSpan {
                pc: byte_offset,
                line: 0,   // MVP: line=0 (unknown) — line tracking requires threading spans through emit_expr
                column: 0,
            }
        })
        .collect()
}
