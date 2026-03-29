//! Contract, impl, and extern component collection.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::{AstContractMember, AstFnParam, AstComponentMember, AstVisibility};
use crate::ast::Ast;
use crate::check::ir::TypedExpr;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::metadata::{TypeDefKind, HookKind, MetadataToken, TableId, field_flags, method_flags};
use crate::emit::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle, ContractDefHandle, ImplDefHandle};

use super::encoding::{
    encode_fn_sig, encode_fn_sig_from_ast_sig, encode_op_sig, encode_type_from_ast,
    emit_fn_params, resolve_type_handle, ast_type_to_ty_simple,
};
use super::lookup::{
    find_contract_decl, find_impl_decl,
    find_component_decl,
};

pub(super) fn collect_contract(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) -> super::ContractDefHandle {
    let entry = def_map.get_entry(def_id);

    let contract_handle = builder.add_contract_def(&entry.name, &entry.namespace, Some(def_id));

    if let Some(contract_decl) = find_contract_decl(asts, entry) {
        // ContractMethod for each method signature (slot assigned later by slots.rs).
        for member in &contract_decl.members {
            match member {
                AstContractMember::FnSig(sig) => {
                    let sig_blob = encode_fn_sig_from_ast_sig(sig, interner, &entry.generics, builder);
                    builder.add_contract_method(contract_handle, &sig.name, sig_blob, 0);
                }
                AstContractMember::OpSig(op_sig) => {
                    let sig_blob = encode_op_sig(op_sig, interner, &entry.generics, builder);
                    let name = format!("operator_{:?}", op_sig.symbol);
                    builder.add_contract_method(contract_handle, &name, sig_blob, 0);
                }
            }
        }

        // GenericParam for contract type params.
        for (i, g) in entry.generics.iter().enumerate() {
            builder.add_generic_param(TableId::ContractDef, contract_handle.0, i as u16, g);
        }
    }

    contract_handle
}

#[allow(clippy::too_many_arguments)] // impl collection requires full emit context (DefMap, builder, handles)
pub(super) fn collect_impl(
    impl_def_id: DefId,
    methods: &[(DefId, TypedExpr)],
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    typedef_handles: &FxHashMap<DefId, TypeDefHandle>,
    contractdef_handles: &FxHashMap<DefId, ContractDefHandle>,
    methoddef_handles: &mut FxHashMap<DefId, MethodDefHandle>,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(impl_def_id);

    // Find matching AST impl decl.
    if let Some(impl_decl) = find_impl_decl(asts, entry) {
        // Resolve target type.
        let target_type_handle = resolve_type_handle(&impl_decl.target, def_map, typedef_handles);

        // Resolve contract (if any).
        let contract_def_id = impl_decl.contract.as_ref().and_then(|c| {
            if let crate::ast::types::AstType::Named { name, .. } = c {
                def_map.get(name)
            } else {
                None
            }
        });
        // Capture contract name for prelude contract TypeRef fallback.
        let contract_name: Option<&str> = impl_decl.contract.as_ref().and_then(|c| {
            if let crate::ast::types::AstType::Named { name, .. } = c {
                Some(name.as_str())
            } else {
                None
            }
        });

        // Emit MethodDefs for each impl method under the target type's TypeDef.
        //
        // NOTE: check_decl.rs stores (impl_def_id, body) for every method — all methods share
        // the same DefId. Because of this, `method_entry.name` resolves to "impl#N" (the impl
        // block name), not the method name. We recover the correct method name by iterating the
        // AST impl_decl.members in the same order as the typed `methods` vec.
        let ast_fn_decls: Vec<&crate::ast::decl::AstFnDecl> = impl_decl.members
            .iter()
            .filter_map(|m| if let crate::ast::decl::AstImplMember::Fn(f) = m { Some(f) } else { None })
            .collect();

        let impl_entry_generics = def_map.get_entry(impl_def_id).generics.clone();
        let impl_is_pub = matches!(def_map.get_entry(impl_def_id).vis, DefVis::Pub);

        for (method_idx, (_method_def_id, _body)) in methods.iter().enumerate() {
            let fn_decl = match ast_fn_decls.get(method_idx) {
                Some(f) => f,
                None => continue,
            };

            let is_pub = impl_is_pub;

            let (sig_blob, _) = encode_fn_sig(fn_decl, interner, &impl_entry_generics, builder);

            let has_self = fn_decl.params.iter().any(|p| matches!(p, AstFnParam::SelfParam { .. }));
            let is_mut_self = fn_decl.params.iter().any(|p| {
                matches!(p, AstFnParam::SelfParam { mutable: true, .. })
            });

            let flags = method_flags(is_pub, !has_self, is_mut_self, HookKind::None);

            // param_count = number of ParamDef rows emitted (regular params only).
            // Self occupies r0 but has no ParamDef row; it is not counted here.
            let regular_param_count = fn_decl.params.iter().filter(|p| matches!(p, AstFnParam::Regular(_))).count() as u16;
            let param_count = regular_param_count;

            // Use impl_def_id as the method's def_id so the body emitter can find it via
            // token_for_def. When there are multiple methods, each gets its own MethodDefHandle
            // but they share the impl_def_id key — the last one wins in def_token_map.
            // The body emitter accesses methods by their MethodDefHandle stored in
            // methoddef_handles, which IS indexed by method_idx uniqueness via handle.
            let method_handle = builder.add_methoddef(
                target_type_handle,
                &fn_decl.name,
                sig_blob,
                flags,
                Some(*_method_def_id),
                param_count,
            );
            methoddef_handles.insert(*_method_def_id, method_handle);

            // ParamDef
            emit_fn_params(fn_decl, interner, &impl_entry_generics, builder, method_handle);

            // Populate fn_param_map: (name, Ty) list including self as first entry.
            // Self occupies r0 by convention (SelfRef always returns r0), so it must
            // appear first so the body emitter pre-allocates r0 for it.
            let mut fn_params: Vec<(String, crate::check::ty::Ty)> = Vec::new();
            if has_self {
                let self_ty = ast_type_to_ty_simple(&impl_decl.target, &impl_entry_generics, def_map);
                fn_params.push(("self".to_string(), self_ty));
            }
            fn_params.extend(fn_decl.params.iter().filter_map(|p| {
                if let AstFnParam::Regular(p) = p {
                    let ty = ast_type_to_ty_simple(&p.ty, &impl_entry_generics, def_map);
                    Some((p.name.clone(), ty))
                } else {
                    None
                }
            }));
            builder.fn_param_map.insert(*_method_def_id, fn_params.clone());
            // Also store by MethodDefHandle for unambiguous per-method lookup
            // (all impl methods share the same DefId — fn_param_map gets overwritten).
            builder.impl_method_param_map.insert(method_handle.0, fn_params);

            // GenericParam for method generics.
            // (Use impl-level generics as a proxy; per-method generics require DefId resolution)
        }

        // ImplDef row linking type to contract.
        let type_token = target_type_handle
            .map(|h| MetadataToken::new(TableId::TypeDef, (h.0 + 1) as u32))
            .unwrap_or(MetadataToken::NULL);
        // Resolve contract token:
        // - User-defined contracts: use ContractDefHandle from contractdef_handles (available
        //   before finalize since contracts are collected before impls in TypedDecl order).
        // - Cross-module contracts (e.g. Add, Eq from writ-runtime): fall back to token_for_def
        //   which resolves TypeRef tokens registered during module refs setup.
        // - Prelude contracts Iterable and Iterator have no user-module DefId but their
        //   writ-runtime ContractDef rows are spec-locked at 14 and 15 (1-based). Use those
        //   hardcoded tokens so the dispatch table type_args_hash matches CALL_VIRT.
        let contract_token = contract_def_id
            .and_then(|id| {
                contractdef_handles.get(&id).map(|h| {
                    MetadataToken::new(TableId::ContractDef, (h.0 + 1) as u32)
                }).or_else(|| builder.token_for_def(id))
            })
            .or_else(|| {
                // Prelude contract fallback: Iterable (row 14) and Iterator (row 15) are
                // spec-locked positions in the writ-runtime virtual module contract table.
                match contract_name {
                    Some("Iterable") => Some(ITERABLE_CONTRACT_TOKEN),
                    Some("Iterator") => Some(ITERATOR_CONTRACT_TOKEN),
                    _ => None,
                }
            })
            .unwrap_or(MetadataToken::NULL);

        // method_list will be set during finalize to point to the impl's methods.
        builder.add_impl_def(type_token, contract_token, 0, Some(impl_def_id));
    }
}

// =============================================================================
// Reflectable auto-impl emission
// =============================================================================

/// The Iterable<T> contract token in the writ-runtime virtual module.
///
/// Iterable is ContractDef at 0-based index 13, 1-based row 14.
/// TableId::ContractDef = 10. This value is spec-locked (virtual_module.rs order).
///
/// MetadataToken bit layout: bits 31-24 = table_id, bits 23-0 = row (1-based).
pub(crate) const ITERABLE_CONTRACT_TOKEN: MetadataToken =
    MetadataToken((10u32 << 24) | 14u32);

/// The Iterator<T> contract token in the writ-runtime virtual module.
///
/// Iterator is ContractDef at 0-based index 14, 1-based row 15.
/// TableId::ContractDef = 10. This value is spec-locked (virtual_module.rs order).
///
/// MetadataToken bit layout: bits 31-24 = table_id, bits 23-0 = row (1-based).
pub(crate) const ITERATOR_CONTRACT_TOKEN: MetadataToken =
    MetadataToken((10u32 << 24) | 15u32);

/// The Reflectable contract token in the writ-runtime virtual module.
///
/// Reflectable is ContractDef at 0-based index 18, 1-based row 19.
/// TableId::ContractDef = 10. This value is spec-locked.
///
/// MetadataToken bit layout: bits 31-24 = table_id, bits 23-0 = row (1-based).
pub(super) const REFLECTABLE_CONTRACT_TOKEN: MetadataToken =
    MetadataToken((10u32 << 24) | 19u32);

/// Emit a synthetic Reflectable ImplDef + get_type() MethodDef for a user-defined type.
///
/// Called immediately after each collect_struct/class/entity/enum to satisfy COMP-03.
/// The MethodDef is parented to the TypeDef so finalize() groups it correctly.
/// The body (TYPEOF + RET) is emitted separately in emit_all_bodies.
///
/// Returns `(MethodDefHandle, ImplDefHandle)` so the caller can:
/// 1. Track the MethodDefHandle for body emission.
/// 2. Fix up the ImplDefHandle's method_list after finalize().
pub(super) fn emit_reflectable_auto_impl(
    typedef_handle: TypeDefHandle,
    def_id: DefId,
    builder: &mut ModuleBuilder,
) -> (MethodDefHandle, ImplDefHandle) {
    // Encode sig blob for () -> Type.
    // Method sig format: u16 param_count (regular params, self not counted) + return type.
    // Type TypeRef is at 0-based index 1 (second add_type_ref call: Range=0, Type=1).
    // Encoding: [0x00, 0x00, 0x10, <token_bytes_le>]
    //   - 0x00, 0x00 = param_count = 0 (u16 LE)
    //   - 0x10 = named/reference type tag
    //   - MetadataToken::new(TypeRef, 2).0 in LE = TypeRef row 2 (1-based) for "Type"
    let type_typeref_token = builder.type_ref_token_by_name("Type");
    let mut sig_bytes: Vec<u8> = Vec::new();
    sig_bytes.extend_from_slice(&0u16.to_le_bytes()); // param_count = 0
    sig_bytes.push(0x10); // named/reference type tag
    sig_bytes.extend_from_slice(&type_typeref_token.to_le_bytes()); // TypeRef token
    let sig_blob = builder.blob_heap.intern(&sig_bytes);

    // MethodDef: pub, not static, not mut_self, no hook.
    // param_count = 0: the binary format's param_count counts ParamDef table rows for this
    // method. Self has no ParamDef row (it is implicit), so 0 regular params = 0 ParamDef rows.
    let flags = method_flags(true, false, false, HookKind::None);
    let method_handle = builder.add_methoddef(
        Some(typedef_handle), // parent = the TypeDef (critical for finalize sort)
        "get_type",
        sig_blob,
        flags,
        None,  // no DefId — synthetic method
        0,     // param_count = 0 ParamDef rows (self is implicit, no regular params)
    );

    // TypeDef token for the ImplDef.type_token field.
    let type_token = MetadataToken::new(TableId::TypeDef, (typedef_handle.0 + 1) as u32);

    // ImplDef: method_list=0 initially; will be fixed up after finalize() in emit_bodies.
    let impl_handle = builder.add_impl_def(type_token, REFLECTABLE_CONTRACT_TOKEN, 0, None);

    // Store def_id for body emission (needed to look up the finalized TypeDef token for TYPEOF).
    // We piggyback the def_id by storing in the fn_param_map with an empty params list
    // so the body emitter can locate it. Actually, we just return def_id to the caller
    // via the reflectable_infos vec — no fn_param_map needed here.
    let _ = def_id; // used by caller

    (method_handle, impl_handle)
}

pub(super) fn collect_extern_component(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    typedef_handles: &mut FxHashMap<DefId, TypeDefHandle>,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    let handle = builder.add_typedef(
        &entry.name,
        &entry.namespace,
        TypeDefKind::Component,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    if let Some(comp_decl) = find_component_decl(asts, entry) {
        for member in &comp_decl.members {
            if let AstComponentMember::Field(f) = member {
                let is_field_pub = matches!(f.vis, Some(AstVisibility::Pub));
                let flags = field_flags(is_field_pub, false, true);
                let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                builder.add_fielddef(handle, &f.name, type_blob, flags);
            }
        }
    }
}
