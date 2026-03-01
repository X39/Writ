//! CST (Concrete Syntax Tree) types for the Writ parser.
//!
//! Every CST node carries its byte-offset span via `Spanned<T>`.
//! Trivia (whitespace, comments) is preserved using the Roslyn-style
//! leading/trailing attachment convention.

use chumsky::span::SimpleSpan;

/// Every CST node carries its byte-offset span.
/// `Spanned<T>` wraps a value with its source location.
pub type Spanned<T> = (T, SimpleSpan);

/// Trivia: whitespace and comments preserved for full-fidelity roundtrip.
///
/// Attached to CST nodes as leading/trailing fields (Roslyn convention):
/// - **Trailing trivia**: whitespace/comments after a token on the same line
///   attach to that token.
/// - **Leading trivia**: everything else (newlines, next-line whitespace/comments)
///   attaches to the following token.
#[derive(Debug, Clone, PartialEq)]
pub enum Trivia {
    /// Whitespace (spaces, tabs, newlines).
    Whitespace(String),
    /// Single-line comment (`// ...`).
    LineComment(String),
    /// Block comment (`/* ... */`), may be nested.
    BlockComment(String),
}

/// A CST token with Roslyn-style leading and trailing trivia.
///
/// This wraps any token kind `T` with its span and attached trivia,
/// enabling lossless source roundtrip.
#[derive(Debug, Clone, PartialEq)]
pub struct CstToken<T> {
    /// Trivia appearing before this token (leading whitespace, comments from
    /// previous lines, etc.).
    pub leading_trivia: Vec<Spanned<Trivia>>,
    /// The token payload.
    pub kind: T,
    /// The byte-offset span of this token in the source.
    pub span: SimpleSpan,
    /// Trivia appearing after this token on the same line.
    pub trailing_trivia: Vec<Spanned<Trivia>>,
}

/// Top-level program node.
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'src> {
    /// Top-level items in the program.
    pub items: Vec<Spanned<Item<'src>>>,
}

/// Top-level item in a Writ program.
///
/// Each variant represents a declaration form that can appear at
/// the top level (or inside namespace blocks).
#[derive(Debug, Clone, PartialEq)]
pub enum Item<'src> {
    /// Namespace declaration: `namespace a::b;` or `namespace a::b { ... }`
    Namespace(Spanned<NamespaceDecl<'src>>),
    /// Using import: `using [alias =] qualified::name;`
    Using(Spanned<UsingDecl<'src>>),
    /// Function declaration: `[vis] fn name(...) [-> type] { body }`
    Fn(Spanned<FnDecl<'src>>),
    /// Dialogue declaration: `[vis] dlg name[(params)] { body }`
    Dlg(Spanned<DlgDecl<'src>>),
    /// Struct declaration: `[vis] struct Name [<generics>] { fields }`
    Struct(Spanned<StructDecl<'src>>),
    /// Enum declaration: `[vis] enum Name [<generics>] { variants }`
    Enum(Spanned<EnumDecl<'src>>),
    /// Contract declaration: `[vis] contract Name [<generics>] { signatures }`
    Contract(Spanned<ContractDecl<'src>>),
    /// Impl block: `impl [Contract for] Type { members }`
    Impl(Spanned<ImplDecl<'src>>),
    /// Entity declaration: `[vis] entity Name { members }`
    Entity(Spanned<EntityDecl<'src>>),
    /// Component declaration: `[vis] component Name { members }`
    Component(Spanned<ComponentDecl<'src>>),
    /// Extern declaration: `extern fn|struct|component ...`
    Extern(Spanned<ExternDecl<'src>>),
    /// Constant declaration: `[vis] const name: type = expr;`
    Const(Spanned<ConstDecl<'src>>),
    /// Global mutable: `[vis] global mut name: type = expr;`
    Global(Spanned<GlobalDecl<'src>>),
    /// Backward-compatible: bare statements at top level
    Stmt(Spanned<Stmt<'src>>),
}

// =========================================================
// Visibility and Attributes
// =========================================================

/// Visibility modifier on declarations.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    /// `pub` -- public visibility
    Pub,
    /// `priv` -- private visibility
    Priv,
}

/// An attribute: `[Name]` or `[Name(args)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'src> {
    /// Attribute name.
    pub name: Spanned<&'src str>,
    /// Optional arguments.
    pub args: Vec<Spanned<AttrArg<'src>>>,
}

/// An attribute argument: positional or named.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrArg<'src> {
    /// Positional argument: `expr`
    Positional(Spanned<Expr<'src>>),
    /// Named argument: `name: expr`
    Named(Spanned<&'src str>, Spanned<Expr<'src>>),
}

// =========================================================
// Declaration Types
// =========================================================

/// Function declaration: `[attrs] [vis] fn name [<generics>] (params) [-> type] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl<'src> {
    /// Stacked attribute blocks (each inner Vec is one `[...]` block).
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Function name.
    pub name: Spanned<&'src str>,
    /// Optional generic parameters.
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Parameter list (may include self/mut self as first param).
    pub params: Vec<Spanned<FnParam<'src>>>,
    /// Optional return type.
    pub return_type: Option<Spanned<TypeExpr<'src>>>,
    /// Function body statements.
    pub body: Vec<Spanned<Stmt<'src>>>,
}

/// Function signature (no body): `fn name [<generics>] (params) [-> type]`
///
/// Used in contracts and extern declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct FnSig<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Optional qualifier for dotted names: `Type.method` -> qualifier = Some("Type")
    pub qualifier: Option<Spanned<&'src str>>,
    /// Function name.
    pub name: Spanned<&'src str>,
    /// Optional generic parameters.
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Parameter list (may include self/mut self as first param).
    pub params: Vec<Spanned<FnParam<'src>>>,
    /// Optional return type.
    pub return_type: Option<Spanned<TypeExpr<'src>>>,
}

/// Namespace declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum NamespaceDecl<'src> {
    /// Declarative form: `namespace a::b::c;`
    Declarative(Vec<Spanned<&'src str>>),
    /// Block form: `namespace a::b { items }`
    Block(Vec<Spanned<&'src str>>, Vec<Spanned<Item<'src>>>),
}

/// Using import: `using [alias =] qualified::name;`
#[derive(Debug, Clone, PartialEq)]
pub struct UsingDecl<'src> {
    /// Optional alias: `alias = ...`
    pub alias: Option<Spanned<&'src str>>,
    /// Qualified path segments.
    pub path: Vec<Spanned<&'src str>>,
}

/// Constant declaration: `[attrs] [vis] const name: type = expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Constant name.
    pub name: Spanned<&'src str>,
    /// Type annotation.
    pub ty: Spanned<TypeExpr<'src>>,
    /// Value expression.
    pub value: Spanned<Expr<'src>>,
}

/// Global mutable: `[attrs] [vis] global mut name: type = expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Global name.
    pub name: Spanned<&'src str>,
    /// Type annotation.
    pub ty: Spanned<TypeExpr<'src>>,
    /// Initial value expression.
    pub value: Spanned<Expr<'src>>,
}

/// Struct declaration: `[attrs] [vis] struct Name [<generics>] { fields }`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Struct name.
    pub name: Spanned<&'src str>,
    /// Optional generic parameters.
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Struct members: fields and lifecycle hooks interleaved.
    pub members: Vec<Spanned<StructMember<'src>>>,
}

/// A struct field: `[vis] name: type [= default]`
#[derive(Debug, Clone, PartialEq)]
pub struct StructField<'src> {
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Field name.
    pub name: Spanned<&'src str>,
    /// Type annotation.
    pub ty: Spanned<TypeExpr<'src>>,
    /// Optional default value.
    pub default: Option<Spanned<Expr<'src>>>,
}

/// A member inside a struct body: field or lifecycle hook.
#[derive(Debug, Clone, PartialEq)]
pub enum StructMember<'src> {
    /// Field: `[vis] name: type [= default]`
    Field(StructField<'src>),
    /// Lifecycle hook: `on event { body }`
    OnHook {
        event: Spanned<&'src str>,
        body: Vec<Spanned<Stmt<'src>>>,
    },
}

/// Enum declaration: `[attrs] [vis] enum Name [<generics>] { variants }`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Enum name.
    pub name: Spanned<&'src str>,
    /// Optional generic parameters.
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Enum variants.
    pub variants: Vec<Spanned<EnumVariant<'src>>>,
}

/// An enum variant: `Name` or `Name(fields)`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant<'src> {
    /// Variant name.
    pub name: Spanned<&'src str>,
    /// Optional tuple fields (named, using Param struct).
    pub fields: Option<Vec<Spanned<Param<'src>>>>,
}

/// Contract declaration: `[attrs] [vis] contract Name [<generics>] { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Contract name.
    pub name: Spanned<&'src str>,
    /// Optional generic parameters.
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Contract members (function signatures, operator signatures).
    pub members: Vec<Spanned<ContractMember<'src>>>,
}

/// A contract member: function signature or operator signature.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractMember<'src> {
    /// Function signature (no body).
    FnSig(FnSig<'src>),
    /// Operator signature (no body).
    OpSig(OpSig<'src>),
}

/// Operator signature (no body): `operator SYMBOL (params) [-> type]`
#[derive(Debug, Clone, PartialEq)]
pub struct OpSig<'src> {
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Operator symbol.
    pub symbol: Spanned<OpSymbol>,
    /// Parameter list.
    pub params: Vec<Spanned<Param<'src>>>,
    /// Optional return type.
    pub return_type: Option<Spanned<TypeExpr<'src>>>,
}

/// Operator symbols for overloading.
#[derive(Debug, Clone, PartialEq)]
pub enum OpSymbol {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Lt,
    Not,
    Index,
    IndexSet,
    BitAnd,
    BitOr,
}

/// Operator declaration with body: `operator SYMBOL (params) [-> type] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct OpDecl<'src> {
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Operator symbol.
    pub symbol: Spanned<OpSymbol>,
    /// Parameter list.
    pub params: Vec<Spanned<Param<'src>>>,
    /// Optional return type.
    pub return_type: Option<Spanned<TypeExpr<'src>>>,
    /// Operator body statements.
    pub body: Vec<Spanned<Stmt<'src>>>,
}

/// Impl block: `impl [<generics>] [Contract for] Type { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl<'src> {
    /// Optional generic parameters: `impl<T> ...`
    pub generics: Option<Vec<Spanned<GenericParam<'src>>>>,
    /// Optional contract being implemented.
    pub contract: Option<Spanned<TypeExpr<'src>>>,
    /// Target type.
    pub target: Spanned<TypeExpr<'src>>,
    /// Impl members (functions and operators).
    pub members: Vec<Spanned<ImplMember<'src>>>,
}

/// An impl member: function or operator declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplMember<'src> {
    /// Function declaration.
    Fn(Spanned<FnDecl<'src>>),
    /// Operator declaration.
    Op(Spanned<OpDecl<'src>>),
}

/// Entity declaration: `[attrs] [vis] entity Name { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Entity name.
    pub name: Spanned<&'src str>,
    /// Entity members.
    pub members: Vec<Spanned<EntityMember<'src>>>,
}

/// An entity member: property, use clause, function, or on handler.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityMember<'src> {
    /// Property: `[vis] name: type [= default]`
    Property {
        vis: Option<Visibility>,
        name: Spanned<&'src str>,
        ty: Spanned<TypeExpr<'src>>,
        default: Option<Spanned<Expr<'src>>>,
    },
    /// Use clause: `use Component { field: expr, ... }`
    Use {
        component: Spanned<&'src str>,
        fields: Vec<Spanned<UseField<'src>>>,
    },
    /// Function method.
    Fn(Spanned<FnDecl<'src>>),
    /// On handler: `on event [(params)] { body }`
    On {
        event: Spanned<&'src str>,
        params: Option<Vec<Spanned<Param<'src>>>>,
        body: Vec<Spanned<Stmt<'src>>>,
    },
}

/// A field assignment in `use Component { field: expr }`.
#[derive(Debug, Clone, PartialEq)]
pub struct UseField<'src> {
    /// Field name.
    pub name: Spanned<&'src str>,
    /// Field value expression.
    pub value: Spanned<Expr<'src>>,
}

/// Component declaration: `[attrs] [vis] component Name { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDecl<'src> {
    /// Stacked attribute blocks.
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Component name.
    pub name: Spanned<&'src str>,
    /// Component members (fields and methods).
    pub members: Vec<Spanned<ComponentMember<'src>>>,
}

/// A component member: field or method.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentMember<'src> {
    /// Field (same as struct field).
    Field(Spanned<StructField<'src>>),
    /// Method.
    Fn(Spanned<FnDecl<'src>>),
}

/// Extern declaration: `[vis] extern fn|struct|component ...`
#[derive(Debug, Clone, PartialEq)]
pub enum ExternDecl<'src> {
    /// Extern function (signature only): `[vis] extern fn name(...) [-> type];`
    Fn(Option<Visibility>, Spanned<FnSig<'src>>),
    /// Extern struct: `[vis] extern struct Name { fields }`
    Struct(Option<Visibility>, Spanned<StructDecl<'src>>),
    /// Extern component: `[vis] extern component Name { fields }`
    Component(Option<Visibility>, Spanned<ComponentDecl<'src>>),
}

// =========================================================
// Type Expressions
// =========================================================

/// Type expressions in Writ source code.
///
/// Covers simple named types, qualified paths, generic types, array types,
/// nullable types, function types, and void.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<'src> {
    /// Simple named type: `int`, `string`, `Entity`
    Named(&'src str),
    /// Qualified type path: `a::b::Type`, `::std::collections::Map`
    Qualified {
        segments: Vec<Spanned<&'src str>>,
        rooted: bool,
    },
    /// Generic type: `List<T>`, `Result<A, B>`
    Generic(Box<Spanned<TypeExpr<'src>>>, Vec<Spanned<TypeExpr<'src>>>),
    /// Array type: `T[]`
    Array(Box<Spanned<TypeExpr<'src>>>),
    /// Nullable type: `T?`
    Nullable(Box<Spanned<TypeExpr<'src>>>),
    /// Function type: `fn(int, string) -> bool`
    Func(Vec<Spanned<TypeExpr<'src>>>, Option<Box<Spanned<TypeExpr<'src>>>>),
    /// Void type
    Void,
}

/// Generic parameter for declaration-site generics: `<T: Bound + Other>`.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam<'src> {
    pub name: Spanned<&'src str>,
    pub bounds: Vec<Spanned<TypeExpr<'src>>>,
}

// =========================================================
// Expressions
// =========================================================

/// All expression forms in Writ.
///
/// Includes literals, identifiers, binary/unary operations, access forms,
/// calls, control flow expressions, ranges, lambdas, concurrency keywords,
/// formattable strings, array literals, and assignments.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    // Literals
    /// Integer literal: `42`, `1_000`
    IntLit(&'src str),
    /// Float literal: `3.14`, `1.5e10`
    FloatLit(&'src str),
    /// String literal: `"hello"`
    StringLit(&'src str),
    /// Boolean literal: `true`, `false`
    BoolLit(bool),
    /// Null literal: `null`
    NullLit,
    /// Self literal: `self`
    SelfLit,

    // Identifiers and paths
    /// Identifier: `x`, `foo`
    Ident(&'src str),
    /// Path expression: `a::b::c`, `::module::func`
    Path {
        segments: Vec<Spanned<&'src str>>,
        rooted: bool,
    },

    // Binary operations
    /// Binary operation: `a + b`, `x == y`
    Binary(Box<Spanned<Expr<'src>>>, BinaryOp, Box<Spanned<Expr<'src>>>),

    // Unary operations
    /// Unary prefix operation: `-x`, `!flag`, `^idx`
    UnaryPrefix(PrefixOp, Box<Spanned<Expr<'src>>>),
    /// Unary postfix operation: `val?`, `val!`
    UnaryPostfix(Box<Spanned<Expr<'src>>>, PostfixOp),

    // Access
    /// Member access: `obj.field`
    MemberAccess(Box<Spanned<Expr<'src>>>, Spanned<&'src str>),
    /// Bracket access: `arr[0]`, `entity[Health]`
    BracketAccess(Box<Spanned<Expr<'src>>>, Box<Spanned<Expr<'src>>>),

    // Calls
    /// Function call or construction: `foo(a, b)`, `Point(x: 1, y: 2)`
    Call(Box<Spanned<Expr<'src>>>, Vec<Spanned<Arg<'src>>>),
    /// Generic call: `f<T>(args)`
    GenericCall(Box<Spanned<Expr<'src>>>, Vec<Spanned<TypeExpr<'src>>>, Vec<Spanned<Arg<'src>>>),

    /// New construction: `new Type { field: value, ... }`
    New {
        ty: Box<Spanned<TypeExpr<'src>>>,
        fields: Vec<Spanned<NewField<'src>>>,
    },

    // Control flow (expression forms)
    /// If expression: `if cond { } else { }`
    If {
        condition: Box<Spanned<Expr<'src>>>,
        then_block: Vec<Spanned<Stmt<'src>>>,
        else_block: Option<Box<Spanned<Expr<'src>>>>,
    },
    /// If-let expression: `if let Pattern = expr { } else { }`
    IfLet {
        pattern: Box<Spanned<Pattern<'src>>>,
        value: Box<Spanned<Expr<'src>>>,
        then_block: Vec<Spanned<Stmt<'src>>>,
        else_block: Option<Box<Spanned<Expr<'src>>>>,
    },
    /// Match expression: `match expr { arms }`
    Match {
        scrutinee: Box<Spanned<Expr<'src>>>,
        arms: Vec<Spanned<MatchArm<'src>>>,
    },
    /// Block expression: `{ stmts }`
    Block(Vec<Spanned<Stmt<'src>>>),

    // Range expressions
    /// Range expression: `a..b`, `a..=b`, `..b`, `a..`
    Range(Option<Box<Spanned<Expr<'src>>>>, RangeKind, Option<Box<Spanned<Expr<'src>>>>),
    /// From-end index: `^expr`
    FromEnd(Box<Spanned<Expr<'src>>>),

    // Lambda
    /// Lambda expression: `fn(x: int, y: int) -> int { x + y }`
    Lambda {
        params: Vec<Spanned<LambdaParam<'src>>>,
        return_type: Option<Box<Spanned<TypeExpr<'src>>>>,
        body: Vec<Spanned<Stmt<'src>>>,
    },

    // Concurrency
    /// Spawn expression: `spawn expr`
    Spawn(Box<Spanned<Expr<'src>>>),
    /// Spawn detached expression: `spawn detached expr` (fused, not nested)
    SpawnDetached(Box<Spanned<Expr<'src>>>),
    /// Join expression: `join expr`
    Join(Box<Spanned<Expr<'src>>>),
    /// Cancel expression: `cancel expr`
    Cancel(Box<Spanned<Expr<'src>>>),
    /// Defer expression: `defer expr`
    Defer(Box<Spanned<Expr<'src>>>),
    /// Try expression: `try expr`
    Try(Box<Spanned<Expr<'src>>>),

    // Formattable strings
    /// Formattable string: `$"Hello {name}!"`
    FormattableString(Vec<Spanned<StringSegment<'src>>>),
    /// Formattable raw string: `$"""Hello {name}!"""`
    FormattableRawString(Vec<Spanned<StringSegment<'src>>>),

    // Array literal
    /// Array literal: `[1, 2, 3]`
    ArrayLit(Vec<Spanned<Expr<'src>>>),

    // Assignment (as expression for compound assignment)
    /// Assignment: `x = 1`, `x += 1`
    Assign(Box<Spanned<Expr<'src>>>, AssignOp, Box<Spanned<Expr<'src>>>),

    // Error recovery sentinel
    /// Placeholder for recovered parse errors. Produced by error recovery
    /// strategies when a sub-expression fails to parse. Downstream passes
    /// should detect and skip these nodes.
    Error,
}

// =========================================================
// Supporting Enums for Expressions
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

/// Assignment operators.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
    /// `%=`
    ModAssign,
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
pub struct Arg<'src> {
    /// Named argument label (e.g., `field:` in `Point(x: 1)`), or None for positional.
    pub name: Option<Spanned<&'src str>>,
    /// The argument value expression.
    pub value: Spanned<Expr<'src>>,
}

/// A field initializer in a `new` expression: `name: expr`
#[derive(Debug, Clone, PartialEq)]
pub struct NewField<'src> {
    /// Field name.
    pub name: Spanned<&'src str>,
    /// Field value expression.
    pub value: Spanned<Expr<'src>>,
}

/// A lambda parameter with optional type annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam<'src> {
    /// Parameter name.
    pub name: Spanned<&'src str>,
    /// Optional type annotation.
    pub ty: Option<Spanned<TypeExpr<'src>>>,
}

/// A match arm: pattern followed by body statements.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm<'src> {
    /// The pattern to match against.
    pub pattern: Spanned<Pattern<'src>>,
    /// The body executed when the pattern matches.
    pub body: Vec<Spanned<Stmt<'src>>>,
}

/// A segment of a formattable string.
#[derive(Debug, Clone, PartialEq)]
pub enum StringSegment<'src> {
    /// Literal text segment.
    Text(&'src str),
    /// Interpolated expression segment: `{expr}`.
    Expr(Box<Spanned<Expr<'src>>>),
}

// =========================================================
// Patterns
// =========================================================

/// Pattern forms for match arms and if-let.
///
/// Seven forms per user decision: literal, wildcard, variable, enum destructuring,
/// or-pattern, and range pattern. Nested destructuring is achieved by nesting
/// patterns within EnumDestructure.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern<'src> {
    /// Literal pattern: `42`, `"key"`, `true`, `null`
    Literal(Spanned<Expr<'src>>),
    /// Wildcard pattern: `_`
    Wildcard,
    /// Variable binding pattern: `x`
    Variable(&'src str),
    /// Enum destructuring: `Result::Ok(val)`, `QuestStatus::InProgress(step)`
    EnumDestructure(Vec<Spanned<&'src str>>, Vec<Spanned<Pattern<'src>>>),
    /// Or-pattern: `A | B | C`
    Or(Vec<Spanned<Pattern<'src>>>),
    /// Range pattern: `1..=5`
    Range(Box<Spanned<Expr<'src>>>, RangeKind, Box<Spanned<Expr<'src>>>),
}

// =========================================================
// Statements
// =========================================================

/// All statement forms in Writ.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'src> {
    /// Variable declaration: `let [mut] name [: type] = expr;`
    Let {
        mutable: bool,
        name: Spanned<&'src str>,
        ty: Option<Spanned<TypeExpr<'src>>>,
        value: Spanned<Expr<'src>>,
    },
    /// Expression statement: `expr;`
    Expr(Spanned<Expr<'src>>),
    /// For loop: `for name in expr { ... }`
    For {
        binding: Spanned<&'src str>,
        iterable: Spanned<Expr<'src>>,
        body: Vec<Spanned<Stmt<'src>>>,
    },
    /// While loop: `while expr { ... }`
    While {
        condition: Spanned<Expr<'src>>,
        body: Vec<Spanned<Stmt<'src>>>,
    },
    /// Break statement: `break [expr]`
    Break(Option<Spanned<Expr<'src>>>),
    /// Continue statement: `continue`
    Continue,
    /// Return statement: `return [expr]`
    Return(Option<Spanned<Expr<'src>>>),
    /// Atomic block: `atomic { ... }`
    Atomic(Vec<Spanned<Stmt<'src>>>),
    /// Transition statement: `-> target(args)` (used in on handlers and dialogue)
    Transition(Spanned<DlgTransition<'src>>),
}

// =========================================================
// Dialogue
// =========================================================

/// A dialogue declaration: `[attrs] [vis] dlg name[(params)] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct DlgDecl<'src> {
    /// Stacked attribute blocks (each inner Vec is one `[...]` block).
    pub attrs: Vec<Spanned<Vec<Attribute<'src>>>>,
    /// Optional visibility modifier.
    pub vis: Option<Visibility>,
    /// Dialogue name identifier.
    pub name: Spanned<&'src str>,
    /// Optional parameter list (None = no parens, Some(vec![]) = empty parens).
    pub params: Option<Vec<Spanned<Param<'src>>>>,
    /// Dialogue body lines.
    pub body: Vec<Spanned<DlgLine<'src>>>,
}

/// A parameter with name and type annotation.
///
/// Reusable for `fn` params, dialogue params, enum variant fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Param<'src> {
    /// Parameter name.
    pub name: Spanned<&'src str>,
    /// Type annotation.
    pub ty: Spanned<TypeExpr<'src>>,
}

/// A function parameter entry: either a regular named param or a self param.
#[derive(Debug, Clone, PartialEq)]
pub enum FnParam<'src> {
    /// Regular parameter: `name: type`
    Regular(Param<'src>),
    /// Self parameter: `self` or `mut self`
    SelfParam { mutable: bool },
}

/// All dialogue line forms.
#[derive(Debug, Clone, PartialEq)]
pub enum DlgLine<'src> {
    /// `@speaker text #key` -- inline speaker with text on same logical line.
    SpeakerLine {
        speaker: Spanned<&'src str>,
        text: Vec<Spanned<DlgTextSegment<'src>>>,
        loc_key: Option<Spanned<&'src str>>,
    },
    /// `@speaker` standalone (no text follows on same logical line).
    SpeakerTag(Spanned<&'src str>),
    /// Plain text continuation after standalone speaker.
    TextLine {
        text: Vec<Spanned<DlgTextSegment<'src>>>,
        loc_key: Option<Spanned<&'src str>>,
    },
    /// `$ statement;` or `$ { block }` -- code escape.
    CodeEscape(Spanned<DlgEscape<'src>>),
    /// `$ choice { ... }` -- player choices.
    Choice(Spanned<DlgChoice<'src>>),
    /// `$ if condition { dialogue } else { dialogue }` -- conditional dialogue.
    If(Spanned<DlgIf<'src>>),
    /// `$ match expr { Pattern => { dialogue } ... }` -- match dialogue.
    Match(Spanned<DlgMatch<'src>>),
    /// `-> name` or `-> name(args)` -- dialogue transition.
    Transition(Spanned<DlgTransition<'src>>),
}

/// Text segment in dialogue content (mirrors `StringSegment`).
#[derive(Debug, Clone, PartialEq)]
pub enum DlgTextSegment<'src> {
    /// Literal text.
    Text(&'src str),
    /// Interpolated expression `{expr}`.
    Expr(Box<Spanned<Expr<'src>>>),
}

/// Code escape forms within dialogue.
#[derive(Debug, Clone, PartialEq)]
pub enum DlgEscape<'src> {
    /// `$ statement;` -- single code statement.
    Statement(Box<Spanned<Stmt<'src>>>),
    /// `$ { block }` -- code block.
    Block(Vec<Spanned<Stmt<'src>>>),
}

/// Choice block: `$ choice { arms }`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgChoice<'src> {
    /// Choice arms.
    pub arms: Vec<Spanned<DlgChoiceArm<'src>>>,
}

/// A single choice arm: `"label" [#key] { dialogue }`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgChoiceArm<'src> {
    /// Quoted string label.
    pub label: Spanned<&'src str>,
    /// Optional localization key `#key`.
    pub loc_key: Option<Spanned<&'src str>>,
    /// Nested dialogue body.
    pub body: Vec<Spanned<DlgLine<'src>>>,
}

/// Conditional dialogue: `$ if condition { then } [else { else }]`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgIf<'src> {
    /// Code condition expression.
    pub condition: Box<Spanned<Expr<'src>>>,
    /// Then-branch dialogue lines.
    pub then_block: Vec<Spanned<DlgLine<'src>>>,
    /// Optional else branch.
    pub else_block: Option<Box<Spanned<DlgElse<'src>>>>,
}

/// Else branch of a dialogue conditional.
#[derive(Debug, Clone, PartialEq)]
pub enum DlgElse<'src> {
    /// Chained `else if`.
    ElseIf(DlgIf<'src>),
    /// Final `else` block.
    Else(Vec<Spanned<DlgLine<'src>>>),
}

/// Match in dialogue: `$ match expr { pattern => { dialogue } ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgMatch<'src> {
    /// Scrutinee expression (code).
    pub scrutinee: Box<Spanned<Expr<'src>>>,
    /// Match arms.
    pub arms: Vec<Spanned<DlgMatchArm<'src>>>,
}

/// A match arm in dialogue: `pattern => { dialogue }`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgMatchArm<'src> {
    /// Pattern to match against (code pattern).
    pub pattern: Spanned<Pattern<'src>>,
    /// Dialogue body for this arm.
    pub body: Vec<Spanned<DlgLine<'src>>>,
}

/// Dialogue transition: `-> target` or `-> target(args)`.
#[derive(Debug, Clone, PartialEq)]
pub struct DlgTransition<'src> {
    /// Target dialogue name.
    pub target: Spanned<&'src str>,
    /// Optional arguments.
    pub args: Option<Vec<Spanned<Expr<'src>>>>,
}
