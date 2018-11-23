#![feature(crate_visibility_modifier)]
#![feature(never_type)]
#![feature(self_in_typedefs)]
#![feature(in_band_lifetimes)]
#![feature(trait_alias)]

use generational_arena::Arena;
use indices::IndexVec;
use lark_entity::{Entity, EntityTables};
use lark_error::{Diagnostic, WithError};
use lark_hir as hir;
use lark_parser::ParserDatabase;
use lark_ty::base_inferred::BaseInferred;
use lark_ty::base_inferred::BaseInferredTables;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::map_family::{FamilyMapper, Map};
use lark_ty::BaseData;
use lark_ty::Generics;
use lark_ty::Placeholder;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_ty::Universe;
use lark_unify::InferVar;
use lark_unify::Inferable;
use lark_unify::UnificationTable;
use map::FxIndexMap;
use std::sync::Arc;

mod base_inference;
mod hir_typeck;
mod ops;
mod query_definitions;
mod resolve_to_base_inferred;
mod substitute;

salsa::query_group! {
    pub trait TypeCheckDatabase: ParserDatabase + AsRef<BaseInferredTables> {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check(key: Entity) -> WithError<Arc<TypeCheckResults<BaseInferred>>> {
            type BaseTypeCheckQuery;
            use fn query_definitions::base_type_check;
        }
    }
}

struct TypeChecker<'me, DB: TypeCheckDatabase, F: TypeCheckerFamily, S> {
    /// Salsa database.
    db: &'me DB,

    /// Intern tables for the family `F`. These are typically local to
    /// the type-check itself.
    f_tables: F::InternTables,

    /// Entity being type-checked.
    fn_entity: Entity,

    /// Storage that depends on the type-checker family.
    storage: S,

    /// HIR for the `fn_entity` being type-checked.
    hir: Arc<hir::FnBody>,

    /// Arena where we allocate suspended type-check operations;
    /// operations are suspended until type-inference variables
    /// get unified.
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp<Self>>>,

    /// Map storing blocked operations: once the given infer variable
    /// is unified, we should execute the operation.
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,

    /// Unification table for the type-check family.
    unify: UnificationTable<F::InternTables, hir::MetaIndex>,

    /// Information about each universe that we have created.
    universe_binders: IndexVec<Universe, UniverseBinder>,

    /// Errors that we encountered during the type-check.
    errors: Vec<Diagnostic>,
}

enum UniverseBinder {
    Root,
    FromItem(Entity),
}

/// A trait alias for a type family that has `Placeholder` mapped to
/// `Placeholder`.  These are the kinds of type families the type
/// checker can use.
trait TypeCheckerFamily: TypeFamily<Placeholder = Placeholder> {}
impl<T: TypeFamily<Placeholder = Placeholder>> TypeCheckerFamily for T {}

/// An "extension trait" for the `TypeChecker` that defines the
/// operations which differ depending on the active type-family.  You
/// will find e.g. one implementation of this for the `BaseInference`
/// type family and one for more complete inference (not yet
/// implemented).
trait TypeCheckerFamilyDependentExt<F: TypeCheckerFamily>: AsRef<F::InternTables>
where
    F::Base: Inferable<F::InternTables, KnownData = BaseData<F>>,
{
    /// Creates a new type with fresh inference variables.
    fn new_infer_ty(&mut self) -> Ty<F>;

    /// Equates two types (producing an error if they are not
    /// equatable).
    fn equate_types(&mut self, cause: hir::MetaIndex, ty1: Ty<F>, ty2: Ty<F>);

    /// Generates the constraint that a value with type `value_ty` is
    /// assignable to a place with the type `place_ty`; `expression`
    /// is the location that is requiring this type to be assignable
    /// (used in case of error).
    fn require_assignable(&mut self, expression: hir::Expression, value_ty: Ty<F>, place_ty: Ty<F>);

    /// Given a permission `perm` written by the user, apply it to the
    /// type of the place `place_ty` that was accessed to produce the
    /// resulting type.
    fn apply_user_perm(&mut self, perm: hir::Perm, place_ty: Ty<F>) -> Ty<F>;

    /// Computes and returns the least-upper-bound of two types. If
    /// the types have no LUB, then reports an error at
    /// `if_expression`.
    fn least_upper_bound(
        &mut self,
        if_expression: hir::Expression,
        true_ty: Ty<F>,
        false_ty: Ty<F>,
    ) -> Ty<F>;

    /// Substitute the given generics into the value `M`, which must
    /// be something in the `Declaration` type family (e.g., the type
    /// of a field).
    fn substitute<M>(
        &mut self,
        location: impl Into<hir::MetaIndex>,
        generics: &Generics<F>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, F>;

    /// Adjust the type of `value` to account for having been
    /// projected from an owned with the given permissions
    /// `owner_perm` (e.g., when accessing a field).
    fn apply_owner_perm<M>(
        &mut self,
        location: impl Into<hir::MetaIndex>,
        owner_perm: F::Perm,
        value: M,
    ) -> M::Output
    where
        M: Map<F, F>;

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn assign_variable_ty(&mut self, var: hir::Variable, ty: Ty<F>);

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn assign_expression_ty(&mut self, expr: hir::Expression, ty: Ty<F>) -> Ty<F>;

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn assign_place_ty(&mut self, place: hir::Place, ty: Ty<F>) -> Ty<F>;

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<F>;

    /// Record the entity to which a particular identifier in the HIR resolved.
    /// Used for:
    ///
    /// - field names, in places and aggregate expressions
    /// - method names, in calls
    fn record_entity(&mut self, index: hir::Identifier, entity: Entity);

    /// Records that `index` refers to `entity` and returns the
    /// generic parameters it uses to do so; may instantiate fresh
    /// type variables.
    fn record_entity_and_get_generics(
        &mut self,
        index: impl Into<hir::MetaIndex>,
        entity: Entity,
    ) -> Generics<F>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeCheckResults<F: TypeFamily> {
    /// The type computed for expressions, identified-expressions, and
    /// other things that have a type.
    types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    /// For references to entities, the generics applied.
    generics: std::collections::BTreeMap<hir::MetaIndex, Generics<F>>,

    /// For "type-relative" identifiers, stores the entity that we resolved
    /// to. Examples:
    ///
    /// - `foo.bar` -- attached to the identifier `bar`, entity of the field
    /// - `foo.bar(..)` -- attached to the identifier `bar`, entity of the method
    /// - `Foo { a: b }` -- attached to the identifier `a`, entity of the field
    /// - `foo` -- when an identifier refers to an entity
    entities: std::collections::BTreeMap<hir::MetaIndex, Entity>,
}

impl<F: TypeFamily> TypeCheckResults<F> {
    /// Record the entity assigned with a given element of the HIR
    /// (e.g. the identifier of a field).
    fn record_entity(&mut self, index: impl Into<hir::MetaIndex>, entity: Entity) {
        let index = index.into();
        let old_entity = self.entities.insert(index, entity);
        assert!(
            old_entity.is_none(),
            "index {:?} already had an entity",
            index
        );
    }

    /// Record the type assigned with a given element of the HIR
    /// (typically an expression).
    fn record_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: Ty<F>) {
        let index = index.into();
        let old_ty = self.types.insert(index, ty);
        assert!(old_ty.is_none(), "index {:?} already had a type", index);
    }

    /// Record the generics for a given element of the HIR
    /// (typically an expression).
    fn record_generics(&mut self, index: impl Into<hir::MetaIndex>, g: &Generics<F>) {
        let index = index.into();
        let old_generics = self.generics.insert(index, g.clone());
        assert!(
            old_generics.is_none(),
            "index {:?} already had generics",
            index
        );
    }

    /// Access the type stored for the given `index`, usually the
    /// index of an expression.
    pub fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.types[&index.into()]
    }

    /// Load the type for `index`, if any is stored, else return `None`.
    pub fn opt_ty(&self, index: impl Into<hir::MetaIndex>) -> Option<Ty<F>> {
        self.types.get(&index.into()).cloned()
    }

    /// Check whether there is a type recorded for `index`.
    pub fn has_recorded_ty(&self, index: impl Into<hir::MetaIndex>) -> bool {
        self.types.contains_key(&index.into())
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            types: Default::default(),
            generics: Default::default(),
            entities: Default::default(),
        }
    }
}

impl<S, T> Map<S, T> for TypeCheckResults<S>
where
    S: TypeFamily,
    T: TypeFamily,
{
    type Output = TypeCheckResults<T>;

    fn map(&self, mapper: &mut impl FamilyMapper<S, T>) -> Self::Output {
        let TypeCheckResults {
            types,
            generics,
            entities,
        } = self;
        TypeCheckResults {
            types: types.map(mapper),
            generics: generics.map(mapper),
            entities: entities.map(mapper),
        }
    }
}

impl<DB, F, S> AsRef<DeclarationTables> for TypeChecker<'_, DB, F, S>
where
    DB: TypeCheckDatabase,
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &DeclarationTables {
        self.db.as_ref()
    }
}

impl<DB, F, S> AsRef<BaseInferredTables> for TypeChecker<'_, DB, F, S>
where
    DB: TypeCheckDatabase,
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &BaseInferredTables {
        self.db.as_ref()
    }
}

impl<DB, F, S> AsRef<EntityTables> for TypeChecker<'_, DB, F, S>
where
    DB: TypeCheckDatabase,
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &EntityTables {
        self.db.as_ref()
    }
}
