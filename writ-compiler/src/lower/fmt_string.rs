use chumsky::span::SimpleSpan;
use writ_parser::cst::{Spanned, StringSegment};
use crate::ast::expr::{AstExpr, BinaryOp};
use crate::ast::types::AstType;
use crate::lower::context::LoweringContext;
use crate::lower::expr::lower_expr;

/// Lowers a formattable string's segment list into a left-associative
/// `AstExpr::Binary { op: BinaryOp::Add, ... }` chain.
///
/// Each `StringSegment::Text(s)` becomes `AstExpr::StringLit`.
/// Each `StringSegment::Expr(e)` becomes `AstExpr::GenericCall` wrapping
/// the lowered expression with `.into<string>()`.
///
/// Invariants:
/// - Empty segment list → `AstExpr::StringLit { value: "", span: outer_span }`
/// - All synthetic `Binary` nodes carry `outer_span` (not tombstone `0..0`)
/// - Interpolated expressions are recursively lowered via `lower_expr`
pub fn lower_fmt_string(
    segments: Vec<Spanned<StringSegment<'_>>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstExpr {
    if segments.is_empty() {
        return AstExpr::StringLit {
            value: String::new(),
            span: outer_span,
        };
    }

    let parts: Vec<AstExpr> = segments
        .into_iter()
        .map(|(seg, seg_span)| match seg {
            StringSegment::Text(s) => AstExpr::StringLit {
                value: s.to_string(),
                span: seg_span,
            },
            StringSegment::Expr(inner_expr) => {
                // Recursively lower the interpolated expression so any nested
                // sugar (compound assigns, null literals, nested fmt strings)
                // is fully lowered before wrapping.
                let lowered = lower_expr(*inner_expr, ctx);

                // Wrap with `.into<string>()` call:
                // lowered.into<string>()
                AstExpr::GenericCall {
                    callee: Box::new(AstExpr::MemberAccess {
                        object: Box::new(lowered),
                        field: "into".to_string(),
                        field_span: seg_span,
                        span: seg_span,
                    }),
                    type_args: vec![AstType::Named {
                        name: "string".to_string(),
                        span: seg_span,
                    }],
                    args: vec![],
                    span: seg_span,
                }
            }
        })
        .collect();

    // Left-associative fold: (((a + b) + c) + d) + ...
    let mut iter = parts.into_iter();
    let first = iter.next().expect("segments non-empty: checked above");
    iter.fold(first, |acc, next| AstExpr::Binary {
        left: Box::new(acc),
        op: BinaryOp::Add,
        right: Box::new(next),
        span: outer_span,
    })
}
