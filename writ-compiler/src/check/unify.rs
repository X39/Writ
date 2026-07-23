//! Type unification using ena union-find.

use ena::unify::InPlaceUnificationTable;

use super::ty::{InferValue, InferVar, Ty, TyInterner, TyKind};

/// Error produced when unification fails.
#[derive(Debug, Clone)]
pub struct UnifyError {
    pub expected: Ty,
    pub found: Ty,
}

/// Unification context wrapping an ena `InPlaceUnificationTable`.
pub struct UnifyCtx {
    table: InPlaceUnificationTable<InferVar>,
    vars: Vec<InferVar>,
}

impl Default for UnifyCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifyCtx {
    pub fn new() -> Self {
        Self {
            table: InPlaceUnificationTable::new(),
            vars: Vec::new(),
        }
    }

    /// Create a fresh inference variable.
    pub fn new_var(&mut self) -> InferVar {
        let var = self.table.new_key(InferValue(None));
        self.vars.push(var);
        var
    }

    /// Resolve an inference variable to its value (if any).
    pub fn resolve(&mut self, var: InferVar) -> Option<Ty> {
        self.table.probe_value(var).0
    }

    /// Recursively resolve a Ty, replacing Infer(var) with its bound value.
    pub fn resolve_ty(&mut self, ty: Ty, interner: &TyInterner) -> Ty {
        match interner.full_kind(ty) {
            TyKind::Infer(var) => {
                let var = *var;
                match self.resolve(var) {
                    Some(resolved) => self.resolve_ty(resolved, interner),
                    None => ty,
                }
            }
            _ => ty,
        }
    }

    /// Recursively resolve inference variables nested inside structural types.
    ///
    /// A generic call can produce a type such as `Crate<?0>` after `?0` has
    /// already been bound by an argument.  Keeping the unresolved handle is
    /// harmless for ordinary unification, but exact impl-pattern matching must
    /// see the concrete `Crate<int>` shape.
    pub fn resolve_ty_deep(&mut self, ty: Ty, interner: &mut TyInterner) -> Ty {
        let ty = self.resolve_ty(ty, interner);
        match interner.full_kind(ty).clone() {
            TyKind::Array(elem) => {
                let elem = self.resolve_ty_deep(elem, interner);
                interner.array(elem)
            }
            TyKind::Option(inner) => {
                let inner = self.resolve_ty_deep(inner, interner);
                interner.option(inner)
            }
            TyKind::Result(ok, err) => {
                let ok = self.resolve_ty_deep(ok, interner);
                let err = self.resolve_ty_deep(err, interner);
                interner.result(ok, err)
            }
            TyKind::TaskHandle(inner) => {
                let inner = self.resolve_ty_deep(inner, interner);
                interner.task_handle(inner)
            }
            TyKind::ReflectionType(inner) => {
                let inner = self.resolve_ty_deep(inner, interner);
                interner.reflection_type(inner)
            }
            TyKind::GenericInstance {
                base,
                namespace,
                name,
                args,
            } => {
                let base = self.resolve_ty_deep(base, interner);
                let args = args
                    .into_iter()
                    .map(|arg| self.resolve_ty_deep(arg, interner))
                    .collect();
                interner.generic_instance(base, namespace, name, args)
            }
            TyKind::Func { params, ret } => {
                let params = params
                    .into_iter()
                    .map(|param| self.resolve_ty_deep(param, interner))
                    .collect();
                let ret = self.resolve_ty_deep(ret, interner);
                interner.func(params, ret)
            }
            _ => ty,
        }
    }

    /// Persist resolved inference bindings in the interner for code generation.
    pub fn record_resolutions(&mut self, interner: &mut TyInterner) {
        for var in self.vars.clone() {
            if let Some(ty) = self.resolve(var) {
                interner.record_infer_resolution(var, ty);
            }
        }
    }

    /// Unify two types. Returns Ok(()) on success, Err(UnifyError) on mismatch.
    pub fn unify(&mut self, a: Ty, b: Ty, interner: &mut TyInterner) -> Result<(), UnifyError> {
        // Short circuit: same Ty id means same type
        if a == b {
            return Ok(());
        }

        let a_kind = interner.full_kind(a).clone();
        let b_kind = interner.full_kind(b).clone();

        match (&a_kind, &b_kind) {
            // Error unifies with anything (poison propagation)
            (TyKind::Error, _) | (_, TyKind::Error) => Ok(()),

            // Inference variable binds to the other type
            (TyKind::Infer(var), _) => {
                let var = *var;
                let resolved = self.resolve(var);
                match resolved {
                    Some(bound) => self.unify(bound, b, interner),
                    None => {
                        self.table.union_value(var, InferValue(Some(b)));
                        Ok(())
                    }
                }
            }
            (_, TyKind::Infer(var)) => {
                let var = *var;
                let resolved = self.resolve(var);
                match resolved {
                    Some(bound) => self.unify(a, bound, interner),
                    None => {
                        self.table.union_value(var, InferValue(Some(a)));
                        Ok(())
                    }
                }
            }

            // Same primitive = ok
            (TyKind::Int, TyKind::Int)
            | (TyKind::Float, TyKind::Float)
            | (TyKind::Bool, TyKind::Bool)
            | (TyKind::String, TyKind::String)
            | (TyKind::Void, TyKind::Void) => Ok(()),

            // Instantiated nominal types have both nominal and structural identity.
            // Constructor spellings are diagnostic/encoding metadata; DefId is the
            // authoritative identity, while every argument must unify recursively.
            (
                TyKind::GenericInstance {
                    base: a_base,
                    args: a_args,
                    ..
                },
                TyKind::GenericInstance {
                    base: b_base,
                    args: b_args,
                    ..
                },
            ) => {
                if a_args.len() != b_args.len() {
                    return Err(UnifyError {
                        expected: a,
                        found: b,
                    });
                }
                self.unify(*a_base, *b_base, interner)?;
                for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                    self.unify(*a_arg, *b_arg, interner)?;
                }
                Ok(())
            }

            // Same named type
            (TyKind::Struct(a_id), TyKind::Struct(b_id)) if a_id == b_id => Ok(()),
            (TyKind::Class(a_id), TyKind::Class(b_id)) if a_id == b_id => Ok(()),
            (TyKind::Entity(a_id), TyKind::Entity(b_id)) if a_id == b_id => Ok(()),
            // AnyEntity (base Entity type) accepts any specific entity type
            (TyKind::AnyEntity, TyKind::AnyEntity) => Ok(()),
            (TyKind::AnyEntity, TyKind::Entity(_)) | (TyKind::Entity(_), TyKind::AnyEntity) => {
                Ok(())
            }
            (TyKind::AnyEntity, TyKind::GenericInstance { base, .. })
                if matches!(interner.kind(*base), TyKind::Entity(_)) =>
            {
                Ok(())
            }
            (TyKind::GenericInstance { base, .. }, TyKind::AnyEntity)
                if matches!(interner.kind(*base), TyKind::Entity(_)) =>
            {
                Ok(())
            }
            (TyKind::Enum(a_id), TyKind::Enum(b_id)) if a_id == b_id => Ok(()),
            // Same contract type (identity only — assignability is directional, handled in check_stmt)
            (TyKind::Contract(a_id), TyKind::Contract(b_id)) if a_id == b_id => Ok(()),

            // Structural: arrays
            (TyKind::Array(a_elem), TyKind::Array(b_elem)) => {
                let a_elem = *a_elem;
                let b_elem = *b_elem;
                self.unify(a_elem, b_elem, interner)
            }

            // Structural: Option
            (TyKind::Option(a_inner), TyKind::Option(b_inner)) => {
                let a_inner = *a_inner;
                let b_inner = *b_inner;
                self.unify(a_inner, b_inner, interner)
            }

            // Structural: Result
            (TyKind::Result(a_ok, a_err), TyKind::Result(b_ok, b_err)) => {
                let (a_ok, a_err) = (*a_ok, *a_err);
                let (b_ok, b_err) = (*b_ok, *b_err);
                self.unify(a_ok, b_ok, interner)?;
                self.unify(a_err, b_err, interner)
            }

            // Structural: TaskHandle
            (TyKind::TaskHandle(a_inner), TyKind::TaskHandle(b_inner)) => {
                let a_inner = *a_inner;
                let b_inner = *b_inner;
                self.unify(a_inner, b_inner, interner)
            }

            // Structural: Func
            (
                TyKind::Func {
                    params: a_params,
                    ret: a_ret,
                },
                TyKind::Func {
                    params: b_params,
                    ret: b_ret,
                },
            ) => {
                if a_params.len() != b_params.len() {
                    return Err(UnifyError {
                        expected: a,
                        found: b,
                    });
                }
                let a_params = a_params.clone();
                let b_params = b_params.clone();
                let a_ret = *a_ret;
                let b_ret = *b_ret;
                for (ap, bp) in a_params.iter().zip(b_params.iter()) {
                    self.unify(*ap, *bp, interner)?;
                }
                self.unify(a_ret, b_ret, interner)
            }

            // Same generic param index
            (TyKind::GenericParam(a_idx), TyKind::GenericParam(b_idx)) if a_idx == b_idx => Ok(()),

            // Generic param unifies with any concrete type (treated as a wildcard).
            // This allows `new Box<int> { value: 42 }` to type-check when `value: T` has
            // type GenericParam(0) in the struct_fields map.
            (TyKind::GenericParam(_), _) | (_, TyKind::GenericParam(_)) => Ok(()),

            // ReflectionType unification: typeof(T) == typeof(U) is valid because at runtime
            // both sides are `Type` objects.  The inner type carries static info used by the
            // emitter (to select the TypeDef token); the outer `Type` is the same runtime type
            // regardless of which concrete type was queried.
            (TyKind::ReflectionType(_), TyKind::ReflectionType(_)) => Ok(()),

            // Mismatch
            _ => Err(UnifyError {
                expected: a,
                found: b,
            }),
        }
    }
}
