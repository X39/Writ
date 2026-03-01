//! Collection pass: walk TypedAst + DefMap + original ASTs, populate ModuleBuilder.
//!
//! This mirrors the pattern in `check/env.rs` — we need the original ASTs for
//! field/param/hook details that TypedDecl doesn't carry.

use rustc_hash::FxHashMap;
use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::decl::*;
use crate::ast::expr::AstExpr;
use crate::ast::Ast;
use crate::check::ir::{TypedAst, TypedDecl};
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};

use super::metadata::*;
use super::module_builder::{ModuleBuilder, TypeDefHandle, MethodDefHandle};

/// Collect all definitions from the TypedAst into the ModuleBuilder.
pub fn collect_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    let def_map = &typed_ast.def_map;

    // 1. ModuleDef: always exactly 1 row.
    let module_name = find_module_name(def_map);
    builder.set_module_def(&module_name, "0.1.0", 0);

    // 2. ModuleRef: always emit writ-runtime.
    builder.add_module_ref("writ-runtime", "1.0.0");

    // 3. Walk TypedDecl list and emit rows.
    // We need to track TypeDefHandles for linking children.
    let mut typedef_handles: FxHashMap<DefId, TypeDefHandle> = FxHashMap::default();
    let mut methoddef_handles: FxHashMap<DefId, MethodDefHandle> = FxHashMap::default();

    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Struct { def_id } => {
                collect_struct(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Entity { def_id } => {
                collect_entity(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Enum { def_id } => {
                collect_enum(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Fn { def_id, .. } => {
                collect_fn(*def_id, def_map, asts, interner, builder, &mut methoddef_handles, diags);
            }
            TypedDecl::Contract { def_id } => {
                collect_contract(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::Impl { def_id, methods } => {
                collect_impl(
                    *def_id,
                    methods,
                    def_map,
                    asts,
                    interner,
                    builder,
                    &typedef_handles,
                    &mut methoddef_handles,
                    diags,
                );
            }
            TypedDecl::Component { def_id } => {
                collect_component(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::ExternFn { def_id } => {
                collect_extern_fn(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::ExternStruct { def_id } => {
                collect_extern_struct(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::ExternComponent { def_id } => {
                collect_extern_component(*def_id, def_map, asts, interner, builder, &mut typedef_handles, diags);
            }
            TypedDecl::Const { def_id, .. } => {
                collect_const(*def_id, def_map, asts, interner, builder, diags);
            }
            TypedDecl::Global { def_id, .. } => {
                collect_global(*def_id, def_map, asts, interner, builder, diags);
            }
        }
    }

    // 4. Component slots: walk entity decls for component slots.
    collect_component_slots(typed_ast, asts, def_map, builder, &typedef_handles);

    // 5. LocaleDef: collected in collect_post_finalize() after token assignment.

    // Note: ExportDef and AttributeDef are collected in collect_post_finalize()
    // after token assignment, because they depend on resolved MetadataTokens.
}

/// Collect exports and attributes that depend on finalized tokens.
///
/// Must be called after `builder.finalize()`.
pub fn collect_post_finalize(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    builder: &mut ModuleBuilder,
) {
    let def_map = &typed_ast.def_map;

    // ExportDef: walk DefMap.by_fqn for all pub-visible items.
    collect_exports(def_map, builder);

    // Attributes: walk all decls and emit AttributeDef rows.
    collect_attributes(typed_ast, asts, builder);

    // LocaleDef: walk all Fn decls for [Locale("tag")] attribute overrides.
    collect_locale_defs(typed_ast, asts, builder);
}

// =============================================================================
// Module name
// =============================================================================

fn find_module_name(def_map: &DefMap) -> String {
    // Use the first namespace found, or "main".
    for entry in def_map.arena.iter() {
        if !entry.1.namespace.is_empty() {
            // Return the root namespace segment.
            let ns = &entry.1.namespace;
            if let Some(root) = ns.split("::").next() {
                return root.to_string();
            }
        }
    }
    "main".to_string()
}

// =============================================================================
// Type signature encoding helper
// =============================================================================

/// Build a generic param name-to-index map.
fn build_generic_map(generics: &[String]) -> FxHashMap<String, u32> {
    generics
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect()
}

// =============================================================================
// Struct collection
// =============================================================================

fn collect_struct(
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
        let _generic_map = build_generic_map(&entry.generics);
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

// =============================================================================
// Entity collection
// =============================================================================

fn collect_entity(
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

// =============================================================================
// Enum collection
// =============================================================================

fn collect_enum(
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

// =============================================================================
// Function collection
// =============================================================================

fn collect_fn(
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

// =============================================================================
// Contract collection
// =============================================================================

fn collect_contract(
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

// =============================================================================
// Impl collection
// =============================================================================

fn collect_impl(
    impl_def_id: DefId,
    methods: &[(DefId, crate::check::ir::TypedExpr)],
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
        for (method_def_id, _body) in methods {
            let method_entry = def_map.get_entry(*method_def_id);
            let is_pub = matches!(method_entry.vis, DefVis::Pub);

            // Find the method's AST FnDecl in the impl block.
            if let Some(fn_decl) = find_method_in_impl(impl_decl, &method_entry.name) {
                let (sig_blob, _) = encode_fn_sig(fn_decl, interner, &method_entry.generics, builder);

                let has_self = fn_decl.params.iter().any(|p| matches!(p, AstFnParam::SelfParam { .. }));
                let is_mut_self = fn_decl.params.iter().any(|p| {
                    matches!(p, AstFnParam::SelfParam { mutable: true, .. })
                });

                let flags = method_flags(is_pub, !has_self, is_mut_self, HookKind::None);

                // param_count = regular params + 1 if has_self (self occupies r0)
                let regular_param_count = fn_decl.params.iter().filter(|p| matches!(p, AstFnParam::Regular(_))).count() as u16;
                let param_count = regular_param_count + if has_self { 1 } else { 0 };

                let method_handle = builder.add_methoddef(
                    target_type_handle,
                    &method_entry.name,
                    sig_blob,
                    flags,
                    Some(*method_def_id),
                    param_count,
                );
                methoddef_handles.insert(*method_def_id, method_handle);

                // ParamDef
                emit_fn_params(fn_decl, interner, &method_entry.generics, builder, method_handle);

                // Populate fn_param_map: (name, Ty) list excluding self.
                let fn_params: Vec<(String, crate::check::ty::Ty)> = fn_decl
                    .params
                    .iter()
                    .filter_map(|p| {
                        if let AstFnParam::Regular(p) = p {
                            let ty = ast_type_to_ty_simple(&p.ty, &method_entry.generics, def_map);
                            Some((p.name.clone(), ty))
                        } else {
                            None
                        }
                    })
                    .collect();
                builder.fn_param_map.insert(*method_def_id, fn_params);

                // GenericParam for method generics.
                for (i, g) in method_entry.generics.iter().enumerate() {
                    builder.add_generic_param(TableId::MethodDef, method_handle.0, i as u16, g);
                }
            }
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

// =============================================================================
// Component collection
// =============================================================================

fn collect_component(
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

// =============================================================================
// Extern fn collection
// =============================================================================

fn collect_extern_fn(
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

// =============================================================================
// Extern struct collection
// =============================================================================

fn collect_extern_struct(
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

// =============================================================================
// Extern component collection
// =============================================================================

fn collect_extern_component(
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

// =============================================================================
// Const/Global collection
// =============================================================================

fn collect_const(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(const_decl) = find_const_decl(asts, entry) {
        let type_blob = encode_type_from_ast(&const_decl.ty, interner, &entry.generics, builder);
        // Flags: bit 0 = pub, bit 1 = is_const
        let flags: u16 = (if is_pub { 1 } else { 0 }) | (1 << 1);
        builder.add_global_def(&entry.name, type_blob, flags, 0, Some(def_id));
    }
}

fn collect_global(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(global_decl) = find_global_decl(asts, entry) {
        let type_blob = encode_type_from_ast(&global_decl.ty, interner, &entry.generics, builder);
        // Flags: bit 0 = pub, bit 2 = is_mutable
        let flags: u16 = (if is_pub { 1 } else { 0 }) | (1 << 2);
        builder.add_global_def(&entry.name, type_blob, flags, 0, Some(def_id));
    }
}

// =============================================================================
// Export collection
// =============================================================================

fn collect_exports(def_map: &DefMap, builder: &mut ModuleBuilder) {
    for (_fqn_str, &def_id) in &def_map.by_fqn {
        let entry = def_map.get_entry(def_id);
        if !matches!(entry.vis, DefVis::Pub) {
            continue;
        }

        // Determine item_kind and get token.
        // item_kind encoding (matches disassembler and cmd_run):
        //   0 = method (Fn, ExternFn)
        //   1 = type   (Struct, Entity, Enum, Component, Contract, ExternStruct, ExternComponent)
        //   2 = global (Const, Global)
        if let Some(token) = builder.token_for_def(def_id) {
            let item_kind = match entry.kind {
                DefKind::Fn | DefKind::ExternFn => 0, // method
                DefKind::Struct | DefKind::Entity | DefKind::Enum | DefKind::Component
                | DefKind::ExternStruct | DefKind::ExternComponent | DefKind::Contract => 1, // type
                DefKind::Const | DefKind::Global => 2, // global
                DefKind::Impl => continue, // impls aren't exported directly
            };
            builder.add_export_def(&entry.name, item_kind, token);
        }
    }
}

// =============================================================================
// Attribute collection
// =============================================================================

fn collect_attributes(typed_ast: &TypedAst, asts: &[(FileId, &Ast)], builder: &mut ModuleBuilder) {
    let def_map = &typed_ast.def_map;

    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::Struct { def_id }
            | TypedDecl::Entity { def_id }
            | TypedDecl::Enum { def_id }
            | TypedDecl::Contract { def_id }
            | TypedDecl::Component { def_id }
            | TypedDecl::ExternStruct { def_id }
            | TypedDecl::ExternComponent { def_id } => *def_id,
            TypedDecl::Fn { def_id, .. }
            | TypedDecl::ExternFn { def_id }
            | TypedDecl::Const { def_id, .. }
            | TypedDecl::Global { def_id, .. } => *def_id,
            TypedDecl::Impl { .. } => continue,
        };

        let entry = def_map.get_entry(def_id);

        // Find the matching AST decl's attributes.
        let attrs = find_attrs_for_entry(asts, entry);
        if attrs.is_empty() {
            continue;
        }

        let owner_token = builder.token_for_def(def_id).unwrap_or(MetadataToken::NULL);
        let owner_kind: u8 = match entry.kind {
            DefKind::Struct | DefKind::Entity | DefKind::Enum | DefKind::Component
            | DefKind::Contract | DefKind::ExternStruct | DefKind::ExternComponent => 0, // type
            DefKind::Fn | DefKind::ExternFn => 1, // method
            _ => 2, // field/global
        };

        for attr in &attrs {
            // Value blob: empty for now (args encoding deferred).
            builder.add_attribute_def(owner_token, owner_kind, &attr.name, 0);
        }
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
fn collect_locale_defs(typed_ast: &TypedAst, asts: &[(FileId, &Ast)], builder: &mut ModuleBuilder) {
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

fn find_attrs_for_entry(asts: &[(FileId, &Ast)], entry: &DefEntry) -> Vec<AstAttribute> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            match decl {
                AstDecl::Struct(s) if s.name == entry.name && s.name_span == entry.name_span => {
                    return s.attrs.clone();
                }
                AstDecl::Entity(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Enum(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Contract(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Component(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Fn(f) if f.name == entry.name && f.name_span == entry.name_span => {
                    return f.attrs.clone();
                }
                AstDecl::Const(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Global(g) if g.name == entry.name && g.name_span == entry.name_span => {
                    return g.attrs.clone();
                }
                _ => {}
            }
        }
    }
    Vec::new()
}

// =============================================================================
// Component slot collection
// =============================================================================

fn collect_component_slots(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
    typedef_handles: &FxHashMap<DefId, TypeDefHandle>,
) {
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
fn ast_type_to_ty_simple(
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
fn encode_type_from_ast(
    ast_type: &crate::ast::types::AstType,
    _interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut buf = Vec::new();
    encode_ast_type_into(ast_type, generics, &mut buf);
    builder.blob_heap.intern(&buf)
}

fn encode_ast_type_into(
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
fn encode_empty_sig(builder: &mut ModuleBuilder) -> u32 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_le_bytes()); // 0 params
    buf.push(0x00); // void return
    builder.blob_heap.intern(&buf)
}

/// Encode a function signature from an AstFnDecl.
fn encode_fn_sig(
    fn_decl: &AstFnDecl,
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
fn encode_fn_sig_from_ast_sig(
    sig: &AstFnSig,
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
fn encode_op_sig(
    op_sig: &AstOpSig,
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
fn encode_hook_sig(
    fn_decl: &AstFnDecl,
    interner: &TyInterner,
    generics: &[String],
    builder: &mut ModuleBuilder,
) -> u32 {
    let (sig_blob, _) = encode_fn_sig(fn_decl, interner, generics, builder);
    sig_blob
}

/// Emit ParamDef rows for a function's parameters.
fn emit_fn_params(
    fn_decl: &AstFnDecl,
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
fn emit_generics_for_typedef(
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

fn resolve_type_handle(
    ast_type: &crate::ast::types::AstType,
    def_map: &DefMap,
    typedef_handles: &FxHashMap<DefId, TypeDefHandle>,
) -> Option<TypeDefHandle> {
    if let crate::ast::types::AstType::Named { name, .. } = ast_type {
        if let Some(def_id) = def_map.get(name) {
            return typedef_handles.get(&def_id).copied();
        }
    }
    None
}

// =============================================================================
// AST lookup helpers (adapted from check/env.rs)
// =============================================================================

fn find_struct_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Struct(s) = decl {
                if s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn find_entity_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEntityDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Entity(e) = decl {
                if e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
            }
        }
    }
    None
}

fn find_enum_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEnumDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Enum(e) = decl {
                if e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
            }
        }
    }
    None
}

fn find_fn_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Fn(f) = decl {
                if f.name == entry.name && f.name_span == entry.name_span {
                    return Some(f);
                }
            }
        }
    }
    None
}

fn find_contract_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstContractDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Contract(c) = decl {
                if c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn find_impl_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstImplDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Impl(i) = decl {
                if i.span == entry.span {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn find_component_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstComponentDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            match decl {
                AstDecl::Component(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return Some(c);
                }
                AstDecl::Extern(AstExternDecl::Component(_, c)) if c.name == entry.name && c.name_span == entry.name_span => {
                    return Some(c);
                }
                _ => {}
            }
        }
    }
    None
}

fn find_extern_fn_sig<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnSig> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Extern(AstExternDecl::Fn(_, sig)) = decl {
                if sig.name == entry.name && sig.name_span == entry.name_span {
                    return Some(sig);
                }
            }
        }
    }
    None
}

fn find_extern_struct_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Extern(AstExternDecl::Struct(_, s)) = decl {
                if s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn find_const_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstConstDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Const(c) = decl {
                if c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn find_global_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstGlobalDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Global(g) = decl {
                if g.name == entry.name && g.name_span == entry.name_span {
                    return Some(g);
                }
            }
        }
    }
    None
}

fn find_method_in_impl<'a>(impl_decl: &'a AstImplDecl, method_name: &str) -> Option<&'a AstFnDecl> {
    for member in &impl_decl.members {
        if let AstImplMember::Fn(f) = member {
            if f.name == method_name {
                return Some(f);
            }
        }
    }
    None
}
