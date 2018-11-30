use crate::full_inference::constraint::Constraint;
use crate::full_inference::constraint::ConstraintAt;
use crate::full_inference::perm::Perm;
use crate::full_inference::perm::PermData;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::substitute::Substitution;
use crate::substitute::SubstitutionDelegate;
use crate::TypeCheckDatabase;
use crate::TypeCheckResults;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use lark_collections::FxIndexSet;
use lark_entity::Entity;
use lark_hir as hir;
use lark_indices::IndexVec;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_ty::declaration;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclaredPermKind;
use lark_ty::identity::Identity;
use lark_ty::map_family::Map;
use lark_ty::BaseKind;
use lark_ty::Erased;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::PermKind;
use lark_ty::ReprKind;
use lark_ty::Ty;

/// The full-inference-specific data stored in the type-checker when
/// doing full inference.
crate struct FullInferenceStorage {
    /// Set of all permission veriables created. Right now we don't
    /// keep any information about them in particular.
    perm_vars: IndexVec<PermVar, ()>,

    /// Constraints we have created during type-checking thus far.
    constraints: FxIndexSet<ConstraintAt>,

    /// Results we have generated thus far.
    results: TypeCheckResults<FullInference>,
}

impl FullInferenceStorage {
    fn new_inferred_perm(&mut self, tables: &dyn AsRef<FullInferenceTables>) -> Perm {
        PermData::Inferred(self.perm_vars.push(())).intern(tables)
    }

    fn add_constraint(&mut self, cause: impl Into<hir::MetaIndex>, constraint: Constraint) {
        self.constraints.insert(ConstraintAt {
            cause: cause.into(),
            constraint,
        });
    }
}

impl<DB> TypeCheckerFamilyDependentExt<FullInference>
    for TypeChecker<'me, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn new_infer_ty(&mut self) -> Ty<FullInference> {
        Ty {
            repr: Erased,
            perm: self.storage.new_inferred_perm(&self.f_tables),
            base: self.unify.new_inferable(),
        }
    }

    fn equate_types(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        ty1: Ty<FullInference>,
        ty2: Ty<FullInference>,
    ) {
        let cause = cause.into();

        let Ty {
            repr: Erased,
            perm: perm1,
            base: base1,
        } = ty1;
        let Ty {
            repr: Erased,
            perm: perm2,
            base: base2,
        } = ty2;

        self.storage
            .add_constraint(cause, Constraint::PermEquate { a: perm1, b: perm2 });

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

    fn require_assignable(&mut self, expression: hir::Expression, place_ty: Ty<FullInference>) {
        let value_ty = self.storage.results.ty(expression);
        self.equate_types(expression, value_ty, place_ty)
    }

    fn substitute<M>(
        &mut self,
        _location: impl Into<hir::MetaIndex>,
        generics: &Generics<FullInference>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, FullInference>,
    {
        value.map(&mut Substitution::new(self, generics))
    }

    fn apply_owner_perm<M>(
        &mut self,
        _location: impl Into<hir::MetaIndex>,
        _owner_perm: Perm,
        value: M,
    ) -> M::Output
    where
        M: Map<FullInference, FullInference>,
    {
        value.map(&mut Identity::new(self))
    }

    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<FullInference>) {
        self.storage.results.record_ty(var, ty);
    }

    fn record_expression_ty(
        &mut self,
        expr: hir::Expression,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.storage.results.record_ty(expr, ty);
        ty
    }

    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<FullInference>) -> Ty<FullInference> {
        self.storage.results.record_ty(place, ty);
        ty
    }

    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<FullInference> {
        self.storage.results.opt_ty(var).unwrap_or_else(|| {
            let ty = self.new_infer_ty();
            self.storage.results.record_ty(var, ty);
            ty
        })
    }

    fn record_entity(&mut self, index: hir::Identifier, entity: Entity) {
        self.storage.results.record_entity(index, entity);
    }

    fn record_entity_and_get_generics(
        &mut self,
        index: impl Into<hir::MetaIndex>,
        entity: Entity,
    ) -> Generics<FullInference> {
        let index: hir::MetaIndex = index.into();
        self.storage.results.record_entity(index, entity);
        let generics = self.inference_variables_for(entity);
        self.storage.results.record_generics(index, &generics);
        generics
    }
}

impl<DB, S> AsRef<FullInferenceTables> for TypeChecker<'_, DB, FullInference, S>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &FullInferenceTables {
        &self.f_tables
    }
}

impl<DB> SubstitutionDelegate<FullInference>
    for TypeChecker<'me, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn as_f_tables(&self) -> &FullInferenceTables {
        self.as_ref()
    }

    fn map_repr_perm(&mut self, _repr: ReprKind, perm: declaration::Perm) -> (Erased, Perm) {
        let perm = match perm.untern(self) {
            DeclaredPermKind::Own => PermData::Known(PermKind::Own).intern(self),
        };

        (Erased, perm)
    }

    fn apply_repr_perm(
        &mut self,
        _repr: ReprKind,
        perm: declaration::Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        match perm.untern(self) {
            DeclaredPermKind::Own => {
                // If you have `own T` and you substitute `U` for `T`,
                // the result is just `U`.
                ty
            }
        }
    }
}
