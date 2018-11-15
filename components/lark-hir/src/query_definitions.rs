use crate::HirDatabase;
use crate::Member;

use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, ItemKind, MemberKind};
use lark_error::{ErrorReported, ErrorSentinel};
use lark_parser::uhir;
use lark_seq::Seq;
use lark_span::FileName;
use lark_string::GlobalIdentifier;

crate fn members(db: &impl HirDatabase, owner: Entity) -> Result<Seq<Member>, ErrorReported> {
    let u = db.uhir_of_entity(owner);
    match &u.value {
        uhir::Entity::Struct(s) => Ok(s
            .fields
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
            .collect()),

        uhir::Entity::Def(_) => panic!("asked for members of a function"),
    }
}

crate fn member_entity(
    db: &impl HirDatabase,
    owner: Entity,
    kind: MemberKind,
    name: GlobalIdentifier,
) -> Option<Entity> {
    match db.members(owner) {
        Err(report) => Some(Entity::error_sentinel(db, report)),

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

crate fn subentities(db: &impl HirDatabase, root: Entity) -> Seq<Entity> {
    let mut entities = vec![root];

    // Go over each thing added to entities and add any nested
    // entities.
    let mut index = 0;
    while let Some(&entity) = entities.get(index) {
        index += 1;

        match entity.untern(db) {
            EntityData::ItemName {
                kind: ItemKind::Struct,
                ..
            } => match db.members(entity) {
                Ok(members) => entities.extend(members.iter().map(|member| member.entity)),
                Err(ErrorReported(_)) => {}
            },

            EntityData::InputFile { file } => {
                entities.extend(
                    db.parsed_file(FileName { id: file })
                        .value
                        .entities()
                        .iter()
                        .map(|e| e.entity)
                        .clone(),
                );
            }

            // No nested entities for these kinds of entities.
            EntityData::LangItem { .. }
            | EntityData::MemberName { .. }
            | EntityData::Error { .. }
            | EntityData::ItemName {
                kind: ItemKind::Function,
                ..
            } => {}
        }
    }

    Seq::from(entities)
}
