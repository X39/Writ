//! Post-resolution validation passes.
//!
//! Validates attribute targets (e.g., [Singleton] only on entities),
//! speaker references in dialogue, and other semantic checks.

use crate::ast::decl::{AstAttribute, AstDecl, AstExternDecl, AstNamespaceDecl};
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
            AstDecl::Extern(ext) => match ext {
                AstExternDecl::Fn(_, sig) => {
                    check_attrs(&sig.attrs, "extern fn", file_id, diags);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_attrs(attrs: &[AstAttribute], decl_kind: &str, file_id: FileId, diags: &mut Vec<Diagnostic>) {
    for attr in attrs {
        if let Some((_, valid_kinds)) = KNOWN_ATTRS.iter().find(|(name, _)| *name == attr.name) {
            if !valid_kinds.contains(&decl_kind) {
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
        }
        // Unknown attributes: we don't warn for them currently (future-proofing)
        // If we wanted to warn:
        // else if !KNOWN_ATTRS.iter().any(|(name, _)| *name == attr.name) { ... }
    }
}

/// Validate speaker references in dialogue.
///
/// Speakers must be [Singleton] entities with a Speaker component.
/// This is a best-effort check based on the lowered AST.
pub fn validate_speakers(
    _asts: &[(FileId, &Ast)],
    _def_map: &DefMap,
    _diags: &mut Vec<Diagnostic>,
) {
    // Speaker validation requires walking lowered dialogue function bodies
    // to extract speaker name strings from say()/say_localized() calls.
    // This is tracked for deeper implementation when dialogue-specific
    // resolution is needed. For now, the error types are in place and
    // the validation structure is ready to be filled in.
    //
    // The E0007 error type is already defined and can be emitted when
    // speaker references are detected.
}
