//! Expression type checking — module root.
//!
//! This module is split into focused submodules:
//! - `ident`        — identifier lookup
//! - `path`         — path/qualified-name resolution
//! - `binary`       — binary and unary-prefix operators
//! - `call`         — function call checking
//! - `control`      — if/block control flow
//! - `access`       — member and bracket access
//! - `match_`       — match expressions and patterns
//! - `lambda`       — lambda/closure checking
//! - `construction` — `new` struct/class/entity construction and array literals

use chumsky::span::SimpleSpan;

use crate::ast::expr::{AstExpr, PrefixOp, PostfixOp};
use crate::resolve::def_map::{DefId, DefKind, DefMap};

use super::env::{LocalEnv, TypeEnv};
use super::error::TypeError;
use super::ir::{TypedExpr, TypedStmt, TypedLiteral};
use super::ty::{Ty, TyInterner, TyKind};
use super::unify::UnifyCtx;
use writ_diagnostics::{Diagnostic, FileId};

mod ident;
mod path;
mod binary;
mod call;
mod control;
mod access;
mod match_;
mod lambda;
mod construction;

use ident::check_ident;
use path::check_path;
use binary::{check_binary, check_unary_prefix};
use call::{check_call, check_generic_call};
use control::check_if;
use access::{check_member_access, check_bracket_access};
use match_::{check_match, check_pattern};
use lambda::check_lambda;
use construction::{check_new_construction, check_array_lit};

/// Central checking context threaded through all checking functions.
pub struct CheckCtx<'def> {
    pub interner: TyInterner,
    pub diags: Vec<Diagnostic>,
    pub def_map: &'def DefMap,
    pub type_env: &'def TypeEnv,
    pub unify: UnifyCtx,
    pub local_env: LocalEnv,
    pub current_fn_ret: Option<Ty>,
    pub current_file: FileId,
    pub self_type: Option<Ty>,
    /// The namespace of the currently-checked function or method (e.g. `"mymod"`).
    /// Set from `DefEntry::namespace` before checking a function/method body.
    /// Used for namespace-prefixed FQN lookup of type annotations in namespaced projects.
    pub current_namespace: String,
}

impl CheckCtx<'_> {
    /// Check if a type is the poison/error type.
    pub fn is_error(&self, ty: Ty) -> bool {
        matches!(self.interner.kind(ty), TyKind::Error)
    }

    /// Emit a type error and return the Error poison type.
    pub fn emit_error(&mut self, err: TypeError) -> Ty {
        self.diags.push(err.into());
        self.interner.error()
    }

    /// Format a type for display in error messages.
    pub fn display_ty(&self, ty: Ty) -> String {
        self.interner.display(ty)
    }
}

/// Type-check an expression, returning a TypedExpr.
pub fn check_expr(ctx: &mut CheckCtx, expr: &AstExpr) -> TypedExpr {
    match expr {
        AstExpr::IntLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.int(),
            span: *span,
            value: TypedLiteral::Int(*value),
        },
        AstExpr::FloatLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.float(),
            span: *span,
            value: TypedLiteral::Float(*value),
        },
        AstExpr::StringLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.string_ty(),
            span: *span,
            value: TypedLiteral::String(value.clone()),
        },
        AstExpr::BoolLit { value, span } => TypedExpr::Literal {
            ty: ctx.interner.bool_ty(),
            span: *span,
            value: TypedLiteral::Bool(*value),
        },

        AstExpr::Ident { name, span } => check_ident(ctx, name, *span),

        AstExpr::Path { segments, span } => check_path(ctx, segments, *span),

        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => check_binary(ctx, left, op, right, *span),

        AstExpr::UnaryPrefix { op, expr, span } => check_unary_prefix(ctx, op, expr, *span),

        AstExpr::UnaryPostfix { expr: inner, op, span } => {
            match op {
                PostfixOp::NullPropagate => {
                    super::desugar::desugar_question(ctx, inner, *span)
                }
                PostfixOp::Unwrap => {
                    super::desugar::desugar_unwrap(ctx, inner, *span)
                }
            }
        }

        AstExpr::Call { callee, args, span } => check_call(ctx, callee, args, *span),

        AstExpr::GenericCall {
            callee,
            type_args,
            args,
            span,
        } => check_generic_call(ctx, callee, type_args, args, *span),

        AstExpr::If {
            condition,
            then_block,
            else_block,
            span,
        } => check_if(ctx, condition, then_block, else_block.as_deref(), *span),

        AstExpr::Block { stmts, span } => check_block(ctx, stmts, *span),

        AstExpr::MemberAccess { object, field, field_span, span } => {
            check_member_access(ctx, object, field, *field_span, *span)
        },
        AstExpr::BracketAccess { object, index, span } => {
            check_bracket_access(ctx, object, index, *span)
        },
        AstExpr::SelfLit { span } => {
            if let Some(self_ty) = ctx.self_type {
                TypedExpr::SelfRef { ty: self_ty, span: *span }
            } else {
                let err_ty = ctx.emit_error(TypeError::UndefinedVariable {
                    name: "self".to_string(),
                    span: *span,
                    file: ctx.current_file,
                });
                TypedExpr::Error { ty: err_ty, span: *span }
            }
        },
        AstExpr::Match { scrutinee, arms, span } => {
            check_match(ctx, scrutinee, arms, *span)
        },
        AstExpr::IfLet { pattern, value, then_block, else_block, span } => {
            let typed_value = check_expr(ctx, value);
            let value_ty = typed_value.ty();

            // Check then block with pattern bindings
            ctx.local_env.push_scope();
            let _typed_pattern = check_pattern(ctx, pattern, value_ty);
            let then_typed = check_block_stmts(ctx, then_block, *span);
            let then_ty = then_typed.ty();
            ctx.local_env.pop_scope();

            // Check else block
            if let Some(else_expr) = else_block {
                let else_typed = check_expr(ctx, else_expr);
                let else_ty = else_typed.ty();

                let result_ty = if ctx.is_error(then_ty) || ctx.is_error(else_ty) {
                    ctx.interner.error()
                } else if ctx.unify.unify(then_ty, else_ty, &mut ctx.interner).is_err() {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(then_ty),
                        found: ctx.display_ty(else_ty),
                        expected_span: then_typed.span(),
                        found_span: else_typed.span(),
                        file: ctx.current_file,
                        help: Some("if-let branches must have the same type".to_string()),
                    })
                } else {
                    then_ty
                };

                TypedExpr::If {
                    ty: result_ty,
                    span: *span,
                    condition: Box::new(typed_value),
                    then_branch: Box::new(then_typed),
                    else_branch: Some(Box::new(else_typed)),
                }
            } else {
                TypedExpr::If {
                    ty: ctx.interner.void(),
                    span: *span,
                    condition: Box::new(typed_value),
                    then_branch: Box::new(then_typed),
                    else_branch: None,
                }
            }
        },
        AstExpr::Lambda { params, return_type, body, span } => {
            check_lambda(ctx, params, return_type.as_deref(), body, *span)
        },
        AstExpr::Spawn { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let task_ty = ctx.interner.task_handle(inner_ty);
            TypedExpr::Spawn {
                ty: task_ty,
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::SpawnDetached { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            TypedExpr::SpawnDetached {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Join { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let result_ty = if ctx.is_error(inner_ty) {
                ctx.interner.error()
            } else {
                match ctx.interner.kind(inner_ty).clone() {
                    TyKind::TaskHandle(inner) => inner,
                    _ => {
                        ctx.emit_error(TypeError::TypeMismatch {
                            expected: "TaskHandle<T>".to_string(),
                            found: ctx.display_ty(inner_ty),
                            expected_span: *span,
                            found_span: typed_inner.span(),
                            file: ctx.current_file,
                            help: Some("join requires a TaskHandle".to_string()),
                        })
                    }
                }
            };
            TypedExpr::Join {
                ty: result_ty,
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Cancel { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            if !ctx.is_error(inner_ty)
                && !matches!(ctx.interner.kind(inner_ty), TyKind::TaskHandle(_)) {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: "TaskHandle<T>".to_string(),
                        found: ctx.display_ty(inner_ty),
                        expected_span: *span,
                        found_span: typed_inner.span(),
                        file: ctx.current_file,
                        help: Some("cancel requires a TaskHandle".to_string()),
                    });
                }
            TypedExpr::Cancel {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Defer { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            TypedExpr::Defer {
                ty: ctx.interner.void(),
                span: *span,
                expr: Box::new(typed_inner),
            }
        },
        AstExpr::Try { expr: inner, span } => {
            super::desugar::desugar_try(ctx, inner, *span)
        },
        AstExpr::New { ty: ast_ty, fields, span } => {
            check_new_construction(ctx, ast_ty, fields, *span)
        },
        AstExpr::ArrayLit { elements, span } => {
            check_array_lit(ctx, elements, *span)
        },
        AstExpr::Assign { target, value, span } => {
            let typed_target = check_expr(ctx, target);
            let typed_value = check_expr(ctx, value);
            let target_ty = typed_target.ty();
            let value_ty = typed_value.ty();

            if !ctx.is_error(target_ty) && !ctx.is_error(value_ty)
                && ctx.unify.unify(target_ty, value_ty, &mut ctx.interner).is_err() {
                    ctx.emit_error(TypeError::TypeMismatch {
                        expected: ctx.display_ty(target_ty),
                        found: ctx.display_ty(value_ty),
                        expected_span: typed_target.span(),
                        found_span: typed_value.span(),
                        file: ctx.current_file,
                        help: None,
                    });
                }

            // Check mutability of the assignment target
            check_assignment_mutability(ctx, &typed_target, *span);

            TypedExpr::Assign {
                ty: ctx.interner.void(),
                span: *span,
                target: Box::new(typed_target),
                value: Box::new(typed_value),
            }
        }
        AstExpr::Range { start, kind, end, span } => {
            let typed_start = start.as_ref().map(|s| check_expr(ctx, s));
            let typed_end = end.as_ref().map(|e| check_expr(ctx, e));

            // If both present, unify their types
            if let (Some(s), Some(e)) = (&typed_start, &typed_end) {
                let s_ty = s.ty();
                let e_ty = e.ty();
                if !ctx.is_error(s_ty) && !ctx.is_error(e_ty) {
                    let _ = ctx.unify.unify(s_ty, e_ty, &mut ctx.interner);
                }
            }

            // Range type is int for now (simplification; proper Range<T> is a runtime type)
            let range_ty = ctx.interner.int();

            TypedExpr::Range {
                ty: range_ty,
                span: *span,
                start: typed_start.map(Box::new),
                end: typed_end.map(Box::new),
                inclusive: matches!(kind, crate::ast::expr::RangeKind::Inclusive),
            }
        },
        AstExpr::FromEnd { expr: inner, span } => {
            let typed_inner = check_expr(ctx, inner);
            let inner_ty = typed_inner.ty();
            let int_ty = ctx.interner.int();
            if !ctx.is_error(inner_ty) && inner_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(inner_ty),
                    expected_span: *span,
                    found_span: typed_inner.span(),
                    file: ctx.current_file,
                    help: Some("from-end index requires int".to_string()),
                });
            }
            // FromEnd produces an int (the index value)
            TypedExpr::UnaryPrefix {
                ty: int_ty,
                span: *span,
                op: PrefixOp::FromEnd,
                expr: Box::new(typed_inner),
            }
        },

        AstExpr::TypeOf { expr: inner_expr, span } => {
            // typeof(Expr) — resolve the static type of the inner expression.
            //
            // Special case: when the inner expression is a bare identifier that names a
            // type declaration (struct, class, entity, enum, contract), resolve it as a
            // type rather than as a variable.  Without this, `typeof(Point)` would fail
            // with "undefined variable `Point`" because `check_ident` only recognises
            // functions, constants, and globals.
            let static_ty = match inner_expr.as_ref() {
                AstExpr::Ident { name, .. } => {
                    // Attempt to resolve the name as a type definition.
                    let def_id = ctx
                        .def_map
                        .get(name)
                        .or_else(|| {
                            ctx.def_map
                                .file_private
                                .values()
                                .find_map(|m| m.get(name.as_str()).copied())
                        });
                    if let Some(did) = def_id {
                        let entry = ctx.def_map.get_entry(did);
                        match entry.kind {
                            DefKind::Struct => ctx.interner.intern(TyKind::Struct(did)),
                            DefKind::Class => ctx.interner.intern(TyKind::Class(did)),
                            DefKind::Entity => ctx.interner.intern(TyKind::Entity(did)),
                            DefKind::Enum => ctx.interner.intern(TyKind::Enum(did)),
                            DefKind::Contract => ctx.interner.intern(TyKind::Contract(did)),
                            // Not a type definition — fall through to regular expression checking.
                            _ => {
                                let typed_inner = check_expr(ctx, inner_expr);
                                typed_inner.ty()
                            }
                        }
                    } else {
                        // Name not in def map — fall through to regular expression checking.
                        let typed_inner = check_expr(ctx, inner_expr);
                        typed_inner.ty()
                    }
                }
                _ => {
                    // Non-ident inner expression (e.g. `typeof(x)` for a variable) — check normally.
                    let typed_inner = check_expr(ctx, inner_expr);
                    typed_inner.ty()
                }
            };
            if ctx.is_error(static_ty) {
                return TypedExpr::Error { ty: ctx.interner.error(), span: *span };
            }
            let reflection_ty = ctx.interner.reflection_type(static_ty);
            TypedExpr::TypeOf {
                ty: reflection_ty,
                span: *span,
                static_ty,
            }
        },

        AstExpr::Error { span } => TypedExpr::Error {
            ty: ctx.interner.error(),
            span: *span,
        },
    }
}

pub(super) fn check_block(
    ctx: &mut CheckCtx,
    stmts: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    check_block_stmts(ctx, stmts, span)
}

/// Check a list of statements as a block expression.
/// The type of the block is the type of the last expression-statement (if it's an Expr without
/// semicolon), otherwise Void.
pub fn check_block_stmts(
    ctx: &mut CheckCtx,
    stmts: &[crate::ast::stmt::AstStmt],
    span: SimpleSpan,
) -> TypedExpr {
    ctx.local_env.push_scope();

    let mut typed_stmts = Vec::new();
    for stmt in stmts {
        typed_stmts.push(super::check_stmt::check_stmt(ctx, stmt));
    }

    ctx.local_env.pop_scope();

    // Block type: type of last Expr statement, or void
    let block_ty = typed_stmts
        .last()
        .and_then(|s| match s {
            TypedStmt::Expr { expr, .. } => Some(expr.ty()),
            TypedStmt::Return { .. } => Some(ctx.interner.void()),
            _ => None,
        })
        .unwrap_or_else(|| ctx.interner.void());

    TypedExpr::Block {
        ty: block_ty,
        span,
        stmts: typed_stmts,
        tail: None,
    }
}

/// Check whether an assignment target is mutable. If not, emit an error.
pub fn check_assignment_mutability(ctx: &mut CheckCtx, target: &TypedExpr, assignment_span: SimpleSpan) {
    if let Some((name, mutability, binding_span)) = find_root_binding(target, &ctx.local_env)
        && mutability == super::env::Mutability::Immutable {
            // Determine if this is a simple reassignment or a field mutation
            match target {
                TypedExpr::Var { .. } => {
                    ctx.diags.push(TypeError::ImmutableReassignment {
                        binding_name: name,
                        binding_span,
                        assignment_span,
                        file: ctx.current_file,
                    }.into());
                }
                TypedExpr::Index { receiver, .. } => {
                    // Arrays are reference types: index-assignment mutates the heap object,
                    // not the binding. Allow arr[i] = v even when arr is an immutable binding,
                    // provided the receiver is an array type.
                    let recv_kind = ctx.interner.kind(receiver.ty()).clone();
                    if matches!(recv_kind, TyKind::Array(_)) {
                        // Allowed — index-assigning into an array reference is always legal.
                    } else {
                        ctx.diags.push(TypeError::ImmutableMutation {
                            binding_name: name,
                            binding_span,
                            mutation_span: assignment_span,
                            mutation_kind: "field assignment".to_string(),
                            file: ctx.current_file,
                        }.into());
                    }
                }
                TypedExpr::Field { .. } => {
                    ctx.diags.push(TypeError::ImmutableMutation {
                        binding_name: name,
                        binding_span,
                        mutation_span: assignment_span,
                        mutation_kind: "field assignment".to_string(),
                        file: ctx.current_file,
                    }.into());
                }
                _ => {}
            }
        }
}

/// Walk a TypedExpr to find its root variable binding.
fn find_root_binding(expr: &TypedExpr, local_env: &LocalEnv) -> Option<(String, super::env::Mutability, SimpleSpan)> {
    match expr {
        TypedExpr::Var { name, .. } => {
            local_env.lookup(name).map(|(_, m, sp)| (name.clone(), m, sp))
        }
        TypedExpr::SelfRef { .. } => {
            local_env.lookup("self").map(|(_, m, sp)| ("self".to_string(), m, sp))
        }
        TypedExpr::Field { receiver, .. } => find_root_binding(receiver, local_env),
        TypedExpr::Index { receiver, .. } => find_root_binding(receiver, local_env),
        _ => None,
    }
}

/// Find a single function DefId by name (non-overloaded fast path).
/// For overload resolution, use `find_fn_candidates` instead.
pub(super) fn find_fn_def_id(ctx: &CheckCtx, name: &str) -> Option<DefId> {
    let candidates = find_fn_candidates(ctx, name);
    candidates.first().copied()
}

/// Find all function DefId candidates by name, supporting overloaded functions.
///
/// Conditional fn DefIds are filtered out so the checker always resolves calls
/// to the non-conditional fallback (COND-04: args still type-check when call is elided).
pub(super) fn find_fn_candidates(ctx: &CheckCtx, name: &str) -> Vec<DefId> {
    // Check by simple name in by_fqn / fn_overloads
    let candidates = ctx.def_map.get_fn_candidates(name);
    let fn_candidates: Vec<DefId> = candidates.into_iter()
        .filter(|&id| {
            let entry = ctx.def_map.get_entry(id);
            matches!(entry.kind, DefKind::Fn | DefKind::ExternFn)
        })
        .filter(|id| !ctx.type_env.conditional_fns.contains_key(id))
        .collect();
    if !fn_candidates.is_empty() {
        return fn_candidates;
    }

    // Check file-private (including overloads)
    for &file_id in ctx.def_map.file_private.keys() {
        let candidates = ctx.def_map.get_private_fn_candidates(file_id, name);
        let fn_candidates: Vec<DefId> = candidates.into_iter()
            .filter(|&id| {
                let entry = ctx.def_map.get_entry(id);
                matches!(entry.kind, DefKind::Fn | DefKind::ExternFn)
            })
            .filter(|id| !ctx.type_env.conditional_fns.contains_key(id))
            .collect();
        if !fn_candidates.is_empty() {
            return fn_candidates;
        }
    }

    // Check all FQN entries that end with this name
    let suffix = format!("::{}", name);
    for (fqn, &def_id) in &ctx.def_map.by_fqn {
        if fqn.ends_with(&suffix) || fqn == name {
            let entry = ctx.def_map.get_entry(def_id);
            if matches!(entry.kind, DefKind::Fn | DefKind::ExternFn) {
                // Check for overloads on this FQN
                if let Some(overloads) = ctx.def_map.fn_overloads.get(fqn.as_str()) {
                    let filtered: Vec<DefId> = overloads.iter().copied()
                        .filter(|id| !ctx.type_env.conditional_fns.contains_key(id))
                        .collect();
                    return filtered;
                }
                // Single fn: skip if it is a conditional fn
                if ctx.type_env.conditional_fns.contains_key(&def_id) {
                    continue;
                }
                return vec![def_id];
            }
        }
    }

    vec![]
}
