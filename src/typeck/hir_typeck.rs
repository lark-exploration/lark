use crate::hir;
use crate::hir::typeck::{ErrorReported, HirTypeChecker, MethodSignature};
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::interners::HasTyInternTables;
use crate::ty::map_family::Map;
use crate::ty::substitute::Substitution;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::ty::{BaseData, BaseKind};
use crate::ty::{Generic, Generics};
use crate::typeck::{BaseTypeChecker, Error, ErrorKind};
use crate::unify::InferVar;
use std::sync::Arc;

impl<Q> HirTypeChecker for BaseTypeChecker<'_, Q>
where
    Q: crate::typeck::TypeCheckQueries,
{
    type FieldId = DefId;
    type MethodId = DefId;
    type Ty = BaseTy;

    /// Return the HIR that we are type-checking.
    fn hir(&self) -> &Arc<hir::FnBody> {
        &self.hir
    }

    /// Fetch the field of the given field from the given owner,
    /// appropriately substituted.
    fn field_ty(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Self::Ty,
        field_def_id: Self::FieldId,
    ) -> Self::Ty {
        self.with_base_data(location.into(), owner_ty.base, move |this, base_data| {
            let field_decl_ty = this.db.ty().get(field_def_id);
            field_decl_ty.map(&mut Substitution::new(
                this.db.ty_intern_tables(),
                &base_data.generics,
            ))
        })
    }

    /// Given the type of a field and its owner, substitute any generics appropriately
    /// and return an instantiated type.
    fn method_sig(
        &mut self,
        _location: impl hir::HirIndex,
        _owner_ty: Self::Ty,
        _method_def_id: Self::MethodId,
    ) -> MethodSignature<Self> {
        unimplemented!()
    }

    /// Records the computed type for an expression, variable, etc.
    fn record_ty(&mut self, index: impl hir::HirIndex, ty: Self::Ty) {
        let index: hir::MetaIndex = index.into();
        let old_value = self.results.types.insert(index, ty);
        assert!(old_value.is_none());
    }

    /// Lookup the type for a variable.
    fn variable_ty(&mut self, var: hir::Variable) -> Self::Ty {
        self.results.types[&hir::MetaIndex::from(var)]
    }

    fn apply_user_perm(&mut self, _perm: hir::Perm, place_ty: Self::Ty) -> Self::Ty {
        // In the "erased type check", we don't care about permissions.
        place_ty
    }

    fn require_assignable(
        &mut self,
        expression: hir::Expression,
        value_ty: Self::Ty,
        place_ty: Self::Ty,
    ) {
        self.equate_types(expression.into(), value_ty, place_ty)
    }

    fn require_boolean(&mut self, expression: hir::Expression, value_ty: Self::Ty) {
        self.equate_types(expression.into(), self.boolean_type(), value_ty)
    }

    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Self::Ty,
        false_ty: Self::Ty,
    ) -> Self::Ty {
        self.equate_types(if_expression.into(), true_ty, false_ty);
        true_ty
    }

    fn with_field(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Self::Ty,
        field_name: hir::Identifier,
        op: impl FnOnce(&mut Self, Self::FieldId) -> Self::Ty + 'static,
    ) -> Self::Ty {
        self.with_base_data(
            location.into(),
            owner_ty.base,
            move |this, base_data| match this.field_def_id(base_data, field_name) {
                Ok(def_id) => op(this, def_id),
                Err(ErrorReported) => this.error_type(),
            },
        )
    }

    fn with_method(
        &mut self,
        location: impl hir::HirIndex,
        owner_ty: Self::Ty,
        method_name: hir::Identifier,
        op: impl FnOnce(&mut Self, Self::MethodId) -> Self::Ty + 'static,
    ) -> Self::Ty {
        self.with_base_data(
            location.into(),
            owner_ty.base,
            move |this, base_data| match this.method_def_id(base_data, method_name) {
                Ok(def_id) => op(this, def_id),
                Err(ErrorReported) => this.error_type(),
            },
        )
    }

    /// Returns a type used to indicate that an error has been reported.
    fn error_type(&mut self) -> Self::Ty {
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
