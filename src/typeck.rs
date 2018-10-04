#![warn(warnings)]

use codespan_reporting::Diagnostic;
use crate::hir;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::ty::base_only::{BaseOnly, BaseTy};
use crate::ty::interners::{HasTyInternTables, TyInternTables};
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::sync::Arc;

mod hir_typeck;
mod infer;
mod ops;
mod query_definitions;

salsa::query_prototype! {
    crate trait TypeCheckQueries: hir::HirQueries + HasTyInternTables {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check() for query_definitions::BaseTypeCheck;
    }
}

crate struct BaseTypeChecker {
    hir: Arc<hir::FnBody>,
    interners: TyInternTables,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp>>,
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,
    errors: Vec<Error>,
    unify: UnificationTable<TyInternTables, hir::MetaIndex>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct BaseTypeCheckResults {
    /// FIXME-- this will actually not want `BaseTy` unless we want to
    /// return the unification table too.
    types: std::collections::BTreeMap<hir::MetaIndex, BaseTy>,
}

#[derive(Copy, Clone, Debug)]
crate struct Error {
    kind: ErrorKind,
    cause: hir::MetaIndex,
}

#[derive(Copy, Clone, Debug)]
crate enum ErrorKind {
    BaseMismatch(BaseTy, BaseTy),
}

impl HasTyInternTables for BaseTypeChecker {
    fn ty_intern_tables(&self) -> &TyInternTables {
        &self.interners
    }
}
