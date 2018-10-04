use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::BaseData;
use crate::ty::Erased;
use crate::ty::Generic;
use crate::ty::Ty;
use crate::typeck::{BaseTypeChecker, Cause, Error, ErrorKind};
use crate::unify::InferVar;
use generational_arena::Arena;

impl BaseTypeChecker {
    /// If `base` can be mapped to a concrete `BaseData`,
    /// invokes `op` and returns the resulting type.
    /// Otherwise, creates a type variable and returns that;
    /// once `base` can be mapped, the closure `op` will be
    /// invoked and the type variable will be unified.
    pub(super) fn with_base_data(
        &mut self,
        cause: Cause,
        base: Base,
        op: impl FnOnce(&mut Self, BaseData<BaseOnly>) -> Ty<BaseOnly> + 'static,
    ) -> Ty<BaseOnly> {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => op(self, data),

            Err(_) => {
                let var: Ty<BaseOnly> = self.new_infer_ty();
                self.with_base_data_unify_with(cause, base, var, op);
                var
            }
        }
    }

    fn with_base_data_unify_with(
        &mut self,
        cause: Cause,
        base: Base,
        output_ty: Ty<BaseOnly>,
        op: impl FnOnce(&mut Self, BaseData<BaseOnly>) -> Ty<BaseOnly> + 'static,
    ) {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => {
                let ty1 = op(self, data);
                self.equate_tys(cause, output_ty, ty1);
            }

            Err(_) => self.enqueue_op(Some(base), move |this| {
                this.with_base_data_unify_with(cause, base, output_ty, op)
            }),
        }
    }

    pub(super) fn new_infer_ty(&mut self) -> Ty<BaseOnly> {
        Ty {
            perm: Erased,
            base: self.unify.new_inferable(),
        }
    }

    pub(super) fn equate_tys(&mut self, cause: Cause, ty1: Ty<BaseOnly>, ty2: Ty<BaseOnly>) {
        let Ty {
            perm: Erased,
            base: base1,
        } = ty1;
        let Ty {
            perm: Erased,
            base: base2,
        } = ty2;

        match self.unify.unify(cause, base1, base2) {
            Ok(()) => {}

            Err((data1, data2)) => {
                if data1.kind != data2.kind {
                    self.errors.push(Error {
                        cause,
                        kind: ErrorKind::BaseMismatch(ty1, ty2),
                    });
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (Generic::Ty(g1), Generic::Ty(g2)) => {
                            self.equate_tys(cause, g1, g2);
                        }
                    }
                }
            }
        }
    }
}
