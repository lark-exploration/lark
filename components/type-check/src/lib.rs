#![feature(crate_visibility_modifier)]
#![feature(never_type)]
#![feature(self_in_typedefs)]
#![feature(in_band_lifetimes)]

use generational_arena::Arena;
use hir;
use indices::IndexVec;
use lark_entity::Entity;
use map::FxIndexMap;
use std::sync::Arc;
use ty::base_inferred::BaseInferred;
use ty::declaration::Declaration;
use ty::interners::TyInternTables;
use ty::map_family::Map;
use ty::Generics;
use ty::Placeholder;
use ty::Ty;
use ty::TypeFamily;
use ty::Universe;
use unify::InferVar;
use unify::Inferable;
use unify::UnificationTable;

mod base_only;
mod hir_typeck;
mod ops;
mod query_definitions;
mod substitute;

salsa::query_group! {
    pub trait TypeCheckDatabase: hir::HirDatabase + AsRef<TyInternTables> {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check(key: Entity) -> TypeCheckResults<BaseInferred> {
            type BaseTypeCheckQuery;
            use fn query_definitions::base_type_check;
        }
    }
}

struct TypeChecker<'db, DB: TypeCheckDatabase, F: TypeCheckFamily> {
    db: &'db DB,
    fn_item_id: Entity,
    hir: Arc<hir::FnBody>,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp<Self>>>,
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,
    unify: UnificationTable<TyInternTables, hir::MetaIndex>,
    results: TypeCheckResults<F>,

    /// Information about each universe that we have created.
    universe_binders: IndexVec<Universe, UniverseBinder>,
}

enum UniverseBinder {
    Root,
    FromItem(Entity),
}

trait TypeCheckFamily: TypeFamily<Placeholder = Placeholder> {
    type TcBase: From<Self::Base>
        + Into<Self::Base>
        + Inferable<TyInternTables, KnownData = ty::BaseData<Self>>;

    fn new_infer_ty(this: &mut impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn equate_types(
        this: &mut impl TypeCheckerFields<Self>,
        cause: hir::MetaIndex,
        ty1: Ty<Self>,
        ty2: Ty<Self>,
    );

    fn boolean_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn error_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn require_assignable(
        this: &mut impl TypeCheckerFields<Self>,
        expression: hir::Expression,
        value_ty: Ty<Self>,
        place_ty: Ty<Self>,
    );

    fn apply_user_perm(
        this: &mut impl TypeCheckerFields<Self>,
        perm: hir::Perm,
        place_ty: Ty<Self>,
    ) -> Ty<Self>;

    fn least_upper_bound(
        this: &mut impl TypeCheckerFields<Self>,
        if_expression: hir::Expression,
        true_ty: Ty<Self>,
        false_ty: Ty<Self>,
    ) -> Ty<Self>;

    // FIXME -- This *almost* could be done generically but that
    // `Substitution` currently requires that `Perm = Erased`; we'll
    // have to push the "perm combination" into `TypeFamily` or
    // something.  Cross that bridge when we come to it.
    fn substitute<M>(
        this: &mut impl TypeCheckerFields<Self>,
        location: hir::MetaIndex,
        generics: &Generics<Self>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, Self>;

    fn apply_owner_perm<M>(
        this: &mut impl TypeCheckerFields<Self>,
        location: impl Into<hir::MetaIndex>,
        owner_perm: Self::Perm,
        value: M,
    ) -> M::Output
    where
        M: Map<Self, Self>;
}

trait TypeCheckerFields<F: TypeCheckFamily>: AsRef<TyInternTables> {
    type DB: TypeCheckDatabase;

    fn db(&self) -> &Self::DB;
    fn unify(&mut self) -> &mut UnificationTable<TyInternTables, hir::MetaIndex>;
    fn results(&mut self) -> &mut TypeCheckResults<F>;
}

impl<'me, DB, F> TypeCheckerFields<F> for TypeChecker<'me, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    type DB = DB;

    fn db(&self) -> &DB {
        &self.db
    }

    fn unify(&mut self) -> &mut UnificationTable<TyInternTables, hir::MetaIndex> {
        &mut self.unify
    }

    fn results(&mut self) -> &mut TypeCheckResults<F> {
        &mut self.results
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeCheckResults<F: TypeFamily> {
    /// FIXME-- this will actually not want `BaseTy` unless we want to
    /// return the unification table too.
    types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    errors: Vec<Error>,
}

impl<F: TypeFamily> TypeCheckResults<F> {
    fn record_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: Ty<F>) {
        self.types.insert(index.into(), ty);
    }

    pub fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.types[&index.into()]
    }

    fn record_error(&mut self, location: impl Into<hir::MetaIndex>) {
        self.errors.push(Error {
            location: location.into(),
        });
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            types: Default::default(),
            errors: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct Error {
    location: hir::MetaIndex,
}

impl<DB, F> AsRef<TyInternTables> for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    fn as_ref(&self) -> &TyInternTables {
        self.db.as_ref()
    }
}
