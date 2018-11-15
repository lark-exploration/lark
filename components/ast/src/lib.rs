//! Encapsulates the parser and other base levels.

#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(macro_at_most_once_rep)]
#![feature(specialization)]

use lark_entity::{Entity, EntityTables};
use lark_parser::ParserDatabase;
use lark_span::{FileName, Span};

mod query_definitions;
mod test;

salsa::query_group! {
    pub trait AstDatabase: ParserDatabase + AsRef<EntityTables> + salsa::Database {
        // These queries don't properly belong here -- probably in
        // parser -- but I want to minimize merge conflicts.

        fn entity_span(entity: Entity) -> Span<FileName> {
            type EntitySpanQuery;
            use fn query_definitions::entity_span;
        }
    }
}
