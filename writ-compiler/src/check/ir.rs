//! Typed IR definitions for the Writ type checker.
//!
//! Every expression variant carries `ty: Ty` and `span: SimpleSpan` directly.
//! No `Option<Ty>` fields exist in this IR.

use chumsky::span::SimpleSpan;

use crate::ast::expr::{BinaryOp, PrefixOp};
use crate::resolve::def_map::{DefId, DefMap};

use super::ty::Ty;

/// The output of type checking: typed declarations plus the DefMap.
#[derive(Debug)]
pub struct TypedAst {
    pub decls: Vec<TypedDecl>,
    pub def_map: DefMap,
}

/// A typed expression. Every variant carries `ty: Ty` and `span: SimpleSpan`.
#[derive(Debug, Clone)]
pub enum TypedExpr {
    Literal {
        ty: Ty,
        span: SimpleSpan,
        value: TypedLiteral,
    },
    Var {
        ty: Ty,
        span: SimpleSpan,
        name: String,
    },
    SelfRef {
        ty: Ty,
        span: SimpleSpan,
    },
    Call {
        ty: Ty,
        span: SimpleSpan,
        callee: Box<TypedExpr>,
        args: Vec<TypedExpr>,
        callee_def_id: Option<DefId>,
    },
    Field {
        ty: Ty,
        span: SimpleSpan,
        receiver: Box<TypedExpr>,
        field: String,
    },
    ComponentAccess {
        ty: Ty,
        span: SimpleSpan,
        receiver: Box<TypedExpr>,
        component: String,
    },
    Index {
        ty: Ty,
        span: SimpleSpan,
        receiver: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Binary {
        ty: Ty,
        span: SimpleSpan,
        left: Box<TypedExpr>,
        op: BinaryOp,
        right: Box<TypedExpr>,
    },
    UnaryPrefix {
        ty: Ty,
        span: SimpleSpan,
        op: PrefixOp,
        expr: Box<TypedExpr>,
    },
    Match {
        ty: Ty,
        span: SimpleSpan,
        scrutinee: Box<TypedExpr>,
        arms: Vec<TypedArm>,
    },
    If {
        ty: Ty,
        span: SimpleSpan,
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Option<Box<TypedExpr>>,
    },
    Block {
        ty: Ty,
        span: SimpleSpan,
        stmts: Vec<TypedStmt>,
        tail: Option<Box<TypedExpr>>,
    },
    Lambda {
        ty: Ty,
        span: SimpleSpan,
        params: Vec<(String, Ty)>,
        ret_ty: Ty,
        captures: Vec<Capture>,
        body: Box<TypedExpr>,
    },
    Assign {
        ty: Ty,
        span: SimpleSpan,
        target: Box<TypedExpr>,
        value: Box<TypedExpr>,
    },
    New {
        ty: Ty,
        span: SimpleSpan,
        target_def_id: DefId,
        fields: Vec<(String, TypedExpr)>,
    },
    ArrayLit {
        ty: Ty,
        span: SimpleSpan,
        elements: Vec<TypedExpr>,
    },
    Range {
        ty: Ty,
        span: SimpleSpan,
        start: Option<Box<TypedExpr>>,
        end: Option<Box<TypedExpr>>,
        inclusive: bool,
    },
    Spawn {
        ty: Ty,
        span: SimpleSpan,
        expr: Box<TypedExpr>,
    },
    SpawnDetached {
        ty: Ty,
        span: SimpleSpan,
        expr: Box<TypedExpr>,
    },
    Join {
        ty: Ty,
        span: SimpleSpan,
        expr: Box<TypedExpr>,
    },
    Cancel {
        ty: Ty,
        span: SimpleSpan,
        expr: Box<TypedExpr>,
    },
    Defer {
        ty: Ty,
        span: SimpleSpan,
        expr: Box<TypedExpr>,
    },
    Path {
        ty: Ty,
        span: SimpleSpan,
        segments: Vec<String>,
    },
    Return {
        ty: Ty,
        span: SimpleSpan,
        value: Option<Box<TypedExpr>>,
    },
    Error {
        ty: Ty,
        span: SimpleSpan,
    },
}

impl TypedExpr {
    pub fn ty(&self) -> Ty {
        match self {
            TypedExpr::Literal { ty, .. }
            | TypedExpr::Var { ty, .. }
            | TypedExpr::SelfRef { ty, .. }
            | TypedExpr::Call { ty, .. }
            | TypedExpr::Field { ty, .. }
            | TypedExpr::ComponentAccess { ty, .. }
            | TypedExpr::Index { ty, .. }
            | TypedExpr::Binary { ty, .. }
            | TypedExpr::UnaryPrefix { ty, .. }
            | TypedExpr::Match { ty, .. }
            | TypedExpr::If { ty, .. }
            | TypedExpr::Block { ty, .. }
            | TypedExpr::Lambda { ty, .. }
            | TypedExpr::Assign { ty, .. }
            | TypedExpr::New { ty, .. }
            | TypedExpr::ArrayLit { ty, .. }
            | TypedExpr::Range { ty, .. }
            | TypedExpr::Spawn { ty, .. }
            | TypedExpr::SpawnDetached { ty, .. }
            | TypedExpr::Join { ty, .. }
            | TypedExpr::Cancel { ty, .. }
            | TypedExpr::Defer { ty, .. }
            | TypedExpr::Path { ty, .. }
            | TypedExpr::Return { ty, .. }
            | TypedExpr::Error { ty, .. } => *ty,
        }
    }

    pub fn span(&self) -> SimpleSpan {
        match self {
            TypedExpr::Literal { span, .. }
            | TypedExpr::Var { span, .. }
            | TypedExpr::SelfRef { span, .. }
            | TypedExpr::Call { span, .. }
            | TypedExpr::Field { span, .. }
            | TypedExpr::ComponentAccess { span, .. }
            | TypedExpr::Index { span, .. }
            | TypedExpr::Binary { span, .. }
            | TypedExpr::UnaryPrefix { span, .. }
            | TypedExpr::Match { span, .. }
            | TypedExpr::If { span, .. }
            | TypedExpr::Block { span, .. }
            | TypedExpr::Lambda { span, .. }
            | TypedExpr::Assign { span, .. }
            | TypedExpr::New { span, .. }
            | TypedExpr::ArrayLit { span, .. }
            | TypedExpr::Range { span, .. }
            | TypedExpr::Spawn { span, .. }
            | TypedExpr::SpawnDetached { span, .. }
            | TypedExpr::Join { span, .. }
            | TypedExpr::Cancel { span, .. }
            | TypedExpr::Defer { span, .. }
            | TypedExpr::Path { span, .. }
            | TypedExpr::Return { span, .. }
            | TypedExpr::Error { span, .. } => *span,
        }
    }
}

/// A typed statement.
#[derive(Debug, Clone)]
pub enum TypedStmt {
    Let {
        name: String,
        name_span: SimpleSpan,
        ty: Ty,
        mutable: bool,
        value: TypedExpr,
        span: SimpleSpan,
    },
    Expr {
        expr: TypedExpr,
        span: SimpleSpan,
    },
    For {
        binding: String,
        binding_span: SimpleSpan,
        binding_ty: Ty,
        mutable: bool,
        iterable: TypedExpr,
        body: Vec<TypedStmt>,
        span: SimpleSpan,
    },
    While {
        condition: TypedExpr,
        body: Vec<TypedStmt>,
        span: SimpleSpan,
    },
    Break {
        value: Option<TypedExpr>,
        span: SimpleSpan,
    },
    Continue {
        span: SimpleSpan,
    },
    Return {
        value: Option<TypedExpr>,
        span: SimpleSpan,
    },
    Atomic {
        body: Vec<TypedStmt>,
        span: SimpleSpan,
    },
    Error {
        span: SimpleSpan,
    },
}

/// A typed top-level declaration.
#[derive(Debug, Clone)]
pub enum TypedDecl {
    Fn {
        def_id: DefId,
        body: TypedExpr,
    },
    Struct {
        def_id: DefId,
    },
    Entity {
        def_id: DefId,
    },
    Enum {
        def_id: DefId,
    },
    Contract {
        def_id: DefId,
    },
    Impl {
        def_id: DefId,
        methods: Vec<(DefId, TypedExpr)>,
    },
    Const {
        def_id: DefId,
        value: TypedExpr,
    },
    Global {
        def_id: DefId,
        value: TypedExpr,
    },
    Component {
        def_id: DefId,
    },
    ExternFn {
        def_id: DefId,
    },
    ExternStruct {
        def_id: DefId,
    },
    ExternComponent {
        def_id: DefId,
    },
}

/// A typed match arm.
#[derive(Debug, Clone)]
pub struct TypedArm {
    pub pattern: TypedPattern,
    pub body: TypedExpr,
    pub span: SimpleSpan,
}

/// A typed pattern.
#[derive(Debug, Clone)]
pub enum TypedPattern {
    Literal {
        value: TypedLiteral,
        span: SimpleSpan,
    },
    Wildcard {
        span: SimpleSpan,
    },
    Variable {
        name: String,
        ty: Ty,
        span: SimpleSpan,
    },
    EnumVariant {
        enum_def_id: DefId,
        variant_name: String,
        bindings: Vec<TypedPattern>,
        span: SimpleSpan,
    },
    Or {
        patterns: Vec<TypedPattern>,
        span: SimpleSpan,
    },
    Range {
        start: TypedLiteral,
        end: TypedLiteral,
        inclusive: bool,
        span: SimpleSpan,
    },
}

/// A closure capture.
#[derive(Debug, Clone)]
pub struct Capture {
    pub name: String,
    pub ty: Ty,
    pub mode: CaptureMode,
    pub binding_span: SimpleSpan,
}

/// How a variable is captured by a closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    ByValue,
    ByRef,
}

/// A typed literal value.
#[derive(Debug, Clone)]
pub enum TypedLiteral {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}
