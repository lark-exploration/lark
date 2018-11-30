use crate::ParserDatabase;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_string::GlobalIdentifier;

crate fn resolve_name(
    db: &impl ParserDatabase,
    scope: Entity,
    name: GlobalIdentifier,
) -> Option<Entity> {
    match scope.untern(db) {
        EntityData::InputFile { .. } => {
            db.child_entities(scope)
                .iter()
                .cloned()
                .filter(|entity| match entity.untern(db) {
                    EntityData::ItemName { id, .. } | EntityData::MemberName { id, .. } => {
                        id == name
                    }

                    EntityData::LangItem(_)
                    | EntityData::Error(_)
                    | EntityData::InputFile { .. } => false,
                })
                .next()
                .or_else(|| {
                    // Implicit root scope:
                    let bool_id = "bool".intern(db);
                    let int_id = "int".intern(db);
                    let uint_id = "uint".intern(db);
                    let false_id = "false".intern(db);
                    let true_id = "true".intern(db);
                    let debug_id = "debug".intern(db);
                    if name == bool_id {
                        Some(EntityData::LangItem(LangItem::Boolean).intern(db))
                    } else if name == int_id {
                        Some(EntityData::LangItem(LangItem::Int).intern(db))
                    } else if name == uint_id {
                        Some(EntityData::LangItem(LangItem::Uint).intern(db))
                    } else if name == false_id {
                        Some(EntityData::LangItem(LangItem::False).intern(db))
                    } else if name == true_id {
                        Some(EntityData::LangItem(LangItem::True).intern(db))
                    } else if name == debug_id {
                        Some(EntityData::LangItem(LangItem::Debug).intern(db))
                    } else {
                        None
                    }
                })
        }

        EntityData::ItemName { base, .. } => {
            // In principle, we could support nested items here, but whatevs.
            db.resolve_name(base, name)
        }

        EntityData::MemberName { base, .. } => db.resolve_name(base, name),

        EntityData::LangItem(_) => panic!("lang item is not a legal scope"),

        EntityData::Error(_) => Some(scope),
    }
}
