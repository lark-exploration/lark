use crate::hir;
use crate::hir::typeck::ErrorReported;
use crate::hir::typeck::HirTypeChecker;
use crate::intern::Intern;
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::BaseData;
use crate::ty::BaseKind;
use crate::ty::Erased;
use crate::ty::Generic;
use crate::ty::Generics;
use crate::ty::InferVarOr;
use crate::ty::Ty;
use crate::typeck::{BaseTypeChecker, Error, ErrorKind};
use crate::unify::InferVar;
use std::sync::Arc;

impl<Q> BaseTypeChecker<'_, Q>
where
    Q: crate::typeck::TypeCheckQueries,
{
    /// If `base` can be mapped to a concrete `BaseData`,
    /// invokes `op` and returns the resulting type.
    /// Otherwise, creates a type variable and returns that;
    /// once `base` can be mapped, the closure `op` will be
    /// invoked and the type variable will be unified.
    pub(super) fn with_base_data(
        &mut self,
        cause: hir::MetaIndex,
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

    pub(super) fn with_base_data_unify_with(
        &mut self,
        cause: hir::MetaIndex,
        base: Base,
        output_ty: Ty<BaseOnly>,
        op: impl FnOnce(&mut Self, BaseData<BaseOnly>) -> Ty<BaseOnly> + 'static,
    ) {
        match self.unify.shallow_resolve_data(base) {
            Ok(data) => {
                let ty1 = op(self, data);
                self.equate_types(cause, output_ty, ty1);
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

    pub(super) fn equate_types(
        &mut self,
        cause: hir::MetaIndex,
        ty1: Ty<BaseOnly>,
        ty2: Ty<BaseOnly>,
    ) {
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
                            self.equate_types(cause, g1, g2);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn boolean_type(&self) -> BaseTy {
        unimplemented!()
        // Ty {
        //     perm: Erased,
        //     base: InferVarOr::Known(BaseData {
        //         kind: BaseKind::Named(boolean_def_id),
        //         generics: Generics::empty(),
        //     }).intern(&self.interners),
        // }
    }

    pub(super) fn field_def_id(
        &mut self,
        base_data: BaseData<BaseOnly>,
        _field_name: hir::Identifier,
    ) -> Result<DefId, ErrorReported> {
        let BaseData { kind, generics: _ } = base_data;
        match kind {
            BaseKind::Named(_def_id) => unimplemented!(),

            BaseKind::Error => Err(ErrorReported),
        }
    }

    pub(super) fn method_def_id(
        &mut self,
        base_data: BaseData<BaseOnly>,
        _method_name: hir::Identifier,
    ) -> Result<DefId, ErrorReported> {
        let BaseData { kind, generics: _ } = base_data;
        match kind {
            BaseKind::Named(_def_id) => unimplemented!(),

            BaseKind::Error => Err(ErrorReported),
        }
    }
}
