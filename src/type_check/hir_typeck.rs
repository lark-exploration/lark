use crate::hir;
use crate::hir::type_check::ErrorReported;
use crate::hir::type_check::HirTypeChecker;
use crate::hir::HirDatabase;
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::HasTyInternTables;
use crate::ty::interners::TyInternTables;
use crate::ty::map_family::Map;
use crate::ty::substitute::Substitution;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::Signature;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::ty::{BaseData, BaseKind};
use crate::ty::{Generic, Generics};
use crate::type_check::Error;
use crate::type_check::TypeCheckFamily;
use crate::type_check::TypeChecker;
use crate::type_check::TypeCheckerFields;
use crate::unify::{InferVar, UnificationTable};
use std::sync::Arc;

impl<DB> HirTypeChecker<DB, BaseOnly> for TypeChecker<'_, DB, BaseOnly>
where
    DB: crate::type_check::TypeCheckDatabase,
{
    type FieldId = DefId;
    type MethodId = DefId;

    fn db(&self) -> &DB {
        self.db
    }

    /// Return the HIR that we are type-checking.
    fn hir(&self) -> &Arc<hir::FnBody> {
        &self.hir
    }

    fn report_error(&mut self, location: impl hir::HirIndex) {
        self.results.errors.push(Error {
            location: location.into(),
        })
    }

    fn with_base_data(
        &mut self,
        cause: impl hir::HirIndex,
        base: Base,
        op: impl FnOnce(&mut Self, BaseData<BaseOnly>) -> Ty<BaseOnly> + 'static,
    ) -> Ty<BaseOnly> {
        self.with_base_data(cause.into(), base, op)
    }

    fn substitute<M>(
        &mut self,
        location: impl hir::HirIndex,
        owner_perm: Erased,
        owner_base_data: &BaseData<BaseOnly>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, BaseOnly>,
    {
        self.substitute(location.into(), owner_perm, owner_base_data, value)
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
        BaseOnly::require_assignable(self, expression, value_ty, place_ty)
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
        BaseOnly::least_upper_bound(self, if_expression, true_ty, false_ty)
    }

    /// Returns a type used to indicate that an error has been reported.
    fn error_type(&mut self) -> Ty<BaseOnly> {
        BaseOnly::error_type(self)
    }
}
