use ast::ast as a;
use crate::HirDatabase;
use debug::DebugWith;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;

crate fn boolean_item_id(_db: &impl HirDatabase, _key: ()) -> Entity {
    unimplemented!()
}

crate fn members(db: &impl HirDatabase, item_id: Entity) -> Arc<Vec<crate::Member>> {
    match db.ast_of_item(item_id) {
        Ok(ast) => match &*ast {
            a::Item::Struct(_s) => unimplemented!(),

            a::Item::Def(_) => panic!("asked for fn-body of struct {:?}", item_id),
        },

        Err(_parse_error) => unimplemented!(),
    }
}

crate fn member_item_id(
    db: &impl HirDatabase,
    (owner, kind, name): (Entity, crate::MemberKind, StringId),
) -> Option<Entity> {
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

crate fn ty(db: &impl HirDatabase, entity: Entity) -> ty::Ty<Declaration> {
    match entity.untern(db) {
        EntityData::ItemName { .. } => unimplemented!(),

        EntityData::MemberName { .. } => unimplemented!(),

        EntityData::InputFile { .. } => panic!(
            "cannot get type of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

crate fn signature(_db: &impl HirDatabase, _key: Entity) -> ty::Signature<Declaration> {
    unimplemented!()
}

crate fn generic_declarations(
    _db: &impl HirDatabase,
    _key: Entity,
) -> Arc<ty::GenericDeclarations> {
    unimplemented!()
}
