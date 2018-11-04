#![feature(crate_visibility_modifier)]
#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(nll)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(bind_by_move_pattern_guards)]
#![feature(cell_update)]

#[macro_use]
pub mod lexer;

pub mod errors;
pub mod intern;
pub mod parser;
crate mod parser2;
pub mod pos;
pub mod prelude;
pub mod query;
pub mod reporting;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
crate use self::test_helpers::init_logger;

pub use self::errors::ParseError;
pub use self::intern::{LookupStringId, ModuleTable, Seahash, StringId};
pub use self::parser::ast;
pub use self::parser::parse;
pub use self::parser::token::Token;
pub use self::parser2::allow::*;
pub use self::parser2::entity_tree::Entities;
pub use self::parser2::macros::macros;
pub use self::parser2::quicklex::Tokenizer;
pub use self::parser2::reader::{PairedDelimiter, Reader};
pub use self::parser2::token_tree::{TokenPos, TokenSpan};
pub use self::parser2::LexToken;
pub use self::query::{
    add_file, initialize_reader, Files, HasParserState, InputText, ParserState, Paths,
    ReaderDatabase, Source, SourceFiles,
};
pub use self::reporting::print_parse_error;
