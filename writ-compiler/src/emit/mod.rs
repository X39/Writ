//! IL metadata emission.
//!
//! Consumes `TypedAst` + `TyInterner` + original ASTs and produces a populated
//! `ModuleBuilder` with all 21 metadata tables filled.

pub mod metadata;
pub mod heaps;
pub mod type_sig;
pub mod error;
pub mod module_builder;
pub mod slots;
pub mod collect;
pub mod body;
pub mod serialize;

use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::Ast;
use crate::check::ir::TypedAst;
use crate::check::ty::TyInterner;

use module_builder::ModuleBuilder;

/// Emit IL metadata from a typed AST.
///
/// This is the entry point for Phase 24 metadata collection. It:
/// 1. Creates a ModuleBuilder
/// 2. Runs the collection pass (populate all tables)
/// 3. Assigns CALL_VIRT vtable slots
/// 4. Finalizes row indices and token assignment
///
/// Returns the populated ModuleBuilder and any diagnostics.
pub fn emit(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
) -> (ModuleBuilder, Vec<Diagnostic>) {
    let mut builder = ModuleBuilder::new();
    let mut diags = Vec::new();

    // Pass 1: collect all definitions into provisional rows.
    collect::collect_defs(typed_ast, asts, interner, &mut builder, &mut diags);

    // Assign CALL_VIRT slot indices from contract declaration order.
    slots::assign_vtable_slots(&mut builder);

    // Pass 2: finalize — assign contiguous row indices, populate def_token_map.
    builder.finalize();

    // Post-finalize: collect exports and attributes that depend on resolved tokens.
    collect::collect_post_finalize(typed_ast, asts, &mut builder);

    (builder, diags)
}

/// Emit method bodies from a typed AST and produce a complete .writil binary.
///
/// This is the Phase 25/26 production entry point:
/// 1. Build metadata tables (TypeDef, MethodDef, ExportDef, etc.)
/// 2. Emit all method bodies
/// 3. Serialize to binary via writ_module::Module::to_bytes()
///
/// `asts` provides the original per-file ASTs needed for metadata collection
/// (field names, method signatures, export detection). Pass `&[]` for unit tests
/// that construct TypedAst programmatically and do not need export/attribute tables.
///
/// Returns `Ok(Vec<u8>)` on success, `Err(Vec<Diagnostic>)` on failure.
pub fn emit_bodies(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    asts: &[(FileId, &Ast)],
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let mut diags = Vec::new();

    // Error pre-pass: abort if TypedAst contains error nodes
    if body::has_error_nodes(typed_ast) {
        diags.push(
            writ_diagnostics::Diagnostic::error(
                "E9000",
                "Codegen aborted: TypedAst contains error nodes",
            ).build()
        );
        return Err(diags);
    }

    // Build metadata tables
    let mut builder = ModuleBuilder::new();
    builder.set_module_def("module", "0.1.0", 0);

    // Pass 1: collect all definitions (TypeDef, MethodDef, FieldDef, ExternDef, etc.)
    // When asts is non-empty, this populates all 21 metadata tables including exports.
    collect::collect_defs(typed_ast, asts, interner, &mut builder, &mut diags);

    if !diags.is_empty() {
        return Err(diags);
    }

    // Assign CALL_VIRT vtable slot indices from contract declaration order.
    slots::assign_vtable_slots(&mut builder);

    // Pre-scan lambdas before finalize (must run before builder.finalize())
    let lambda_infos = body::closure::pre_scan_lambdas(typed_ast, interner, &mut builder);

    // Finalize: assign contiguous row indices, populate def_token_map.
    builder.finalize();

    // Post-finalize: collect exports and attributes that depend on resolved tokens.
    // This populates ExportDef rows for all pub-visible items.
    collect::collect_post_finalize(typed_ast, asts, &mut builder);

    // Emit all method bodies (including lambda bodies via lambda_infos)
    let (mut bodies, body_diags) = body::emit_all_bodies(typed_ast, interner, &builder, &lambda_infos);
    diags.extend(body_diags);

    if !diags.is_empty() {
        return Err(diags);
    }

    // String interning fixup pass.
    // Body emission records pending_strings for LoadString placeholders (string_idx: 0).
    // Now that we hold the mutable builder, intern each string and patch instructions.
    for body in &mut bodies {
        if body.pending_strings.is_empty() {
            continue;
        }
        let pending = std::mem::take(&mut body.pending_strings);
        for (instr_idx, s) in pending {
            let string_idx = builder.string_heap.intern(&s);
            if let Some(writ_module::instruction::Instruction::LoadString { string_idx: idx, .. }) = body.instructions.get_mut(instr_idx) {
                *idx = string_idx;
            }
        }
    }

    // Serialize to binary
    match serialize::serialize(&mut builder, &bodies, interner) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            diags.push(
                writ_diagnostics::Diagnostic::error("E9001", &format!("Serialization failed: {}", e)).build()
            );
            Err(diags)
        }
    }
}
