//! Definition of a type family + type-checker methods for doing "base
//! only" inference. This is inference where we ignore permissions and
//! representations and focus only on the base types.

use crate::substitute::Substitution;
use crate::TypeCheckDatabase;
use crate::TypeCheckResults;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_hir as hir;
use lark_ty::declaration::Declaration;
use lark_ty::identity::Identity;
use lark_ty::map_family::Map;
use lark_ty::BaseData;
use lark_ty::BaseKind;
use lark_ty::Erased;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::InferVarOr;
use lark_ty::Placeholder;
use lark_ty::ReprKind;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_unify::{InferVar, Inferable};

/// Type family for "base inference" -- inferring just the base types.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BaseInference;

impl TypeFamily for BaseInference {
    type InternTables = BaseInferenceTables;
    type Repr = Erased;
    type Perm = Erased;
    type Base = Base;
    type Placeholder = Placeholder;

    fn own_perm(_tables: &dyn AsRef<BaseInferenceTables>) -> Erased {
        Erased
    }

    fn known_repr(_tables: &dyn AsRef<BaseInferenceTables>, _repr_kind: ReprKind) -> Self::Repr {
        Erased
    }

    fn intern_base_data(
        tables: &dyn AsRef<BaseInferenceTables>,
        base_data: BaseData<Self>,
    ) -> Self::Base {
        InferVarOr::Known(base_data).intern(tables)
    }
}

indices::index_type! {
    pub struct Base { .. }
}

impl Inferable<BaseInferenceTables> for Base {
    type KnownData = BaseData<BaseInference>;
    type Data = InferVarOr<BaseData<BaseInference>>;

    /// Check if this is an inference variable and return the inference
    /// index if so.
    fn as_infer_var(self, interners: &BaseInferenceTables) -> Option<InferVar> {
        match self.untern(interners) {
            InferVarOr::InferVar(var) => Some(var),
            InferVarOr::Known(_) => None,
        }
    }

    /// Create an inferable representing the inference variable `var`.
    fn from_infer_var(var: InferVar, interners: &BaseInferenceTables) -> Self {
        let i: InferVarOr<BaseData<BaseInference>> = InferVarOr::InferVar(var);
        i.intern(interners)
    }

    /// Asserts that this is not an inference variable and returns the
    /// "known data" that it represents.
    fn assert_known(self, interners: &BaseInferenceTables) -> Self::KnownData {
        self.untern(interners).assert_known()
    }
}

debug::debug_fallback_impl!(Base);

impl<Cx> debug::FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseInferenceTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

intern::intern_tables! {
    pub struct BaseInferenceTables {
        struct BaseInferenceTablesData {
            base_inference_base: map(Base, InferVarOr<BaseData<BaseInference>>),
        }
    }
}

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
        cause: impl Into<hir::MetaIndex>,
        ty1: Ty<BaseInference>,
        ty2: Ty<BaseInference>,
    ) {
        let cause = cause.into();

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

    fn require_assignable(&mut self, expression: hir::Expression, place_ty: Ty<BaseInference>) {
        let value_ty = self.storage.ty(expression);
        self.equate_types(expression, value_ty, place_ty)
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
