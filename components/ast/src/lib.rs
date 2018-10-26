//! Encapsulates the parser and other base levels.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(macro_at_most_once_rep)]
#![feature(specialization)]

pub use crate::parser_state::ParserState;
use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_error::{ErrorReported, WithError};
pub use parser::ast;
use parser::StringId;
use std::sync::Arc;

mod parser_state;
mod query_definitions;
mod test;

salsa::query_group! {
    pub trait AstDatabase: HasParserState + AsRef<EntityTables> + salsa::Database {
        // These queries don't properly belong here -- probably in
        // parser -- but I want to minimize merge conflicts.

        fn input_files(key: ()) -> Arc<Vec<StringId>> {
            type InputFiles;
            storage input;
        }

        fn input_text(path: StringId) -> Option<StringId> {
            type InputText;
            storage input;
        }

        fn ast_of_file(path: StringId) -> WithError<Result<Arc<ast::Module>, ErrorReported>> {
            type AstOfFile;
            use fn query_definitions::ast_of_file;
        }

        fn items_in_file(path: StringId) -> Arc<Vec<Entity>> {
            type ItemsInFile;
            use fn query_definitions::items_in_file;
        }

        fn ast_of_item(item: Entity) -> Result<Arc<ast::Item>, ErrorReported> {
            type AstOfItem;
            use fn query_definitions::ast_of_item;
        }
    }
}

/// Trait encapsulating the String interner. This should be
/// synchronized with the `intern` crate eventually.
pub trait HasParserState: parser::LookupStringId {
    fn parser_state(&self) -> &ParserState;

    fn untern_string(&self, string_id: StringId) -> Arc<String> {
        self.parser_state().untern_string(string_id)
    }

    fn intern_string(&self, hashable: impl parser::Seahash) -> StringId {
        self.parser_state().intern_string(hashable)
    }
}
