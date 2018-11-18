use crate::HirDatabase;
use intern::{Intern, Untern};
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_string::global::GlobalIdentifier;

crate fn resolve_name(
    db: &impl HirDatabase,
    scope: Entity,
    name: GlobalIdentifier,
) -> Option<Entity> {
    match scope.untern(db) {
        EntityData::InputFile { file } => {
            let items_in_file = db.items_in_file(file);
            items_in_file
                .iter()
                .filter(|entity| match entity.untern(db) {
                    EntityData::ItemName { id, .. } | EntityData::MemberName { id, .. } => {
                        id == name
                    }

                    EntityData::LangItem(_)
                    | EntityData::Error(_)
                    | EntityData::InputFile { .. } => false,
                })
                .cloned()
                .next()
                .or_else(|| {
                    // Implicit root scope:
                    let bool_id = db.intern_string("bool");
                    let int_id = db.intern_string("int");
                    let uint_id = db.intern_string("uint");
                    let string_id = db.intern_string("String");
                    let false_id = db.intern_string("false");
                    let true_id = db.intern_string("true");
                    let debug_id = db.intern_string("debug");
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
