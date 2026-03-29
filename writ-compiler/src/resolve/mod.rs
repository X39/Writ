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
pub(crate) mod inject_library;
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
/// `library_modules` is a slice of pre-compiled module binaries whose type definitions
/// should be available to user code. Pass `&[]` when compiling without library dependencies.
/// Library types are injected into the DefMap BEFORE Pass 1 (collect_declarations) so
/// they are visible during all resolution passes.
///
/// Performs Pass 1 (collection), Pass 2 (body resolution), and post-resolution validation.
pub fn resolve(
    asts: &[(FileId, &Ast)],
    file_paths: &[(FileId, &str)],
    library_modules: &[&writ_module::Module],
) -> (NameResolvedAst, Vec<Diagnostic>) {
    // Step 0: Create empty DefMap
    let mut def_map = def_map::DefMap::new();

    // Step 1: Inject library module types FIRST -- they must be in DefMap before
    // collect_declarations so that Pass 2 body resolution can see them. User code
    // re-declaring a library type will produce a duplicate-definition error (correct).
    inject_library::inject_module_types(library_modules, &mut def_map);

    // Step 2: Pass 1 -- collect user declarations into the same DefMap
    let mut diags = collector::collect_declarations(asts, file_paths, &mut def_map);

    // Step 3: Inject synthetic namespaces (log, dialogue)
    // Inject synthetic log namespace DefIds (log::trace .. log::error).
    inject_log_namespace(&mut def_map);

    // Inject synthetic dialogue builtin DefIds (say, say_localized, choice, ChoiceOption).
    inject_dialogue_namespace(&mut def_map);

    // Step 4: Pass 2 -- resolve bodies (DefMap now has library + user + synthetic entries)
    let (decls, mut resolve_diags) = resolver::resolve_bodies(asts, file_paths, &def_map);
    diags.append(&mut resolve_diags);

    // Post-resolution validation
    validate::validate_attributes(asts, &def_map, &mut diags);
    validate::validate_speakers(asts, &def_map, &mut diags);

    let resolved = NameResolvedAst { decls, def_map };

    (resolved, diags)
}
