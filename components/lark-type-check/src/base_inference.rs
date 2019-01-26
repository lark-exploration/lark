//! Definition of a type family + type-checker methods for doing "base
//! only" inference. This is inference where we ignore permissions and
//! representations and focus only on the base types.

use crate::results::TypeCheckResults;
use crate::substitute::Substitution;
use crate::substitute::SubstitutionDelegate;
use crate::HirLocation;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use crate::TypeCheckerVariableExt;
use lark_debug_derive::DebugWith;
use lark_debug_with::DebugWith;
use lark_entity::Entity;
use lark_hir as hir;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_pretty_print::PrettyPrint;
use lark_ty::declaration;
use lark_ty::declaration::Declaration;
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

crate mod query_definition;

mod resolve_to_base_inferred;

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

lark_collections::index_type! {
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

lark_debug_with::debug_fallback_impl!(Base);

impl<Cx> lark_debug_with::FmtWithSpecialized<Cx> for Base
where
    Cx: AsRef<BaseInferenceTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.untern(cx).fmt_with(cx, fmt)
    }
}

lark_intern::intern_tables! {
    pub struct BaseInferenceTables {
        struct BaseInferenceTablesData {
            base_inference_base: map(Base, InferVarOr<BaseData<BaseInference>>),
        }
    }
}

impl TypeCheckerFamilyDependentExt<BaseInference>
    for TypeChecker<'_, BaseInference, TypeCheckResults<BaseInference>>
{
    fn substitute<M>(
        &mut self,
        _location: impl Into<HirLocation>,
        generics: &Generics<BaseInference>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, BaseInference>,
    {
        value.map(&mut Substitution::new(self, generics))
    }

    fn apply_owner_perm(
        &mut self,
        _cause: impl Into<hir::MetaIndex>,
        _location: impl Into<HirLocation>,
        _access_perm: Erased,
        field_ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        field_ty
    }

    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<BaseInference>) {
        self.storage.record_max_ty(var, ty);
    }

    fn record_max_expression_ty(
        &mut self,
        expr: hir::Expression,
        ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        self.storage.record_max_ty(expr, ty);
        self.storage.record_access_ty(expr, ty);
        self.storage.record_access_permission(expr, Erased);
        ty
    }

    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<BaseInference>) -> Ty<BaseInference> {
        self.storage.record_max_ty(place, ty);
        ty
    }

    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<BaseInference> {
        self.storage.opt_ty(var).unwrap_or_else(|| {
            let ty = self.new_variable();
            self.storage.record_max_ty(var, ty);
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

impl TypeCheckerVariableExt<BaseInference, Ty<BaseInference>>
    for TypeChecker<'_, BaseInference, TypeCheckResults<BaseInference>>
{
    fn new_variable(&mut self) -> Ty<BaseInference> {
        Ty {
            repr: Erased,
            perm: Erased,
            base: self.unify.new_inferable(),
        }
    }

    fn equate(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        ty1: Ty<BaseInference>,
        ty2: Ty<BaseInference>,
    ) {
        let cause: hir::MetaIndex = cause.into();
        let location: HirLocation = location.into();

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

impl<S> AsRef<BaseInferenceTables> for TypeChecker<'_, BaseInference, S> {
    fn as_ref(&self) -> &BaseInferenceTables {
        &self.f_tables
    }
}

impl SubstitutionDelegate<BaseInference>
    for TypeChecker<'_, BaseInference, TypeCheckResults<BaseInference>>
{
    fn as_f_tables(&self) -> &BaseInferenceTables {
        self.as_ref()
    }

    fn map_repr_perm(&mut self, _repr: ReprKind, _perm: declaration::Perm) -> (Erased, Erased) {
        (Erased, Erased)
    }

    fn map_perm(&mut self, _perm: declaration::Perm) -> Erased {
        Erased
    }

    fn apply_repr_perm(
        &mut self,
        _repr: ReprKind,
        _perm: declaration::Perm,
        ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        ty
    }
}
