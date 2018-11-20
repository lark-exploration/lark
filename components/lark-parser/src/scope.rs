use crate::ParserDatabase;
use lark_entity::Entity;
use lark_string::global::GlobalIdentifier;

crate fn resolve_name(
    _db: &impl ParserDatabase,
    _scope: Entity,
    _name: GlobalIdentifier,
) -> Option<Entity> {
    unimplemented!()
    //match scope.untern(db) {
    //    EntityData::InputFile { file } => {
    //        let parsed_file = db.parsed_file(FileName { id: file });
    //        parsed_file
    //            .value
    //            .entities()
    //            .iter()
    //            .map(|e| e.entity)
    //            .filter(|entity| match entity.untern(db) {
    //                EntityData::ItemName { id, .. } | EntityData::MemberName { id, .. } => {
    //                    id == name
    //                }
    //
    //                EntityData::LangItem(_)
    //                | EntityData::Error(_)
    //                | EntityData::InputFile { .. } => false,
    //            })
    //            .next()
    //            .or_else(|| {
    //                // Implicit root scope:
    //                let bool_id = "bool".intern(db);
    //                let int_id = "int".intern(db);
    //                let uint_id = "uint".intern(db);
    //                let string_id = "String".intern(db);
    //                if name == bool_id {
    //                    Some(EntityData::LangItem(LangItem::Boolean).intern(db))
    //                } else if name == int_id {
    //                    Some(EntityData::LangItem(LangItem::Int).intern(db))
    //                } else if name == uint_id {
    //                    Some(EntityData::LangItem(LangItem::Uint).intern(db))
    //                } else if name == string_id {
    //                    Some(EntityData::LangItem(LangItem::String).intern(db))
    //                } else {
    //                    None
    //                }
    //            })
    //    }
    //
    //    EntityData::ItemName { base, .. } => {
    //        // In principle, we could support nested items here, but whatevs.
    //        db.resolve_name(base, name)
    //    }
    //
    //    EntityData::MemberName { base, .. } => db.resolve_name(base, name),
    //
    //    EntityData::LangItem(_) => panic!("lang item is not a legal scope"),
    //
    //    EntityData::Error(_) => Some(scope),
    //}
}
