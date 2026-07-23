//! Function and component definition collection.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::Ast;
use crate::ast::decl::{AstComponentMember, AstFnParam, AstVisibility};
use crate::ast::types::AstType;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::metadata::{HookKind, TableId, TypeDefKind, field_flags, method_flags};
use crate::emit::module_builder::{MethodDefHandle, ModuleBuilder, TypeDefHandle};

use super::encoding::{
    ast_type_to_ty_simple, emit_fn_params, encode_fn_sig, encode_fn_sig_from_ast_sig,
    encode_type_from_ast, method_param_register_count,
};
use super::lookup::{find_component_decl, find_extern_fn_sig, find_fn_decl};

pub(super) fn collect_fn(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    methoddef_handles: &mut FxHashMap<DefId, MethodDefHandle>,
    diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(fn_decl) = find_fn_decl(asts, entry) {
        let (sig_blob, _param_types) =
            encode_fn_sig(fn_decl, interner, &entry.generics, def_map, builder);
        let mut flags = method_flags(is_pub, true, false, HookKind::None);
        if def_map.dialogue_defs.contains(&def_id) {
            flags |= writ_module::tables::METHOD_FLAG_DIALOGUE;
        }

        // Free functions have no self, so every source parameter is a regular
        // parameter register.
        let param_count = method_param_register_count(fn_decl);

        let method_handle = builder.add_methoddef(
            None,
            &entry.name,
            sig_blob,
            flags,
            Some(def_id),
            param_count,
        );
        methoddef_handles.insert(def_id, method_handle);

        // ParamDef for each parameter.
        emit_fn_params(
            fn_decl,
            interner,
            &entry.generics,
            def_map,
            builder,
            method_handle,
        );

        // Populate fn_param_map: (name, Ty) list in declaration order, excluding self.
        let fn_params: Vec<(String, crate::check::ty::Ty)> = fn_decl
            .params
            .iter()
            .filter_map(|p| {
                if let AstFnParam::Regular(p) = p {
                    let ty = ast_type_to_ty_simple(&p.ty, &entry.generics, def_map, interner)
                        .unwrap_or_else(|message| {
                            diags.push(
                                Diagnostic::error("E2002", message)
                                    .with_primary(
                                        entry.file_id,
                                        p.name_span,
                                        "failed to recover checked parameter type",
                                    )
                                    .build(),
                            );
                            crate::check::ty::Ty(5)
                        });
                    Some((p.name.clone(), ty))
                } else {
                    None
                }
            })
            .collect();
        builder.fn_param_map.insert(def_id, fn_params);

        // GenericParam + GenericConstraint
        for (i, (g, ast_gp)) in entry
            .generics
            .iter()
            .zip(fn_decl.generics.iter())
            .enumerate()
        {
            let param_idx =
                builder.add_generic_param(TableId::MethodDef, method_handle.0, i as u16, g);
            // Emit GenericConstraint rows for each bound on this param.
            for bound_ast_ty in &ast_gp.bounds {
                if let AstType::Named { name, .. } = bound_ast_ty {
                    if let Some(contract_def_id) = def_map.get(name) {
                        builder.add_generic_constraint(param_idx, contract_def_id);
                    }
                }
            }
        }
    }
}

pub(super) fn collect_extern_fn(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(sig) = find_extern_fn_sig(asts, entry) {
        let sig_blob = encode_fn_sig_from_ast_sig(sig, interner, &entry.generics, def_map, builder);

        // Build import name: qualifier.name if present, else just name.
        let import_name = if let Some(ref q) = sig.qualifier {
            format!("{}.{}", q, entry.name)
        } else {
            entry.name.clone()
        };

        let flags: u16 = if is_pub { 1 } else { 0 };
        builder.add_extern_def(&entry.name, sig_blob, &import_name, flags, Some(def_id));
    }
}

pub(super) fn collect_component(
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
                let has_default = f.default.is_some();
                let flags = field_flags(is_field_pub, has_default, true, f.is_mutable);
                let type_blob =
                    encode_type_from_ast(&f.ty, interner, &entry.generics, def_map, builder);
                builder.add_fielddef(handle, &f.name, type_blob, flags);
            }
        }
    }
}
