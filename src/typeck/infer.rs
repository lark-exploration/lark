use crate::ty;
use crate::ty::unify::InferValue;
use crate::ty::BaseData;
use crate::ty::InferVar;
use crate::typeck::TypeChecker;
use generational_arena::Arena;

impl TypeChecker {
    /// If `base` can be mapped to a concrete `BaseData`,
    /// invokes `op` and returns the resulting type.
    /// Otherwise, creates a type variable and returns that;
    /// once `base` can be mapped, the closure `op` will be
    /// invoked and the type variable will be unified.
    pub(super) fn with_base_data(
        &mut self,
        base: ty::Base,
        op: impl FnOnce(&mut Self, BaseData) -> ty::Ty + 'static,
    ) -> ty::Ty {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => op(self, data),

            Err(_) => {
                let var: ty::Ty = self.new_infer_ty();
                self.with_base_data_unify_with(base, var, op);
                var
            }
        }
    }

    fn with_base_data_unify_with(
        &mut self,
        base: ty::Base,
        output_ty: ty::Ty,
        op: impl FnOnce(&mut Self, BaseData) -> ty::Ty + 'static,
    ) {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => {
                let ty1 = op(self, data);
                self.equate_tys(output_ty, ty1);
            }

            Err(_) => self.enqueue_op(Some(base), move |this| {
                this.with_base_data_unify_with(base, output_ty, op)
            }),
        }
    }

    pub(super) fn new_infer_ty(&mut self) -> ty::Ty {
        ty::Ty {
            perm: self.unify.new_inferable(),
            base: self.unify.new_inferable(),
        }
    }

    pub(super) fn equate_tys(&mut self, _ty1: ty::Ty, _ty2: ty::Ty) {
        unimplemented!()
    }
}
