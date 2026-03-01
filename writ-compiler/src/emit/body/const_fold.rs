//! Constant folding for compile-time arithmetic evaluation.
//!
//! `const_fold` reduces constant expressions to literal values at compile time.
//! Used for `TypedDecl::Const` to emit a single LOAD_INT/LOAD_FLOAT instead of
//! emitting runtime arithmetic instructions.

use crate::ast::expr::{BinaryOp, PrefixOp};
use crate::check::ir::{TypedExpr, TypedLiteral};
use crate::check::ty::TyInterner;

/// Attempt to evaluate a TypedExpr as a compile-time constant.
///
/// Returns `Some(TypedLiteral)` if the expression can be fully evaluated,
/// or `None` if it contains any non-constant subexpressions (variables,
/// calls, field accesses, etc.).
pub fn const_fold(expr: &TypedExpr, interner: &TyInterner) -> Option<TypedLiteral> {
    match expr {
        // ── Literals are always foldable ─────────────────────────────────────
        TypedExpr::Literal { value, .. } => Some(value.clone()),

        // ── Binary arithmetic/logic ───────────────────────────────────────────
        TypedExpr::Binary { left, op, right, .. } => {
            let l = const_fold(left, interner)?;
            let r = const_fold(right, interner)?;
            fold_binary(op, l, r)
        }

        // ── Unary negation ────────────────────────────────────────────────────
        TypedExpr::UnaryPrefix { op, expr: inner, .. } => {
            let v = const_fold(inner, interner)?;
            match op {
                PrefixOp::Neg => match v {
                    TypedLiteral::Int(i) => Some(TypedLiteral::Int(-i)),
                    TypedLiteral::Float(f) => Some(TypedLiteral::Float(-f)),
                    _ => None,
                },
                PrefixOp::Not => match v {
                    TypedLiteral::Bool(b) => Some(TypedLiteral::Bool(!b)),
                    _ => None,
                },
                _ => None,
            }
        }

        // ── All other expressions are non-constant ────────────────────────────
        _ => None,
    }
}

fn fold_binary(op: &BinaryOp, l: TypedLiteral, r: TypedLiteral) -> Option<TypedLiteral> {
    match (op, l, r) {
        // Int arithmetic
        (BinaryOp::Add, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Int(a.wrapping_add(b)))
        }
        (BinaryOp::Sub, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Int(a.wrapping_sub(b)))
        }
        (BinaryOp::Mul, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Int(a.wrapping_mul(b)))
        }
        (BinaryOp::Div, TypedLiteral::Int(a), TypedLiteral::Int(b)) if b != 0 => {
            Some(TypedLiteral::Int(a / b))
        }
        (BinaryOp::Mod, TypedLiteral::Int(a), TypedLiteral::Int(b)) if b != 0 => {
            Some(TypedLiteral::Int(a % b))
        }

        // Float arithmetic
        (BinaryOp::Add, TypedLiteral::Float(a), TypedLiteral::Float(b)) => {
            Some(TypedLiteral::Float(a + b))
        }
        (BinaryOp::Sub, TypedLiteral::Float(a), TypedLiteral::Float(b)) => {
            Some(TypedLiteral::Float(a - b))
        }
        (BinaryOp::Mul, TypedLiteral::Float(a), TypedLiteral::Float(b)) => {
            Some(TypedLiteral::Float(a * b))
        }
        (BinaryOp::Div, TypedLiteral::Float(a), TypedLiteral::Float(b)) => {
            Some(TypedLiteral::Float(a / b))
        }
        (BinaryOp::Mod, TypedLiteral::Float(a), TypedLiteral::Float(b)) => {
            Some(TypedLiteral::Float(a % b))
        }

        // Boolean logic
        (BinaryOp::And, TypedLiteral::Bool(a), TypedLiteral::Bool(b)) => {
            Some(TypedLiteral::Bool(a && b))
        }
        (BinaryOp::Or, TypedLiteral::Bool(a), TypedLiteral::Bool(b)) => {
            Some(TypedLiteral::Bool(a || b))
        }

        // Comparison (produce Bool)
        (BinaryOp::Eq, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a == b))
        }
        (BinaryOp::NotEq, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a != b))
        }
        (BinaryOp::Lt, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a < b))
        }
        (BinaryOp::Gt, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a > b))
        }
        (BinaryOp::LtEq, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a <= b))
        }
        (BinaryOp::GtEq, TypedLiteral::Int(a), TypedLiteral::Int(b)) => {
            Some(TypedLiteral::Bool(a >= b))
        }

        _ => None,
    }
}
