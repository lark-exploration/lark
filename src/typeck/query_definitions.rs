use codespan_reporting::Diagnostic;
use crate::hir;
use crate::hir::HirQueries;
use crate::ir::DefId;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::ty::base_inferred::BaseInferred;
use crate::ty::base_only::{BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::{HasTyInternTables, TyInternTables};
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::typeck::BaseTypeCheckResults;
use crate::typeck::BaseTypeChecker;
use crate::typeck::TypeCheckQueries;
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::sync::Arc;

salsa::query_definition! {
    crate BaseTypeCheck(
        db: &impl TypeCheckQueries,
        key: DefId,
    ) -> BaseTypeCheckResults<BaseInferred> {
        let fn_body = db.fn_body().get(key);
        let base_type_checker = BaseTypeChecker {
            db,
            hir: fn_body,
            ops_arena: Arena::new(),
            ops_blocked: FxIndexMap::default(),
            unify: UnificationTable::new(db.ty_intern_tables().clone()),
            results: BaseTypeCheckResults::default(),
        };
        drop(base_type_checker); // FIXME
        unimplemented!()
    }
}
