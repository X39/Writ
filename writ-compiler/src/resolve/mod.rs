//! Name resolution for the Writ compiler.
//!
//! Name resolution consists of three stages:
//! - **Pass 1 (collector):** Collects all top-level declarations into a DefMap.
//! - **Pass 2 (resolver):** Resolves all name references in bodies against the DefMap.
//! - **Validation:** Checks attribute targets, speaker references, and semantic constraints.
//!
//! This module also provides the prelude, IR types, scope chain, suggestion engine,
//! and error definitions.

pub mod collector;
pub mod def_map;
pub mod error;
pub mod ir;
pub mod prelude;
pub mod resolver;
pub mod scope;
pub mod suggest;
pub mod validate;

use crate::ast::Ast;
use ir::NameResolvedAst;
use writ_diagnostics::{Diagnostic, FileId};

/// Entry point for name resolution.
///
/// Takes a list of parsed/lowered ASTs (one per file) with their FileIds and file paths,
/// and produces a `NameResolvedAst` with all names resolved to `DefId`s.
///
/// Performs Pass 1 (collection), Pass 2 (body resolution), and post-resolution validation.
pub fn resolve(
    asts: &[(FileId, &Ast)],
    file_paths: &[(FileId, &str)],
) -> (NameResolvedAst, Vec<Diagnostic>) {
    // Pass 1: Collect declarations
    let (def_map, mut diags) = collector::collect_declarations(asts, file_paths);

    // Pass 2: Resolve bodies
    let (decls, mut resolve_diags) = resolver::resolve_bodies(asts, file_paths, &def_map);
    diags.append(&mut resolve_diags);

    // Post-resolution validation
    validate::validate_attributes(asts, &def_map, &mut diags);
    validate::validate_speakers(asts, &def_map, &mut diags);

    let resolved = NameResolvedAst { decls, def_map };

    (resolved, diags)
}
