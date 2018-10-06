use codespan_reporting::Diagnostic;
use crate::hir;
use crate::hir::HirDatabase;
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
use crate::type_check::TypeCheckDatabase;
use crate::type_check::TypeCheckResults;
use crate::type_check::TypeChecker;
use crate::unify::InferVar;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::sync::Arc;

crate fn base_type_check(
    db: &impl TypeCheckDatabase,
    key: DefId,
) -> TypeCheckResults<BaseInferred> {
    let fn_body = db.fn_body(key);
    let base_type_checker = TypeChecker {
        db,
        hir: fn_body,
        ops_arena: Arena::new(),
        ops_blocked: FxIndexMap::default(),
        unify: UnificationTable::new(db.ty_intern_tables().clone()),
        results: TypeCheckResults::default(),
    };
    drop(base_type_checker); // FIXME
    unimplemented!()
}
