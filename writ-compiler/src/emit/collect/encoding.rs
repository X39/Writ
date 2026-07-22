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
            let is_local_definition = match item_kind {
                0 => matches!(token.table(), TableId::MethodDef | TableId::ExternDef),
                1 => matches!(token.table(), TableId::TypeDef | TableId::ContractDef),
                2 => matches!(token.table(), TableId::GlobalDef),
                _ => false,
            };
            if !is_local_definition {
                continue;
            }
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

/// Recover the checker Ty for an AST annotation without mutating the interner.
///
/// Type checking has already interned every declaration signature before metadata
/// collection runs, so the emitter can reconstruct the exact structural key and
/// look it up. A failed lookup is an internal pipeline error that callers surface
/// as an emission diagnostic; it must never silently degrade metadata to Error.
pub(super) fn ast_type_to_ty_simple(
    ast_type: &crate::ast::types::AstType,
    generics: &[String],
    def_map: &DefMap,
    interner: &TyInterner,
) -> Result<crate::check::ty::Ty, String> {
    use crate::check::ty::{Ty, TyKind};

    let lookup = |kind: TyKind| {
        interner
            .lookup(&kind)
            .ok_or_else(|| format!("type checker did not intern metadata type `{kind:?}`"))
    };
    match ast_type {
        crate::ast::types::AstType::Named { name, .. } => {
            if let Some(idx) = generics.iter().position(|g| g == name) {
                return lookup(TyKind::GenericParam(idx as u32));
            }
            match name.as_str() {
                "void" => Ok(Ty(4)),
                "int" => Ok(Ty(0)),
                "float" => Ok(Ty(1)),
                "bool" => Ok(Ty(2)),
                "string" => Ok(Ty(3)),
                "Entity" => lookup(TyKind::AnyEntity),
                _ => resolve_named_type_definition(name, def_map)
                    .map(|(def_id, entry)| nominal_ty_kind(def_id, entry.kind))
                    .ok_or_else(|| format!("unresolved metadata type `{name}`"))
                    .and_then(lookup),
            }
        }
        crate::ast::types::AstType::Generic { name, args, .. } => {
            let args: Vec<Ty> = args
                .iter()
                .map(|arg| ast_type_to_ty_simple(arg, generics, def_map, interner))
                .collect::<Result<_, _>>()?;
            match (name.as_str(), args.as_slice()) {
                ("Option", [inner]) => lookup(TyKind::Option(*inner)),
                ("Result", [ok, err]) => lookup(TyKind::Result(*ok, *err)),
                ("TaskHandle", [inner]) => lookup(TyKind::TaskHandle(*inner)),
                ("Array", [inner]) => lookup(TyKind::Array(*inner)),
                _ => resolve_named_type_definition(name, def_map)
                    .ok_or_else(|| format!("unresolved metadata type `{name}`"))
                    .and_then(|(def_id, entry)| {
                        let base = lookup(nominal_ty_kind(def_id, entry.kind))?;
                        Ok(TyKind::GenericInstance {
                            base,
                            namespace: entry.namespace.clone(),
                            name: entry.name.clone(),
                            args,
                        })
                    })
                    .and_then(lookup),
            }
        }
        crate::ast::types::AstType::Array { elem, .. } => {
            let elem = ast_type_to_ty_simple(elem, generics, def_map, interner)?;
            lookup(TyKind::Array(elem))
        }
        crate::ast::types::AstType::Func { params, ret, .. } => {
            let params = params
                .iter()
                .map(|param| ast_type_to_ty_simple(param, generics, def_map, interner))
                .collect::<Result<_, _>>()?;
            let ret = ret
                .as_deref()
                .map(|ret| ast_type_to_ty_simple(ret, generics, def_map, interner))
                .transpose()?
                .unwrap_or(Ty(4));
            lookup(TyKind::Func { params, ret })
        }
        crate::ast::types::AstType::Void { .. } => Ok(Ty(4)),
    }
}

fn nominal_ty_kind(def_id: DefId, kind: DefKind) -> crate::check::ty::TyKind {
    use crate::check::ty::TyKind;
    match kind {
        DefKind::Struct | DefKind::Component | DefKind::ExternComponent => TyKind::Struct(def_id),
        DefKind::Class => TyKind::Class(def_id),
        DefKind::Entity => TyKind::Entity(def_id),
        DefKind::Enum => TyKind::Enum(def_id),
        DefKind::Contract => TyKind::Contract(def_id),
        _ => TyKind::Error,
    }
}

/// Encode an AST type as a blob heap entry for FieldDef/ParamDef type signatures.
pub(super) fn encode_type_from_ast(
    ast_type: &crate::ast::types::AstType,
    _interner: &TyInterner,
    generics: &[String],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut buf = Vec::new();
    encode_ast_type_into(ast_type, generics, def_map, builder, &mut buf);
    builder.blob_heap.intern(&buf)
}

pub(super) fn encode_ast_type_into(
    ast_type: &crate::ast::types::AstType,
    generics: &[String],
    def_map: &DefMap,
    builder: &ModuleBuilder,
    buf: &mut Vec<u8>,
) {
    let signature = ast_type_signature(ast_type, generics, def_map, builder);
    let bytes = writ_module::signature::encode_type_signature(&signature)
        .expect("AST type signature exceeds module format limits");
    buf.extend_from_slice(&bytes);
}

fn ast_type_signature(
    ast_type: &crate::ast::types::AstType,
    generics: &[String],
    def_map: &DefMap,
    builder: &ModuleBuilder,
) -> writ_module::signature::TypeSignature {
    use writ_module::signature::TypeSignature;

    match ast_type {
        crate::ast::types::AstType::Named { name, .. } => {
            if let Some(idx) = generics.iter().position(|generic| generic == name) {
                return TypeSignature::GenericParam(idx as u16);
            }
            match name.as_str() {
                "void" => TypeSignature::Void,
                "int" => TypeSignature::Int,
                "float" => TypeSignature::Float,
                "bool" => TypeSignature::Bool,
                "string" => TypeSignature::String,
                "Entity" => TypeSignature::Entity,
                _ => TypeSignature::Named(resolve_named_type_token(name, def_map, builder)),
            }
        }
        crate::ast::types::AstType::Generic { name, args, .. } => {
            if name == "Array" {
                let element = args
                    .first()
                    .map(|arg| ast_type_signature(arg, generics, def_map, builder))
                    .unwrap_or(TypeSignature::Void);
                return TypeSignature::Array(Box::new(element));
            }

            let (namespace, constructor) =
                if matches!(name.as_str(), "Option" | "Result" | "TaskHandle" | "Type") {
                    ("writ".to_string(), name.clone())
                } else if let Some((_, entry)) = resolve_named_type_definition(name, def_map) {
                    (entry.namespace.clone(), entry.name.clone())
                } else {
                    let normalized = name.strip_prefix("::").unwrap_or(name);
                    if let Some((namespace, constructor)) = normalized.rsplit_once("::") {
                        (namespace.to_string(), constructor.to_string())
                    } else {
                        (String::new(), normalized.to_string())
                    }
                };
            TypeSignature::Generic {
                namespace,
                name: constructor,
                args: args
                    .iter()
                    .map(|arg| ast_type_signature(arg, generics, def_map, builder))
                    .collect(),
            }
        }
        crate::ast::types::AstType::Array { elem, .. } => {
            TypeSignature::Array(Box::new(ast_type_signature(
                elem, generics, def_map, builder,
            )))
        }
        crate::ast::types::AstType::Func { params, ret, .. } => TypeSignature::Function {
            params: params
                .iter()
                .map(|param| ast_type_signature(param, generics, def_map, builder))
                .collect(),
            ret: Box::new(
                ret.as_deref()
                    .map(|ret| ast_type_signature(ret, generics, def_map, builder))
                    .unwrap_or(TypeSignature::Void),
            ),
        },
        crate::ast::types::AstType::Void { .. } => TypeSignature::Void,
    }
}

fn resolve_named_type_token(
    name: &str,
    def_map: &DefMap,
    builder: &ModuleBuilder,
) -> writ_module::MetadataToken {
    resolve_named_type_definition(name, def_map)
        .map(|(def_id, _)| def_id)
        .and_then(|def_id| builder.token_for_def(def_id))
        .map(|token| writ_module::MetadataToken(token.0))
        .unwrap_or(writ_module::MetadataToken::NULL)
}

fn resolve_named_type_definition<'a>(
    name: &str,
    def_map: &'a DefMap,
) -> Option<(DefId, &'a crate::resolve::def_map::DefEntry)> {
    let rooted = name.starts_with("::");
    let normalized = name.strip_prefix("::").unwrap_or(name);

    // Qualified names (including explicit root names) must resolve exactly. Falling
    // back to a short name here could silently bind `a::Thing` to `b::Thing`.
    if rooted || normalized.contains("::") {
        let def_id = def_map.get(normalized)?;
        let entry = def_map.get_entry(def_id);
        return is_type_definition(entry.kind).then_some((def_id, entry));
    }

    // Mirror the checker's root-scope lookup before considering short-name
    // matches in other namespaces. A user-defined root `Box`, for example,
    // must win even when the runtime also provides `writ::Box`.
    if let Some(def_id) = def_map.get(normalized) {
        let entry = def_map.get_entry(def_id);
        return is_type_definition(entry.kind).then_some((def_id, entry));
    }

    // The AST retains an unqualified spelling rather than its resolved DefId. A
    // short-name fallback is therefore safe only when exactly one type definition
    // has that spelling across all namespaces.
    let mut matches = def_map
        .arena
        .iter()
        .filter(|(_, entry)| entry.name == normalized && is_type_definition(entry.kind));
    let candidate = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(candidate)
}

fn is_type_definition(kind: DefKind) -> bool {
    matches!(
        kind,
        DefKind::Struct
            | DefKind::Class
            | DefKind::Entity
            | DefKind::Enum
            | DefKind::Contract
            | DefKind::Component
            | DefKind::ExternComponent
    )
}

/// Encode an empty method signature (void -> void).
pub(super) fn encode_empty_sig(builder: &mut ModuleBuilder) -> u32 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_le_bytes()); // 0 params
    buf.push(0x00); // void return
    builder.blob_heap.intern(&buf)
}

/// Number of registers populated with source arguments at method entry.
///
/// Unlike the method signature blob, MethodDef.param_count includes an explicit
/// `self` parameter. Entity hook lowering also materializes its implicit receiver
/// as an AstFnParam::SelfParam, so the AST parameter count is the runtime count.
pub(super) fn method_param_register_count(fn_decl: &crate::ast::decl::AstFnDecl) -> u16 {
    u16::try_from(fn_decl.params.len())
        .expect("method parameter count exceeds the module format limit")
}

/// Encode a function signature from an AstFnDecl.
pub(super) fn encode_fn_sig(
    fn_decl: &crate::ast::decl::AstFnDecl,
    _interner: &TyInterner,
    generics: &[String],
    def_map: &DefMap,
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
        encode_ast_type_into(&param.ty, generics, def_map, builder, &mut sig_buf);
        // Also encode each param type for ParamDef
        let mut param_buf = Vec::new();
        encode_ast_type_into(&param.ty, generics, def_map, builder, &mut param_buf);
        param_blobs.push(builder.blob_heap.intern(&param_buf));
    }

    // Return type
    match &fn_decl.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, def_map, builder, &mut sig_buf),
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
    def_map: &DefMap,
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
        encode_ast_type_into(&param.ty, generics, def_map, builder, &mut sig_buf);
    }
    match &sig.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, def_map, builder, &mut sig_buf),
        None => sig_buf.push(0x00),
    }

    builder.blob_heap.intern(&sig_buf)
}

/// Encode an operator signature.
pub(super) fn encode_op_sig(
    op_sig: &crate::ast::decl::AstOpSig,
    _interner: &TyInterner,
    generics: &[String],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) -> u32 {
    let mut sig_buf = Vec::new();
    sig_buf.extend_from_slice(&(op_sig.params.len() as u16).to_le_bytes());
    for param in &op_sig.params {
        encode_ast_type_into(&param.ty, generics, def_map, builder, &mut sig_buf);
    }
    match &op_sig.return_type {
        Some(rt) => encode_ast_type_into(rt, generics, def_map, builder, &mut sig_buf),
        None => sig_buf.push(0x00),
    }
    builder.blob_heap.intern(&sig_buf)
}

/// Encode a hook method signature.
pub(super) fn encode_hook_sig(
    fn_decl: &crate::ast::decl::AstFnDecl,
    interner: &TyInterner,
    generics: &[String],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
) -> u32 {
    let (sig_blob, _) = encode_fn_sig(fn_decl, interner, generics, def_map, builder);
    sig_blob
}

/// Emit ParamDef rows for a function's parameters.
pub(super) fn emit_fn_params(
    fn_decl: &crate::ast::decl::AstFnDecl,
    _interner: &TyInterner,
    generics: &[String],
    def_map: &DefMap,
    builder: &mut ModuleBuilder,
    method_handle: MethodDefHandle,
) {
    let mut seq: u16 = 0;
    for param in &fn_decl.params {
        if let AstFnParam::Regular(p) = param {
            let mut buf = Vec::new();
            encode_ast_type_into(&p.ty, generics, def_map, builder, &mut buf);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::span::SimpleSpan;

    use crate::resolve::def_map::{DefEntry, DefVis};

    fn add_public_type(def_map: &mut DefMap, namespace: &str, name: &str) -> DefId {
        let span = SimpleSpan {
            start: 0,
            end: 0,
            context: (),
        };
        let fqn = if namespace.is_empty() {
            name.to_string()
        } else {
            format!("{namespace}::{name}")
        };
        let mut diags = Vec::new();
        let def_id = def_map.insert(
            fqn,
            DefEntry {
                id: None,
                kind: DefKind::Struct,
                vis: DefVis::Pub,
                file_id: FileId(0),
                namespace: namespace.to_string(),
                name: name.to_string(),
                name_span: span,
                generics: Vec::new(),
                span,
            },
            &mut diags,
        );
        assert!(diags.is_empty());
        def_id
    }

    #[test]
    fn named_type_short_match_must_be_unique() {
        let mut def_map = DefMap::new();
        let alpha = add_public_type(&mut def_map, "alpha", "Thing");
        let beta = add_public_type(&mut def_map, "beta", "Thing");
        let mut builder = ModuleBuilder::new();
        let alpha_token = MetadataToken::new(TableId::TypeDef, 1);
        let beta_token = MetadataToken::new(TableId::TypeDef, 2);
        builder.def_token_map.insert(alpha, alpha_token);
        builder.def_token_map.insert(beta, beta_token);

        assert_eq!(
            resolve_named_type_token("Thing", &def_map, &builder),
            writ_module::MetadataToken::NULL
        );
        assert_eq!(
            resolve_named_type_token("alpha::Thing", &def_map, &builder),
            writ_module::MetadataToken(alpha_token.0)
        );
        assert_eq!(
            resolve_named_type_token("missing::Thing", &def_map, &builder),
            writ_module::MetadataToken::NULL
        );
    }

    #[test]
    fn named_type_prefers_exact_root_definition() {
        let mut def_map = DefMap::new();
        let root = add_public_type(&mut def_map, "", "Box");
        let runtime = add_public_type(&mut def_map, "writ", "Box");
        let mut builder = ModuleBuilder::new();
        let root_token = MetadataToken::new(TableId::TypeDef, 1);
        let runtime_token = MetadataToken::new(TableId::TypeDef, 2);
        builder.def_token_map.insert(root, root_token);
        builder.def_token_map.insert(runtime, runtime_token);

        assert_eq!(
            resolve_named_type_token("Box", &def_map, &builder),
            writ_module::MetadataToken(root_token.0)
        );
        assert_eq!(
            resolve_named_type_token("writ::Box", &def_map, &builder),
            writ_module::MetadataToken(runtime_token.0)
        );
    }

    #[test]
    fn generic_user_constructor_uses_resolved_namespace() {
        let mut def_map = DefMap::new();
        add_public_type(&mut def_map, "collections", "Crate");
        let builder = ModuleBuilder::new();
        let span = SimpleSpan {
            start: 0,
            end: 0,
            context: (),
        };
        let ast_type = crate::ast::types::AstType::Generic {
            name: "Crate".to_string(),
            args: vec![crate::ast::types::AstType::Named {
                name: "int".to_string(),
                span,
            }],
            span,
        };

        assert_eq!(
            ast_type_signature(&ast_type, &[], &def_map, &builder),
            writ_module::signature::TypeSignature::Generic {
                namespace: "collections".to_string(),
                name: "Crate".to_string(),
                args: vec![writ_module::signature::TypeSignature::Int],
            }
        );
    }
}
