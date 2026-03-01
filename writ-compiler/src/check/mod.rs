//! Type checking for the Writ compiler.
//!
//! Consumes a `NameResolvedAst` and the original ASTs, producing a `TypedAst`
//! where every expression carries a fully resolved `Ty`.

pub mod ty;
pub mod ir;
pub mod env;
pub mod unify;
pub mod infer;
pub mod check_expr;
pub mod check_stmt;
pub mod check_decl;
pub mod error;
pub mod mutability;
pub mod desugar;
pub mod pattern;

use crate::ast::Ast;
use crate::resolve::ir::NameResolvedAst;
use ir::TypedAst;
use writ_diagnostics::{Diagnostic, FileId};

/// Entry point for type checking.
///
/// Takes ownership of `NameResolvedAst` (moves def_map into output) and
/// borrows the original ASTs (needed for declaration bodies and field types).
pub fn typecheck(
    resolved: NameResolvedAst,
    asts: &[(FileId, &Ast)],
) -> (TypedAst, ty::TyInterner, Vec<Diagnostic>) {
    // 1. Build TyInterner with primitives pre-interned
    let mut interner = ty::TyInterner::new();

    // 2. Build TypeEnv from resolved decls + original ASTs
    let (type_env, env_diags) = env::TypeEnv::build(&resolved, asts, &mut interner);

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
    };

    // 4. Check each declaration
    let mut typed_decls = Vec::new();
    for decl in &resolved.decls {
        let typed = check_decl::check_decl(&mut ctx, decl, asts);
        typed_decls.push(typed);
    }

    // 5. Collect diagnostics and extract interner
    all_diags.append(&mut ctx.diags);
    let interner = std::mem::replace(&mut ctx.interner, ty::TyInterner::new());

    // 6. Build TypedAst with def_map moved from resolved
    let typed_ast = TypedAst {
        decls: typed_decls,
        def_map: resolved.def_map,
    };

    (typed_ast, interner, all_diags)
}
