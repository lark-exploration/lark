#![feature(const_fn)]
#![feature(const_let)]
#![feature(crate_visibility_modifier)]
#![feature(macro_at_most_once_rep)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::lexer::token::LexToken;
use crate::syntax::entity::ParsedEntity;

use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_entity::EntityTables;
use lark_error::Diagnostic;
use lark_error::WithError;
use lark_seq::Seq;
use lark_span::{CurrentFile, Span, Spanned};
use lark_string::global::GlobalIdentifier;
use lark_string::global::GlobalIdentifierTables;
use lark_string::text::Text;

pub mod current_file;
mod lexer;
mod macros;
mod parser;
mod query_definitions;
pub mod syntax;

salsa::query_group! {
    pub trait ParserDatabase: AsRef<GlobalIdentifierTables>
        + AsRef<EntityTables>
        + salsa::Database
    {
        fn file_names() -> Seq<FileName> {
            type FileNamesQuery;
            storage input;
        }

        fn file_text(id: FileName) -> Text {
            type FileTextQuery;
            storage input;
        }

        // FIXME: In general, this is wasteful of space, and not
        // esp. incremental friendly. It would be better store
        // e.g. the length of each token only, so that we can adjust
        // the previous value (not to mention perhaps using a rope or
        // some other similar data structure that permits insertions).
        fn file_tokens(id: FileName) -> WithError<Seq<Spanned<LexToken>>> {
            type FileTokensQuery;
            use fn query_definitions::file_tokens;
        }

        fn child_parsed_entities(entity: Entity) -> WithError<Seq<ParsedEntity>> {
            type ChildParsedEntitiesQuery;
            use fn query_definitions::child_parsed_entities;
        }

        fn parsed_entity(entity: Entity) -> ParsedEntity {
            type ParsedEntityQuery;
            use fn query_definitions::parsed_entity;
        }

        fn child_entities(entity: Entity) -> Seq<Entity> {
            type ChildEntitiesQuery;
            use fn query_definitions::child_entities;
        }
    }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FileName {
    pub id: GlobalIdentifier,
}

fn diagnostic(message: impl Into<String>, span: Span<CurrentFile>) -> Diagnostic {
    drop(span); // FIXME -- Diagostic uses the old codemap spans
    Diagnostic::new(message.into(), ::parser::pos::Span::Synthetic)
}
