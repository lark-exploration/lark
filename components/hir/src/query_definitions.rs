use ast::ast as a;
use crate::HirDatabase;
use crate::Member;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_entity::MemberKind;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use parser::StringId;
use std::sync::Arc;

crate fn boolean_entity(db: &impl HirDatabase) -> Entity {
    EntityData::LangItem(LangItem::Boolean).intern(db)
}

crate fn members(db: &impl HirDatabase, owner: Entity) -> Result<Arc<Vec<Member>>, ErrorReported> {
    match &*db.ast_of_item(owner)? {
        a::Item::Struct(s) => Ok(Arc::new(
            s.fields
                .iter()
                .map(|f| {
                    let field_entity = EntityData::MemberName {
                        base: owner,
                        kind: MemberKind::Field,
                        id: *f.name,
                    }
                    .intern(db);

                    Member {
                        name: *f.name,
                        kind: MemberKind::Field,
                        entity: field_entity,
                    }
                })
                .collect(),
        )),

        a::Item::Def(_) => panic!("asked for members of a function"),
    }
}

crate fn member_entity(
    db: &impl HirDatabase,
    owner: Entity,
    kind: MemberKind,
    name: StringId,
) -> Option<Entity> {
    match &db.members(owner) {
        Err(ErrorReported(spans)) => Some(Entity::error_sentinel(db, spans)),

        Ok(members) => members
            .iter()
            .filter_map(|member| {
                if member.kind == kind && member.name == name {
                    Some(member.entity)
                } else {
                    None
                }
            })
            .next(),
    }
}
