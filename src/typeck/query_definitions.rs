use crate::hir;
use crate::hir::HirQueries;
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_inferred::BaseInferred;
use crate::ty::declaration::Declaration;
use crate::typeck::{BaseTypeCheckResults, TypeCheckQueries};
use std::sync::Arc;

salsa::query_definition! {
    crate BaseTypeCheck(
        _db: &impl TypeCheckQueries,
        _key: DefId,
    ) -> BaseTypeCheckResults<BaseInferred> {
        unimplemented!()
    }
}
