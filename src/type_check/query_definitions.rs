use codespan_reporting::Diagnostic;
use crate::hir;
use crate::hir::HirDatabase;
use crate::indices::IndexVec;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::ty::base_inferred::BaseInferred;
use crate::ty::base_only::{BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::{HasTyInternTables, TyInternTables};
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::type_check::TypeCheckDatabase;
use crate::type_check::TypeCheckResults;
use crate::type_check::TypeChecker;
use crate::type_check::UniverseBinder;
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use ir::DefId;
use std::sync::Arc;

crate fn base_type_check(
    db: &impl TypeCheckDatabase,
    fn_def_id: DefId,
) -> TypeCheckResults<BaseInferred> {
    let fn_body = db.fn_body(fn_def_id);
    let base_type_checker: TypeChecker<'_, _, BaseOnly> = TypeChecker {
        db,
        fn_def_id,
        hir: fn_body,
        ops_arena: Arena::new(),
        ops_blocked: FxIndexMap::default(),
        unify: UnificationTable::new(db.ty_intern_tables().clone()),
        results: TypeCheckResults::default(),
        universe_binders: IndexVec::from(vec![UniverseBinder::Root]),
    };
    drop(base_type_checker); // FIXME
    unimplemented!()
}
