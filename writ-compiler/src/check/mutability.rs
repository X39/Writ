//! Mutability enforcement for the Writ type checker.
//!
//! Central module for mutation checks. Provides:
//! - Root-binding propagation: walks expression trees to find the root binding
//! - Immutable reassignment detection (E0108)
//! - Immutable field mutation detection (E0107)
//! - `mut self` method call checking on immutable bindings (E0107)
//!
//! All mutation checks route through `check_mutation` which delegates
//! to the appropriate checker based on the mutation kind.

use chumsky::span::SimpleSpan;

use super::check_expr::CheckCtx;
use super::env::{LocalEnv, Mutability};
use super::error::TypeError;
use super::ir::TypedExpr;

/// Check whether a `mut self` method is being called on an immutable binding.
pub fn check_method_mutation(
    ctx: &mut CheckCtx,
    receiver: &TypedExpr,
    method_name: &str,
    is_mut_self: bool,
    call_span: SimpleSpan,
) {
    if !is_mut_self {
        return;
    }

    if let Some((name, mutability, binding_span)) = find_root_binding(receiver, &ctx.local_env) {
        if mutability == Mutability::Immutable {
            ctx.diags.push(
                TypeError::ImmutableMutation {
                    binding_name: name,
                    binding_span,
                    mutation_span: call_span,
                    mutation_kind: format!("call `mut self` method `{}`", method_name),
                    file: ctx.current_file,
                }
                .into(),
            );
        }
    }
}

/// Walk a TypedExpr to find its root variable binding.
fn find_root_binding(
    expr: &TypedExpr,
    local_env: &LocalEnv,
) -> Option<(String, Mutability, SimpleSpan)> {
    match expr {
        TypedExpr::Var { name, .. } => {
            local_env
                .lookup(name)
                .map(|(_, m, sp)| (name.clone(), m, sp))
        }
        TypedExpr::SelfRef { .. } => {
            local_env
                .lookup("self")
                .map(|(_, m, sp)| ("self".to_string(), m, sp))
        }
        TypedExpr::Field { receiver, .. } => find_root_binding(receiver, local_env),
        TypedExpr::Index { receiver, .. } => find_root_binding(receiver, local_env),
        _ => None,
    }
}
