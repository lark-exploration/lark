use crate::AstDatabase;

use lark_entity::Entity;
use lark_span::{FileName, Span};

crate fn entity_span(db: &impl AstDatabase, entity: Entity) -> Span<FileName> {
    db.parsed_entity(entity).value.full_span.in_file_named(
        entity
            .input_file(db)
            .expect("Unexpected entity_span for LangItem or Error"),
    )
}
