use std::collections::HashMap;

use writ_module::module::MethodBody;
use writ_module::{Instruction, MetadataToken, ModuleBuilder, Module};

use crate::ast::*;
use crate::error::AssembleError;

/// Name-resolution context built during the first pass.
struct ResolutionCtx {
    /// Maps "TypeName" -> MetadataToken (TypeDef)
    type_map: HashMap<String, MetadataToken>,
    /// Maps "TypeName::method_name" or "method_name" -> MetadataToken (MethodDef)
    method_map: HashMap<String, MetadataToken>,
    /// Maps "TypeName::field_name" -> MetadataToken (FieldDef)
    field_map: HashMap<String, MetadataToken>,
    /// Maps contract name -> MetadataToken (ContractDef)
    contract_map: HashMap<String, MetadataToken>,
    /// Maps module ref name -> MetadataToken (ModuleRef)
    module_ref_map: HashMap<String, MetadataToken>,
    /// Errors collected during assembly.
    errors: Vec<AssembleError>,
}

impl ResolutionCtx {
    fn new() -> Self {
        ResolutionCtx {
            type_map: HashMap::new(),
            method_map: HashMap::new(),
            field_map: HashMap::new(),
            contract_map: HashMap::new(),
            module_ref_map: HashMap::new(),
            errors: Vec::new(),
        }
    }
}

/// Assemble an AST module into a binary Module.
pub fn assemble_module(ast: AsmModule) -> Result<Module, Vec<AssembleError>> {
    let mut builder = ModuleBuilder::new(&ast.name).version(&ast.version);
    let mut ctx = ResolutionCtx::new();

    // ── Phase 1: Declare all entities and build lookup tables ──

    // 1. Module refs (externs)
    for ext in &ast.externs {
        let tok = builder.add_module_ref(&ext.name, &ext.min_version);
        ctx.module_ref_map.insert(ext.name.clone(), tok);
    }

    // 2. Types and their fields
    for ty in &ast.types {
        let kind = match ty.kind {
            AsmTypeKind::Struct => 0,
            AsmTypeKind::Enum => 1,
            AsmTypeKind::Entity => 2,
            AsmTypeKind::Component => 3,
        };
        let type_tok = builder.add_type_def(&ty.name, "", kind, ty.flags);
        ctx.type_map.insert(ty.name.clone(), type_tok);

        // Add fields immediately after type (field_list ordering)
        for field in &ty.fields {
            let type_sig = encode_type_ref(&field.type_ref, &ctx);
            let field_tok = builder.add_field_def(&field.name, &type_sig, field.flags);
            let key = format!("{}::{}", ty.name, field.name);
            ctx.field_map.insert(key, field_tok);
        }
    }

    // 3. Contracts and their methods
    for contract in &ast.contracts {
        let contract_tok = builder.add_contract_def(&contract.name, "");
        ctx.contract_map.insert(contract.name.clone(), contract_tok);

        // Add generic params if any
        for (ordinal, param_name) in contract.generic_params.iter().enumerate() {
            builder.add_generic_param(contract_tok, 1, ordinal as u16, param_name);
        }

        // Add contract methods
        for cm in &contract.methods {
            let sig = encode_method_sig(&cm.signature, &ctx);
            builder.add_contract_method(&cm.name, &sig, cm.slot);
        }
    }

    // 4. Impl blocks: add_impl_def and pre-register methods with placeholder bodies
    for imp in &ast.impls {
        let type_tok = ctx.type_map.get(&imp.type_name).copied().unwrap_or_else(|| {
            ctx.errors.push(AssembleError::new(
                format!("undefined type '{}' in .impl", imp.type_name),
                0, 0,
            ));
            MetadataToken::NULL
        });
        let contract_tok = ctx.contract_map.get(&imp.contract_name).copied().unwrap_or_else(|| {
            ctx.errors.push(AssembleError::new(
                format!("undefined contract '{}' in .impl", imp.contract_name),
                0, 0,
            ));
            MetadataToken::NULL
        });
        builder.add_impl_def(type_tok, contract_tok);

        // Pre-register impl methods with placeholder bodies
        for method in &imp.methods {
            let sig = encode_method_sig_from_params(&method.params, &method.return_type, &ctx);
            let placeholder_body = MethodBody {
                register_types: vec![0; method.registers.len()],
                code: Vec::new(),
                debug_locals: Vec::new(),
                source_spans: Vec::new(),
            };
            let reg_count = method.registers.len() as u16;
            let tok = builder.add_method(&method.name, &sig, method.flags, reg_count, placeholder_body);
            let key = format!("{}::{}", imp.type_name, method.name);
            ctx.method_map.insert(key, tok);
        }
    }

    // 5. Global definitions
    for global in &ast.globals {
        let type_sig = encode_type_ref(&global.type_ref, &ctx);
        let init = global.init_value.as_deref().unwrap_or(&[]);
        builder.add_global_def(&global.name, &type_sig, global.flags, init);
    }

    // 6. Extern functions
    for ext_fn in &ast.extern_fns {
        let sig = encode_method_sig(&ext_fn.signature, &ctx);
        builder.add_extern_def(&ext_fn.name, &sig, &ext_fn.import_name, ext_fn.flags);
    }

    // 7. Top-level methods: pre-register with placeholder bodies
    for method in &ast.methods {
        let sig = encode_method_sig_from_params(&method.params, &method.return_type, &ctx);
        let placeholder_body = MethodBody {
            register_types: vec![0; method.registers.len()],
            code: Vec::new(),
            debug_locals: Vec::new(),
            source_spans: Vec::new(),
        };
        let reg_count = method.registers.len() as u16;
        let tok = builder.add_method(&method.name, &sig, method.flags, reg_count, placeholder_body);
        ctx.method_map.insert(method.name.clone(), tok);
    }

    // ── Phase 2: Assemble method bodies with label resolution ──
    // Now that all names are registered, we can resolve references in instruction operands.

    // Collect all methods with their owner type (for method_map key lookup)
    let mut all_methods: Vec<(&AsmMethod, Option<String>)> = Vec::new();
    for imp in &ast.impls {
        for method in &imp.methods {
            all_methods.push((method, Some(imp.type_name.clone())));
        }
    }
    for method in &ast.methods {
        all_methods.push((method, None));
    }

    // Assemble each method body
    let mut assembled_bodies: Vec<MethodBody> = Vec::new();
    for (method, _owner) in &all_methods {
        match assemble_method_body(method, &ctx) {
            Ok(body) => assembled_bodies.push(body),
            Err(errs) => ctx.errors.extend(errs),
        }
    }

    if !ctx.errors.is_empty() {
        return Err(ctx.errors);
    }

    // Build the module, then patch method bodies into it
    let mut module = builder.build();

    // Replace placeholder method bodies with assembled ones
    // The method_bodies vec in Module corresponds 1:1 to method_defs
    for (i, body) in assembled_bodies.into_iter().enumerate() {
        if i < module.method_bodies.len() {
            module.method_bodies[i] = body;
        }
    }

    Ok(module)
}

/// Assemble the body of a method using two-pass label resolution.
fn assemble_method_body(
    method: &AsmMethod,
    ctx: &ResolutionCtx,
) -> Result<MethodBody, Vec<AssembleError>> {
    let mut errors = Vec::new();

    // Pass 1: Convert instructions and collect label byte offsets
    let mut label_offsets: HashMap<String, u32> = HashMap::new();
    // Stores: (instruction, byte_offset, optional_label_patches)
    // A label patch means this instruction has a branch offset that needs patching
    struct InstrEntry {
        instr: Instruction,
        offset: u32,
        /// For each label reference in this instruction, store (label_name, line, col)
        label_patches: Vec<String>,
    }
    let mut entries: Vec<InstrEntry> = Vec::new();
    let mut byte_offset: u32 = 0;

    for stmt in &method.body {
        match stmt {
            AsmStatement::Label(name) => {
                label_offsets.insert(name.clone(), byte_offset);
            }
            AsmStatement::Instruction(instr) => {
                // Collect label names from operands for branch patching
                let label_names: Vec<String> = instr.operands.iter().filter_map(|op| {
                    if let AsmOperand::LabelRef(name) = op {
                        Some(name.clone())
                    } else {
                        None
                    }
                }).collect();

                match map_instruction(&instr.mnemonic, &instr.operands, ctx, instr.line, instr.col) {
                    Ok(instruction) => {
                        let size = instruction_size(&instruction);
                        entries.push(InstrEntry {
                            instr: instruction,
                            offset: byte_offset,
                            label_patches: label_names,
                        });
                        byte_offset += size;
                    }
                    Err(e) => {
                        errors.push(e);
                    }
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Pass 2: Patch branch offsets and emit code
    let mut code = Vec::new();
    for entry in &entries {
        let patched = if !entry.label_patches.is_empty() {
            patch_branch(&entry.instr, entry.offset, &entry.label_patches, &label_offsets, &mut errors)
        } else {
            entry.instr.clone()
        };

        if let Err(e) = patched.encode(&mut code) {
            errors.push(AssembleError::new(
                format!("instruction encode error: {}", e),
                0, 0,
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Register types: store 0 as placeholder blob heap offsets.
    // ModuleBuilder doesn't expose blob heap for external interning of register types.
    let register_types = vec![0u32; method.registers.len()];

    Ok(MethodBody {
        register_types,
        code,
        debug_locals: Vec::new(),
        source_spans: Vec::new(),
    })
}

/// Patch branch offsets in an instruction using the resolved label map.
///
/// Branch offset = target_offset - (current_offset + instruction_size)
fn patch_branch(
    instr: &Instruction,
    current_offset: u32,
    label_names: &[String],
    label_offsets: &HashMap<String, u32>,
    errors: &mut Vec<AssembleError>,
) -> Instruction {
    let instr_size = instruction_size(instr);
    let after_instr = current_offset + instr_size;

    // Resolve the first label name (used by BR, BR_TRUE, BR_FALSE)
    let mut resolve = |name: &str| -> i32 {
        if let Some(&target) = label_offsets.get(name) {
            (target as i64 - after_instr as i64) as i32
        } else {
            errors.push(AssembleError::new(
                format!("undefined label '.{}'", name),
                0, 0,
            ));
            0
        }
    };

    match instr {
        Instruction::Br { .. } => {
            if let Some(name) = label_names.first() {
                Instruction::Br { offset: resolve(name) }
            } else {
                instr.clone()
            }
        }
        Instruction::BrTrue { r_cond, .. } => {
            if let Some(name) = label_names.first() {
                Instruction::BrTrue { r_cond: *r_cond, offset: resolve(name) }
            } else {
                instr.clone()
            }
        }
        Instruction::BrFalse { r_cond, .. } => {
            if let Some(name) = label_names.first() {
                Instruction::BrFalse { r_cond: *r_cond, offset: resolve(name) }
            } else {
                instr.clone()
            }
        }
        Instruction::Switch { r_tag, offsets } => {
            // Each offset in SWITCH corresponds to a label name
            let mut new_offsets = Vec::with_capacity(offsets.len());
            for (i, _) in offsets.iter().enumerate() {
                if let Some(name) = label_names.get(i) {
                    new_offsets.push(resolve(name));
                } else {
                    new_offsets.push(0);
                }
            }
            Instruction::Switch { r_tag: *r_tag, offsets: new_offsets }
        }
        _ => instr.clone(),
    }
}

/// Compute the encoded byte size of an instruction.
fn instruction_size(instr: &Instruction) -> u32 {
    let mut buf = Vec::new();
    let _ = instr.encode(&mut buf);
    buf.len() as u32
}

/// Encode an AsmTypeRef to its blob byte representation.
fn encode_type_ref(type_ref: &AsmTypeRef, ctx: &ResolutionCtx) -> Vec<u8> {
    match type_ref {
        AsmTypeRef::Void => vec![0x00],
        AsmTypeRef::Int => vec![0x01],
        AsmTypeRef::Float => vec![0x02],
        AsmTypeRef::Bool => vec![0x03],
        AsmTypeRef::String_ => vec![0x04],
        AsmTypeRef::Named(name) => {
            if let Some(tok) = ctx.type_map.get(name) {
                let mut bytes = vec![0x10];
                bytes.extend_from_slice(&tok.0.to_le_bytes());
                bytes
            } else {
                // Unknown type -- encode as void fallback, error reported elsewhere
                vec![0x00]
            }
        }
        AsmTypeRef::Array(elem) => {
            let mut bytes = vec![0x20];
            bytes.extend(encode_type_ref(elem, ctx));
            bytes
        }
        AsmTypeRef::Generic(_name, _args) => {
            // Generic type instantiation not fully supported yet
            vec![0x00]
        }
        AsmTypeRef::RawBlob(raw) => raw.clone(),
    }
}

/// Encode a method signature (from AsmMethodSig) to blob bytes.
/// Format: u16(param_count) + param_type_blobs + return_type_blob
fn encode_method_sig(sig: &AsmMethodSig, ctx: &ResolutionCtx) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(sig.params.len() as u16).to_le_bytes());
    for p in &sig.params {
        bytes.extend(encode_type_ref(p, ctx));
    }
    bytes.extend(encode_type_ref(&sig.return_type, ctx));
    bytes
}

/// Encode a method signature from named params and return type.
fn encode_method_sig_from_params(
    params: &[AsmParam],
    return_type: &AsmTypeRef,
    ctx: &ResolutionCtx,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(params.len() as u16).to_le_bytes());
    for p in params {
        bytes.extend(encode_type_ref(&p.type_ref, ctx));
    }
    bytes.extend(encode_type_ref(return_type, ctx));
    bytes
}

/// Map a text mnemonic + operands to an Instruction variant.
///
/// Handles all 91 opcodes. Mnemonics are matched case-insensitively.
/// For branch instructions, label references produce a placeholder offset of 0.
fn map_instruction(
    mnemonic: &str,
    operands: &[AsmOperand],
    ctx: &ResolutionCtx,
    line: u32,
    col: u32,
) -> Result<Instruction, AssembleError> {
    let upper = mnemonic.to_uppercase();

    // Helper closures
    let reg = |idx: usize| -> Result<u16, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::Register(r)) => Ok(*r),
            _ => Err(AssembleError::new(
                format!("{}: expected register at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    let int_lit = |idx: usize| -> Result<i64, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::IntLit(v)) => Ok(*v),
            _ => Err(AssembleError::new(
                format!("{}: expected integer at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    let float_lit = |idx: usize| -> Result<f64, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::FloatLit(v)) => Ok(*v),
            _ => Err(AssembleError::new(
                format!("{}: expected float at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    let string_idx = |idx: usize| -> Result<u32, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::IntLit(v)) => Ok(*v as u32),
            Some(AsmOperand::StringLit(_)) => Ok(0), // placeholder
            _ => Err(AssembleError::new(
                format!("{}: expected string index at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    let token_val = |idx: usize| -> Result<u32, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::IntLit(v)) => Ok(*v as u32),
            Some(AsmOperand::Token(t)) => Ok(*t),
            Some(AsmOperand::TypeRef(tr)) => {
                if let AsmTypeRef::Named(name) = tr {
                    if let Some(tok) = ctx.type_map.get(name) {
                        return Ok(tok.0);
                    }
                }
                Ok(0)
            }
            Some(AsmOperand::MethodRef(mr)) => {
                resolve_method_ref(mr, ctx, line, col)
            }
            Some(AsmOperand::FieldRef(fr)) => {
                resolve_field_ref(fr, ctx, line, col)
            }
            _ => Err(AssembleError::new(
                format!("{}: expected token/index at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    // For branch instructions: label ref -> offset placeholder 0, integer literal passes through
    let label_offset = |idx: usize| -> Result<i32, AssembleError> {
        match operands.get(idx) {
            Some(AsmOperand::LabelRef(_)) => Ok(0), // placeholder, patched in pass 2
            Some(AsmOperand::IntLit(v)) => Ok(*v as i32),
            _ => Err(AssembleError::new(
                format!("{}: expected label reference at operand {}", upper, idx + 1),
                line, col,
            )),
        }
    };

    match upper.as_str() {
        // ── 0x00 Meta ──
        "NOP" => Ok(Instruction::Nop),
        "CRASH" => Ok(Instruction::Crash { r_msg: reg(0)? }),

        // ── 0x01 Data Movement ──
        "MOV" => Ok(Instruction::Mov { r_dst: reg(0)?, r_src: reg(1)? }),
        "LOAD_INT" => Ok(Instruction::LoadInt { r_dst: reg(0)?, value: int_lit(1)? }),
        "LOAD_FLOAT" => Ok(Instruction::LoadFloat { r_dst: reg(0)?, value: float_lit(1)? }),
        "LOAD_TRUE" => Ok(Instruction::LoadTrue { r_dst: reg(0)? }),
        "LOAD_FALSE" => Ok(Instruction::LoadFalse { r_dst: reg(0)? }),
        "LOAD_STRING" => Ok(Instruction::LoadString { r_dst: reg(0)?, string_idx: string_idx(1)? }),
        "LOAD_NULL" => Ok(Instruction::LoadNull { r_dst: reg(0)? }),

        // ── 0x02 Integer Arithmetic ──
        "ADD_I" => Ok(Instruction::AddI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "SUB_I" => Ok(Instruction::SubI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "MUL_I" => Ok(Instruction::MulI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "DIV_I" => Ok(Instruction::DivI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "MOD_I" => Ok(Instruction::ModI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "NEG_I" => Ok(Instruction::NegI { r_dst: reg(0)?, r_src: reg(1)? }),

        // ── 0x03 Float Arithmetic ──
        "ADD_F" => Ok(Instruction::AddF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "SUB_F" => Ok(Instruction::SubF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "MUL_F" => Ok(Instruction::MulF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "DIV_F" => Ok(Instruction::DivF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "MOD_F" => Ok(Instruction::ModF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "NEG_F" => Ok(Instruction::NegF { r_dst: reg(0)?, r_src: reg(1)? }),

        // ── 0x04 Bitwise & Logical ──
        "BIT_AND" => Ok(Instruction::BitAnd { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "BIT_OR" => Ok(Instruction::BitOr { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "SHL" => Ok(Instruction::Shl { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "SHR" => Ok(Instruction::Shr { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "NOT" => Ok(Instruction::Not { r_dst: reg(0)?, r_src: reg(1)? }),

        // ── 0x05 Comparison ──
        "CMP_EQ_I" => Ok(Instruction::CmpEqI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "CMP_EQ_F" => Ok(Instruction::CmpEqF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "CMP_EQ_B" => Ok(Instruction::CmpEqB { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "CMP_EQ_S" => Ok(Instruction::CmpEqS { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "CMP_LT_I" => Ok(Instruction::CmpLtI { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "CMP_LT_F" => Ok(Instruction::CmpLtF { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),

        // ── 0x06 Control Flow ──
        "BR" => Ok(Instruction::Br { offset: label_offset(0)? }),
        "BR_TRUE" => Ok(Instruction::BrTrue { r_cond: reg(0)?, offset: label_offset(1)? }),
        "BR_FALSE" => Ok(Instruction::BrFalse { r_cond: reg(0)?, offset: label_offset(1)? }),
        "SWITCH" => {
            let r_tag = reg(0)?;
            let mut offsets = Vec::new();
            for i in 1..operands.len() {
                offsets.push(label_offset(i)?);
            }
            Ok(Instruction::Switch { r_tag, offsets })
        }
        "RET" => Ok(Instruction::Ret { r_src: reg(0)? }),
        "RET_VOID" => Ok(Instruction::RetVoid),

        // ── 0x07 Calls & Delegates ──
        "CALL" => Ok(Instruction::Call {
            r_dst: reg(0)?,
            method_idx: token_val(1)?,
            r_base: reg(2)?,
            argc: int_lit(3)? as u16,
        }),
        "CALL_VIRT" => Ok(Instruction::CallVirt {
            r_dst: reg(0)?,
            r_obj: reg(1)?,
            contract_idx: token_val(2)?,
            slot: int_lit(3)? as u16,
            r_base: reg(4)?,
            argc: int_lit(5)? as u16,
        }),
        "CALL_EXTERN" => Ok(Instruction::CallExtern {
            r_dst: reg(0)?,
            extern_idx: token_val(1)?,
            r_base: reg(2)?,
            argc: int_lit(3)? as u16,
        }),
        "NEW_DELEGATE" => Ok(Instruction::NewDelegate {
            r_dst: reg(0)?,
            method_idx: token_val(1)?,
            r_target: reg(2)?,
        }),
        "CALL_INDIRECT" => Ok(Instruction::CallIndirect {
            r_dst: reg(0)?,
            r_delegate: reg(1)?,
            r_base: reg(2)?,
            argc: int_lit(3)? as u16,
        }),
        "TAIL_CALL" => Ok(Instruction::TailCall {
            method_idx: token_val(0)?,
            r_base: reg(1)?,
            argc: int_lit(2)? as u16,
        }),

        // ── 0x08 Object Model ──
        "NEW" => Ok(Instruction::New { r_dst: reg(0)?, type_idx: token_val(1)? }),
        "GET_FIELD" => Ok(Instruction::GetField {
            r_dst: reg(0)?,
            r_obj: reg(1)?,
            field_idx: token_val(2)?,
        }),
        "SET_FIELD" => Ok(Instruction::SetField {
            r_obj: reg(0)?,
            field_idx: token_val(1)?,
            r_val: reg(2)?,
        }),
        "SPAWN_ENTITY" => Ok(Instruction::SpawnEntity { r_dst: reg(0)?, type_idx: token_val(1)? }),
        "INIT_ENTITY" => Ok(Instruction::InitEntity { r_entity: reg(0)? }),
        "GET_COMPONENT" => Ok(Instruction::GetComponent {
            r_dst: reg(0)?,
            r_entity: reg(1)?,
            comp_type_idx: token_val(2)?,
        }),
        "GET_OR_CREATE" => Ok(Instruction::GetOrCreate { r_dst: reg(0)?, type_idx: token_val(1)? }),
        "FIND_ALL" => Ok(Instruction::FindAll { r_dst: reg(0)?, type_idx: token_val(1)? }),
        "DESTROY_ENTITY" => Ok(Instruction::DestroyEntity { r_entity: reg(0)? }),
        "ENTITY_IS_ALIVE" => Ok(Instruction::EntityIsAlive { r_dst: reg(0)?, r_entity: reg(1)? }),

        // ── 0x09 Arrays ──
        "NEW_ARRAY" => Ok(Instruction::NewArray { r_dst: reg(0)?, elem_type: token_val(1)? }),
        "ARRAY_INIT" => Ok(Instruction::ArrayInit {
            r_dst: reg(0)?,
            elem_type: token_val(1)?,
            count: int_lit(2)? as u16,
            r_base: reg(3)?,
        }),
        "ARRAY_LOAD" => Ok(Instruction::ArrayLoad { r_dst: reg(0)?, r_arr: reg(1)?, r_idx: reg(2)? }),
        "ARRAY_STORE" => Ok(Instruction::ArrayStore { r_arr: reg(0)?, r_idx: reg(1)?, r_val: reg(2)? }),
        "ARRAY_LEN" => Ok(Instruction::ArrayLen { r_dst: reg(0)?, r_arr: reg(1)? }),
        "ARRAY_ADD" => Ok(Instruction::ArrayAdd { r_arr: reg(0)?, r_val: reg(1)? }),
        "ARRAY_REMOVE" => Ok(Instruction::ArrayRemove { r_arr: reg(0)?, r_idx: reg(1)? }),
        "ARRAY_INSERT" => Ok(Instruction::ArrayInsert { r_arr: reg(0)?, r_idx: reg(1)?, r_val: reg(2)? }),
        "ARRAY_SLICE" => Ok(Instruction::ArraySlice {
            r_dst: reg(0)?,
            r_arr: reg(1)?,
            r_start: reg(2)?,
            r_end: reg(3)?,
        }),

        // ── 0x0A Type Operations — Option ──
        "WRAP_SOME" => Ok(Instruction::WrapSome { r_dst: reg(0)?, r_val: reg(1)? }),
        "UNWRAP" => Ok(Instruction::Unwrap { r_dst: reg(0)?, r_opt: reg(1)? }),
        "IS_SOME" => Ok(Instruction::IsSome { r_dst: reg(0)?, r_opt: reg(1)? }),
        "IS_NONE" => Ok(Instruction::IsNone { r_dst: reg(0)?, r_opt: reg(1)? }),

        // ── 0x0A Type Operations — Result ──
        "WRAP_OK" => Ok(Instruction::WrapOk { r_dst: reg(0)?, r_val: reg(1)? }),
        "WRAP_ERR" => Ok(Instruction::WrapErr { r_dst: reg(0)?, r_err: reg(1)? }),
        "UNWRAP_OK" => Ok(Instruction::UnwrapOk { r_dst: reg(0)?, r_result: reg(1)? }),
        "IS_OK" => Ok(Instruction::IsOk { r_dst: reg(0)?, r_result: reg(1)? }),
        "IS_ERR" => Ok(Instruction::IsErr { r_dst: reg(0)?, r_result: reg(1)? }),
        "EXTRACT_ERR" => Ok(Instruction::ExtractErr { r_dst: reg(0)?, r_result: reg(1)? }),

        // ── 0x0A Type Operations — Enum ──
        "NEW_ENUM" => Ok(Instruction::NewEnum {
            r_dst: reg(0)?,
            type_idx: token_val(1)?,
            tag: int_lit(2)? as u16,
            field_count: int_lit(3)? as u16,
            r_base: reg(4)?,
        }),
        "GET_TAG" => Ok(Instruction::GetTag { r_dst: reg(0)?, r_enum: reg(1)? }),
        "EXTRACT_FIELD" => Ok(Instruction::ExtractField {
            r_dst: reg(0)?,
            r_enum: reg(1)?,
            field_idx: int_lit(2)? as u16,
        }),

        // ── 0x0B Concurrency ──
        "SPAWN_TASK" => Ok(Instruction::SpawnTask {
            r_dst: reg(0)?,
            method_idx: token_val(1)?,
            r_base: reg(2)?,
            argc: int_lit(3)? as u16,
        }),
        "SPAWN_DETACHED" => Ok(Instruction::SpawnDetached {
            r_dst: reg(0)?,
            method_idx: token_val(1)?,
            r_base: reg(2)?,
            argc: int_lit(3)? as u16,
        }),
        "JOIN" => Ok(Instruction::Join { r_dst: reg(0)?, r_task: reg(1)? }),
        "CANCEL" => Ok(Instruction::Cancel { r_task: reg(0)? }),
        "DEFER_PUSH" => Ok(Instruction::DeferPush { r_dst: reg(0)?, method_idx: token_val(1)? }),
        "DEFER_POP" => Ok(Instruction::DeferPop),
        "DEFER_END" => Ok(Instruction::DeferEnd),

        // ── 0x0C Globals & Atomics ──
        "LOAD_GLOBAL" => Ok(Instruction::LoadGlobal { r_dst: reg(0)?, global_idx: token_val(1)? }),
        "STORE_GLOBAL" => Ok(Instruction::StoreGlobal { global_idx: token_val(0)?, r_src: reg(1)? }),
        "ATOMIC_BEGIN" => Ok(Instruction::AtomicBegin),
        "ATOMIC_END" => Ok(Instruction::AtomicEnd),

        // ── 0x0D Conversion ──
        "I2F" => Ok(Instruction::I2f { r_dst: reg(0)?, r_src: reg(1)? }),
        "F2I" => Ok(Instruction::F2i { r_dst: reg(0)?, r_src: reg(1)? }),
        "I2S" => Ok(Instruction::I2s { r_dst: reg(0)?, r_src: reg(1)? }),
        "F2S" => Ok(Instruction::F2s { r_dst: reg(0)?, r_src: reg(1)? }),
        "B2S" => Ok(Instruction::B2s { r_dst: reg(0)?, r_src: reg(1)? }),
        "CONVERT" => Ok(Instruction::Convert {
            r_dst: reg(0)?,
            r_src: reg(1)?,
            target_type: token_val(2)?,
        }),

        // ── 0x0E Strings ──
        "STR_CONCAT" => Ok(Instruction::StrConcat { r_dst: reg(0)?, r_a: reg(1)?, r_b: reg(2)? }),
        "STR_BUILD" => Ok(Instruction::StrBuild {
            r_dst: reg(0)?,
            count: int_lit(1)? as u16,
            r_base: reg(2)?,
        }),
        "STR_LEN" => Ok(Instruction::StrLen { r_dst: reg(0)?, r_str: reg(1)? }),

        // ── 0x0F Boxing ──
        "BOX" => Ok(Instruction::Box { r_dst: reg(0)?, r_val: reg(1)? }),
        "UNBOX" => Ok(Instruction::Unbox { r_dst: reg(0)?, r_boxed: reg(1)? }),

        _ => Err(AssembleError::new(
            format!("unknown instruction mnemonic '{}'", mnemonic),
            line, col,
        )),
    }
}

/// Resolve a method reference to a metadata token value.
fn resolve_method_ref(
    mr: &AsmMethodRef,
    ctx: &ResolutionCtx,
    line: u32,
    col: u32,
) -> Result<u32, AssembleError> {
    let key = if let Some(type_name) = &mr.type_name {
        format!("{}::{}", type_name, mr.method_name)
    } else {
        mr.method_name.clone()
    };

    if let Some(tok) = ctx.method_map.get(&key) {
        Ok(tok.0)
    } else {
        Err(AssembleError::new(
            format!("undefined method reference '{}'", key),
            line, col,
        ))
    }
}

/// Resolve a field reference to a metadata token value.
fn resolve_field_ref(
    fr: &AsmFieldRef,
    ctx: &ResolutionCtx,
    line: u32,
    col: u32,
) -> Result<u32, AssembleError> {
    let key = format!("{}::{}", fr.type_name, fr.field_name);
    if let Some(tok) = ctx.field_map.get(&key) {
        Ok(tok.0)
    } else {
        Err(AssembleError::new(
            format!("undefined field reference '{}'", key),
            line, col,
        ))
    }
}
