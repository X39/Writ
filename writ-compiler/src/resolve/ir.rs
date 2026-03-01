//! Name-resolved IR types.
//!
//! After name resolution, the AST is transformed into a `NameResolvedAst`
//! where every type and name reference carries a `DefId` or `ResolvedType`.

use crate::resolve::def_map::{DefId, DefMap};

/// The output of name resolution: resolved declarations plus the symbol table.
#[derive(Debug)]
pub struct NameResolvedAst {
    /// The resolved declarations.
    pub decls: Vec<ResolvedDecl>,
    /// The DefMap containing all definitions (travels with the IR for downstream use).
    pub def_map: DefMap,
}

/// A resolved type reference.
///
/// Every `AstType` in the input is resolved to one of these variants,
/// where named types carry their `DefId` from the DefMap.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    /// A primitive type (int, float, bool, string, void).
    Primitive(PrimitiveTag),
    /// A named type resolved to its definition.
    Named {
        def_id: DefId,
        type_args: Vec<ResolvedType>,
    },
    /// An array type: T[].
    Array(Box<ResolvedType>),
    /// A function type: fn(params) -> ret.
    Func {
        params: Vec<ResolvedType>,
        ret: Box<ResolvedType>,
    },
    /// Void type.
    Void,
    /// A generic type parameter (e.g., T from `fn foo<T>`).
    GenericParam(String),
    /// A prelude type (Option, Result, Range, Array, Entity).
    PreludeType(String),
    /// A prelude contract (Add, Eq, Iterator, etc.).
    PreludeContract(String),
    /// Error recovery: type could not be resolved.
    Error,
}

/// Primitive type tags corresponding to IL primitive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTag {
    Int,
    Float,
    Bool,
    String,
    Void,
}

/// A resolved declaration.
///
/// Stub variants for Plan 01. Full population happens in Plan 02 (resolver).
#[derive(Debug, Clone)]
pub enum ResolvedDecl {
    /// A resolved function declaration.
    Fn { def_id: DefId },
    /// A resolved struct declaration.
    Struct { def_id: DefId },
    /// A resolved entity declaration.
    Entity { def_id: DefId },
    /// A resolved enum declaration.
    Enum { def_id: DefId },
    /// A resolved contract declaration.
    Contract { def_id: DefId },
    /// A resolved impl block.
    Impl { def_id: DefId },
    /// A resolved component declaration.
    Component { def_id: DefId },
    /// A resolved extern function.
    ExternFn { def_id: DefId },
    /// A resolved extern struct.
    ExternStruct { def_id: DefId },
    /// A resolved extern component.
    ExternComponent { def_id: DefId },
    /// A resolved constant.
    Const { def_id: DefId },
    /// A resolved global.
    Global { def_id: DefId },
}
