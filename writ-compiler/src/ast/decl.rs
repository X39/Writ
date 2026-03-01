use chumsky::span::SimpleSpan;
use crate::ast::expr::AstExpr;
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;

/// All top-level declaration forms that survive lowering into the AST.
///
/// Key invariants:
/// - NO `Dlg` variant — lowered to `Fn` before reaching AST.
/// - YES `Entity` variant — entity declarations are preserved with their full structure.
/// - YES all structural pass-through types.
/// - All data is owned (`String`, `Box<T>`, `Vec<T>`) — no `'src` lifetime.
///
/// Note: `AstDecl` does not have a direct `span` field because each variant is a struct
/// that carries its own span. This mirrors the CST `Item` enum pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum AstDecl {
    Namespace(AstNamespaceDecl),
    Using(AstUsingDecl),
    Fn(AstFnDecl),
    Struct(AstStructDecl),
    Entity(AstEntityDecl),
    Enum(AstEnumDecl),
    Contract(AstContractDecl),
    Impl(AstImplDecl),
    Component(AstComponentDecl),
    Extern(AstExternDecl),
    Const(AstConstDecl),
    Global(AstGlobalDecl),
    /// Bare top-level statement
    Stmt(AstStmt),
}

// =========================================================
// Shared helper types
// =========================================================

/// Visibility modifier.
#[derive(Debug, Clone, PartialEq)]
pub enum AstVisibility {
    /// `pub` — public visibility
    Pub,
    /// `priv` — private visibility
    Priv,
}

/// An attribute: `[Name]` or `[Name(args)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AstAttribute {
    /// Attribute name.
    pub name: String,
    pub name_span: SimpleSpan,
    /// Optional arguments (positional or named expressions).
    pub args: Vec<AstAttributeArg>,
    pub span: SimpleSpan,
}

/// An attribute argument: positional or named.
#[derive(Debug, Clone, PartialEq)]
pub enum AstAttributeArg {
    /// Positional argument: `expr`
    Positional(AstExpr),
    /// Named argument: `name: expr`
    Named { name: String, name_span: SimpleSpan, value: AstExpr },
}

/// A function/method parameter: `name: type`.
#[derive(Debug, Clone, PartialEq)]
pub struct AstParam {
    pub name: String,
    pub name_span: SimpleSpan,
    pub ty: AstType,
    pub span: SimpleSpan,
}

/// A function parameter: regular or self.
#[derive(Debug, Clone, PartialEq)]
pub enum AstFnParam {
    /// Regular parameter: `name: type`
    Regular(AstParam),
    /// Self parameter: `self` or `mut self`
    SelfParam { mutable: bool, span: SimpleSpan },
}

/// A generic type parameter: `<T: Bound + Other>`.
#[derive(Debug, Clone, PartialEq)]
pub struct AstGenericParam {
    pub name: String,
    pub name_span: SimpleSpan,
    pub bounds: Vec<AstType>,
    pub span: SimpleSpan,
}

/// Operator symbols for overloading.
#[derive(Debug, Clone, PartialEq)]
pub enum AstOpSymbol {
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

// =========================================================
// Namespace and Using
// =========================================================

/// Namespace declaration: `namespace a::b;` or `namespace a::b { items }`
#[derive(Debug, Clone, PartialEq)]
pub enum AstNamespaceDecl {
    /// Declarative form: `namespace a::b::c;`
    Declarative { path: Vec<String>, span: SimpleSpan },
    /// Block form: `namespace a::b { items }`
    Block { path: Vec<String>, items: Vec<AstDecl>, span: SimpleSpan },
}

/// Using import: `using [alias =] qualified::name;`
#[derive(Debug, Clone, PartialEq)]
pub struct AstUsingDecl {
    /// Optional alias: `alias = ...`
    pub alias: Option<String>,
    /// Qualified path segments.
    pub path: Vec<String>,
    pub span: SimpleSpan,
}

// =========================================================
// Function
// =========================================================

/// Function declaration: `[attrs] [vis] fn name [<generics>] (params) [-> type] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstFnDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub generics: Vec<AstGenericParam>,
    pub params: Vec<AstFnParam>,
    pub return_type: Option<AstType>,
    pub body: Vec<AstStmt>,
    pub span: SimpleSpan,
}

/// Function signature (no body): used in contracts and extern declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct AstFnSig {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    /// Optional qualifier for dotted extern names: `Type.method`
    pub qualifier: Option<String>,
    pub qualifier_span: Option<SimpleSpan>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub generics: Vec<AstGenericParam>,
    pub params: Vec<AstFnParam>,
    pub return_type: Option<AstType>,
    pub span: SimpleSpan,
}

// =========================================================
// Struct and Enum
// =========================================================

/// A member in a struct body: field or lifecycle hook.
#[derive(Debug, Clone, PartialEq)]
pub enum AstStructMember {
    /// Regular field.
    Field(AstStructField),
    /// Lifecycle hook: `on event { body }`
    OnHook {
        event: String,
        event_span: SimpleSpan,
        body: Vec<AstStmt>,
        span: SimpleSpan,
    },
}

/// Struct declaration: `[attrs] [vis] struct Name [<generics>] { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstStructDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub generics: Vec<AstGenericParam>,
    pub members: Vec<AstStructMember>,
    pub span: SimpleSpan,
}

/// A struct field: `[vis] name: type [= default]`
#[derive(Debug, Clone, PartialEq)]
pub struct AstStructField {
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub ty: AstType,
    pub default: Option<AstExpr>,
    pub span: SimpleSpan,
}

// =========================================================
// Entity
// =========================================================

/// A component slot descriptor: `use ComponentName { field: value, ... }`
///
/// Models components as host-managed slots (not inline struct fields).
/// Override values are preserved as AstExpr for codegen evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct AstComponentSlot {
    /// Component type name.
    pub component: String,
    pub component_span: SimpleSpan,
    /// Per-field override values: (field_name, field_name_span, override_value).
    pub overrides: Vec<(String, SimpleSpan, AstExpr)>,
    pub span: SimpleSpan,
}

/// An entity lifecycle hook registration.
#[derive(Debug, Clone, PartialEq)]
pub struct AstEntityHook {
    /// Contract name: OnCreate, OnDestroy, OnInteract, OnFinalize, OnSerialize, OnDeserialize.
    pub contract: String,
    pub contract_span: SimpleSpan,
    /// Hook method with implicit mut self as first param.
    pub method: AstFnDecl,
    pub span: SimpleSpan,
}

/// Entity declaration: `[attrs] [vis] entity Name { properties, component_slots, methods, hooks }`
///
/// Unlike AstStructDecl, this preserves the entity-specific structure:
/// - Properties (regular typed fields)
/// - Component slots (host-managed, not inline fields)
/// - Hooks (lifecycle event registrations with implicit mut self)
/// - Methods (gathered into an inherent impl)
#[derive(Debug, Clone, PartialEq)]
pub struct AstEntityDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    /// Entity properties (regular typed fields).
    pub properties: Vec<AstStructField>,
    /// Component slot descriptors (host-managed).
    pub component_slots: Vec<AstComponentSlot>,
    /// Lifecycle hook registrations.
    pub hooks: Vec<AstEntityHook>,
    /// Inherent impl if methods exist (contract: None).
    pub inherent_impl: Option<AstImplDecl>,
    pub span: SimpleSpan,
}

/// Enum declaration: `[attrs] [vis] enum Name [<generics>] { variants }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstEnumDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub generics: Vec<AstGenericParam>,
    pub variants: Vec<AstEnumVariant>,
    pub span: SimpleSpan,
}

/// An enum variant: `Name` or `Name(fields)`
#[derive(Debug, Clone, PartialEq)]
pub struct AstEnumVariant {
    pub name: String,
    pub name_span: SimpleSpan,
    /// Optional tuple fields (named, using Param struct).
    pub fields: Option<Vec<AstParam>>,
    pub span: SimpleSpan,
}

// =========================================================
// Contract and Impl
// =========================================================

/// Contract declaration: `[attrs] [vis] contract Name [<generics>] { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstContractDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub generics: Vec<AstGenericParam>,
    pub members: Vec<AstContractMember>,
    pub span: SimpleSpan,
}

/// A contract member: function signature or operator signature.
#[derive(Debug, Clone, PartialEq)]
pub enum AstContractMember {
    /// Function signature (no body).
    FnSig(AstFnSig),
    /// Operator signature (no body).
    OpSig(AstOpSig),
}

/// Operator signature (no body): `operator SYMBOL (params) [-> type]`
#[derive(Debug, Clone, PartialEq)]
pub struct AstOpSig {
    pub vis: Option<AstVisibility>,
    pub symbol: AstOpSymbol,
    pub symbol_span: SimpleSpan,
    pub params: Vec<AstParam>,
    pub return_type: Option<AstType>,
    pub span: SimpleSpan,
}

/// Operator declaration with body: `operator SYMBOL (params) [-> type] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstOpDecl {
    pub vis: Option<AstVisibility>,
    pub symbol: AstOpSymbol,
    pub symbol_span: SimpleSpan,
    pub params: Vec<AstParam>,
    pub return_type: Option<AstType>,
    pub body: Vec<AstStmt>,
    pub span: SimpleSpan,
}

/// Impl block: `impl [<generics>] [Contract for] Type { members }`
#[derive(Debug, Clone, PartialEq)]
pub struct AstImplDecl {
    /// Optional generic parameters.
    pub generics: Vec<AstGenericParam>,
    /// Optional contract being implemented.
    pub contract: Option<AstType>,
    /// Target type.
    pub target: AstType,
    pub members: Vec<AstImplMember>,
    pub span: SimpleSpan,
}

/// An impl member: function or operator declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum AstImplMember {
    /// Function declaration.
    Fn(AstFnDecl),
    /// Operator declaration.
    Op(AstOpDecl),
}

// =========================================================
// Component
// =========================================================

/// Component declaration: `[attrs] [vis] component Name { members }`
///
/// Components are extern-only, host-managed data containers (no script-defined components).
/// Entity `use` clauses lower to `AstComponentSlot` descriptors, not component decls.
#[derive(Debug, Clone, PartialEq)]
pub struct AstComponentDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub members: Vec<AstComponentMember>,
    pub span: SimpleSpan,
}

/// A component member: field or method.
#[derive(Debug, Clone, PartialEq)]
pub enum AstComponentMember {
    /// Field (same structure as struct field).
    Field(AstStructField),
    /// Method.
    Fn(AstFnDecl),
}

// =========================================================
// Extern
// =========================================================

/// Extern declaration: `extern fn|struct|component ...`
#[derive(Debug, Clone, PartialEq)]
pub enum AstExternDecl {
    /// Extern function (signature only): `[vis] extern fn name(...) [-> type];`
    Fn(Option<AstVisibility>, AstFnSig),
    /// Extern struct: `[vis] extern struct Name { fields }`
    Struct(Option<AstVisibility>, AstStructDecl),
    /// Extern component: `[vis] extern component Name { fields }`
    Component(Option<AstVisibility>, AstComponentDecl),
}

// =========================================================
// Const and Global
// =========================================================

/// Constant declaration: `[attrs] [vis] const name: type = expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct AstConstDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub ty: AstType,
    pub value: AstExpr,
    pub span: SimpleSpan,
}

/// Global mutable: `[attrs] [vis] global mut name: type = expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct AstGlobalDecl {
    pub attrs: Vec<AstAttribute>,
    pub vis: Option<AstVisibility>,
    pub name: String,
    pub name_span: SimpleSpan,
    pub ty: AstType,
    pub value: AstExpr,
    pub span: SimpleSpan,
}
