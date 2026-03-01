//! Type representation for the Writ type checker.
//!
//! `Ty` is a cheap `Copy` interned ID into a `TyInterner`. `TyKind` holds the
//! actual type data. Structural deduplication ensures that the same type always
//! gets the same `Ty` id.

use crate::resolve::def_map::DefId;
use ena::unify::UnifyKey;
use rustc_hash::FxHashMap;

/// An interned type handle. Copy, Eq, Hash — identity by index.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ty(pub u32);

/// The actual shape of a type.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TyKind {
    Int,
    Float,
    Bool,
    String,
    Void,
    Struct(DefId),
    Entity(DefId),
    Enum(DefId),
    Array(Ty),
    Func { params: Vec<Ty>, ret: Ty },
    Option(Ty),
    Result(Ty, Ty),
    TaskHandle(Ty),
    /// Generic type parameter (index into the current function's generic param list).
    GenericParam(u32),
    /// Inference variable (unresolved during type checking).
    Infer(InferVar),
    /// Poison type: suppresses cascading errors.
    Error,
}

/// An inference variable used during type unification.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct InferVar(pub u32);

/// Wrapper for InferVar's value to satisfy orphan rules.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InferValue(pub Option<Ty>);

impl ena::unify::UnifyValue for InferValue {
    type Error = ena::unify::NoError;
    fn unify_values(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        match (a.0, b.0) {
            (Some(_), _) => Ok(*a),
            (_, Some(_)) => Ok(*b),
            _ => Ok(InferValue(None)),
        }
    }
}

impl UnifyKey for InferVar {
    type Value = InferValue;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> Self {
        InferVar(u)
    }
    fn tag() -> &'static str {
        "InferVar"
    }
}

/// Arena-backed type interner with structural deduplication.
pub struct TyInterner {
    kinds: Vec<TyKind>,
    map: FxHashMap<TyKind, Ty>,
}

impl TyInterner {
    pub fn new() -> Self {
        let mut interner = Self {
            kinds: Vec::new(),
            map: FxHashMap::default(),
        };
        // Pre-intern primitives so they always get consistent ids
        interner.intern(TyKind::Int);
        interner.intern(TyKind::Float);
        interner.intern(TyKind::Bool);
        interner.intern(TyKind::String);
        interner.intern(TyKind::Void);
        interner.intern(TyKind::Error);
        interner
    }

    /// Intern a `TyKind`, returning its deduplicated `Ty` handle.
    pub fn intern(&mut self, kind: TyKind) -> Ty {
        if let Some(&ty) = self.map.get(&kind) {
            return ty;
        }
        let id = Ty(self.kinds.len() as u32);
        self.kinds.push(kind.clone());
        self.map.insert(kind, id);
        id
    }

    /// Look up the `TyKind` for a `Ty`.
    pub fn kind(&self, ty: Ty) -> &TyKind {
        &self.kinds[ty.0 as usize]
    }

    // Convenience constructors
    pub fn int(&mut self) -> Ty {
        self.intern(TyKind::Int)
    }
    pub fn float(&mut self) -> Ty {
        self.intern(TyKind::Float)
    }
    pub fn bool_ty(&mut self) -> Ty {
        self.intern(TyKind::Bool)
    }
    pub fn string_ty(&mut self) -> Ty {
        self.intern(TyKind::String)
    }
    pub fn void(&mut self) -> Ty {
        self.intern(TyKind::Void)
    }
    pub fn error(&mut self) -> Ty {
        self.intern(TyKind::Error)
    }
    pub fn option(&mut self, inner: Ty) -> Ty {
        self.intern(TyKind::Option(inner))
    }
    pub fn result(&mut self, ok: Ty, err: Ty) -> Ty {
        self.intern(TyKind::Result(ok, err))
    }
    pub fn array(&mut self, elem: Ty) -> Ty {
        self.intern(TyKind::Array(elem))
    }
    pub fn task_handle(&mut self, inner: Ty) -> Ty {
        self.intern(TyKind::TaskHandle(inner))
    }
    pub fn func(&mut self, params: Vec<Ty>, ret: Ty) -> Ty {
        self.intern(TyKind::Func { params, ret })
    }

    /// Format a type as a human-readable string.
    pub fn display(&self, ty: Ty) -> String {
        match self.kind(ty) {
            TyKind::Int => "int".to_string(),
            TyKind::Float => "float".to_string(),
            TyKind::Bool => "bool".to_string(),
            TyKind::String => "string".to_string(),
            TyKind::Void => "void".to_string(),
            TyKind::Struct(_) => "struct".to_string(),
            TyKind::Entity(_) => "entity".to_string(),
            TyKind::Enum(_) => "enum".to_string(),
            TyKind::Array(elem) => format!("{}[]", self.display(*elem)),
            TyKind::Func { params, ret } => {
                let ps: Vec<String> = params.iter().map(|p| self.display(*p)).collect();
                format!("fn({}) -> {}", ps.join(", "), self.display(*ret))
            }
            TyKind::Option(inner) => format!("Option<{}>", self.display(*inner)),
            TyKind::Result(ok, err) => {
                format!("Result<{}, {}>", self.display(*ok), self.display(*err))
            }
            TyKind::TaskHandle(inner) => format!("TaskHandle<{}>", self.display(*inner)),
            TyKind::GenericParam(idx) => format!("T{}", idx),
            TyKind::Infer(var) => format!("?{}", var.0),
            TyKind::Error => "<error>".to_string(),
        }
    }
}
