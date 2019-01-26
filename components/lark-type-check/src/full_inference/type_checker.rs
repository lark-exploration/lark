use crate::full_inference::apply_perm::ApplyPerm;
use crate::full_inference::constraint::Constraint;
use crate::full_inference::constraint::ConstraintAt;
use crate::full_inference::perm::Perm;
use crate::full_inference::perm::PermData;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::results::TypeCheckResults;
use crate::substitute::Substitution;
use crate::substitute::SubstitutionDelegate;
use crate::HirLocation;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use crate::TypeCheckerVariableExt;
use lark_collections::{FxIndexSet, IndexVec};
use lark_entity::Entity;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_pretty_print::PrettyPrint;
use lark_ty::declaration;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclaredPermKind;
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
#[derive(Default)]
crate struct FullInferenceStorage {
    /// Set of all permission veriables created. Right now we don't
    /// keep any information about them in particular.
    perm_vars: IndexVec<PermVar, ()>,

    /// Constraints we have created during type-checking thus far.
    crate constraints: FxIndexSet<ConstraintAt>,

    /// Results we have generated thus far.
    crate results: TypeCheckResults<FullInference>,
}

impl FullInferenceStorage {
    crate fn new_inferred_perm(&mut self, tables: &dyn AsRef<FullInferenceTables>) -> Perm {
        PermData::Inferred(self.perm_vars.push(())).intern(tables)
    }

    crate fn add_constraint(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        constraint: Constraint,
    ) {
        self.constraints.insert(ConstraintAt {
            cause: cause.into(),
            location: location.into(),
            constraint,
        });
    }
}

impl TypeCheckerFamilyDependentExt<FullInference>
    for TypeChecker<'me, FullInference, FullInferenceStorage>
{
    fn substitute<M>(
        &mut self,
        _location: impl Into<HirLocation>,
        generics: &Generics<FullInference>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, FullInference>,
    {
        value.map(&mut Substitution::new(self, generics))
    }

    fn apply_owner_perm(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        owner_perm: Perm,
        field_ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.apply_access_perm(cause.into(), location.into(), owner_perm, field_ty)
    }

    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<FullInference>) {
        self.storage.results.record_max_ty(var, ty);
    }

    fn record_max_expression_ty(
        &mut self,
        expression: hir::Expression,
        max_ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        // When assigning a value into a place, we do not *have* to
        // transfer the full permissions of that value into the
        // place. So create a permission variable for the amount of
        // access and use it to modify the value access ty.
        let access_perm = self.storage.new_inferred_perm(&self.f_tables);
        let access_ty =
            self.apply_access_perm(expression.into(), expression.into(), access_perm, max_ty);

        self.storage.results.record_max_ty(expression, max_ty);
        self.storage.results.record_access_ty(expression, access_ty);
        self.storage
            .results
            .record_access_permission(expression, access_perm);

        access_ty
    }

    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<FullInference>) -> Ty<FullInference> {
        self.storage.results.record_max_ty(place, ty);
        ty
    }

    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<FullInference> {
        self.storage.results.opt_ty(var).unwrap_or_else(|| {
            let ty = self.new_variable();
            self.storage.results.record_max_ty(var, ty);
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

impl TypeCheckerVariableExt<FullInference, Ty<FullInference>>
    for TypeChecker<'me, FullInference, FullInferenceStorage>
{
    fn new_variable(&mut self) -> Ty<FullInference> {
        Ty {
            repr: Erased,
            perm: self.storage.new_inferred_perm(&self.f_tables),
            base: self.unify.new_inferable(),
        }
    }

    fn equate(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        ty1: Ty<FullInference>,
        ty2: Ty<FullInference>,
    ) {
        let cause: hir::MetaIndex = cause.into();
        let location: HirLocation = location.into();

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

        self.storage.add_constraint(
            cause,
            location,
            Constraint::PermEquate { a: perm1, b: perm2 },
        );

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
                    self.record_error(
                        format!(
                            "mismatched types ({} vs {})",
                            data1.kind.pretty_print(self.db),
                            data2.kind.pretty_print(self.db)
                        ),
                        cause,
                    );
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (GenericKind::Ty(g1), GenericKind::Ty(g2)) => {
                            self.equate(cause, location, g1, g2);
                        }
                    }
                }
            }
        }
    }
}

impl<S> AsRef<FullInferenceTables> for TypeChecker<'_, FullInference, S> {
    fn as_ref(&self) -> &FullInferenceTables {
        &self.f_tables
    }
}

impl SubstitutionDelegate<FullInference> for TypeChecker<'me, FullInference, FullInferenceStorage> {
    fn as_f_tables(&self) -> &FullInferenceTables {
        self.as_ref()
    }

    fn map_repr_perm(&mut self, _repr: ReprKind, perm: declaration::Perm) -> (Erased, Perm) {
        let perm = self.map_perm(perm);

        (Erased, perm)
    }

    fn map_perm(&mut self, perm: declaration::Perm) -> Perm {
        match perm.untern(self) {
            DeclaredPermKind::Own => PermData::Known(PermKind::Own).intern(self),
        }
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
