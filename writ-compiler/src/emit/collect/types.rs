//! Type definition collection: struct, entity, enum, class, extern struct.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::{AstStructMember, AstVisibility};
use crate::ast::Ast;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::metadata::{TypeDefKind, HookKind, field_flags, method_flags};
use crate::emit::module_builder::{ModuleBuilder, TypeDefHandle};

use super::encoding::{encode_type_from_ast, encode_empty_sig, emit_generics_for_typedef, encode_hook_sig};
use super::lookup::{find_struct_decl, find_entity_decl, find_enum_decl, find_class_decl, find_extern_struct_decl};

pub(super) fn collect_struct(
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
        TypeDefKind::Struct,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    // Find AST struct and emit fields.
    if let Some(struct_decl) = find_struct_decl(asts, entry) {
        let _generic_map = super::build_generic_map(&entry.generics);
        for member in &struct_decl.members {
            match member {
                AstStructMember::Field(f) => {
                    let is_field_pub = matches!(f.vis, Some(AstVisibility::Pub));
                    let has_default = f.default.is_some();
                    let flags = field_flags(is_field_pub, has_default, false);
                    // Encode type signature as blob.
                    let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                    builder.add_fielddef(handle, &f.name, type_blob, flags);
                }
                AstStructMember::OnHook { event, body: _, span: _, .. } => {
                    let hook = HookKind::from_event_name(event);
                    let flags = method_flags(false, false, true, hook);
                    // Hook methods have no params and void return.
                    let sig_blob = encode_empty_sig(builder);
                    builder.add_methoddef(Some(handle), &format!("on_{}", event), sig_blob, flags, None, 0);
                }
            }
        }

        // Generics
        emit_generics_for_typedef(def_id, &entry.generics, handle, builder);
    }
}

pub(super) fn collect_entity(
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
        TypeDefKind::Entity,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    if let Some(entity_decl) = find_entity_decl(asts, entry) {
        // Properties -> FieldDef
        for prop in &entity_decl.properties {
            let is_field_pub = matches!(prop.vis, Some(AstVisibility::Pub));
            let has_default = prop.default.is_some();
            let flags = field_flags(is_field_pub, has_default, false);
            let type_blob = encode_type_from_ast(&prop.ty, interner, &entry.generics, builder);
            builder.add_fielddef(handle, &prop.name, type_blob, flags);
        }

        // Hooks -> MethodDef with hook_kind
        for hook in &entity_decl.hooks {
            let hook_kind = HookKind::from_event_name(&hook.contract);
            let flags = method_flags(false, false, true, hook_kind);
            let sig_blob = encode_hook_sig(&hook.method, interner, &entry.generics, builder);
            builder.add_methoddef(
                Some(handle),
                &format!("on_{}", hook.contract),
                sig_blob,
                flags,
                None,
                0, // hook methods have no params besides implicit self
            );
        }
    }
}

pub(super) fn collect_enum(
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
        TypeDefKind::Enum,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    if let Some(enum_decl) = find_enum_decl(asts, entry) {
        // Variant payload fields -> FieldDef
        for variant in &enum_decl.variants {
            if let Some(fields) = &variant.fields {
                for field in fields {
                    let type_blob = encode_type_from_ast(&field.ty, interner, &entry.generics, builder);
                    // Enum fields are implicitly pub (accessed by pattern matching).
                    let flags = field_flags(true, false, false);
                    builder.add_fielddef(handle, &field.name, type_blob, flags);
                }
            }
        }

        emit_generics_for_typedef(def_id, &entry.generics, handle, builder);
    }
}

pub(super) fn collect_class(
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

    if let Some(class_decl) = find_class_decl(asts, entry) {
        let _generic_map = super::build_generic_map(&entry.generics);
        for member in &class_decl.members {
            match member {
                AstStructMember::Field(f) => {
                    let is_field_pub = matches!(f.vis, Some(AstVisibility::Pub));
                    let has_default = f.default.is_some();
                    let flags = field_flags(is_field_pub, has_default, false);
                    let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                    builder.add_fielddef(handle, &f.name, type_blob, flags);
                }
                AstStructMember::OnHook { event, body: _, span: _, .. } => {
                    let hook = HookKind::from_event_name(event);
                    let flags = method_flags(false, false, true, hook);
                    let sig_blob = encode_empty_sig(builder);
                    builder.add_methoddef(Some(handle), &format!("on_{}", event), sig_blob, flags, None, 0);
                }
            }
        }

        emit_generics_for_typedef(def_id, &entry.generics, handle, builder);
    }
}

pub(super) fn collect_extern_struct(
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
        TypeDefKind::Struct,
        if is_pub { 1 } else { 0 },
        Some(def_id),
    );
    typedef_handles.insert(def_id, handle);

    if let Some(struct_decl) = find_extern_struct_decl(asts, entry) {
        for member in &struct_decl.members {
            if let AstStructMember::Field(f) = member {
                let is_field_pub = matches!(f.vis, Some(AstVisibility::Pub));
                let flags = field_flags(is_field_pub, false, false);
                let type_blob = encode_type_from_ast(&f.ty, interner, &entry.generics, builder);
                builder.add_fielddef(handle, &f.name, type_blob, flags);
            }
        }
    }
}
