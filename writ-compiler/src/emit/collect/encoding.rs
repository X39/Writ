//! Type encoding helpers and post-finalize collection passes.

use rustc_hash::FxHashMap;
use writ_diagnostics::FileId;

use writ_module::attr::{AttrValue, encode_attr_args, ATTR_TAG_STRING, ATTR_TAG_INT, ATTR_TAG_BOOL};
use writ_module::tables::ATTR_OWNER_KIND_DECL;

use crate::ast::decl::{AstDecl, AstFnParam, AstParam};
use crate::ast::expr::AstExpr;
use crate::ast::decl::AstAttributeArg;
use crate::ast::Ast;
use crate::check::ir::{TypedAst, TypedDecl};
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefKind, DefMap, DefVis};

use crate::emit::metadata::{MetadataToken, TableId};
use crate::emit::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle};

use super::lookup::find_attrs_for_entry;

// =============================================================================
// Export collection
// =============================================================================

pub(super) fn collect_exports(def_map: &DefMap, builder: &mut ModuleBuilder) {
    // Collect all public DefIds, including overloaded functions.
    // by_fqn has the first overload; fn_overloads has all overloads for overloaded names.
    let mut seen = rustc_hash::FxHashSet::default();
    let all_ids: Vec<_> = def_map.by_fqn.values().copied()
        .chain(def_map.fn_overloads.values().flat_map(|ids| ids.iter().copied()))
        .filter(|id| seen.insert(*id))
        .collect();
    for def_id in all_ids {
        let entry = def_map.get_entry(def_id);
        if !matches!(entry.vis, DefVis::Pub) {
            continue;
        }
        // Skip synthetic entries (compiler-injected builtins like log:: levels).
        if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
            continue;
        }

        // Determine item_kind and get token.
        // item_kind encoding (matches disassembler and cmd_run):
        //   0 = method (Fn, ExternFn)
        //   1 = type   (Struct, Entity, Enum, Component, Contract, ExternComponent)
        //   2 = global (Const, Global)
        if let Some(token) = builder.token_for_def(def_id) {
            let item_kind = match entry.kind {
                DefKind::Fn | DefKind::ExternFn => 0, // method
                DefKind::Struct | DefKind::Class | DefKind::Entity | DefKind::Enum
                | DefKind::Component
                | DefKind::ExternComponent | DefKind::Contract => 1, // type
                DefKind::Const | DefKind::Global => 2, // global
                DefKind::Impl => continue, // impls aren't exported directly
                DefKind::AttributeDef => continue, // attribute decls are not exports
            };
            builder.add_export_def(&entry.name, item_kind, token);
        }
    }
}

// =============================================================================
// Attribute collection
// =============================================================================

/// Map an AstAttributeArg to an AttrValue for blob encoding.
fn map_attr_arg(arg: &AstAttributeArg) -> Option<AttrValue> {
    match arg {
        AstAttributeArg::Positional(expr) => map_attr_expr(expr),
        AstAttributeArg::Named { name, value, .. } => {
            map_attr_expr(value).map(|v| AttrValue::Named {
                name: name.clone(),
                value: Box::new(v),
            })
        }
    }
}

/// Map an AstExpr to an AttrValue. Returns None for unsupported expression types.
fn map_attr_expr(expr: &AstExpr) -> Option<AttrValue> {
    match expr {
        AstExpr::StringLit { value, .. } => Some(AttrValue::String(value.clone())),
        AstExpr::IntLit { value, .. } => Some(AttrValue::Int(*value)),
        AstExpr::BoolLit { value, .. } => Some(AttrValue::Bool(*value)),
        _ => None, // Unsupported expr types silently skipped in Phase 93
    }
}

pub(super) fn collect_attributes(typed_ast: &TypedAst, asts: &[(FileId, &Ast)], builder: &mut ModuleBuilder) {
    let def_map = &typed_ast.def_map;

    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::Struct { def_id }
            | TypedDecl::Class { def_id }
            | TypedDecl::Entity { def_id }
            | TypedDecl::Enum { def_id }
            | TypedDecl::Contract { def_id }
            | TypedDecl::Component { def_id }
            | TypedDecl::ExternComponent { def_id } => *def_id,
            TypedDecl::Fn { def_id, .. }
            | TypedDecl::ExternFn { def_id }
            | TypedDecl::Const { def_id, .. }
            | TypedDecl::Global { def_id, .. } => *def_id,
            TypedDecl::Impl { .. } => continue,
            // Attribute declarations do not produce attribute rows themselves.
            TypedDecl::AttributeDef { .. } => continue,
        };

        let entry = def_map.get_entry(def_id);

        // Find the matching AST decl's attributes.
        let attrs = find_attrs_for_entry(asts, entry);
        if attrs.is_empty() {
            continue;
        }

        let owner_token = match builder.token_for_def(def_id) {
            Some(t) => t,
            None => continue,
        };
        let owner_kind: u8 = match entry.kind {
            DefKind::Struct | DefKind::Entity | DefKind::Enum | DefKind::Component
            | DefKind::Contract | DefKind::ExternComponent => 0, // type
            DefKind::Fn | DefKind::ExternFn => 1, // method
            _ => 2, // field/global
        };

        for attr in &attrs {
            let values: Vec<AttrValue> = attr.args.iter()
                .filter_map(map_attr_arg)
                .collect();
            let blob_offset = if values.is_empty() {
                0u32
            } else {
                let bytes = encode_attr_args(&values);
                builder.blob_heap.intern(&bytes)
            };
            builder.add_attribute_def(owner_token, owner_kind, &attr.name, blob_offset);
        }
    }
}

// =============================================================================
// Attribute declaration collection (UATTR-02)
// =============================================================================

/// Collect AttributeDef rows for user-defined attribute declarations.
///
/// Each `attribute Name(params...);` declaration produces one AttributeDef row
/// with owner_kind = ATTR_OWNER_KIND_DECL (3) and owner = MetadataToken::NULL.
/// The blob value encodes the parameter type signature: u16 count + one tag byte
/// per parameter (ATTR_TAG_STRING / ATTR_TAG_INT / ATTR_TAG_BOOL).
pub(super) fn collect_attribute_decl_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    builder: &mut ModuleBuilder,
) {
    let def_map = &typed_ast.def_map;

    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::AttributeDef { def_id } => *def_id,
            _ => continue,
        };

        let entry = def_map.get_entry(def_id);

        // Find the AstAttributeDecl to get param types.
        let mut param_tags: Vec<u8> = Vec::new();
        'outer: for (fid, ast) in asts {
            if *fid != entry.file_id {
                continue;
            }
            for d in &ast.items {
                if let AstDecl::Attribute(a) = d {
                    if a.name == entry.name && a.name_span == entry.name_span {
                        for param in &a.params {
                            let tag = match &param.ty {
                                crate::ast::types::AstType::Named { name, .. } => {
                                    match name.as_str() {
                                        "string" => ATTR_TAG_STRING,
                                        "int" => ATTR_TAG_INT,
                                        "bool" => ATTR_TAG_BOOL,
                                        _ => continue, // unsupported type, already caught by checker
                                    }
                                }
                                _ => continue, // unsupported type
                            };
                            param_tags.push(tag);
                        }
                        break 'outer;
                    }
                }
            }
        }

        // Encode param type signature: u16 param count + one tag byte per param.
        let mut sig_buf = Vec::new();
        sig_buf.extend_from_slice(&(param_tags.len() as u16).to_le_bytes());
        sig_buf.extend_from_slice(&param_tags);

        let blob_offset = builder.blob_heap.intern(&sig_buf);

        builder.add_attribute_def(
            MetadataToken::NULL,
            ATTR_OWNER_KIND_DECL,
            &entry.name,
            blob_offset,
        );
    }
}

// =============================================================================
// LocaleDef collection
// =============================================================================

/// Collect LocaleDef rows for all Fn decls that have a [Locale("tag")] attribute.
///
/// Must be called from collect_post_finalize() after token assignment, because
/// it uses builder.token_for_def() and builder.methoddef_token_by_name() which
/// depend on finalized MethodDef tokens.
pub(super) fn collect_locale_defs(typed_ast: &TypedAst, asts: &[(FileId, &Ast)], builder: &mut ModuleBuilder) {
    let def_map = &typed_ast.def_map;

    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::Fn { def_id, .. } => *def_id,
            _ => continue,
        };

        let entry = def_map.get_entry(def_id);
        let attrs = find_attrs_for_entry(asts, entry);

        // Look for [Locale("tag")] attribute.
        let locale_tag = attrs.iter().find_map(|a| {
            if a.name != "Locale" {
                return None;
            }
            a.args.iter().find_map(|arg| {
                if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                    Some(value.clone())
                } else {
                    None
                }
            })
        });

        let tag = match locale_tag {
            Some(t) => t,
            None => continue,
        };

        // This is a locale override. Its name in the DefMap is "baseName$tag"
        // (set by lower_dialogue's suffix logic). Extract the base name.
        let base_name = entry.name.split('$').next().unwrap_or(&entry.name);

        // Look up the base dlg's MethodDef token by its un-suffixed name.
        let base_token = builder
            .methoddef_token_by_name(base_name)
            .map(MetadataToken);

        // Look up this override's MethodDef token via its DefId.
        let loc_method_token = builder.token_for_def(def_id);

        if let (Some(base), Some(loc)) = (base_token, loc_method_token) {
            builder.add_locale_def(base, &tag, loc);
        }
    }
}

// =============================================================================
// Component slot collection
// =============================================================================

pub(super) fn collect_component_slots(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
    typedef_handles: &FxHashMap<DefId, TypeDefHandle>,
) {
    use super::lookup::find_entity_decl;

    for decl in &typed_ast.decls {
        if let TypedDecl::Entity { def_id } = decl {
            let entry = def_map.get_entry(*def_id);
            if let Some(entity_decl) = find_entity_decl(asts, entry) {
                let entity_token = typedef_handles
                    .get(def_id)
                    .map(|h| MetadataToken::new(TableId::TypeDef, (h.0 + 1) as u32))
                    .unwrap_or(MetadataToken::NULL);

                for slot in &entity_decl.component_slots {
                    // Resolve component name to DefId.
                    let comp_token = def_map
                        .get(&slot.component)
                        .and_then(|comp_id| {
                            typedef_handles
                                .get(&comp_id)
                                .map(|h| MetadataToken::new(TableId::TypeDef, (h.0 + 1) as u32))
                        })
                        .unwrap_or(MetadataToken::NULL);

                    builder.add_component_slot(entity_token, comp_token);
                }
            }
        }
    }
}

// =============================================================================
// Type signature encoding helpers
// =============================================================================

/// Convert a primitive or generic-param AstType to a Ty without mutating the interner.
///
/// Primitive types (int, float, bool, string, void) have fixed pre-interned indices
/// from TyInterner::new(): Int=0, Float=1, Bool=2, String=3, Void=4.
///
/// For non-primitive named types (structs, enums, entities) we fall back to
/// crate::check::ty::Ty(5) (Error), which is acceptable for register allocation
/// since the register type table is used only for debug info and the disassembler.
pub(super) fn ast_type_to_ty_simple(
    ast_type: &crate::ast::types::AstType,
    generics: &[String],
    def_map: &DefMap,
) -> crate::check::ty::Ty {
    use crate::check::ty::Ty;
    match ast_type {
        crate::ast::types::AstType::Named { name, .. } => {
            // Check generic param — use GenericParam index
            if let Some(idx) = generics.iter().position(|g| g == name) {
                // GenericParam types are not pre-interned; use Error as a safe fallback
                // for the register allocator (only affects debug info, not correctness).
                let _ = idx;
                return Ty(5); // Error
            }
            match name.as_str() {
                "void" => Ty(4),
                "int" => Ty(0),
                "float" => Ty(1),
                "bool" => Ty(2),
                "string" => Ty(3),
                _ => {
                    // Named user type — look up DefId to construct Struct/Entity/Enum Ty.
                    // These Ty values may not be pre-interned; use Error as safe fallback.
                    let _ = def_map;
                    Ty(5) // Error
                }
            }
        }
        crate::ast::types::AstType::Void { .. } => Ty(4),
        // Array, Generic, Func — not pre-interned; fall back to Error
        _ => Ty(5),
    }
}

/// Encode an AST type as a blob heap entry for FieldDef/ParamDef type signatures.
pub(super) fn encode_type_from_ast(
    ast_type: &crate::ast::types::AstType,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut buf = Vec::new();
    encode_ast_type_into(ast_type, generics, &mut buf);
    builder.blob_heap.intern(&buf)
}

pub(super) fn encode_ast_type_into(
    ast_type: &crate::ast::types::AstType,
    generics: &[String],
    buf: &mut Vec<u8>,
) {
    match ast_type {
        crate::ast::types::AstType::Named { name, .. } => {
            // Check generic param
            if let Some(idx) = generics.iter().position(|g| g == name) {
                buf.push(0x12);
                buf.extend_from_slice(&(idx as u16).to_le_bytes());
                return;
            }
            match name.as_str() {
                "void" => buf.push(0x00),
                "int" => buf.push(0x01),
                "float" => buf.push(0x02),
                "bool" => buf.push(0x03),
                "string" => buf.push(0x04),
                _ => {
                    // Named type — encode as TypeDef reference (0x10).
                    // Row index will be resolved during finalize.
                    buf.push(0x10);
                    buf.extend_from_slice(&0u32.to_le_bytes()); // placeholder
                }
            }
        }
        crate::ast::types::AstType::Generic { name, args, .. } => {
            match name.as_str() {
                "Option" | "Result" | "TaskHandle" => {
                    // writ-runtime generic type: TypeSpec reference
                    buf.push(0x11);
                    buf.extend_from_slice(&0u32.to_le_bytes()); // placeholder
                }
                "Array" => {
                    buf.push(0x20);
                    if let Some(inner) = args.first() {
                        encode_ast_type_into(inner, generics, buf);
                    }
                }
                _ => {
                    buf.push(0x11);
                    buf.extend_from_slice(&0u32.to_le_bytes()); // placeholder TypeSpec
                }
            }
        }
        crate::ast::types::AstType::Array { elem, .. } => {
            buf.push(0x20);
            encode_ast_type_into(elem, generics, buf);
        }
        crate::ast::types::AstType::Func { params: _, ret: _, .. } => {
            buf.push(0x30);
            // Inline the signature for now (blob offset will be computed).
            buf.extend_from_slice(&0u32.to_le_bytes()); // placeholder blob offset
        }
        crate::ast::types::AstType::Void { .. } => {
            buf.push(0x00);
        }
    }
}

/// Encode an empty method signature (void -> void).
pub(super) fn encode_empty_sig(builder: &mut ModuleBuilder) -> u32 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_le_bytes()); // 0 params
    buf.push(0x00); // void return
    builder.blob_heap.intern(&buf)
}

/// Encode a function signature from an AstFnDecl.
pub(super) fn encode_fn_sig(
    fn_decl: &crate::ast::decl::AstFnDecl,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> (u32, Vec<u32>) {
    let mut sig_buf = Vec::new();
    let mut param_blobs = Vec::new();

    // Count regular params (excluding self).
    let regular_params: Vec<&AstParam> = fn_decl
        .params
        .iter()
        .filter_map(|p| match p {
            AstFnParam::Regular(r) => Some(r),
            _ => None,
        })
        .collect();

    sig_buf.extend_from_slice(&(regular_params.len() as u16).to_le_bytes());

    for param in &regular_params {
        encode_ast_type_into(&param.ty, generics, &mut sig_buf);
        // Also encode each param type for ParamDef
        let mut param_buf = Vec::new();
        encode_ast_type_into(&param.ty, generics, &mut param_buf);
        param_blobs.push(builder.blob_heap.intern(&param_buf));
    }

    // Return type
    match &fn_decl.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, &mut sig_buf),
        None => sig_buf.push(0x00), // void
    }

    let blob = builder.blob_heap.intern(&sig_buf);
    (blob, param_blobs)
}

/// Encode a function signature from an AstFnSig (contract method / extern fn).
pub(super) fn encode_fn_sig_from_ast_sig(
    sig: &crate::ast::decl::AstFnSig,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut sig_buf = Vec::new();
    let regular_params: Vec<&AstParam> = sig
        .params
        .iter()
        .filter_map(|p| match p {
            AstFnParam::Regular(r) => Some(r),
            _ => None,
        })
        .collect();

    sig_buf.extend_from_slice(&(regular_params.len() as u16).to_le_bytes());
    for param in &regular_params {
        encode_ast_type_into(&param.ty, generics, &mut sig_buf);
    }
    match &sig.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, &mut sig_buf),
        None => sig_buf.push(0x00),
    }

    builder.blob_heap.intern(&sig_buf)
}

/// Encode an operator signature.
pub(super) fn encode_op_sig(
    op_sig: &crate::ast::decl::AstOpSig,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut sig_buf = Vec::new();
    sig_buf.extend_from_slice(&(op_sig.params.len() as u16).to_le_bytes());
    for param in &op_sig.params {
        encode_ast_type_into(&param.ty, generics, &mut sig_buf);
    }
    match &op_sig.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, &mut sig_buf),
        None => sig_buf.push(0x00),
    }
    builder.blob_heap.intern(&sig_buf)
}

/// Encode a hook method signature.
pub(super) fn encode_hook_sig(
    fn_decl: &crate::ast::decl::AstFnDecl,
    interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let (sig_blob, _) = encode_fn_sig(fn_decl, interner, generics, builder);
    sig_blob
}

/// Emit ParamDef rows for a function's parameters.
pub(super) fn emit_fn_params(
    fn_decl: &crate::ast::decl::AstFnDecl,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
    method_handle: MethodDefHandle,
) {
    let mut seq: u16 = 0;
    for param in &fn_decl.params {
        if let AstFnParam::Regular(p) = param {
            let mut buf = Vec::new();
            encode_ast_type_into(&p.ty, generics, &mut buf);
            let type_blob = builder.blob_heap.intern(&buf);
            builder.add_paramdef(method_handle, &p.name, type_blob, seq);
            seq += 1;
        }
    }
}

/// Emit GenericParam rows for a typedef's generics.
pub(super) fn emit_generics_for_typedef(
    _def_id: DefId,
    generics: &[String],
    handle: TypeDefHandle,
    builder: &mut ModuleBuilder,
) {
    for (i, g) in generics.iter().enumerate() {
        builder.add_generic_param(TableId::TypeDef, handle.0, i as u16, g);
    }
}

// =============================================================================
// Helper: resolve type handle from AstType
// =============================================================================

pub(super) fn resolve_type_handle(
    ast_type: &crate::ast::types::AstType,
    def_map: &DefMap,
    typedef_handles: &FxHashMap<DefId, TypeDefHandle>,
) -> Option<TypeDefHandle> {
    // Handle both `impl Foo` (Named) and `impl<T> Foo<T>` (Generic).
    let name = match ast_type {
        crate::ast::types::AstType::Named { name, .. } => name.as_str(),
        crate::ast::types::AstType::Generic { name, .. } => name.as_str(),
        _ => return None,
    };
    let def_id = def_map.get(name)?;
    typedef_handles.get(&def_id).copied()
}
