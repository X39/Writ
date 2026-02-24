pub mod expr;
pub mod stmt;
pub mod decl;
pub mod types;

pub use expr::AstExpr;
pub use stmt::AstStmt;
pub use decl::AstDecl;
pub use types::AstType;

/// The lowered AST: a flat list of top-level declarations.
/// No CST sugar variants. No `'src` lifetime.
#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub items: Vec<AstDecl>,
}

impl Ast {
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }
}
