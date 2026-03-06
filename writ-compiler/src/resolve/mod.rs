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
pub(crate) mod error;
pub mod ir;
pub mod prelude;
pub(crate) mod resolver;
pub(crate) mod scope;
pub(crate) mod suggest;
pub(crate) mod validate;

use crate::ast::Ast;
use ir::NameResolvedAst;
use writ_diagnostics::{Diagnostic, FileId};

/// Inject 5 synthetic ExternFn DefIds for the `log::` namespace into the DefMap.
///
/// Called between Pass 1 (collect_declarations) and Pass 2 (resolve_bodies) so that
/// the entries are visible to all body-resolution and type-checking code.
///
/// Each level gets a FQN of `"log::{level}"` (e.g. `"log::info"`), DefKind::ExternFn,
/// FileId(u32::MAX) as a synthetic sentinel, and zeroed spans.
fn inject_log_namespace(def_map: &mut def_map::DefMap) {
    use chumsky::span::SimpleSpan;
    use def_map::{DefEntry, DefKind, DefVis};

    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };
    for &level_name in prelude::LOG_NAMESPACE_LEVELS {
        let fqn = format!("log::{}", level_name);
        // Skip if already present (e.g. user declared `extern fn log::trace`)
        if def_map.by_fqn.contains_key(&fqn) {
            continue;
        }
        let entry = DefEntry {
            id: None,
            kind: DefKind::ExternFn,
            vis: DefVis::Pub,
            file_id: FileId(u32::MAX),
            namespace: "log".to_string(),
            name: level_name.to_string(),
            name_span: synthetic_span,
            generics: Vec::new(),
            span: synthetic_span,
        };
        let id = def_map.arena.alloc(entry);
        def_map.by_fqn.insert(fqn, id);
    }
}

/// Inject synthetic ExternFn DefIds for dialogue builtins into the DefMap.
///
/// Unlike log-level builtins which live under the `log::` namespace, dialogue
/// builtins are root-level: `say`, `say_localized`, `choice`, `ChoiceOption`.
fn inject_dialogue_namespace(def_map: &mut def_map::DefMap) {
    use chumsky::span::SimpleSpan;
    use def_map::{DefEntry, DefKind, DefVis};

    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };
    for &builtin_name in prelude::DIALOGUE_BUILTINS {
        // Skip if already present (e.g. user declared `extern fn say`)
        if def_map.by_fqn.contains_key(builtin_name) {
            continue;
        }
        let entry = DefEntry {
            id: None,
            kind: DefKind::ExternFn,
            vis: DefVis::Pub,
            file_id: FileId(u32::MAX),
            namespace: String::new(),
            name: builtin_name.to_string(),
            name_span: synthetic_span,
            generics: Vec::new(),
            span: synthetic_span,
        };
        let id = def_map.arena.alloc(entry);
        def_map.by_fqn.insert(builtin_name.to_string(), id);
    }
}

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
    let (mut def_map, mut diags) = collector::collect_declarations(asts, file_paths);

    // Inject synthetic log namespace DefIds (log::trace .. log::error).
    inject_log_namespace(&mut def_map);

    // Inject synthetic dialogue builtin DefIds (say, say_localized, choice, ChoiceOption).
    inject_dialogue_namespace(&mut def_map);

    // Pass 2: Resolve bodies
    let (decls, mut resolve_diags) = resolver::resolve_bodies(asts, file_paths, &def_map);
    diags.append(&mut resolve_diags);

    // Post-resolution validation
    validate::validate_attributes(asts, &def_map, &mut diags);
    validate::validate_speakers(asts, &def_map, &mut diags);

    let resolved = NameResolvedAst { decls, def_map };

    (resolved, diags)
}
