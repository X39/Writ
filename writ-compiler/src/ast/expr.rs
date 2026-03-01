use chumsky::span::SimpleSpan;
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;

/// All expression forms that survive lowering into the AST.
///
/// Key invariants:
/// - NO `FormattableString` — lowered to string concatenation chains before reaching AST.
/// - NO compound `Assign` variants — `+=` lowered to `a = a + b` before reaching AST.
/// - NO `NullLit` — lowered to `Option::None` (a path expression) before reaching AST.
/// - YES `Spawn`, `SpawnDetached`, `Join`, `Cancel`, `Defer` — concurrency pass-through (R1).
/// - YES `Error` — error recovery sentinel (R1).
/// - `Assign` is plain assignment only (`=`), no compound operators.
/// - All data is owned (`String`, `Box<T>`, `Vec<T>`) — no `'src` lifetime.
/// - Every variant carries `span: SimpleSpan` — no exceptions.
#[derive(Debug, Clone, PartialEq)]
pub enum AstExpr {
    // --- Literals ---

    /// Integer literal: `42`, `1_000`
    IntLit { value: i64, span: SimpleSpan },
    /// Float literal: `3.14`, `1.5e10`
    FloatLit { value: f64, span: SimpleSpan },
    /// String literal: `"hello"`
    StringLit { value: String, span: SimpleSpan },
    /// Boolean literal: `true`, `false`
    BoolLit { value: bool, span: SimpleSpan },
    /// Self literal: `self`
    SelfLit { span: SimpleSpan },
    // NOTE: No NullLit — lowered to path expression Option::None before reaching AST.

    // --- Identifiers and paths ---

    /// Identifier: `x`, `foo`
    Ident { name: String, span: SimpleSpan },
    /// Path expression: `a::b::c`
    Path { segments: Vec<String>, span: SimpleSpan },

    // --- Binary and unary operations ---

    /// Binary operation: `a + b`, `x == y`
    Binary { left: Box<AstExpr>, op: BinaryOp, right: Box<AstExpr>, span: SimpleSpan },
    /// Unary prefix operation: `-x`, `!flag`
    UnaryPrefix { op: PrefixOp, expr: Box<AstExpr>, span: SimpleSpan },
    /// Unary postfix operation: `val?`, `val!`
    UnaryPostfix { expr: Box<AstExpr>, op: PostfixOp, span: SimpleSpan },

    // --- Access ---

    /// Member access: `obj.field`
    MemberAccess { object: Box<AstExpr>, field: String, field_span: SimpleSpan, span: SimpleSpan },
    /// Bracket access: `arr[0]`, `entity[Health]`
    BracketAccess { object: Box<AstExpr>, index: Box<AstExpr>, span: SimpleSpan },

    // --- Calls ---

    /// Function call or construction: `foo(a, b)`, `Point(x: 1, y: 2)`
    Call { callee: Box<AstExpr>, args: Vec<AstArg>, span: SimpleSpan },
    /// Generic call: `f<T>(args)`
    GenericCall { callee: Box<AstExpr>, type_args: Vec<AstType>, args: Vec<AstArg>, span: SimpleSpan },

    // --- Control flow (expression forms) ---

    /// If expression: `if cond { } else { }`
    If {
        condition: Box<AstExpr>,
        then_block: Vec<AstStmt>,
        else_block: Option<Box<AstExpr>>,
        span: SimpleSpan,
    },
    /// If-let expression: `if let Pattern = expr { } else { }`
    IfLet {
        pattern: Box<AstPattern>,
        value: Box<AstExpr>,
        then_block: Vec<AstStmt>,
        else_block: Option<Box<AstExpr>>,
        span: SimpleSpan,
    },
    /// Match expression: `match expr { arms }`
    Match { scrutinee: Box<AstExpr>, arms: Vec<AstMatchArm>, span: SimpleSpan },
    /// Block expression: `{ stmts }`
    Block { stmts: Vec<AstStmt>, span: SimpleSpan },

    // --- Range ---

    /// Range expression: `a..b`, `a..=b`, `..b`, `a..`
    Range {
        start: Option<Box<AstExpr>>,
        kind: RangeKind,
        end: Option<Box<AstExpr>>,
        span: SimpleSpan,
    },
    /// From-end index: `^expr`
    FromEnd { expr: Box<AstExpr>, span: SimpleSpan },

    // --- Lambda ---

    /// Lambda expression: `fn(x: int, y: int) -> int { x + y }`
    Lambda {
        params: Vec<AstLambdaParam>,
        return_type: Option<Box<AstType>>,
        body: Vec<AstStmt>,
        span: SimpleSpan,
    },

    // --- Concurrency pass-through (R1: first-class AST nodes) ---

    /// Spawn expression: `spawn expr`
    Spawn { expr: Box<AstExpr>, span: SimpleSpan },
    /// Spawn detached expression: `spawn detached expr` (fused)
    SpawnDetached { expr: Box<AstExpr>, span: SimpleSpan },
    /// Join expression: `join expr`
    Join { expr: Box<AstExpr>, span: SimpleSpan },
    /// Cancel expression: `cancel expr`
    Cancel { expr: Box<AstExpr>, span: SimpleSpan },
    /// Defer expression: `defer expr`
    Defer { expr: Box<AstExpr>, span: SimpleSpan },
    /// Try expression: `try expr`
    Try { expr: Box<AstExpr>, span: SimpleSpan },

    // --- Array literal ---

    /// Array literal: `[1, 2, 3]`
    ArrayLit { elements: Vec<AstExpr>, span: SimpleSpan },

    // --- New construction ---

    /// New construction expression: `new Type { field: value }`
    New {
        ty: AstType,
        fields: Vec<AstNewField>,
        span: SimpleSpan,
    },

    // --- Assignment ---

    /// Plain assignment only (`=`).
    /// NOTE: Compound assignments (+=, -=, etc.) are lowered to `a = a op b` before reaching AST.
    Assign { target: Box<AstExpr>, value: Box<AstExpr>, span: SimpleSpan },

    // --- Error recovery sentinel ---

    /// Placeholder for recovered lowering errors.
    /// Downstream passes should detect and skip these nodes.
    Error { span: SimpleSpan },
}

// =========================================================
// Supporting types for AstExpr
// =========================================================

/// Binary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

/// Prefix (unary) operators.
#[derive(Debug, Clone, PartialEq)]
pub enum PrefixOp {
    /// Negation: `-`
    Neg,
    /// Logical not: `!`
    Not,
    /// From-end index: `^`
    FromEnd,
}

/// Postfix (unary) operators.
#[derive(Debug, Clone, PartialEq)]
pub enum PostfixOp {
    /// Null propagation: `?`
    NullPropagate,
    /// Unwrap: `!`
    Unwrap,
}

/// Range kind: exclusive (`..`) or inclusive (`..=`).
#[derive(Debug, Clone, PartialEq)]
pub enum RangeKind {
    /// Exclusive range: `..`
    Exclusive,
    /// Inclusive range: `..=`
    Inclusive,
}

/// A function/construction argument, optionally named.
#[derive(Debug, Clone, PartialEq)]
pub struct AstArg {
    /// Named argument label, or None for positional.
    pub name: Option<String>,
    /// The argument value expression.
    pub value: AstExpr,
    pub span: SimpleSpan,
}

/// A field initializer in a `new` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct AstNewField {
    pub name: String,
    pub name_span: SimpleSpan,
    pub value: AstExpr,
    pub span: SimpleSpan,
}

/// A lambda parameter with optional type annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct AstLambdaParam {
    /// Parameter name.
    pub name: String,
    /// Optional type annotation.
    pub ty: Option<AstType>,
    pub span: SimpleSpan,
}

/// A match arm: pattern followed by body statements.
#[derive(Debug, Clone, PartialEq)]
pub struct AstMatchArm {
    /// The pattern to match against.
    pub pattern: AstPattern,
    /// The body executed when the pattern matches.
    pub body: Vec<AstStmt>,
    pub span: SimpleSpan,
}

/// Pattern forms for match arms and if-let.
#[derive(Debug, Clone, PartialEq)]
pub enum AstPattern {
    /// Literal pattern: `42`, `"key"`, `true`
    Literal { expr: Box<AstExpr>, span: SimpleSpan },
    /// Wildcard pattern: `_`
    Wildcard { span: SimpleSpan },
    /// Variable binding pattern: `x`
    Variable { name: String, span: SimpleSpan },
    /// Enum destructuring: `Result::Ok(val)`, `QuestStatus::InProgress(step)`
    EnumDestructure { path: Vec<String>, fields: Vec<AstPattern>, span: SimpleSpan },
    /// Or-pattern: `A | B | C`
    Or { patterns: Vec<AstPattern>, span: SimpleSpan },
    /// Range pattern: `1..=5`
    Range { start: Box<AstExpr>, kind: RangeKind, end: Box<AstExpr>, span: SimpleSpan },
}
