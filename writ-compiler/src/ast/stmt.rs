use chumsky::span::SimpleSpan;
use crate::ast::expr::AstExpr;
use crate::ast::types::AstType;

/// All statement forms that survive lowering into the AST.
///
/// Key invariants:
/// - NO `DlgDecl` variant — dialogue is lowered to `Fn` before reaching the AST.
/// - NO `Transition` variant — dialogue transitions lower to `Return` statements.
/// - `Atomic` survives as-is.
/// - YES `Error` variant for error recovery (R1).
/// - All data is owned (`String`, `Box<T>`, `Vec<T>`) — no `'src` lifetime.
/// - Every variant carries `span: SimpleSpan` — no exceptions.
#[derive(Debug, Clone, PartialEq)]
pub enum AstStmt {
    /// Variable declaration: `let [mut] name [: type] = expr;`
    Let {
        mutable: bool,
        name: String,
        name_span: SimpleSpan,
        ty: Option<AstType>,
        value: AstExpr,
        span: SimpleSpan,
    },
    /// Expression statement: `expr;`
    Expr { expr: AstExpr, span: SimpleSpan },
    /// For loop: `for name in expr { body }`
    For {
        binding: String,
        binding_span: SimpleSpan,
        iterable: AstExpr,
        body: Vec<AstStmt>,
        span: SimpleSpan,
    },
    /// While loop: `while condition { body }`
    While { condition: AstExpr, body: Vec<AstStmt>, span: SimpleSpan },
    /// Break: `break [expr]`
    Break { value: Option<AstExpr>, span: SimpleSpan },
    /// Continue
    Continue { span: SimpleSpan },
    /// Return: `return [expr]`
    Return { value: Option<AstExpr>, span: SimpleSpan },
    /// Atomic block: `atomic { body }`
    Atomic { body: Vec<AstStmt>, span: SimpleSpan },
    /// Error recovery sentinel
    Error { span: SimpleSpan },
}
