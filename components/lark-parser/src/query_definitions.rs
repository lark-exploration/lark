use crate::macros;
use crate::parser::Parser;
use crate::syntax::entity::ParsedEntity;
use crate::FileName;
use crate::ParserDatabase;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_error::WithError;
use std::sync::Arc;

crate fn child_parsed_entities(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Arc<Vec<ParsedEntity>>> {
    log::debug!("child_parsed_entities({})", entity.debug_with(db));

    match entity.untern(db) {
        EntityData::InputFile { file } => {
            let file_name = FileName { id: file };
            let entity_macro_definitions = &macros::default_entity_macros(db);
            let input = &db.file_text(file_name);
            let parser = Parser::new(db, entity_macro_definitions, input);
            let file_entity = EntityData::InputFile { file: file_name.id }.intern(db);
            parser.parse_all_entities(file_entity)
        }

        EntityData::ItemName { .. } => db
            .parsed_entity(entity)
            .thunk
            .parse_children()
            .map(Arc::new),

        EntityData::Error { .. } | EntityData::MemberName { .. } | EntityData::LangItem(_) => {
            WithError::ok(Arc::new(vec![]))
        }
    }
}

crate fn parsed_entity(db: &impl ParserDatabase, entity: Entity) -> ParsedEntity {
    match entity.untern(db) {
        EntityData::ItemName { base, .. } => {
            let siblings = db.child_parsed_entities(base).into_value();
            siblings
                .iter()
                .find(|p| p.entity == entity)
                .unwrap_or_else(|| {
                    panic!(
                        "parsed_entity({}): entity not found amongst its siblings `{:?}`",
                        entity.debug_with(db),
                        siblings.debug_with(db),
                    )
                })
                .clone()
        }

        EntityData::Error { .. }
        | EntityData::InputFile { .. }
        | EntityData::MemberName { .. }
        | EntityData::LangItem(_) => {
            panic!(
                "cannot compute: `parsed_entity({:?})`",
                entity.debug_with(db),
            );
        }
    }
}

crate fn child_entities(db: &impl ParserDatabase, entity: Entity) -> Arc<Vec<Entity>> {
    Arc::new(
        db.child_parsed_entities(entity)
            .into_value()
            .iter()
            .map(|parsed_entity| parsed_entity.entity)
            .collect(),
    )
}
