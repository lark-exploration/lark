use crate::hir;
use crate::hir::HirQueries;
use crate::ir::DefId;
use crate::parser::StringId;
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
    crate Members(_db: &impl HirQueries, _key: DefId) -> Arc<Vec<hir::Member>> {
        unimplemented!()
    }
}

salsa::query_definition! {
    crate MemberDefId(db: &impl HirQueries, (owner, kind, name): (DefId, hir::MemberKind, StringId)) -> Option<DefId> {
        db.members().get(owner)
        .iter()
        .filter_map(|member| {
            if member.kind == kind && member.name == name {
                Some(member.def_id)
            } else {
                None
            }
        })
        .next()
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
