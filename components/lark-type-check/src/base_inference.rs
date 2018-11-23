use crate::substitute::Substitution;
use crate::TypeCheckDatabase;
use crate::TypeCheckResults;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use lark_entity::Entity;
use lark_hir as hir;
use lark_ty::base_inference::{BaseInference, BaseInferenceTables};
use lark_ty::declaration::Declaration;
use lark_ty::identity::Identity;
use lark_ty::map_family::Map;
use lark_ty::BaseKind;
use lark_ty::Erased;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::Ty;

impl<DB> TypeCheckerFamilyDependentExt<BaseInference>
    for TypeChecker<'me, DB, BaseInference, TypeCheckResults<BaseInference>>
where
    DB: TypeCheckDatabase,
{
    fn new_infer_ty(&mut self) -> Ty<BaseInference> {
        Ty {
            repr: Erased,
            perm: Erased,
            base: self.unify.new_inferable(),
        }
    }

    fn equate_types(
        &mut self,
        cause: hir::MetaIndex,
        ty1: Ty<BaseInference>,
        ty2: Ty<BaseInference>,
    ) {
        let Ty {
            repr: Erased,
            perm: Erased,
            base: base1,
        } = ty1;
        let Ty {
            repr: Erased,
            perm: Erased,
            base: base2,
        } = ty2;

        match self.unify.unify(cause, base1, base2) {
            Ok(()) => {}

            Err((data1, data2)) => {
                match (data1.kind, data2.kind) {
                    (BaseKind::Error, _) => {
                        self.propagate_error(cause, &data2.generics);
                        return;
                    }
                    (_, BaseKind::Error) => {
                        self.propagate_error(cause, &data1.generics);
                        return;
                    }
                    _ => {}
                }

                if data1.kind != data2.kind {
                    self.record_error("Mismatched types", cause);
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (GenericKind::Ty(g1), GenericKind::Ty(g2)) => {
                            self.equate_types(cause, g1, g2);
                        }
                    }
                }
            }
        }
    }

    fn apply_user_perm(
        &mut self,
        _perm: hir::Perm,
        place_ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        // In the "erased type check", we don't care about permissions.
        place_ty
    }

    fn require_assignable(
        &mut self,
        expression: hir::Expression,
        value_ty: Ty<BaseInference>,
        place_ty: Ty<BaseInference>,
    ) {
        self.equate_types(expression.into(), value_ty, place_ty)
    }

    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Ty<BaseInference>,
        false_ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        self.equate_types(if_expression.into(), true_ty, false_ty);
        true_ty
    }

    fn substitute<M>(
        &mut self,
        _location: impl Into<hir::MetaIndex>,
        generics: &Generics<BaseInference>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, BaseInference>,
    {
        value.map(&mut Substitution::new(self, self, generics))
    }

    fn apply_owner_perm<M>(
        &mut self,
        _location: impl Into<hir::MetaIndex>,
        _owner_perm: Erased,
        value: M,
    ) -> M::Output
    where
        M: Map<BaseInference, BaseInference>,
    {
        value.map(&mut Identity::new(self))
    }

    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<BaseInference>) {
        self.storage.record_ty(var, ty);
    }

    fn record_expression_ty(
        &mut self,
        expr: hir::Expression,
        ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        self.storage.record_ty(expr, ty);
        ty
    }

    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<BaseInference>) -> Ty<BaseInference> {
        self.storage.record_ty(place, ty);
        ty
    }

    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<BaseInference> {
        self.storage.opt_ty(var).unwrap_or_else(|| {
            let ty = self.new_infer_ty();
            self.storage.record_ty(var, ty);
            ty
        })
    }

    fn record_entity(&mut self, index: hir::Identifier, entity: Entity) {
        self.storage.record_entity(index, entity);
    }

    fn record_entity_and_get_generics(
        &mut self,
        index: impl Into<hir::MetaIndex>,
        entity: Entity,
    ) -> Generics<BaseInference> {
        let index: hir::MetaIndex = index.into();
        self.storage.record_entity(index, entity);
        let generics = self.inference_variables_for(entity);
        self.storage.record_generics(index, &generics);
        generics
    }
}

impl<DB, S> AsRef<BaseInferenceTables> for TypeChecker<'_, DB, BaseInference, S>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &BaseInferenceTables {
        &self.f_tables
    }
}
