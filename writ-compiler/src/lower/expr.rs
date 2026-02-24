use writ_parser::cst::{
    Arg, AssignOp, BinaryOp as CstBinaryOp, Expr, LambdaParam, MatchArm, Pattern,
    PostfixOp as CstPostfixOp, PrefixOp as CstPrefixOp, RangeKind as CstRangeKind, Spanned,
};
use crate::ast::expr::{
    AstArg, AstExpr, AstLambdaParam, AstMatchArm, AstPattern, BinaryOp, PostfixOp, PrefixOp,
    RangeKind,
};
use crate::lower::context::LoweringContext;
use crate::lower::fmt_string::lower_fmt_string;
use crate::lower::optional::lower_type;
use crate::lower::stmt::lower_stmt;

/// Lowers a CST `Expr` into a lowered `AstExpr`.
///
/// This is the central recursive fold over all CST expression variants.
/// Key desugarings performed here:
///
/// - R3: `NullLit` → `Path { segments: ["Option", "None"] }`
/// - R4: `FormattableString` / `FormattableRawString` → concat chain via `lower_fmt_string`
/// - R5: Compound `Assign` (`+=`, `-=`, etc.) → expanded `Assign { Binary { ... } }`
///
/// All other variants are structural translations that recursively call `lower_expr`.
/// No `_ =>` wildcard — every `Expr` variant is matched explicitly.
pub fn lower_expr(spanned: Spanned<Expr<'_>>, ctx: &mut LoweringContext) -> AstExpr {
    let (expr, span) = spanned;
    match expr {
        // --- Literals ---

        Expr::IntLit(s) => AstExpr::IntLit {
            value: s.parse::<i64>().unwrap_or(0),
            span,
        },

        Expr::FloatLit(s) => AstExpr::FloatLit {
            value: s.parse::<f64>().unwrap_or(0.0),
            span,
        },

        Expr::StringLit(s) => AstExpr::StringLit {
            value: s.to_string(),
            span,
        },

        Expr::BoolLit(b) => AstExpr::BoolLit { value: b, span },

        // R3: null → Option::None path expression
        Expr::NullLit => AstExpr::Path {
            segments: vec!["Option".to_string(), "None".to_string()],
            span,
        },

        Expr::SelfLit => AstExpr::SelfLit { span },

        // --- Identifiers and paths ---

        Expr::Ident(s) => AstExpr::Ident {
            name: s.to_string(),
            span,
        },

        Expr::Path(segs) => AstExpr::Path {
            segments: segs.into_iter().map(|(s, _)| s.to_string()).collect(),
            span,
        },

        // --- Binary and unary operations ---

        Expr::Binary(lhs, op, rhs) => AstExpr::Binary {
            left: Box::new(lower_expr(*lhs, ctx)),
            op: lower_binop(op),
            right: Box::new(lower_expr(*rhs, ctx)),
            span,
        },

        Expr::UnaryPrefix(op, e) => AstExpr::UnaryPrefix {
            op: lower_prefix_op(op),
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::UnaryPostfix(e, op) => AstExpr::UnaryPostfix {
            expr: Box::new(lower_expr(*e, ctx)),
            op: lower_postfix_op(op),
            span,
        },

        // --- Access ---

        Expr::MemberAccess(obj, (field, field_span)) => AstExpr::MemberAccess {
            object: Box::new(lower_expr(*obj, ctx)),
            field: field.to_string(),
            field_span,
            span,
        },

        Expr::BracketAccess(obj, idx) => AstExpr::BracketAccess {
            object: Box::new(lower_expr(*obj, ctx)),
            index: Box::new(lower_expr(*idx, ctx)),
            span,
        },

        // --- Calls ---

        Expr::Call(callee, args) => AstExpr::Call {
            callee: Box::new(lower_expr(*callee, ctx)),
            args: args.into_iter().map(|a| lower_arg(a, ctx)).collect(),
            span,
        },

        Expr::GenericCall(callee, type_args, args) => AstExpr::GenericCall {
            callee: Box::new(lower_expr(*callee, ctx)),
            type_args: type_args.into_iter().map(lower_type).collect(),
            args: args.into_iter().map(|a| lower_arg(a, ctx)).collect(),
            span,
        },

        // --- Control flow (expression forms) ---

        Expr::If {
            condition,
            then_block,
            else_block,
        } => {
            AstExpr::If {
                condition: Box::new(lower_expr(*condition, ctx)),
                then_block: then_block
                    .into_iter()
                    .map(|s| lower_stmt(s, ctx))
                    .collect(),
                else_block: else_block.map(|e| Box::new(lower_expr(*e, ctx))),
                span,
            }
        }

        Expr::IfLet {
            pattern,
            value,
            then_block,
            else_block,
        } => {
            AstExpr::IfLet {
                pattern: Box::new(lower_pattern(*pattern, ctx)),
                value: Box::new(lower_expr(*value, ctx)),
                then_block: then_block
                    .into_iter()
                    .map(|s| lower_stmt(s, ctx))
                    .collect(),
                else_block: else_block.map(|e| Box::new(lower_expr(*e, ctx))),
                span,
            }
        }

        Expr::Match { scrutinee, arms } => {
            AstExpr::Match {
                scrutinee: Box::new(lower_expr(*scrutinee, ctx)),
                arms: arms
                    .into_iter()
                    .map(|(arm, arm_span)| lower_match_arm(arm, arm_span, ctx))
                    .collect(),
                span,
            }
        }

        Expr::Block(stmts) => {
            AstExpr::Block {
                stmts: stmts.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
                span,
            }
        }

        // --- Range ---

        Expr::Range(start, kind, end) => AstExpr::Range {
            start: start.map(|s| Box::new(lower_expr(*s, ctx))),
            kind: lower_range_kind(kind),
            end: end.map(|e| Box::new(lower_expr(*e, ctx))),
            span,
        },

        Expr::FromEnd(e) => AstExpr::FromEnd {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        // --- Lambda ---

        Expr::Lambda {
            params,
            return_type,
            body,
        } => {
            AstExpr::Lambda {
                params: params
                    .into_iter()
                    .map(|(lp, lp_span)| lower_lambda_param(lp, lp_span))
                    .collect(),
                return_type: return_type.map(|rt| Box::new(lower_type(*rt))),
                body: body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
                span,
            }
        }

        // --- Concurrency pass-through ---

        Expr::Spawn(e) => AstExpr::Spawn {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::Detached(e) => AstExpr::Detached {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::Join(e) => AstExpr::Join {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::Cancel(e) => AstExpr::Cancel {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::Defer(e) => AstExpr::Defer {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        Expr::Try(e) => AstExpr::Try {
            expr: Box::new(lower_expr(*e, ctx)),
            span,
        },

        // --- R4: Formattable strings → left-associative Add chain ---

        Expr::FormattableString(segs) => lower_fmt_string(segs, span, ctx),

        // Identical lowering to FormattableString — raw vs. non-raw distinction
        // is resolved by the lexer before the CST is constructed.
        Expr::FormattableRawString(segs) => lower_fmt_string(segs, span, ctx),

        // --- Array literal ---

        Expr::ArrayLit(elems) => AstExpr::ArrayLit {
            elements: elems
                .into_iter()
                .map(|e| lower_expr(e, ctx))
                .collect(),
            span,
        },

        // --- R5: Assignment and compound assignment ---

        Expr::Assign(lhs, op, rhs) => {
            let lowered_lhs = lower_expr(*lhs, ctx);
            let lowered_rhs = lower_expr(*rhs, ctx);

            match op {
                AssignOp::Assign => AstExpr::Assign {
                    target: Box::new(lowered_lhs),
                    value: Box::new(lowered_rhs),
                    span,
                },
                // a += b → a = a + b
                AssignOp::AddAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Add,
                        right: Box::new(lowered_rhs),
                        span,
                    }),
                    span,
                },
                // a -= b → a = a - b
                AssignOp::SubAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Sub,
                        right: Box::new(lowered_rhs),
                        span,
                    }),
                    span,
                },
                // a *= b → a = a * b
                AssignOp::MulAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Mul,
                        right: Box::new(lowered_rhs),
                        span,
                    }),
                    span,
                },
                // a /= b → a = a / b
                AssignOp::DivAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Div,
                        right: Box::new(lowered_rhs),
                        span,
                    }),
                    span,
                },
                // a %= b → a = a % b
                AssignOp::ModAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Mod,
                        right: Box::new(lowered_rhs),
                        span,
                    }),
                    span,
                },
            }
        }

        // --- Error recovery sentinel ---

        Expr::Error => AstExpr::Error { span },
    }
}

// =========================================================
// Operator mapping helpers
// =========================================================

fn lower_binop(op: CstBinaryOp) -> BinaryOp {
    match op {
        CstBinaryOp::Add => BinaryOp::Add,
        CstBinaryOp::Sub => BinaryOp::Sub,
        CstBinaryOp::Mul => BinaryOp::Mul,
        CstBinaryOp::Div => BinaryOp::Div,
        CstBinaryOp::Mod => BinaryOp::Mod,
        CstBinaryOp::Eq => BinaryOp::Eq,
        CstBinaryOp::NotEq => BinaryOp::NotEq,
        CstBinaryOp::Lt => BinaryOp::Lt,
        CstBinaryOp::Gt => BinaryOp::Gt,
        CstBinaryOp::LtEq => BinaryOp::LtEq,
        CstBinaryOp::GtEq => BinaryOp::GtEq,
        CstBinaryOp::And => BinaryOp::And,
        CstBinaryOp::Or => BinaryOp::Or,
        CstBinaryOp::BitAnd => BinaryOp::BitAnd,
        CstBinaryOp::BitOr => BinaryOp::BitOr,
    }
}

fn lower_prefix_op(op: CstPrefixOp) -> PrefixOp {
    match op {
        CstPrefixOp::Neg => PrefixOp::Neg,
        CstPrefixOp::Not => PrefixOp::Not,
        CstPrefixOp::FromEnd => PrefixOp::FromEnd,
    }
}

fn lower_postfix_op(op: CstPostfixOp) -> PostfixOp {
    match op {
        CstPostfixOp::NullPropagate => PostfixOp::NullPropagate,
        CstPostfixOp::Unwrap => PostfixOp::Unwrap,
    }
}

fn lower_range_kind(kind: CstRangeKind) -> RangeKind {
    match kind {
        CstRangeKind::Exclusive => RangeKind::Exclusive,
        CstRangeKind::Inclusive => RangeKind::Inclusive,
    }
}

// =========================================================
// Helper: lower_arg
// =========================================================

fn lower_arg(spanned: Spanned<Arg<'_>>, ctx: &mut LoweringContext) -> AstArg {
    let (arg, arg_span) = spanned;
    AstArg {
        name: arg.name.map(|(n, _)| n.to_string()),
        value: lower_expr(arg.value, ctx),
        span: arg_span,
    }
}

// =========================================================
// Helper: lower_lambda_param
// =========================================================

fn lower_lambda_param(lp: LambdaParam<'_>, lp_span: chumsky::span::SimpleSpan) -> AstLambdaParam {
    AstLambdaParam {
        name: lp.name.0.to_string(),
        ty: lp.ty.map(lower_type),
        span: lp_span,
    }
}

// =========================================================
// Helper: lower_match_arm
// =========================================================

fn lower_match_arm(
    arm: MatchArm<'_>,
    arm_span: chumsky::span::SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstMatchArm {
    AstMatchArm {
        pattern: lower_pattern(arm.pattern, ctx),
        body: arm.body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
        span: arm_span,
    }
}

// =========================================================
// Helper: lower_pattern
// =========================================================

/// Lowers a CST `Pattern` into a lowered `AstPattern`.
pub fn lower_pattern(spanned: Spanned<Pattern<'_>>, ctx: &mut LoweringContext) -> AstPattern {
    let (pattern, span) = spanned;
    match pattern {
        Pattern::Literal(expr) => AstPattern::Literal {
            expr: Box::new(lower_expr(expr, ctx)),
            span,
        },

        Pattern::Wildcard => AstPattern::Wildcard { span },

        Pattern::Variable(s) => AstPattern::Variable {
            name: s.to_string(),
            span,
        },

        Pattern::EnumDestructure(path_segs, fields) => AstPattern::EnumDestructure {
            path: path_segs.into_iter().map(|(s, _)| s.to_string()).collect(),
            fields: fields
                .into_iter()
                .map(|p| lower_pattern(p, ctx))
                .collect(),
            span,
        },

        Pattern::Or(patterns) => AstPattern::Or {
            patterns: patterns
                .into_iter()
                .map(|p| lower_pattern(p, ctx))
                .collect(),
            span,
        },

        Pattern::Range(start, kind, end) => AstPattern::Range {
            start: Box::new(lower_expr(*start, ctx)),
            kind: lower_range_kind(kind),
            end: Box::new(lower_expr(*end, ctx)),
            span,
        },
    }
}
