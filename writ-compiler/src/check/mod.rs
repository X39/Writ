//! Type checking for the Writ compiler.
//!
//! Consumes a `NameResolvedAst` and the original ASTs, producing a `TypedAst`
//! where every expression carries a fully resolved `Ty`.

pub mod ty;
pub mod ir;
pub mod env;
pub(crate) mod env_build;
pub(crate) mod library_sigs;
pub mod unify;
pub(crate) mod infer;
pub(crate) mod check_expr;
pub(crate) mod check_stmt;
pub(crate) mod check_decl;
pub(crate) mod error;
pub(crate) mod mutability;
pub(crate) mod desugar;
pub(crate) mod pattern;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::ast::Ast;
use crate::resolve::def_map::{DefId, DefKind, DefMap};
use crate::resolve::ir::NameResolvedAst;
use env::TypeEnv;
use error::TypeError;
use ir::TypedAst;
use ty::{TyInterner, TyKind};
use writ_diagnostics::{Diagnostic, FileId};

/// Entry point for type checking.
///
/// Takes ownership of `NameResolvedAst` (moves def_map into output) and
/// borrows the original ASTs (needed for declaration bodies and field types).
///
/// `library_modules` is a slice of pre-compiled module binaries. Their method
/// signatures are injected into `TypeEnv` after `TypeEnv::build` so that method
/// calls on library types resolve correctly. Pass `&[]` when compiling without
/// library dependencies.
///
/// Returns a 4-element tuple: `(TypedAst, TyInterner, TypeEnv, Vec<Diagnostic>)`.
/// The `TypeEnv` carries function signatures, struct/entity/enum fields, impl
/// associations, and other type-level metadata needed by LSP handlers.
pub fn typecheck(
    mut resolved: NameResolvedAst,
    asts: &[(FileId, &Ast)],
    library_modules: &[&writ_module::Module],
) -> (TypedAst, ty::TyInterner, env::TypeEnv, Vec<Diagnostic>) {
    // 1. Build TyInterner with primitives pre-interned
    let mut interner = ty::TyInterner::new();

    // 2. Build TypeEnv from resolved decls + original ASTs
    let (mut type_env, env_diags) = env::TypeEnv::build(&resolved, asts, &mut interner);

    // 2b. Inject library method signatures into TypeEnv.
    // This must happen AFTER TypeEnv::build so user-source sigs are already present.
    // resolved is owned (mut) and TypeEnv::build has returned; no outstanding borrows exist.
    library_sigs::inject_library_sigs(
        library_modules,
        &mut resolved.def_map,
        &mut type_env,
        &mut interner,
    );

    let mut all_diags = env_diags;

    // 3. Build CheckCtx
    let file_id = asts.first().map(|(id, _)| *id).unwrap_or(FileId(0));
    let mut ctx = check_expr::CheckCtx {
        interner,
        diags: Vec::new(),
        def_map: &resolved.def_map,
        type_env: &type_env,
        unify: unify::UnifyCtx::new(),
        local_env: env::LocalEnv::new(),
        current_fn_ret: None,
        current_file: file_id,
        self_type: None,
        current_namespace: String::new(),
    };

    // 4. Check each declaration
    let mut typed_decls = Vec::new();
    for decl in &resolved.decls {
        let typed = check_decl::check_decl(&mut ctx, decl, asts);
        typed_decls.push(typed);
    }

    // 4b. Post-type-check pass: detect recursive value-type structs.
    //     Must run after all declarations are checked so that struct_fields is fully populated.
    {
        let mut recursive_diags = Vec::new();
        detect_recursive_structs(&resolved.def_map, &type_env, &ctx.interner, &mut recursive_diags);
        ctx.diags.extend(recursive_diags);
    }

    // 5. Extract struct field types from TypeEnv before it's dropped.
    //    Map: DefId -> Vec<(field_name, field_ty)>, dropping the span.
    let struct_field_types: FxHashMap<crate::resolve::def_map::DefId, Vec<(String, ty::Ty)>> =
        type_env.struct_fields
            .iter()
            .map(|(def_id, fields)| {
                (*def_id, fields.iter().map(|(name, field_ty, _span)| (name.clone(), *field_ty)).collect())
            })
            .collect();

    // 6. Collect diagnostics and extract interner
    all_diags.append(&mut ctx.diags);
    let interner = std::mem::take(&mut ctx.interner);

    // 7. Build TypedAst with def_map moved from resolved
    let typed_ast = TypedAst {
        decls: typed_decls,
        def_map: resolved.def_map,
        struct_field_types,
        conditional_fns: type_env.conditional_fns.clone(),
        fallback_for_conditional: type_env.fallback_for_conditional.clone(),
    };

    (typed_ast, interner, type_env, all_diags)
}

// =============================================================================
// Recursive struct detection
// =============================================================================

/// Detect value-type structs that directly or transitively contain themselves
/// as a field, which would produce an infinite-size type.
///
/// Classes are explicitly excluded because they are heap-allocated (reference
/// semantics), so a class field is always a pointer — not an inline value.
///
/// This pass runs after type checking so that `struct_fields` is fully
/// populated in `type_env`.
fn detect_recursive_structs(
    def_map: &DefMap,
    type_env: &TypeEnv,
    interner: &TyInterner,
    diags: &mut Vec<Diagnostic>,
) {
    // Collect all value-type struct DefIds.
    let struct_ids: Vec<DefId> = def_map
        .arena
        .iter()
        .filter(|(_, entry)| matches!(entry.kind, DefKind::Struct))
        .map(|(id, _)| id)
        .collect();

    // globally_visited: structs whose full reachability has been checked.
    // We never need to re-visit them.
    let mut globally_visited: FxHashSet<DefId> = FxHashSet::default();

    for root_id in struct_ids {
        if globally_visited.contains(&root_id) {
            continue;
        }

        // DFS path: (def_id, field_name_that_led_here).
        // The first entry has an empty field name (it's the root).
        let mut path: Vec<(DefId, String)> = Vec::new();
        let mut in_path: FxHashSet<DefId> = FxHashSet::default();

        dfs_struct(
            root_id,
            &mut path,
            &mut in_path,
            &mut globally_visited,
            def_map,
            type_env,
            interner,
            diags,
        );
    }
}

/// Recursive DFS helper for struct cycle detection.
///
/// `path` holds the current DFS stack as `(def_id, field_name_that_led_here)`.
/// `in_path` is the set of DefIds currently on the DFS stack (for O(1) cycle detection).
/// `globally_visited` marks structs that have been fully explored — no need to revisit.
#[allow(clippy::too_many_arguments)] // DFS cycle detection requires path state + type resolution context
fn dfs_struct(
    def_id: DefId,
    path: &mut Vec<(DefId, String)>,
    in_path: &mut FxHashSet<DefId>,
    globally_visited: &mut FxHashSet<DefId>,
    def_map: &DefMap,
    type_env: &TypeEnv,
    interner: &TyInterner,
    diags: &mut Vec<Diagnostic>,
) {
    if globally_visited.contains(&def_id) {
        return;
    }

    if in_path.contains(&def_id) {
        // Cycle detected: build a chain description and emit the diagnostic.
        emit_recursive_struct_error(def_id, path, def_map, diags);
        return;
    }

    // Push onto the DFS path.
    in_path.insert(def_id);

    // Walk each field of this struct.
    if let Some(fields) = type_env.struct_fields.get(&def_id) {
        for (field_name, field_ty, _span) in fields {
            // Only value-type struct fields can create infinite-size cycles.
            // TyKind::Class, TyKind::Entity, TyKind::Enum, primitives, Array, Option,
            // Result, Func, TaskHandle, GenericParam, Infer, Error — all safe.
            if let TyKind::Struct(field_def_id) = interner.kind(*field_ty) {
                path.push((def_id, field_name.clone()));
                dfs_struct(
                    *field_def_id,
                    path,
                    in_path,
                    globally_visited,
                    def_map,
                    type_env,
                    interner,
                    diags,
                );
                path.pop();
            }
        }
    }

    in_path.remove(&def_id);
    globally_visited.insert(def_id);
}

/// Build and emit a `RecursiveStruct` diagnostic for the detected cycle.
///
/// `path` is the current DFS stack where each entry `(from_id, field_name)`
/// means: struct `from_id` has a value-type field `field_name` pointing to
/// the struct in the next path entry (or to `cycle_start` for the last entry).
///
/// `cycle_start` is the struct DefId that closes the cycle (it was already
/// `in_path` when we tried to visit it, so it appears both in `path` and as
/// the current target).
fn emit_recursive_struct_error(
    cycle_start: DefId,
    path: &[(DefId, String)],
    def_map: &DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    // Find where `cycle_start` first appears in `path` to isolate the cycle.
    let cycle_pos = path
        .iter()
        .position(|(id, _)| *id == cycle_start)
        .unwrap_or(0);

    // Build the chain description.
    //
    // Each path[i] = (from_id, field_name) means:
    //   struct `from_id` has field `field_name` of type `to_id`
    // where `to_id` = path[i+1].0 if i+1 < len, or `cycle_start` for the last entry.
    //
    // Example — direct self-reference `struct A { x: A }`:
    //   path = [(A, "x")], cycle_start = A
    //   chain: "`A` contains `A` (field `x`)"
    //
    // Example — transitive `struct A { b: B }`, `struct B { a: A }`:
    //   path = [(A, "b"), (B, "a")], cycle_start = A
    //   chain: "`A` contains `B` (field `b`), which contains `A` (field `a`)"
    let cycle_slice = &path[cycle_pos..];
    let mut chain_parts: Vec<String> = Vec::new();

    for (i, (from_id, field_name)) in cycle_slice.iter().enumerate() {
        let from_name = def_map.get_entry(*from_id).name.clone();
        let to_name = if i + 1 < cycle_slice.len() {
            def_map.get_entry(cycle_slice[i + 1].0).name.clone()
        } else {
            def_map.get_entry(cycle_start).name.clone()
        };
        chain_parts.push(format!(
            "`{}` contains `{}` (field `{}`)",
            from_name, to_name, field_name
        ));
    }

    let chain = chain_parts.join(", which ");

    // Use the cycle-starting struct's entry for the span and file.
    let entry = def_map.get_entry(cycle_start);
    let diag: Diagnostic = TypeError::RecursiveStruct {
        struct_name: entry.name.clone(),
        chain,
        span: entry.name_span,
        file: entry.file_id,
    }
    .into();
    diags.push(diag);
}
