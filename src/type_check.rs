#![warn(warnings)]

use codespan_reporting::Diagnostic;
use crate::hir;
use crate::ir::DefId;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::ty::base_inferred::BaseInferred;
use crate::ty::base_only::{BaseOnly, BaseTy};
use crate::ty::interners::{HasTyInternTables, TyInternTables};
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::sync::Arc;

mod hir_typeck;
mod infer;
mod ops;
mod query_definitions;

salsa::query_group! {
    crate trait TypeCheckDatabase: hir::HirDatabase + HasTyInternTables {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check(key: DefId) -> TypeCheckResults<BaseInferred> {
            type BaseTypeCheckQuery;
            use fn query_definitions::base_type_check;
        }
    }
}

struct TypeChecker<'db, DB: TypeCheckDatabase, F: TypeFamily> {
    db: &'db DB,
    hir: Arc<hir::FnBody>,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp<Self>>>,
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,
    unify: UnificationTable<TyInternTables, hir::MetaIndex>,
    results: TypeCheckResults<F>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct TypeCheckResults<F: TypeFamily> {
    /// FIXME-- this will actually not want `BaseTy` unless we want to
    /// return the unification table too.
    types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    errors: Vec<Error>,
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

impl<DB, F> HasTyInternTables for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeFamily,
{
    fn ty_intern_tables(&self) -> &TyInternTables {
        self.db.ty_intern_tables()
    }
}
