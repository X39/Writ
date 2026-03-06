//! Contract, impl, extern class, and extern component collection.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::{AstContractMember, AstFnParam, AstStructMember, AstComponentMember, AstVisibility};
use crate::ast::Ast;
use crate::check::ir::TypedExpr;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::metadata::{TypeDefKind, HookKind, MetadataToken, TableId, field_flags, method_flags};
use crate::emit::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle};

use super::encoding::{
    encode_fn_sig, encode_fn_sig_from_ast_sig, encode_op_sig, encode_type_from_ast,
    emit_fn_params, resolve_type_handle, ast_type_to_ty_simple,
};
use super::lookup::{
    find_contract_decl, find_impl_decl, find_extern_class_decl,
    find_component_decl,
};

pub(super) fn collect_contract(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
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

            // param_count = regular params + 1 if has_self (self occupies r0)
            let regular_param_count = fn_decl.params.iter().filter(|p| matches!(p, AstFnParam::Regular(_))).count() as u16;
            let param_count = regular_param_count + if has_self { 1 } else { 0 };

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

            // Populate fn_param_map: (name, Ty) list excluding self.
            let fn_params: Vec<(String, crate::check::ty::Ty)> = fn_decl
                .params
                .iter()
                .filter_map(|p| {
                    if let AstFnParam::Regular(p) = p {
                        let ty = ast_type_to_ty_simple(&p.ty, &impl_entry_generics, def_map);
                        Some((p.name.clone(), ty))
                    } else {
                        None
                    }
                })
                .collect();
            builder.fn_param_map.insert(*_method_def_id, fn_params);

            // GenericParam for method generics.
            // (Use impl-level generics as a proxy; per-method generics require DefId resolution)
        }

        // ImplDef row linking type to contract.
        let type_token = target_type_handle
            .map(|h| MetadataToken::new(TableId::TypeDef, (h.0 + 1) as u32))
            .unwrap_or(MetadataToken::NULL);
        let contract_token = contract_def_id
            .and_then(|id| builder.token_for_def(id))
            .unwrap_or(MetadataToken::NULL);

        // method_list will be set during finalize to point to the impl's methods.
        builder.add_impl_def(type_token, contract_token, 0, Some(impl_def_id));
    }
}

pub(super) fn collect_extern_class(
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
        TypeDefKind::Class,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    if let Some(class_decl) = find_extern_class_decl(asts, entry) {
        for member in &class_decl.members {
            if let AstStructMember::Field(f) = member {
                let is_field_pub = matches!(f.vis, Some(AstVisibility::Pub));
                let flags = field_flags(is_field_pub, false, false);
                let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                builder.add_fielddef(handle, &f.name, type_blob, flags);
            }
        }
    }
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
