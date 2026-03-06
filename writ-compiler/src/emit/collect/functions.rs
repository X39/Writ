//! Function and component definition collection.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::{AstComponentMember, AstFnParam, AstVisibility};
use crate::ast::Ast;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::metadata::{TypeDefKind, HookKind, field_flags, method_flags, TableId};
use crate::emit::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle};

use super::encoding::{encode_fn_sig, encode_fn_sig_from_ast_sig, encode_type_from_ast, emit_fn_params, ast_type_to_ty_simple};
use super::lookup::{find_fn_decl, find_extern_fn_sig, find_component_decl};

pub(super) fn collect_fn(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    methoddef_handles: &mut FxHashMap<DefId, MethodDefHandle>,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(fn_decl) = find_fn_decl(asts, entry) {
        let (sig_blob, _param_types) = encode_fn_sig(fn_decl, interner, &entry.generics, builder);
        let flags = method_flags(is_pub, true, false, HookKind::None);

        // Free functions have no self; param_count = number of regular params.
        let param_count = fn_decl.params.iter().filter(|p| matches!(p, AstFnParam::Regular(_))).count() as u16;

        let method_handle = builder.add_methoddef(None, &entry.name, sig_blob, flags, Some(def_id), param_count);
        methoddef_handles.insert(def_id, method_handle);

        // ParamDef for each parameter.
        emit_fn_params(fn_decl, interner, &entry.generics, builder, method_handle);

        // Populate fn_param_map: (name, Ty) list in declaration order, excluding self.
        let fn_params: Vec<(String, crate::check::ty::Ty)> = fn_decl
            .params
            .iter()
            .filter_map(|p| {
                if let AstFnParam::Regular(p) = p {
                    let ty = ast_type_to_ty_simple(&p.ty, &entry.generics, def_map);
                    Some((p.name.clone(), ty))
                } else {
                    None
                }
            })
            .collect();
        builder.fn_param_map.insert(def_id, fn_params);

        // GenericParam
        for (i, g) in entry.generics.iter().enumerate() {
            builder.add_generic_param(TableId::MethodDef, method_handle.0, i as u16, g);
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
        let sig_blob = encode_fn_sig_from_ast_sig(sig, interner, &entry.generics, builder);

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
                let flags = field_flags(is_field_pub, has_default, true);
                let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                builder.add_fielddef(handle, &f.name, type_blob, flags);
            }
        }
    }
}
