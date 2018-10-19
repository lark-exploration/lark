use crate::ErrorReported;
use crate::HirDatabase;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use parser::StringId;

crate fn resolve_name(
    db: &impl HirDatabase,
    scope: Entity,
    name: StringId,
) -> Result<Option<Entity>, ErrorReported> {
    match scope.untern(db) {
        EntityData::InputFile { file } => {
            let items_in_file = db.items_in_file(file);
            Ok(items_in_file
                .iter()
                .filter(|entity| match entity.untern(db) {
                    EntityData::ItemName { id, .. } | EntityData::MemberName { id, .. } => {
                        id == name
                    }

                    EntityData::InputFile { .. } => false,
                })
                .cloned()
                .next())
        }

        EntityData::ItemName { base, .. } => {
            // In principle, we could support nested items here, but whatevs.
            db.resolve_name(base, name)
        }

        EntityData::MemberName { base, .. } => db.resolve_name(base, name),
    }
}
