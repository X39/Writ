//! Typed IR definitions for the Writ type checker.
//!
//! Every expression variant carries `ty: Ty` and `span: SimpleSpan` directly.
//! No `Option<Ty>` fields exist in this IR.

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;

use crate::ast::expr::{BinaryOp, PrefixOp};
use crate::resolve::def_map::{DefId, DefMap};

use super::ty::Ty;

/// The output of type checking: typed declarations plus the DefMap.
#[derive(Debug)]
pub struct TypedAst {
    pub decls: Vec<TypedDecl>,
    pub def_map: DefMap,
    /// Struct field types extracted from TypeEnv after type checking.
    /// Maps struct DefId -> ordered list of (field_name, field_ty).
    /// Used by the emitter for field-by-field structural equality emission.
    pub struct_field_types: FxHashMap<DefId, Vec<(String, Ty)>>,
    /// Condition-name map for [Conditional] functions. Empty when no conditions are used.
    /// Transferred from TypeEnv after type checking for downstream emit consumption.
    pub conditional_fns: FxHashMap<DefId, String>,
    /// Conditional fn -> fallback fn mapping. Empty when no conditions are used.
    /// Transferred from TypeEnv after type checking for downstream emit consumption.
    pub fallback_for_conditional: FxHashMap<DefId, DefId>,
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
    /// Intentional runtime crash (e.g., force-unwrap failure on None/Err).
    /// NOT a compilation error — this emits a Crash instruction in the IL.
    Crash {
        ty: Ty,
        span: SimpleSpan,
        message: String,
    },
    Error {
        ty: Ty,
        span: SimpleSpan,
    },
    /// typeof(expr) — static compile-time type query.
    /// `ty` is always TyKind::ReflectionType(static_ty).
    /// `static_ty` is the compile-time type of the inner expression (used by the emitter).
    TypeOf {
        ty: Ty,
        span: SimpleSpan,
        static_ty: Ty,
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
            | TypedExpr::Crash { ty, .. }
            | TypedExpr::Error { ty, .. }
            | TypedExpr::TypeOf { ty, .. } => *ty,
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
            | TypedExpr::Crash { span, .. }
            | TypedExpr::Error { span, .. }
            | TypedExpr::TypeOf { span, .. } => *span,
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
        /// Span of the explicit type annotation (e.g., `MyStruct` in `let x: MyStruct = ...`).
        /// `None` when the type was inferred.
        type_ann_span: Option<SimpleSpan>,
        /// DefId of the type annotation's named type (for go-to-def on type annotations).
        /// `None` when annotation is absent or non-named (array, func, void, generic).
        type_ann_def_id: Option<DefId>,
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
        /// Contract DefId for Iterable<T> — set when iterating a class type.
        /// None for array/range iteration.
        iterable_contract_def_id: Option<DefId>,
        /// Contract DefId for Iterator<T> — set when iterating a class type.
        /// None for array/range iteration.
        iterator_contract_def_id: Option<DefId>,
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
        /// Spans of parameter name identifiers (in declaration order), for LSP hover on param names.
        /// Includes spans for `self` parameters. Empty for extern functions.
        param_name_spans: Vec<SimpleSpan>,
    },
    Struct {
        def_id: DefId,
    },
    /// Class declaration (reference type, heap-allocated).
    Class {
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
    ExternComponent {
        def_id: DefId,
    },
    /// A user-defined attribute declaration (type-checking is a passthrough).
    AttributeDef {
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
