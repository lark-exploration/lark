use crate::HirDatabase;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, LangItem};
use lark_span::FileName;
use lark_string::global::GlobalIdentifier;

crate fn resolve_name(
    db: &impl HirDatabase,
    scope: Entity,
    name: GlobalIdentifier,
) -> Option<Entity> {
    match scope.untern(db) {
        EntityData::InputFile { file } => {
            let parsed_file = db.parsed_file(FileName { id: file });
            parsed_file
                .value
                .entities()
                .iter()
                .map(|e| e.entity)
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
                    if name == bool_id {
                        Some(EntityData::LangItem(LangItem::Boolean).intern(db))
                    } else if name == int_id {
                        Some(EntityData::LangItem(LangItem::Int).intern(db))
                    } else if name == uint_id {
                        Some(EntityData::LangItem(LangItem::Uint).intern(db))
                    } else if name == string_id {
                        Some(EntityData::LangItem(LangItem::String).intern(db))
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
