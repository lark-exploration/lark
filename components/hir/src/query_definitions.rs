use ast::item_id::ItemId;
use crate::HirDatabase;
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;

crate fn boolean_item_id(_db: &impl HirDatabase, _key: ()) -> ItemId {
    unimplemented!()
}

crate fn fn_body(_db: &impl HirDatabase, _key: ItemId) -> Arc<crate::FnBody> {
    unimplemented!()
}

crate fn members(_db: &impl HirDatabase, _key: ItemId) -> Arc<Vec<crate::Member>> {
    unimplemented!()
}

crate fn member_item_id(
    db: &impl HirDatabase,
    (owner, kind, name): (ItemId, crate::MemberKind, StringId),
) -> Option<ItemId> {
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

crate fn ty(_db: &impl HirDatabase, _key: ItemId) -> ty::Ty<Declaration> {
    unimplemented!()
}

crate fn signature(_db: &impl HirDatabase, _key: ItemId) -> ty::Signature<Declaration> {
    unimplemented!()
}

crate fn generic_declarations(
    _db: &impl HirDatabase,
    _key: ItemId,
) -> Arc<ty::GenericDeclarations> {
    unimplemented!()
}
