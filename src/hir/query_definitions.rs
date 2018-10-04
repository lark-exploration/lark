use crate::hir;
use crate::hir::HirQueries;
use crate::ir::DefId;
use crate::ty;
use crate::ty::declaration::Declaration;
use std::sync::Arc;

salsa::query_definition! {
    crate BooleanDefId(_query: &impl HirQueries, _key: ()) -> DefId {
        unimplemented!()
    }
}

salsa::query_definition! {
    crate FnBody(_query: &impl HirQueries, _key: DefId) -> Arc<hir::FnBody> {
        unimplemented!()
    }
}

salsa::query_definition! {
    crate Members(_query: &impl HirQueries, _key: DefId) -> Arc<Vec<hir::Member>> {
        unimplemented!()
    }
}

salsa::query_definition! {
    crate Ty(_query: &impl HirQueries, _key: DefId) -> ty::Ty<Declaration> {
        unimplemented!()
    }
}

salsa::query_definition! {
    crate Signature(_query: &impl HirQueries, _key: DefId) -> ty::Signature<Declaration> {
        unimplemented!()
    }
}
