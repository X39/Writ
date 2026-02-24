use chumsky::span::SimpleSpan;

/// Lowered type representation.
///
/// Key invariants:
/// - NO `Nullable` variant — `T?` lowers to `Generic { name: "Option", args: [T] }` before reaching AST.
/// - Every variant carries `span: SimpleSpan` — no exceptions.
/// - All data is owned (`String`, `Box<T>`, `Vec<T>`) — no `'src` lifetime.
#[derive(Debug, Clone, PartialEq)]
pub enum AstType {
    /// Named type: `int`, `string`, `Guard`
    Named { name: String, span: SimpleSpan },
    /// Generic type: `Option<T>`, `List<T>`, `Result<A, B>`
    ///
    /// NOTE: `T?` lowers to `Generic { name: "Option", args: [T] }` — no Nullable variant.
    Generic { name: String, args: Vec<AstType>, span: SimpleSpan },
    /// Array type: `T[]`
    Array { elem: Box<AstType>, span: SimpleSpan },
    /// Function type: `fn(int, string) -> bool`
    Func { params: Vec<AstType>, ret: Option<Box<AstType>>, span: SimpleSpan },
    /// Void type
    Void { span: SimpleSpan },
}
