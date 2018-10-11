//! Encapsulates the parser and other base levels.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(macro_at_most_once_rep)]

use crate::item_id::ItemId;
use crate::item_id::ItemIdTables;
use crate::parser_state::ParserState;
use intern::Has;
pub use parser::ast;
use parser::ParseError;
use parser::StringId;
use std::sync::Arc;

pub mod item_id;
mod parser_state;
mod query_definitions;

salsa::query_group! {
    pub trait AstDatabase: HasParserState + Has<ItemIdTables> + salsa::Database {
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

        fn ast_of_file(path: StringId) -> Result<Arc<ast::Module>, ParseError> {
            type AstOfFile;
            use fn query_definitions::ast_of_file;
        }

        fn items_in_file(path: StringId) -> Arc<Vec<ItemId>> {
            type ItemsInFile;
            use fn query_definitions::items_in_file;
        }

        fn ast_of_item(item: ItemId) -> Result<Arc<ast::Item>, ParseError> {
            type AstOfItem;
            use fn query_definitions::ast_of_item;
        }
    }
}

/// Trait encapsulating the String interner. This should be
/// synchronized with the `intern` crate eventually.
pub trait HasParserState {
    fn parser_state(&self) -> &ParserState;

    fn untern_string(&self, string_id: StringId) -> Arc<String> {
        self.parser_state().untern_string(string_id)
    }

    fn intern_string(&self, hashable: impl parser::program::Seahash) -> StringId {
        self.parser_state().intern_string(hashable)
    }
}
