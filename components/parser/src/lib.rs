#![feature(crate_visibility_modifier)]
#![feature(box_syntax)]
#![feature(box_patterns)]
#![feature(nll)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(bind_by_move_pattern_guards)]
#![feature(cell_update)]
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
mod lexer;

crate mod parser;
crate mod parser2;

pub mod prelude;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
crate use self::test_helpers::init_logger;

pub use self::parser::ast;
pub use self::parser::lexer_helpers::ParseError;
pub use self::parser::parse;
pub use self::parser::pos;
pub use self::parser::program::{LookupStringId, ModuleTable, Seahash, StringId};
pub use self::parser::token::Token;
pub use self::parser2::reader::{PairedDelimiter, Reader};
pub use self::parser2::token_tree::{TokenPos, TokenSpan};
pub use self::parser2::LexToken;
