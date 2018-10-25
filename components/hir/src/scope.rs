use crate::HirDatabase;
use intern::{Intern, Untern};
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use parser::StringId;

crate fn resolve_name(db: &impl HirDatabase, scope: Entity, name: StringId) -> Option<Entity> {
    match scope.untern(db) {
        EntityData::InputFile { file } => {
            let items_in_file = db.items_in_file(file);
            items_in_file
                .iter()
                .filter(|entity| match entity.untern(db) {
                    EntityData::ItemName { id, .. } | EntityData::MemberName { id, .. } => {
                        id == name
                    }

                    EntityData::LangItem(_) | EntityData::Error | EntityData::InputFile { .. } => {
                        false
                    }
                })
                .cloned()
                .next()
                .or_else(|| {
                    // Implicit root scope:
                    let bool_id = db.intern_string("bool");
                    if name == bool_id {
                        Some(EntityData::LangItem(LangItem::Boolean).intern(db))
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

        EntityData::Error => Some(scope),
    }
}
