//! Post-resolution validation passes.
//!
//! Validates attribute targets (e.g., [Singleton] only on entities),
//! speaker references in dialogue, and other semantic checks.

use rustc_hash::FxHashSet;

use crate::ast::decl::{AstDecl, AstEntityDecl, AstAttribute, AstExternDecl, AstNamespaceDecl};
use crate::ast::expr::AstExpr;
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;
use crate::ast::Ast;
use crate::resolve::def_map::DefMap;
use crate::resolve::error::ResolutionError;
use writ_diagnostics::{Diagnostic, FileId};

/// Known attribute names and their valid targets.
const KNOWN_ATTRS: &[(&str, &[&str])] = &[
    ("Singleton", &["entity"]),
    ("Conditional", &["fn"]),
];

/// Validate attribute targets across all files.
///
/// Checks that attributes like [Singleton] and [Conditional] are only
/// used on their valid declaration kinds.
pub fn validate_attributes(
    asts: &[(FileId, &Ast)],
    _def_map: &DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    for &(file_id, ast) in asts {
        validate_attrs_in_items(&ast.items, file_id, diags);
    }
}

fn validate_attrs_in_items(items: &[AstDecl], file_id: FileId, diags: &mut Vec<Diagnostic>) {
    for item in items {
        match item {
            AstDecl::Namespace(AstNamespaceDecl::Block { items, .. }) => {
                validate_attrs_in_items(items, file_id, diags);
            }
            AstDecl::Fn(f) => {
                check_attrs(&f.attrs, "fn", file_id, diags);
            }
            AstDecl::Struct(s) => {
                check_attrs(&s.attrs, "struct", file_id, diags);
            }
            AstDecl::Entity(e) => {
                check_attrs(&e.attrs, "entity", file_id, diags);
            }
            AstDecl::Enum(e) => {
                check_attrs(&e.attrs, "enum", file_id, diags);
            }
            AstDecl::Contract(c) => {
                check_attrs(&c.attrs, "contract", file_id, diags);
            }
            AstDecl::Component(c) => {
                check_attrs(&c.attrs, "component", file_id, diags);
            }
            AstDecl::Const(c) => {
                check_attrs(&c.attrs, "const", file_id, diags);
            }
            AstDecl::Global(g) => {
                check_attrs(&g.attrs, "global", file_id, diags);
            }
            AstDecl::Extern(AstExternDecl::Fn(_, sig)) => {
                check_attrs(&sig.attrs, "extern fn", file_id, diags);
            }
            _ => {}
        }
    }
}

fn check_attrs(attrs: &[AstAttribute], decl_kind: &str, file_id: FileId, diags: &mut Vec<Diagnostic>) {
    for attr in attrs {
        if let Some((_, valid_kinds)) = KNOWN_ATTRS.iter().find(|(name, _)| *name == attr.name)
            && !valid_kinds.contains(&decl_kind) {
                diags.push(
                    ResolutionError::InvalidAttributeTarget {
                        attr_name: attr.name.clone(),
                        target_kind: format!("{decl_kind} declaration"),
                        file: file_id,
                        span: attr.span,
                    }
                    .into(),
                );
            }
        // Unknown attributes: we don't warn for them currently (future-proofing)
        // If we wanted to warn:
        // else if !KNOWN_ATTRS.iter().any(|(name, _)| *name == attr.name) { ... }
    }
}

/// Validate speaker references in dialogue.
///
/// Dialogue `@Speaker` references are lowered (by `lower/dialogue.rs`) into
/// hoisted `let _speaker = Entity.getOrCreate<Name>()` statements.  We walk
/// all function bodies looking for that exact pattern, then validate that the
/// extracted entity name:
///
/// 1. Actually exists as an `entity` declaration in any of the compilation
///    files → if not, emit E0003 (UnresolvedName).
/// 2. Bears the `[Singleton]` attribute → if not, emit E0007 (InvalidSpeaker).
///
/// Contract-typed param speakers (`dlg greet(npc: Entity)`) are lowered
/// differently (no hoisted let) so they are naturally invisible to this pass,
/// which means no false positives for them.
pub fn validate_speakers(
    asts: &[(FileId, &Ast)],
    _def_map: &DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    // Build entity name sets from all ASTs.
    let (singleton_entities, all_entities) = collect_entity_sets(asts);

    // Walk every function body for the hoisted-let speaker pattern.
    for &(file_id, ast) in asts {
        validate_speakers_in_items(
            &ast.items,
            file_id,
            &singleton_entities,
            &all_entities,
            diags,
        );
    }
}

/// Build (singleton_names, all_entity_names) from the combined ASTs.
fn collect_entity_sets(asts: &[(FileId, &Ast)]) -> (FxHashSet<String>, FxHashSet<String>) {
    let mut singletons: FxHashSet<String> = FxHashSet::default();
    let mut all: FxHashSet<String> = FxHashSet::default();
    for &(_, ast) in asts {
        collect_entities_in_items(&ast.items, &mut singletons, &mut all);
    }
    (singletons, all)
}

/// Recurse into items, recording entity names into the two sets.
fn collect_entities_in_items(
    items: &[AstDecl],
    singletons: &mut FxHashSet<String>,
    all: &mut FxHashSet<String>,
) {
    for item in items {
        match item {
            AstDecl::Entity(e) => {
                all.insert(e.name.clone());
                if has_singleton_attr(e) {
                    singletons.insert(e.name.clone());
                }
            }
            AstDecl::Namespace(AstNamespaceDecl::Block { items, .. }) => {
                collect_entities_in_items(items, singletons, all);
            }
            _ => {}
        }
    }
}

/// Returns `true` if the entity bears a `[Singleton]` attribute.
fn has_singleton_attr(e: &AstEntityDecl) -> bool {
    e.attrs.iter().any(|a| a.name == "Singleton")
}

/// Walk items in one file, delegating function body checks.
fn validate_speakers_in_items(
    items: &[AstDecl],
    file_id: FileId,
    singletons: &FxHashSet<String>,
    all_entities: &FxHashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            AstDecl::Fn(f) => {
                check_stmts_for_speakers(&f.body, file_id, singletons, all_entities, diags);
            }
            AstDecl::Entity(e) => {
                // Check methods in entity inherent impl.
                if let Some(impl_decl) = &e.inherent_impl {
                    for member in &impl_decl.members {
                        if let crate::ast::decl::AstImplMember::Fn(f) = member {
                            check_stmts_for_speakers(
                                &f.body,
                                file_id,
                                singletons,
                                all_entities,
                                diags,
                            );
                        }
                    }
                }
                // Check hook bodies.
                for hook in &e.hooks {
                    check_stmts_for_speakers(
                        &hook.method.body,
                        file_id,
                        singletons,
                        all_entities,
                        diags,
                    );
                }
            }
            AstDecl::Namespace(AstNamespaceDecl::Block { items, .. }) => {
                validate_speakers_in_items(items, file_id, singletons, all_entities, diags);
            }
            _ => {}
        }
    }
}

/// Walk `stmts` looking for the hoisted singleton-speaker let pattern:
///
/// ```text
/// let _<name> = Entity.getOrCreate<Name>()
/// ```
///
/// For each match, validate the entity name.
fn check_stmts_for_speakers(
    stmts: &[AstStmt],
    file_id: FileId,
    singletons: &FxHashSet<String>,
    all_entities: &FxHashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    for stmt in stmts {
        match stmt {
            AstStmt::Let { name, name_span, value, .. } if name.starts_with('_') => {
                // Try to match `Entity.getOrCreate<EntityName>()`
                if let Some(entity_name) = extract_get_or_create_entity(value) {
                    if !all_entities.contains(&entity_name) {
                        diags.push(
                            ResolutionError::UnresolvedName {
                                name: entity_name,
                                file: file_id,
                                span: *name_span,
                                suggestion: None,
                            }
                            .into(),
                        );
                    } else if !singletons.contains(&entity_name) {
                        diags.push(
                            ResolutionError::InvalidSpeaker {
                                name: entity_name,
                                file: file_id,
                                span: *name_span,
                            }
                            .into(),
                        );
                    }
                    // else: entity exists and is [Singleton] — valid, no diagnostic
                }
            }
            // Recurse into block-like statements.
            AstStmt::For { body, .. } => {
                check_stmts_for_speakers(body, file_id, singletons, all_entities, diags);
            }
            AstStmt::While { body, .. } => {
                check_stmts_for_speakers(body, file_id, singletons, all_entities, diags);
            }
            AstStmt::Atomic { body, .. } => {
                check_stmts_for_speakers(body, file_id, singletons, all_entities, diags);
            }
            _ => {}
        }
    }
}

/// If `expr` is the pattern `Entity.getOrCreate<Name>()`, return `Some(Name)`.
/// Otherwise return `None`.
fn extract_get_or_create_entity(expr: &AstExpr) -> Option<String> {
    if let AstExpr::GenericCall { callee, type_args, args, .. } = expr {
        if !args.is_empty() || type_args.len() != 1 {
            return None;
        }
        if let AstExpr::MemberAccess { object, field, .. } = callee.as_ref() {
            if field != "getOrCreate" {
                return None;
            }
            if let AstExpr::Ident { name: obj_name, .. } = object.as_ref() {
                if obj_name != "Entity" {
                    return None;
                }
                if let AstType::Named { name: entity_name, .. } = &type_args[0] {
                    return Some(entity_name.clone());
                }
            }
        }
    }
    None
}
