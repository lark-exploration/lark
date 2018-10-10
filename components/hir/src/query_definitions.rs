use crate::HirDatabase;
use mir::DefId;
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;

crate fn boolean_def_id(_db: &impl HirDatabase, _key: ()) -> DefId {
    unimplemented!()
}

crate fn fn_body(_db: &impl HirDatabase, _key: DefId) -> Arc<crate::FnBody> {
    unimplemented!()
}

crate fn members(_db: &impl HirDatabase, _key: DefId) -> Arc<Vec<crate::Member>> {
    unimplemented!()
}

crate fn member_def_id(
    db: &impl HirDatabase,
    (owner, kind, name): (DefId, crate::MemberKind, StringId),
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

crate fn generic_declarations(_db: &impl HirDatabase, _key: DefId) -> Arc<ty::GenericDeclarations> {
    unimplemented!()
}
