//! Binary-to-text disassembler for `.writil` modules.
//!
//! Converts a `writ_module::Module` struct into human-readable `.writil` text format,
//! symmetric with `assembler::assemble_module`. The output is round-trippable: feeding
//! it back to `assemble()` produces a module with the same table structure.

use std::fmt::Write;
use std::io::Cursor;

use writ_module::heap::{read_blob, read_string};
use writ_module::tables::TypeDefKind;
use writ_module::{Instruction, Module};

/// Disassemble a binary `Module` into clean `.writil` text.
///
/// The output can be fed back to `assemble()` without errors.
pub fn disassemble(module: &Module) -> String {
    disassemble_inner(module, false)
}

/// Disassemble a binary `Module` into annotated `.writil` text with hex offset comments.
///
/// Same as `disassemble()` but adds `// +0xNNNN` comments before each instruction.
pub fn disassemble_verbose(module: &Module) -> String {
    disassemble_inner(module, true)
}

/// Internal disassembler that handles both clean and verbose modes.
fn disassemble_inner(module: &Module, verbose: bool) -> String {
    let mut out = String::new();

    // Helper: resolve string heap offset to &str
    let s = |offset: u32| -> &str { read_string(&module.string_heap, offset).unwrap_or("") };

    // Precompute cumulative ParamDef offsets for each MethodDef. MethodDef.param_count
    // includes an implicit instance receiver, while method signatures and ParamDef rows
    // describe regular source parameters only. Derive each range from the signature so
    // receiver registers cannot shift every later method's parameter names.
    let mut method_param_start: Vec<usize> = Vec::with_capacity(module.method_defs.len() + 1);
    {
        let mut running = 0usize;
        for md in &module.method_defs {
            method_param_start.push(running);
            let regular_param_count = read_blob(&module.blob_heap, md.signature)
                .ok()
                .and_then(|blob| blob.get(..2))
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
                .unwrap_or(0);
            running += regular_param_count;
        }
        method_param_start.push(running); // sentinel
    }

    // Helper: get param names for a method index from the ParamDef table.
    let get_param_names = |method_idx: usize| -> Vec<String> {
        let start = method_param_start
            .get(method_idx)
            .copied()
            .unwrap_or(0)
            .min(module.param_defs.len());
        let end = method_param_start
            .get(method_idx + 1)
            .copied()
            .unwrap_or(start)
            .min(module.param_defs.len())
            .max(start);
        module.param_defs[start..end]
            .iter()
            .map(|pd| {
                read_string(&module.string_heap, pd.name)
                    .unwrap_or("")
                    .to_string()
            })
            .collect()
    };

    // Emit module header
    let name = s(module.header.module_name);
    let ver = s(module.header.module_version);
    writeln!(out, ".module {:?} {:?} {{", name, ver).unwrap();

    // ── 1. Module refs (.extern "name" "min_version") ──
    for mr in &module.module_refs {
        writeln!(out, "    .extern {:?} {:?}", s(mr.name), s(mr.min_version)).unwrap();
    }

    // ── 2. Type defs with their fields and directly-owned methods ──
    for (ti, td) in module.type_defs.iter().enumerate() {
        let kind_str = match TypeDefKind::from_u8(td.kind) {
            Some(TypeDefKind::Struct) => "struct",
            Some(TypeDefKind::Enum) => "enum",
            Some(TypeDefKind::Entity) => "entity",
            Some(TypeDefKind::Component) => "component",
            Some(TypeDefKind::Class) => "class",
            None => unreachable!("reader validates TypeDef kinds"),
        };

        // Emit type flags as keyword(s) if recognized, otherwise as integer
        let type_flags_str = flags_to_str(td.flags);
        if type_flags_str.is_empty() {
            writeln!(out, "    .type {:?} {} {{", s(td.name), kind_str).unwrap();
        } else {
            writeln!(
                out,
                "    .type {:?} {} {} {{",
                s(td.name),
                kind_str,
                type_flags_str
            )
            .unwrap();
        }

        // Fields: field_list range [field_list-1, next_type.field_list-1)
        let field_start = td.field_list.saturating_sub(1) as usize;
        let field_end = module
            .type_defs
            .get(ti + 1)
            .map(|next| next.field_list.saturating_sub(1) as usize)
            .unwrap_or(module.field_defs.len());
        for fd in &module.field_defs[field_start..field_end] {
            let type_text = decode_type_sig(&module.blob_heap, fd.type_sig, module);
            let field_flags_str = flags_to_str(fd.flags);
            if field_flags_str.is_empty() {
                writeln!(out, "        .field {:?} {}", s(fd.name), type_text).unwrap();
            } else {
                writeln!(
                    out,
                    "        .field {:?} {} {}",
                    s(fd.name),
                    type_text,
                    field_flags_str
                )
                .unwrap();
            }
        }

        for method_idx in module.type_method_indices(ti) {
            let md = &module.method_defs[method_idx];
            let param_names = get_param_names(method_idx);
            let (params, ret) =
                decode_method_sig(&module.blob_heap, md.signature, module, &param_names);
            let method_flags_str = flags_to_str(md.flags);

            if method_flags_str.is_empty() {
                writeln!(
                    out,
                    "        .method {:?} ({}) -> {} {{",
                    s(md.name),
                    params.join(", "),
                    ret
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "        .method {:?} ({}) -> {} {} {{",
                    s(md.name),
                    params.join(", "),
                    ret,
                    method_flags_str
                )
                .unwrap();
            }

            if method_idx < module.method_bodies.len() {
                let body = &module.method_bodies[method_idx];
                let body_text = disassemble_body(body, module, verbose, "            ");
                out.push_str(&body_text);
            }

            writeln!(out, "        }}").unwrap();
        }

        writeln!(out, "    }}").unwrap();
    }

    // ── 3. Contract defs with their methods ──
    for (ci, cd) in module.contract_defs.iter().enumerate() {
        // Collect generic params for this contract
        let generic_params = collect_generic_params(module, ci as u32, 1); // owner_kind=1 for Contract

        if generic_params.is_empty() {
            writeln!(out, "    .contract {:?} {{", s(cd.name)).unwrap();
        } else {
            writeln!(
                out,
                "    .contract {:?} <{}> {{",
                s(cd.name),
                generic_params.join(", ")
            )
            .unwrap();
        }

        // Contract methods: method_list range
        let cm_start = cd.method_list.saturating_sub(1) as usize;
        let cm_end = module
            .contract_defs
            .get(ci + 1)
            .map(|next| next.method_list.saturating_sub(1) as usize)
            .unwrap_or(module.contract_methods.len());
        for cm in &module.contract_methods[cm_start..cm_end] {
            let (params, ret) = decode_method_sig(&module.blob_heap, cm.signature, module, &[]);
            writeln!(
                out,
                "        .method {:?} ({}) -> {} slot {}",
                s(cm.name),
                params.join(", "),
                ret,
                cm.slot
            )
            .unwrap();
        }

        writeln!(out, "    }}").unwrap();
    }

    // ── 4. Impl defs with their methods ──
    for (ii, id) in module.impl_defs.iter().enumerate() {
        // Resolve type name from type_token
        let type_name = resolve_type_name(module, id.type_token.0);
        // Resolve contract name from contract token
        let contract_name = resolve_contract_name(module, id.contract.0);

        writeln!(out, "    .impl {} : {} {{", type_name, contract_name).unwrap();

        for real_idx in module.impl_method_indices(ii) {
            let md = &module.method_defs[real_idx];
            let param_names = get_param_names(real_idx);
            let (params, ret) =
                decode_method_sig(&module.blob_heap, md.signature, module, &param_names);
            let method_flags_str = flags_to_str(md.flags);

            if method_flags_str.is_empty() {
                writeln!(
                    out,
                    "        .method {:?} ({}) -> {} {{",
                    s(md.name),
                    params.join(", "),
                    ret
                )
                .unwrap();
            } else {
                writeln!(
                    out,
                    "        .method {:?} ({}) -> {} {} {{",
                    s(md.name),
                    params.join(", "),
                    ret,
                    method_flags_str
                )
                .unwrap();
            }

            // Emit method body if available
            if real_idx < module.method_bodies.len() {
                let body = &module.method_bodies[real_idx];
                let body_text = disassemble_body(body, module, verbose, "            ");
                out.push_str(&body_text);
            }

            writeln!(out, "        }}").unwrap();
        }

        writeln!(out, "    }}").unwrap();
    }

    // ── 5. Global defs ──
    for gd in &module.global_defs {
        let type_text = decode_type_sig(&module.blob_heap, gd.type_sig, module);
        let global_flags_str = flags_to_str(gd.flags);
        if global_flags_str.is_empty() {
            writeln!(out, "    .global {:?} {}", s(gd.name), type_text).unwrap();
        } else {
            writeln!(
                out,
                "    .global {:?} {} {}",
                s(gd.name),
                type_text,
                global_flags_str
            )
            .unwrap();
        }
    }

    // ── 6. Extern functions ──
    // Module-level extern function declarations (different from module refs).
    for ed in &module.extern_defs {
        let (params, ret) = decode_method_sig(&module.blob_heap, ed.signature, module, &[]);
        writeln!(
            out,
            "    .extern_fn {:?} ({}) -> {} {:?}",
            s(ed.name),
            params.join(", "),
            ret,
            s(ed.import_name)
        )
        .unwrap();
    }

    // ── 7. Top-level methods (explicitly owned by the module) ──
    for mi in module.top_level_method_indices() {
        let md = &module.method_defs[mi];
        let param_names = get_param_names(mi);
        let (params, ret) =
            decode_method_sig(&module.blob_heap, md.signature, module, &param_names);
        let method_flags_str = flags_to_str(md.flags);

        if method_flags_str.is_empty() {
            writeln!(
                out,
                "    .method {:?} ({}) -> {} {{",
                s(md.name),
                params.join(", "),
                ret
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "    .method {:?} ({}) -> {} {} {{",
                s(md.name),
                params.join(", "),
                ret,
                method_flags_str
            )
            .unwrap();
        }

        // Emit method body if available
        if mi < module.method_bodies.len() {
            let body = &module.method_bodies[mi];
            let body_text = disassemble_body(body, module, verbose, "        ");
            out.push_str(&body_text);
        }

        writeln!(out, "    }}").unwrap();
    }

    // ── 8. Export defs ──
    for ed in &module.export_defs {
        let item_kind_str = match ed.item_kind {
            0 => "method",
            1 => "type",
            2 => "global",
            _ => "unknown",
        };
        writeln!(
            out,
            "    .export {:?} {} {}",
            s(ed.name),
            item_kind_str,
            ed.item.0
        )
        .unwrap();
    }

    // ── 9. Component slots ──
    for cs in &module.component_slots {
        writeln!(
            out,
            "    .component_slot {} {}",
            cs.owner_entity.0, cs.component_type.0
        )
        .unwrap();
    }

    // ── 10. Locale defs ──
    for ld in &module.locale_defs {
        writeln!(
            out,
            "    .locale {} {:?} {}",
            ld.dlg_method.0,
            s(ld.locale),
            ld.loc_method.0
        )
        .unwrap();
    }

    // ── 11. Attribute defs ──
    for ad in &module.attribute_defs {
        writeln!(
            out,
            "    .attribute {} {} {:?}",
            ad.owner.0,
            ad.owner_kind,
            s(ad.name)
        )
        .unwrap();
    }

    writeln!(out, "}}").unwrap();
    out
}

/// Collect generic parameter names for a given owner (identified by ordinal in type or contract table).
///
/// `owner_kind`: 1=Contract, 2=Method
fn collect_generic_params(module: &Module, owner_ordinal: u32, owner_kind: u8) -> Vec<String> {
    // GenericParamRow.owner is a MetadataToken; for contracts, table_id=10 (ContractDef).
    // The contract's MetadataToken = (10 << 24) | (1-based row index)
    let expected_table_id: u32 = match owner_kind {
        1 => 10, // ContractDef
        2 => 7,  // MethodDef
        _ => return Vec::new(),
    };
    let expected_token = (expected_table_id << 24) | (owner_ordinal + 1);

    let mut params: Vec<(u16, String)> = module
        .generic_params
        .iter()
        .filter(|gp| gp.owner.0 == expected_token && gp.owner_kind == owner_kind)
        .map(|gp| {
            let name = read_string(&module.string_heap, gp.name)
                .unwrap_or("T")
                .to_string();
            (gp.ordinal, name)
        })
        .collect();

    params.sort_by_key(|(ord, _)| *ord);
    params.into_iter().map(|(_, name)| name).collect()
}

/// Resolve a MetadataToken value to a type name string.
fn resolve_type_name(module: &Module, token: u32) -> String {
    let row_idx = (token & 0x00FF_FFFF) as usize;
    if row_idx == 0 {
        return "?".to_string();
    }
    let idx = row_idx - 1;
    let s = |offset: u32| -> &str { read_string(&module.string_heap, offset).unwrap_or("?") };
    if let Some(td) = module.type_defs.get(idx) {
        s(td.name).to_string()
    } else if let Some(tr) = module.type_refs.get(idx) {
        s(tr.name).to_string()
    } else {
        format!("type_{}", token)
    }
}

/// Resolve a MetadataToken value to a contract name string.
fn resolve_contract_name(module: &Module, token: u32) -> String {
    let row_idx = (token & 0x00FF_FFFF) as usize;
    if row_idx == 0 {
        return "?".to_string();
    }
    let idx = row_idx - 1;
    let s = |offset: u32| -> &str { read_string(&module.string_heap, offset).unwrap_or("?") };
    if let Some(cd) = module.contract_defs.get(idx) {
        s(cd.name).to_string()
    } else {
        format!("contract_{}", token)
    }
}

/// Decode a type signature blob at the given offset into a type name string.
pub(crate) fn decode_type_sig(blob_heap: &[u8], sig_offset: u32, module: &Module) -> String {
    let blob = read_blob(blob_heap, sig_offset).unwrap_or(&[]);
    let mut pos = 0;
    decode_type_ref(blob, &mut pos, module)
}

/// Decode a type reference from a blob byte slice starting at `*pos`.
/// Advances `*pos` past the consumed bytes.
pub(crate) fn decode_type_ref(blob: &[u8], pos: &mut usize, module: &Module) -> String {
    let Some(bytes) = blob.get(*pos..) else {
        return "?".to_string();
    };
    match writ_module::signature::decode_type_signature_prefix(bytes) {
        Ok((signature, consumed)) => {
            *pos += consumed;
            render_type_signature(&signature, module)
        }
        Err(_) => {
            *pos = blob.len();
            "?".to_string()
        }
    }
}

fn render_type_signature(
    signature: &writ_module::signature::TypeSignature,
    module: &Module,
) -> String {
    use writ_module::signature::TypeSignature;
    match signature {
        TypeSignature::Void => "void".to_string(),
        TypeSignature::Int => "int".to_string(),
        TypeSignature::Float => "float".to_string(),
        TypeSignature::Bool => "bool".to_string(),
        TypeSignature::String => "string".to_string(),
        TypeSignature::Entity => "Entity".to_string(),
        TypeSignature::Named(token) => {
            let Some(row) = token.row_index() else {
                return "?".to_string();
            };
            let idx = row as usize - 1;
            match token.table_id() {
                2 | 0 => module
                    .type_defs
                    .get(idx)
                    .and_then(|row| read_string(&module.string_heap, row.name).ok()),
                3 => module
                    .type_refs
                    .get(idx)
                    .and_then(|row| read_string(&module.string_heap, row.name).ok()),
                _ => None,
            }
            .map(str::to_string)
            .unwrap_or_else(|| format!("type_{}", token.0))
        }
        TypeSignature::Generic {
            namespace,
            name,
            args,
        } => {
            let constructor = if namespace.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", namespace, name)
            };
            let args = args
                .iter()
                .map(|arg| render_type_signature(arg, module))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", constructor, args)
        }
        TypeSignature::GenericParam(ordinal) => format!("T{}", ordinal),
        TypeSignature::Array(element) => {
            format!("array<{}>", render_type_signature(element, module))
        }
        TypeSignature::Function { params, ret } => {
            let params = params
                .iter()
                .map(|param| render_type_signature(param, module))
                .collect::<Vec<_>>()
                .join(", ");
            let ret = render_type_signature(ret, module);
            if matches!(ret.as_str(), "void") {
                format!("fn({})", params)
            } else {
                format!("fn({}) -> {}", params, ret)
            }
        }
    }
}

/// Decode a method signature blob into (param_strings, return_type).
///
/// Method sig format: u16(param_count) + param_type_blobs + return_type_blob
///
/// If `param_names` is non-empty, each parameter is rendered as "name: type".
/// If `param_names` is shorter than the param count, unnamed params fall back to type-only.
pub(crate) fn decode_method_sig(
    blob_heap: &[u8],
    sig_offset: u32,
    module: &Module,
    param_names: &[String],
) -> (Vec<String>, String) {
    let blob = read_blob(blob_heap, sig_offset).unwrap_or(&[]);
    if blob.len() < 2 {
        return (vec![], "void".to_string());
    }

    let param_count = u16::from_le_bytes([blob[0], blob[1]]) as usize;
    let mut pos = 2;

    let mut params = Vec::with_capacity(param_count);
    for i in 0..param_count {
        if pos >= blob.len() {
            break;
        }
        let type_str = decode_type_ref(blob, &mut pos, module);
        let name = param_names.get(i).map(|n| n.as_str()).unwrap_or("");
        if name.is_empty() {
            params.push(type_str);
        } else {
            params.push(format!("{}: {}", name, type_str));
        }
    }

    let ret = if pos < blob.len() {
        decode_type_ref(blob, &mut pos, module)
    } else {
        "void".to_string()
    };

    (params, ret)
}

/// Disassemble a method body into text lines with the given indentation.
fn disassemble_body(
    body: &writ_module::module::MethodBody,
    module: &Module,
    verbose: bool,
    indent: &str,
) -> String {
    let mut out = String::new();

    // ── .locals section (PREP-05) ──────────────────────────────────────────────
    // Emit named locals (skip unnamed temporaries where name == 0).
    let named_locals: Vec<_> = body.debug_locals.iter().filter(|dl| dl.name != 0).collect();
    if !named_locals.is_empty() {
        writeln!(out, "{}.locals {{", indent).unwrap();
        for dl in &named_locals {
            let name = read_string(&module.string_heap, dl.name).unwrap_or("?");
            let type_text = if dl.type_ref != 0 {
                decode_type_sig(&module.blob_heap, dl.type_ref, module)
            } else {
                "?".to_string()
            };
            writeln!(
                out,
                "{}    r{}: {} \"{}\" [{}, {})",
                indent, dl.register, type_text, name, dl.start_pc, dl.end_pc
            )
            .unwrap();
        }
        writeln!(out, "{}}}", indent).unwrap();
    }

    // Emit register declarations
    // register_types are blob heap offsets (stored as 0 by current assembler, but we try to decode)
    // If a register_type offset is 0, we emit "int" as a reasonable default (the blob at offset 0 is empty)
    // In practice the current assembler stores 0 for all register blobs.
    for (i, &rt_offset) in body.register_types.iter().enumerate() {
        let type_text = if rt_offset == 0 {
            // Empty blob at offset 0 = unknown register type, default to int for text output
            // This is a known limitation of the current assembler (doesn't intern register types)
            "int".to_string()
        } else {
            decode_type_sig(&module.blob_heap, rt_offset, module)
        };
        writeln!(out, "{}.reg r{} {}", indent, i, type_text).unwrap();
    }

    // Decode and emit instructions
    let code = &body.code;
    if code.is_empty() {
        return out;
    }

    // Build a sorted cursor into source_spans for efficient linear scan.
    // source_spans is already sorted by pc (inserted in instruction order by the compiler).
    let mut span_cursor = 0usize;

    let mut cursor = Cursor::new(code.as_slice());
    while (cursor.position() as usize) < code.len() {
        let byte_offset = cursor.position() as usize;

        match Instruction::decode(&mut cursor) {
            Ok(instr) => {
                let (mnemonic, operands) = instr_to_text(&instr);

                if verbose {
                    writeln!(out, "{}// +{:#06x}", indent, byte_offset).unwrap();
                }

                // ── Inline source location comment (PREP-01) ──────────────────
                // Advance span_cursor to find the last SourceSpan with pc <= byte_offset.
                while span_cursor + 1 < body.source_spans.len()
                    && body.source_spans[span_cursor + 1].pc <= byte_offset as u32
                {
                    span_cursor += 1;
                }
                let source_comment = if !body.source_spans.is_empty()
                    && body.source_spans[span_cursor].pc == byte_offset as u32
                    && (body.source_spans[span_cursor].line != 0
                        || body.source_spans[span_cursor].column != 0)
                {
                    let ss = &body.source_spans[span_cursor];
                    Some(format!("; line:{} col:{}", ss.line, ss.column))
                } else {
                    None
                };

                if operands.is_empty() {
                    if let Some(comment) = source_comment {
                        writeln!(out, "{}{}  {}", indent, mnemonic, comment).unwrap();
                    } else {
                        writeln!(out, "{}{}", indent, mnemonic).unwrap();
                    }
                } else if let Some(comment) = source_comment {
                    writeln!(
                        out,
                        "{}{} {}  {}",
                        indent,
                        mnemonic,
                        operands.join(", "),
                        comment
                    )
                    .unwrap();
                } else {
                    writeln!(out, "{}{} {}", indent, mnemonic, operands.join(", ")).unwrap();
                }
            }
            Err(e) => {
                writeln!(
                    out,
                    "{}// decode error at +{:#06x}: {:?}",
                    indent, byte_offset, e
                )
                .unwrap();
                break;
            }
        }
    }

    out
}

/// Convert an `Instruction` variant to its (mnemonic, operand_strings) representation.
///
/// All 94 instruction variants are covered. Mnemonics match the assembler's `map_instruction` table.
fn instr_to_text(instr: &Instruction) -> (String, Vec<String>) {
    let r = |n: u16| format!("r{}", n);
    let tok = |t: u32| format!("{}", t);

    match instr {
        // ── 0x00 Meta ──
        Instruction::Nop => ("NOP".into(), vec![]),
        Instruction::Crash { r_msg } => ("CRASH".into(), vec![r(*r_msg)]),

        // ── 0x01 Data Movement ──
        Instruction::Mov { r_dst, r_src } => ("MOV".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::LoadInt { r_dst, value } => {
            ("LOAD_INT".into(), vec![r(*r_dst), format!("{}", value)])
        }
        Instruction::LoadFloat { r_dst, value } => {
            ("LOAD_FLOAT".into(), vec![r(*r_dst), format!("{}", value)])
        }
        Instruction::LoadTrue { r_dst } => ("LOAD_TRUE".into(), vec![r(*r_dst)]),
        Instruction::LoadFalse { r_dst } => ("LOAD_FALSE".into(), vec![r(*r_dst)]),
        Instruction::LoadString { r_dst, string_idx } => (
            "LOAD_STRING".into(),
            vec![r(*r_dst), format!("{}", string_idx)],
        ),
        Instruction::LoadNull { r_dst } => ("LOAD_NULL".into(), vec![r(*r_dst)]),

        // ── 0x02 Integer Arithmetic ──
        Instruction::AddI { r_dst, r_a, r_b } => {
            ("ADD_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::SubI { r_dst, r_a, r_b } => {
            ("SUB_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::MulI { r_dst, r_a, r_b } => {
            ("MUL_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::DivI { r_dst, r_a, r_b } => {
            ("DIV_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::ModI { r_dst, r_a, r_b } => {
            ("MOD_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::NegI { r_dst, r_src } => ("NEG_I".into(), vec![r(*r_dst), r(*r_src)]),

        // ── 0x03 Float Arithmetic ──
        Instruction::AddF { r_dst, r_a, r_b } => {
            ("ADD_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::SubF { r_dst, r_a, r_b } => {
            ("SUB_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::MulF { r_dst, r_a, r_b } => {
            ("MUL_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::DivF { r_dst, r_a, r_b } => {
            ("DIV_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::ModF { r_dst, r_a, r_b } => {
            ("MOD_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::NegF { r_dst, r_src } => ("NEG_F".into(), vec![r(*r_dst), r(*r_src)]),

        // ── 0x04 Bitwise & Logical ──
        Instruction::BitAnd { r_dst, r_a, r_b } => {
            ("BIT_AND".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::BitOr { r_dst, r_a, r_b } => {
            ("BIT_OR".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::Shl { r_dst, r_a, r_b } => ("SHL".into(), vec![r(*r_dst), r(*r_a), r(*r_b)]),
        Instruction::Shr { r_dst, r_a, r_b } => ("SHR".into(), vec![r(*r_dst), r(*r_a), r(*r_b)]),
        Instruction::Not { r_dst, r_src } => ("NOT".into(), vec![r(*r_dst), r(*r_src)]),

        // ── 0x05 Comparison ──
        Instruction::CmpEqI { r_dst, r_a, r_b } => {
            ("CMP_EQ_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::CmpEqF { r_dst, r_a, r_b } => {
            ("CMP_EQ_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::CmpEqB { r_dst, r_a, r_b } => {
            ("CMP_EQ_B".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::CmpEqS { r_dst, r_a, r_b } => {
            ("CMP_EQ_S".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::CmpLtI { r_dst, r_a, r_b } => {
            ("CMP_LT_I".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::CmpLtF { r_dst, r_a, r_b } => {
            ("CMP_LT_F".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }

        // ── 0x06 Control Flow ──
        Instruction::Br { offset } => ("BR".into(), vec![format!("{}", offset)]),
        Instruction::BrTrue { r_cond, offset } => {
            ("BR_TRUE".into(), vec![r(*r_cond), format!("{}", offset)])
        }
        Instruction::BrFalse { r_cond, offset } => {
            ("BR_FALSE".into(), vec![r(*r_cond), format!("{}", offset)])
        }
        Instruction::Switch { r_tag, offsets } => {
            let mut ops = vec![r(*r_tag)];
            for o in offsets {
                ops.push(format!("{}", o));
            }
            ("SWITCH".into(), ops)
        }
        Instruction::Ret { r_src } => ("RET".into(), vec![r(*r_src)]),
        Instruction::RetVoid => ("RET_VOID".into(), vec![]),

        // ── 0x07 Calls & Delegates ──
        Instruction::Call {
            r_dst,
            method_idx,
            r_base,
            argc,
        } => (
            "CALL".into(),
            vec![r(*r_dst), tok(*method_idx), r(*r_base), format!("{}", argc)],
        ),
        Instruction::CallVirt {
            r_dst,
            r_obj,
            contract_idx,
            slot,
            r_base,
            argc,
        } => (
            "CALL_VIRT".into(),
            vec![
                r(*r_dst),
                r(*r_obj),
                tok(*contract_idx),
                format!("{}", slot),
                r(*r_base),
                format!("{}", argc),
            ],
        ),
        Instruction::CallExtern {
            r_dst,
            extern_idx,
            r_base,
            argc,
        } => (
            "CALL_EXTERN".into(),
            vec![r(*r_dst), tok(*extern_idx), r(*r_base), format!("{}", argc)],
        ),
        Instruction::NewDelegate {
            r_dst,
            method_idx,
            r_target,
        } => (
            "NEW_DELEGATE".into(),
            vec![r(*r_dst), tok(*method_idx), r(*r_target)],
        ),
        Instruction::CallIndirect {
            r_dst,
            r_delegate,
            r_base,
            argc,
        } => (
            "CALL_INDIRECT".into(),
            vec![r(*r_dst), r(*r_delegate), r(*r_base), format!("{}", argc)],
        ),
        Instruction::TailCall {
            method_idx,
            r_base,
            argc,
        } => (
            "TAIL_CALL".into(),
            vec![tok(*method_idx), r(*r_base), format!("{}", argc)],
        ),

        // ── 0x08 Object Model ──
        Instruction::New { r_dst, type_idx } => ("NEW".into(), vec![r(*r_dst), tok(*type_idx)]),
        Instruction::GetField {
            r_dst,
            r_obj,
            field_idx,
        } => (
            "GET_FIELD".into(),
            vec![r(*r_dst), r(*r_obj), format!("{}", field_idx)],
        ),
        Instruction::SetField {
            r_obj,
            field_idx,
            r_val,
        } => (
            "SET_FIELD".into(),
            vec![r(*r_obj), format!("{}", field_idx), r(*r_val)],
        ),
        Instruction::SpawnEntity { r_dst, type_idx } => {
            ("SPAWN_ENTITY".into(), vec![r(*r_dst), tok(*type_idx)])
        }
        Instruction::InitEntity { r_entity } => ("INIT_ENTITY".into(), vec![r(*r_entity)]),
        Instruction::GetComponent {
            r_dst,
            r_entity,
            comp_type_idx,
        } => (
            "GET_COMPONENT".into(),
            vec![r(*r_dst), r(*r_entity), tok(*comp_type_idx)],
        ),
        Instruction::GetOrCreate { r_dst, type_idx } => {
            ("GET_OR_CREATE".into(), vec![r(*r_dst), tok(*type_idx)])
        }
        Instruction::FindAll { r_dst, type_idx } => {
            ("FIND_ALL".into(), vec![r(*r_dst), tok(*type_idx)])
        }
        Instruction::DestroyEntity { r_entity } => ("DESTROY_ENTITY".into(), vec![r(*r_entity)]),
        Instruction::EntityIsAlive { r_dst, r_entity } => {
            ("ENTITY_IS_ALIVE".into(), vec![r(*r_dst), r(*r_entity)])
        }

        // ── 0x09 Arrays ──
        Instruction::NewArray { r_dst, elem_type } => {
            ("NEW_ARRAY".into(), vec![r(*r_dst), tok(*elem_type)])
        }
        Instruction::ArrayInit {
            r_dst,
            elem_type,
            count,
            r_base,
        } => (
            "ARRAY_INIT".into(),
            vec![r(*r_dst), tok(*elem_type), format!("{}", count), r(*r_base)],
        ),
        Instruction::ArrayLoad {
            r_dst,
            r_arr,
            r_idx,
        } => ("ARRAY_LOAD".into(), vec![r(*r_dst), r(*r_arr), r(*r_idx)]),
        Instruction::ArrayStore {
            r_arr,
            r_idx,
            r_val,
        } => ("ARRAY_STORE".into(), vec![r(*r_arr), r(*r_idx), r(*r_val)]),
        Instruction::ArrayLen { r_dst, r_arr } => ("ARRAY_LEN".into(), vec![r(*r_dst), r(*r_arr)]),
        Instruction::ArrayResize { r_arr, r_new_len } => {
            ("ARRAY_RESIZE".into(), vec![r(*r_arr), r(*r_new_len)])
        }
        Instruction::ArrayCopy {
            r_dst_arr,
            r_dst_idx,
            r_src_arr,
            r_src_idx,
            r_len,
        } => (
            "ARRAY_COPY".into(),
            vec![
                r(*r_dst_arr),
                r(*r_dst_idx),
                r(*r_src_arr),
                r(*r_src_idx),
                r(*r_len),
            ],
        ),
        Instruction::ArraySlice {
            r_dst,
            r_arr,
            r_start,
            r_end,
        } => (
            "ARRAY_SLICE".into(),
            vec![r(*r_dst), r(*r_arr), r(*r_start), r(*r_end)],
        ),
        Instruction::NewArraySized {
            r_dst,
            elem_type,
            r_len,
        } => (
            "NEW_ARRAY_SIZED".into(),
            vec![r(*r_dst), tok(*elem_type), r(*r_len)],
        ),
        Instruction::NewArrayFilled {
            r_dst,
            elem_type,
            r_len,
            r_fill,
        } => (
            "NEW_ARRAY_FILLED".into(),
            vec![r(*r_dst), tok(*elem_type), r(*r_len), r(*r_fill)],
        ),

        // ── 0x0A Type Operations — Option ──
        Instruction::WrapSome { r_dst, r_val } => ("WRAP_SOME".into(), vec![r(*r_dst), r(*r_val)]),
        Instruction::Unwrap { r_dst, r_opt } => ("UNWRAP".into(), vec![r(*r_dst), r(*r_opt)]),
        Instruction::IsSome { r_dst, r_opt } => ("IS_SOME".into(), vec![r(*r_dst), r(*r_opt)]),
        Instruction::IsNone { r_dst, r_opt } => ("IS_NONE".into(), vec![r(*r_dst), r(*r_opt)]),

        // ── 0x0A Type Operations — Result ──
        Instruction::WrapOk { r_dst, r_val } => ("WRAP_OK".into(), vec![r(*r_dst), r(*r_val)]),
        Instruction::WrapErr { r_dst, r_err } => ("WRAP_ERR".into(), vec![r(*r_dst), r(*r_err)]),
        Instruction::UnwrapOk { r_dst, r_result } => {
            ("UNWRAP_OK".into(), vec![r(*r_dst), r(*r_result)])
        }
        Instruction::IsOk { r_dst, r_result } => ("IS_OK".into(), vec![r(*r_dst), r(*r_result)]),
        Instruction::IsErr { r_dst, r_result } => ("IS_ERR".into(), vec![r(*r_dst), r(*r_result)]),
        Instruction::ExtractErr { r_dst, r_result } => {
            ("EXTRACT_ERR".into(), vec![r(*r_dst), r(*r_result)])
        }

        // ── 0x0A Type Operations — Enum ──
        Instruction::NewEnum {
            r_dst,
            type_idx,
            tag,
            field_count,
            r_base,
        } => (
            "NEW_ENUM".into(),
            vec![
                r(*r_dst),
                tok(*type_idx),
                format!("{}", tag),
                format!("{}", field_count),
                r(*r_base),
            ],
        ),
        Instruction::GetTag { r_dst, r_enum } => ("GET_TAG".into(), vec![r(*r_dst), r(*r_enum)]),
        Instruction::ExtractField {
            r_dst,
            r_enum,
            field_idx,
        } => (
            "EXTRACT_FIELD".into(),
            vec![r(*r_dst), r(*r_enum), format!("{}", field_idx)],
        ),

        // ── 0x0A Type Operations — Reflection ──
        Instruction::TypeOf { r_dst, type_idx } => {
            ("TYPEOF".into(), vec![r(*r_dst), tok(*type_idx)])
        }

        // ── 0x0B Concurrency ──
        Instruction::SpawnTask {
            r_dst,
            method_idx,
            r_base,
            argc,
        } => (
            "SPAWN_TASK".into(),
            vec![r(*r_dst), tok(*method_idx), r(*r_base), format!("{}", argc)],
        ),
        Instruction::Join { r_dst, r_task } => ("JOIN".into(), vec![r(*r_dst), r(*r_task)]),
        Instruction::Cancel { r_task } => ("CANCEL".into(), vec![r(*r_task)]),
        Instruction::DeferPush { r_dst, method_idx } => {
            ("DEFER_PUSH".into(), vec![r(*r_dst), tok(*method_idx)])
        }
        Instruction::DeferPop => ("DEFER_POP".into(), vec![]),
        Instruction::DeferEnd => ("DEFER_END".into(), vec![]),

        // ── 0x0C Globals & Atomics ──
        Instruction::LoadGlobal { r_dst, global_idx } => {
            ("LOAD_GLOBAL".into(), vec![r(*r_dst), tok(*global_idx)])
        }
        Instruction::StoreGlobal { global_idx, r_src } => {
            ("STORE_GLOBAL".into(), vec![tok(*global_idx), r(*r_src)])
        }
        Instruction::AtomicBegin => ("ATOMIC_BEGIN".into(), vec![]),
        Instruction::AtomicEnd => ("ATOMIC_END".into(), vec![]),

        // ── 0x0D Conversion ──
        Instruction::I2f { r_dst, r_src } => ("I2F".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::F2i { r_dst, r_src } => ("F2I".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::I2s { r_dst, r_src } => ("I2S".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::F2s { r_dst, r_src } => ("F2S".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::B2s { r_dst, r_src } => ("B2S".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::Convert {
            r_dst,
            r_src,
            target_type,
        } => (
            "CONVERT".into(),
            vec![r(*r_dst), r(*r_src), tok(*target_type)],
        ),
        Instruction::S2i { r_dst, r_src } => ("S2I".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::S2f { r_dst, r_src } => ("S2F".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::S2b { r_dst, r_src } => ("S2B".into(), vec![r(*r_dst), r(*r_src)]),

        // ── 0x0E Strings ──
        Instruction::StrConcat { r_dst, r_a, r_b } => {
            ("STR_CONCAT".into(), vec![r(*r_dst), r(*r_a), r(*r_b)])
        }
        Instruction::StrBuild {
            r_dst,
            count,
            r_base,
        } => (
            "STR_BUILD".into(),
            vec![r(*r_dst), format!("{}", count), r(*r_base)],
        ),
        Instruction::StrLen { r_dst, r_str } => ("STR_LEN".into(), vec![r(*r_dst), r(*r_str)]),
        Instruction::StrTrim { r_dst, r_src } => ("STR_TRIM".into(), vec![r(*r_dst), r(*r_src)]),
        Instruction::StrToUpper { r_dst, r_src } => {
            ("STR_TO_UPPER".into(), vec![r(*r_dst), r(*r_src)])
        }
        Instruction::StrToLower { r_dst, r_src } => {
            ("STR_TO_LOWER".into(), vec![r(*r_dst), r(*r_src)])
        }
        Instruction::StrStartsWith {
            r_dst,
            r_str,
            r_prefix,
        } => (
            "STR_STARTS_WITH".into(),
            vec![r(*r_dst), r(*r_str), r(*r_prefix)],
        ),
        Instruction::StrEndsWith {
            r_dst,
            r_str,
            r_suffix,
        } => (
            "STR_ENDS_WITH".into(),
            vec![r(*r_dst), r(*r_str), r(*r_suffix)],
        ),
        Instruction::StrContains {
            r_dst,
            r_str,
            r_sub,
        } => ("STR_CONTAINS".into(), vec![r(*r_dst), r(*r_str), r(*r_sub)]),
        Instruction::StrSplit {
            r_dst,
            r_str,
            r_sep,
        } => ("STR_SPLIT".into(), vec![r(*r_dst), r(*r_str), r(*r_sep)]),
        Instruction::StrReplace {
            r_dst,
            r_str,
            r_from,
            r_to,
        } => (
            "STR_REPLACE".into(),
            vec![r(*r_dst), r(*r_str), r(*r_from), r(*r_to)],
        ),

        // ── 0x0F Boxing ──
        Instruction::Box { r_dst, r_val } => ("BOX".into(), vec![r(*r_dst), r(*r_val)]),
        Instruction::Unbox { r_dst, r_boxed } => ("UNBOX".into(), vec![r(*r_dst), r(*r_boxed)]),
    }
}

/// Convert a flags u16 to keyword string(s) for text output.
///
/// Supports: 0x0001=pub, 0x0002=mut, 0x0004=static. Unknown flags emitted as integer.
fn flags_to_str(flags: u16) -> String {
    if flags == 0 {
        return String::new();
    }

    let mut parts = Vec::new();
    let mut remaining = flags;

    if remaining & 0x0001 != 0 {
        parts.push("pub");
        remaining &= !0x0001;
    }
    if remaining & 0x0002 != 0 {
        parts.push("mut");
        remaining &= !0x0002;
    }
    if remaining & 0x0004 != 0 {
        parts.push("static");
        remaining &= !0x0004;
    }

    if remaining != 0 {
        // Unknown flags: emit as integer
        return format!("{}", flags);
    }

    parts.join(" ")
}
