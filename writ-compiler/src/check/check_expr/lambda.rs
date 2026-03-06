//! Lambda / closure type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::AstLambdaParam;
use crate::ast::types::AstType;
use super::CheckCtx;
use super::check_block_stmts;
use super::super::env::Mutability;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;

pub(super) fn check_lambda(
    ctx: &mut CheckCtx,
    params: &[AstLambdaParam],
    return_type: Option<&AstType>,
    body: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    let generic_map = rustc_hash::FxHashMap::default();

    // Resolve parameter types
    let mut param_tys = Vec::new();
    let mut param_names = Vec::new();
    for p in params {
        let ty = if let Some(ref annotation) = p.ty {
            super::super::env::resolve_ast_type_with_file(annotation, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file)
        } else {
            // No annotation: create an inference variable
            let var = ctx.unify.new_var();
            ctx.interner.intern(TyKind::Infer(var))
        };
        param_tys.push(ty);
        param_names.push(p.name.clone());
    }

    // Resolve return type
    let ret_ty = if let Some(rt) = return_type {
        super::super::env::resolve_ast_type_with_file(rt, ctx.def_map, &mut ctx.interner, &generic_map, ctx.current_file)
    } else {
        ctx.interner.void()
    };

    // Set up context for body checking
    let old_ret = ctx.current_fn_ret;
    ctx.current_fn_ret = Some(ret_ty);
    ctx.local_env.push_scope();

    // Define params in scope
    for (name, ty) in param_names.iter().zip(param_tys.iter()) {
        ctx.local_env.define(
            name.clone(),
            *ty,
            Mutability::Immutable,
            span,
        );
    }

    // Check body
    let typed_body = check_block_stmts(ctx, body, span);

    ctx.local_env.pop_scope();
    ctx.current_fn_ret = old_ret;

    // Build captures list (simplified: any outer variables referenced in the body
    // would be tracked here, but for now we produce an empty list since we don't
    // have a capture tracking mechanism in LocalEnv yet)
    let captures = Vec::new();

    // Build function type
    let func_ty = ctx.interner.func(param_tys.clone(), ret_ty);

    let typed_params: Vec<(String, super::super::ty::Ty)> = param_names
        .into_iter()
        .zip(param_tys)
        .collect();

    TypedExpr::Lambda {
        ty: func_ty,
        span,
        params: typed_params,
        ret_ty,
        captures,
        body: Box::new(typed_body),
    }
}
