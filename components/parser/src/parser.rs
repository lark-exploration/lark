#![allow(unused_variables)]
#![allow(unused_mut)]

use crate::prelude::*;

pub mod ast;

crate mod grammar;
crate mod keywords;
crate mod program;
pub mod reporting;
crate mod token;
crate mod tokenizer;

#[cfg(test)]
pub mod test_helpers;

crate use self::grammar::ProgramParser;
crate use self::token::Token;
crate use self::tokenizer::Tokenizer;

use crate::intern::ModuleTable;
use crate::lexer::KeywordList;

use codespan::ByteIndex;
use std::borrow::{Borrow, Cow};

pub fn parse(
    source: impl Into<Cow<'source, str>>,
    table: &'source ModuleTable,
    start: u32,
) -> Result<ast::Module, crate::errors::ParseError> {
    let cow = source.into();
    let tokenizer = Tokenizer::new(table, cow.borrow(), start);
    let parser = ProgramParser::new();
    let module = parser
        .parse(tokenizer)
        .map_err(|err| lalrpop_err(err, table));
    Ok(module?)
}

pub fn lalrpop_err(
    err: lalrpop_util::ParseError<ByteIndex, Token, ParseError>,
    table: &ModuleTable,
) -> ParseError {
    use lalrpop_util::ParseError::*;

    match err {
        InvalidToken { location } => ParseError::from_pos("Invalid Token", location),
        UnrecognizedToken {
            token: Some((left, token, right)),
            expected,
        } => ParseError::from(
            format!(
                "Unexpected token {}, expected: {}",
                token.source(table),
                KeywordList::new(expected)
            ),
            left,
            right,
        ),

        UnrecognizedToken {
            token: None,
            expected,
        } => ParseError::from_eof(format!(
            "Unrecognized EOF, expected: {}",
            KeywordList::new(expected)
        )),

        ExtraToken {
            token: (left, token, right),
        } => ParseError::from(format!("Extra Token {}", token.source(table)), left, right),

        User { error } => error,
    }
}
