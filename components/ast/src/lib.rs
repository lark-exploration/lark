//! Encapsulates the parser and other base levels.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(macro_at_most_once_rep)]
#![feature(specialization)]

use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_error::{ErrorReported, WithError};
pub use parser::ast;
use parser::pos::Span;
use parser::{HasParserState, InputText, StringId};
use std::sync::Arc;

mod query_definitions;
mod test;

salsa::query_group! {
    pub trait AstDatabase: HasParserState + AsRef<EntityTables> + salsa::Database {
        // These queries don't properly belong here -- probably in
        // parser -- but I want to minimize merge conflicts.

        fn input_files(key: ()) -> Arc<Vec<StringId>> {
            type InputFilesQuery;
            storage input;
        }

        fn input_text(path: StringId) -> Option<InputText> {
            type InputTextQuery;
            storage input;
        }

        fn ast_of_file(path: StringId) -> WithError<Result<Arc<ast::Module>, ErrorReported>> {
            type AstOfFileQuery;
            use fn query_definitions::ast_of_file;
        }

        fn items_in_file(path: StringId) -> Arc<Vec<Entity>> {
            type ItemsInFileQuery;
            use fn query_definitions::items_in_file;
        }

        fn entity_span(entity: Entity) -> Option<Span> {
            type EntitySpanQuery;
            use fn query_definitions::entity_span;
        }

        fn ast_of_item(item: Entity) -> Result<Arc<ast::Item>, ErrorReported> {
            type AstOfItemQuery;
            use fn query_definitions::ast_of_item;
        }

        fn ast_of_field(item: Entity) -> Result<ast::Field, ErrorReported> {
            type AstOfFieldQuery;
            use fn query_definitions::ast_of_field;
        }
    }
}
