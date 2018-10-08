use crate::hir;
use crate::hir::HirDatabase;
use crate::ir::DefId;
use crate::parser::StringId;
use crate::ty;
use crate::ty::declaration::Declaration;
use std::sync::Arc;

crate fn boolean_def_id(_db: &impl HirDatabase, _key: ()) -> DefId {
    unimplemented!()
}

crate fn fn_body(_db: &impl HirDatabase, _key: DefId) -> Arc<hir::FnBody> {
    unimplemented!()
}

crate fn members(_db: &impl HirDatabase, _key: DefId) -> Arc<Vec<hir::Member>> {
    unimplemented!()
}

crate fn member_def_id(
    db: &impl HirDatabase,
    (owner, kind, name): (DefId, hir::MemberKind, StringId),
) -> Option<DefId> {
    db.members(owner)
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

crate fn ty(_db: &impl HirDatabase, _key: DefId) -> ty::Ty<Declaration> {
    unimplemented!()
}

crate fn signature(_db: &impl HirDatabase, _key: DefId) -> ty::Signature<Declaration> {
    unimplemented!()
}
