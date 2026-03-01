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
}

impl UnifyCtx {
    pub fn new() -> Self {
        Self {
            table: InPlaceUnificationTable::new(),
        }
    }

    /// Create a fresh inference variable.
    pub fn new_var(&mut self) -> InferVar {
        self.table.new_key(InferValue(None))
    }

    /// Resolve an inference variable to its value (if any).
    pub fn resolve(&mut self, var: InferVar) -> Option<Ty> {
        self.table.probe_value(var).0
    }

    /// Recursively resolve a Ty, replacing Infer(var) with its bound value.
    pub fn resolve_ty(&mut self, ty: Ty, interner: &TyInterner) -> Ty {
        match interner.kind(ty) {
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

    /// Unify two types. Returns Ok(()) on success, Err(UnifyError) on mismatch.
    pub fn unify(
        &mut self,
        a: Ty,
        b: Ty,
        interner: &mut TyInterner,
    ) -> Result<(), UnifyError> {
        // Short circuit: same Ty id means same type
        if a == b {
            return Ok(());
        }

        let a_kind = interner.kind(a).clone();
        let b_kind = interner.kind(b).clone();

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

            // Same named type
            (TyKind::Struct(a_id), TyKind::Struct(b_id)) if a_id == b_id => Ok(()),
            (TyKind::Entity(a_id), TyKind::Entity(b_id)) if a_id == b_id => Ok(()),
            (TyKind::Enum(a_id), TyKind::Enum(b_id)) if a_id == b_id => Ok(()),

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

            // Mismatch
            _ => Err(UnifyError {
                expected: a,
                found: b,
            }),
        }
    }
}
