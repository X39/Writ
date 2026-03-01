use writ_parser::cst::{Spanned, TypeExpr};
use crate::ast::types::AstType;

/// Folds a CST `TypeExpr` into a lowered `AstType`.
///
/// Key lowering:
/// - `TypeExpr::Nullable(T)` → `AstType::Generic { name: "Option", args: [lower_type(T)] }`
///
/// This function is stateless — no `LoweringContext` needed, because all type
/// lowering in Phase 2 is a pure structural rewrite.
pub fn lower_type(spanned: Spanned<TypeExpr<'_>>) -> AstType {
    let (ty, span) = spanned;
    match ty {
        TypeExpr::Named(name) => AstType::Named {
            name: name.to_string(),
            span,
        },

        TypeExpr::Qualified { segments, rooted } => AstType::Named {
            name: {
                let joined: String = segments.iter().map(|(seg, _)| *seg).collect::<Vec<_>>().join("::");
                if rooted {
                    format!("::{}", joined)
                } else {
                    joined
                }
            },
            span,
        },

        TypeExpr::Generic(base, args) => {
            let name = match base.0 {
                TypeExpr::Named(n) => n.to_string(),
                TypeExpr::Qualified { segments, rooted } => {
                    let joined: String = segments.iter().map(|(s, _)| *s).collect::<Vec<_>>().join("::");
                    if rooted {
                        format!("::{}", joined)
                    } else {
                        joined
                    }
                }
                // Non-Named/Qualified generic bases do not occur in valid Writ programs.
                _ => todo!("non-Named/Qualified generic base in TypeExpr::Generic"),
            };
            AstType::Generic {
                name,
                args: args.into_iter().map(lower_type).collect(),
                span,
            }
        }

        TypeExpr::Array(elem) => AstType::Array {
            elem: Box::new(lower_type(*elem)),
            span,
        },

        // R3 CORE: T? → Option<T>
        TypeExpr::Nullable(inner) => AstType::Generic {
            name: "Option".to_string(),
            args: vec![lower_type(*inner)],
            span,
        },

        TypeExpr::Func(params, ret) => AstType::Func {
            params: params.into_iter().map(lower_type).collect(),
            ret: ret.map(|r| Box::new(lower_type(*r))),
            span,
        },

        TypeExpr::Void => AstType::Void { span },
    }
}
