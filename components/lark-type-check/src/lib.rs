#![feature(crate_visibility_modifier)]
#![feature(never_type)]
#![feature(self_in_typedefs)]
#![feature(in_band_lifetimes)]

use generational_arena::Arena;
use hir;
use indices::IndexVec;
use lark_entity::{Entity, EntityTables};
use lark_error::WithError;
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
use parser::pos::Span;
use std::sync::Arc;

mod base_only;
mod hir_typeck;
mod ops;
mod query_definitions;
mod resolve_to_base_inferred;
mod substitute;

salsa::query_group! {
    pub trait TypeCheckDatabase: hir::HirDatabase + AsRef<BaseInferredTables> {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check(key: Entity) -> WithError<Arc<TypeCheckResults<BaseInferred>>> {
            type BaseTypeCheckQuery;
            use fn query_definitions::base_type_check;
        }
    }
}

struct TypeChecker<'me, DB: TypeCheckDatabase, F: TypeCheckFamily> {
    /// Salsa database.
    db: &'me DB,

    /// Intern tables for the family `F`. These are typically local to
    /// the type-check itself.
    f_tables: F::InternTables,

    /// Entity being type-checked.
    fn_entity: Entity,

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

    /// Results that we are generating.
    results: TypeCheckResults<F>,

    /// Information about each universe that we have created.
    universe_binders: IndexVec<Universe, UniverseBinder>,

    /// Errors that we encountered during the type-check.
    errors: Vec<Span>,
}

enum UniverseBinder {
    Root,
    FromItem(Entity),
}

/// An extension of the `TypeFamily` trait, describing a family of
/// types that can be used in the type-checker. This family must
/// support inference.
trait TypeCheckFamily: TypeFamily<Placeholder = Placeholder> {
    type TcBase: From<Self::Base>
        + Into<Self::Base>
        + Inferable<Self::InternTables, KnownData = BaseData<Self>>;

    /// Creates a new type with fresh inference variables.
    fn new_infer_ty(this: &mut impl TypeCheckerFields<Self>) -> Ty<Self>;

    /// Equates two types (producing an error if they are not
    /// equatable).
    fn equate_types(
        this: &mut impl TypeCheckerFields<Self>,
        cause: hir::MetaIndex,
        ty1: Ty<Self>,
        ty2: Ty<Self>,
    );

    /// Returns the type for booleans.
    fn boolean_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    /// Returns the type for signed integers.
    fn int_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    /// Returns the type for unsigned integers.
    fn uint_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    /// Returns the type for `()`.
    fn unit_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    /// Generates the constraint that a value with type `value_ty` is
    /// assignable to a place with the type `place_ty`; `expression`
    /// is the location that is requiring this type to be assignable
    /// (used in case of error).
    fn require_assignable(
        this: &mut impl TypeCheckerFields<Self>,
        expression: hir::Expression,
        value_ty: Ty<Self>,
        place_ty: Ty<Self>,
    );

    /// Given a permission `perm` written by the user, apply it to the
    /// type of the place `place_ty` that was accessed to produce the
    /// resulting type.
    fn apply_user_perm(
        this: &mut impl TypeCheckerFields<Self>,
        perm: hir::Perm,
        place_ty: Ty<Self>,
    ) -> Ty<Self>;

    /// Computes and returns the least-upper-bound of two types. If
    /// the types have no LUB, then reports an error at
    /// `if_expression`.
    fn least_upper_bound(
        this: &mut impl TypeCheckerFields<Self>,
        if_expression: hir::Expression,
        true_ty: Ty<Self>,
        false_ty: Ty<Self>,
    ) -> Ty<Self>;

    /// Substitute the given generics into the value `M`, which must
    /// be something in the `Declaration` type family (e.g., the type
    /// of a field).
    fn substitute<M>(
        this: &mut impl TypeCheckerFields<Self>,
        location: hir::MetaIndex,
        generics: &Generics<Self>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, Self>;

    /// Adjust the type of `value` to account for having been
    /// projected from an owned with the given permissions
    /// `owner_perm` (e.g., when accessing a field).
    fn apply_owner_perm<M>(
        this: &mut impl TypeCheckerFields<Self>,
        location: impl Into<hir::MetaIndex>,
        owner_perm: Self::Perm,
        value: M,
    ) -> M::Output
    where
        M: Map<Self, Self>;
}

/// Trait implemented by `TypeChecker` to allow access to a few useful
/// fields. This is used in the implementations of `TypeCheckFamily`.
trait TypeCheckerFields<F: TypeCheckFamily>:
    AsRef<F::InternTables> + AsRef<DeclarationTables> + AsRef<BaseInferredTables> + AsRef<EntityTables>
{
    type DB: TypeCheckDatabase;

    fn db(&self) -> &Self::DB;
    fn unify(&mut self) -> &mut UnificationTable<F::InternTables, hir::MetaIndex>;
    fn results(&mut self) -> &mut TypeCheckResults<F>;
    fn record_error(&mut self, location: impl Into<hir::MetaIndex>);
}

impl<'me, DB, F> TypeCheckerFields<F> for TypeChecker<'me, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
    Self: AsRef<F::InternTables>,
{
    type DB = DB;

    fn db(&self) -> &DB {
        &self.db
    }

    fn unify(&mut self) -> &mut UnificationTable<F::InternTables, hir::MetaIndex> {
        &mut self.unify
    }

    fn results(&mut self) -> &mut TypeCheckResults<F> {
        &mut self.results
    }

    fn record_error(&mut self, location: impl Into<hir::MetaIndex>) {
        self.record_error(location);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeCheckResults<F: TypeFamily> {
    /// The type computed for expressions, identified-expressions, and
    /// other things that have a type.
    types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    /// For "type-relative" identifiers, stores the entity that we resolved
    /// to. Examples:
    ///
    /// - `foo.bar` -- attached to the identifier `bar`, entity of the field
    /// - `foo.bar(..)` -- attached to the identifier `bar`, entity of the method
    /// - `Foo { a: b }` -- attached to the identifier `a`, entity of the field
    entities: std::collections::BTreeMap<hir::Identifier, Entity>,
}

impl<F: TypeFamily> TypeCheckResults<F> {
    /// Record the entity assigned with a given element of the HIR
    /// (e.g. the identifier of a field).
    fn record_entity(&mut self, index: hir::Identifier, entity: Entity) {
        self.entities.insert(index.into(), entity);
    }

    /// Record the type assigned with a given element of the HIR
    /// (typically an expression).
    fn record_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: Ty<F>) {
        self.types.insert(index.into(), ty);
    }

    /// Access the type stored for the given `index`, usually the
    /// index of an expression.
    pub fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.types[&index.into()]
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            types: Default::default(),
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
        let TypeCheckResults { types, entities } = self;
        TypeCheckResults {
            types: types.map(mapper),
            entities: entities.map(mapper),
        }
    }
}

impl<DB, F> AsRef<DeclarationTables> for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    fn as_ref(&self) -> &DeclarationTables {
        self.db.as_ref()
    }
}

impl<DB, F> AsRef<BaseInferredTables> for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    fn as_ref(&self) -> &BaseInferredTables {
        self.db.as_ref()
    }
}

impl<DB, F> AsRef<EntityTables> for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    fn as_ref(&self) -> &EntityTables {
        self.db.as_ref()
    }
}
