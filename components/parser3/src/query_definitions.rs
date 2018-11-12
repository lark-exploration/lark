use crate::macros;
use crate::parser::Parser;
use crate::syntax::entity::ParsedEntity;
use crate::FileName;
use crate::ParserDatabase;
use intern::Intern;
use lark_entity::EntityData;
use lark_error::WithError;
use std::sync::Arc;

crate fn parsed_entities(
    db: &impl ParserDatabase,
    file_name: FileName,
) -> WithError<Arc<Vec<ParsedEntity>>> {
    let entity_macro_definitions = &macros::default_entity_macros(db);
    let input = &db.file_text(file_name);
    let parser = Parser::new(db, entity_macro_definitions, input);
    let file_entity = EntityData::InputFile { file: file_name.id }.intern(db);
    parser.parse_all_entities(file_entity)
}
