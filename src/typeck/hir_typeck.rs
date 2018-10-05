use crate::hir;
use crate::hir::typeck::{ErrorReported, HirTypeChecker};
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::HasTyInternTables;
use crate::ty::map_family::Map;
use crate::ty::substitute::Substitution;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::Signature;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::ty::{BaseData, BaseKind};
use crate::ty::{Generic, Generics};
use crate::typeck::{BaseTypeChecker, Error, ErrorKind};
use crate::unify::InferVar;
use std::sync::Arc;

impl<Q> HirTypeChecker<Q, BaseOnly> for BaseTypeChecker<'_, Q>
where
    Q: crate::typeck::TypeCheckQueries,
{
    type FieldId = DefId;
    type MethodId = DefId;

    fn db(&self) -> &Q {
        self.db
    }

    /// Return the HIR that we are type-checking.
    fn hir(&self) -> &Arc<hir::FnBody> {
        &self.hir
    }

    fn report_error(&mut self, location: impl hir::HirIndex) {
        unimplemented!()
    }

    fn with_base_data(
        &mut self,
        cause: impl hir::HirIndex,
        base: Base,
        op: impl FnOnce(&mut Self, BaseData<BaseOnly>) -> Ty<BaseOnly> + 'static,
    ) -> Ty<BaseOnly> {
        unimplemented!()
    }

    fn substitute_ty(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Ty<BaseOnly>,
        field_decl_ty: Ty<Declaration>,
    ) -> Ty<BaseOnly> {
        unimplemented!()
    }

    fn substitute_signature(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Ty<BaseOnly>,
        field_decl_ty: Signature<Declaration>,
    ) -> Signature<BaseOnly> {
        unimplemented!()
    }

    /// Records the computed type for an expression, variable, etc.
    fn record_ty(&mut self, index: impl hir::HirIndex, ty: Ty<BaseOnly>) {
        let index: hir::MetaIndex = index.into();
        let old_value = self.results.types.insert(index, ty);
        assert!(old_value.is_none());
    }

    /// Lookup the type for a variable.
    fn variable_ty(&mut self, var: hir::Variable) -> Ty<BaseOnly> {
        self.results.types[&hir::MetaIndex::from(var)]
    }

    fn apply_user_perm(&mut self, _perm: hir::Perm, place_ty: Ty<BaseOnly>) -> Ty<BaseOnly> {
        // In the "erased type check", we don't care about permissions.
        place_ty
    }

    fn require_assignable(
        &mut self,
        expression: hir::Expression,
        value_ty: Ty<BaseOnly>,
        place_ty: Ty<BaseOnly>,
    ) {
        self.equate_types(expression.into(), value_ty, place_ty)
    }

    fn require_boolean(&mut self, expression: hir::Expression, value_ty: Ty<BaseOnly>) {
        self.equate_types(expression.into(), self.boolean_type(), value_ty)
    }

    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Ty<BaseOnly>,
        false_ty: Ty<BaseOnly>,
    ) -> Ty<BaseOnly> {
        self.equate_types(if_expression.into(), true_ty, false_ty);
        true_ty
    }

    /// Returns a type used to indicate that an error has been reported.
    fn error_type(&mut self) -> Ty<BaseOnly> {
        Ty {
            perm: Erased,
            base: BaseOnly::intern_base_data(
                self.db,
                BaseData {
                    kind: BaseKind::Error,
                    generics: Generics::empty(),
                },
            ),
        }
    }
}
