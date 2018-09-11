crate mod ast;
crate mod grammar;
crate mod grammar_helpers;
crate mod keywords;
crate mod lexer_helpers;
crate mod pos;
crate mod program;
crate mod token;
crate mod tokenizer;

#[cfg(test)]
pub mod test_helpers;

crate use self::ast::Ast;
crate use self::grammar::ProgramParser;
crate use self::lexer_helpers::ParseError;
crate use self::pos::{Span, Spanned};
crate use self::program::{Environment, Module, ModuleTable, NameId, StringId};
crate use self::token::Token;
crate use self::tokenizer::Tokenizer;

use self::keywords::KeywordList;

use codespan::ByteIndex;
use derive_new::new;
use std::borrow::{Borrow, Cow};
use std::error::Error;
use std::fmt;

pub fn parse(
    source: impl Into<Cow<'source, str>>,
    table: &'source mut ModuleTable,
    start: u32,
) -> Result<ast::Module, ParseError> {
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

#[cfg(test)]
mod test {
    use super::parse;
    use codespan::ByteIndex;
    use codespan::CodeMap;
    use crate::parser::ast::{Debuggable, Mode};
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::pos::Span;
    use crate::parser::program::ModuleTable;
    use crate::parser::test_helpers;
    use crate::parser::test_helpers::ModuleBuilder;
    use crate::parser::{self, ast, Ast};
    use language_reporting::{emit, Diagnostic, Label, Severity};
    use termcolor::{ColorChoice, StandardStream};
    use unindent::unindent;

    fn init() {
        pretty_env_logger::init();
    }

    fn parse_string(source: String) -> (ast::Module, ModuleTable, u32) {
        let mut codemap = CodeMap::new();

        let mut table = parser::ModuleTable::new();
        let filemap = codemap.add_filemap("test".into(), source.to_string());
        let start = filemap.span().start().0;

        let module = parse(&source[..], &mut table, start)
            .unwrap_or_else(|e| print_parse_error(e, &codemap));

        (module, table, start)
    }

    #[test]
    fn test_struct() -> Result<(), Box<dyn std::error::Error>> {
        init();

        let source = unindent(
            "
            struct Diagnostic {
              msg: own String,
              level: String,
            }

            def new(msg: own String, level: String) -> Diagnostic {
              Diagnostic { mgs, level }
            }
            ",
        );

        let (actual, mut table, start) = parse_string(source);

        let expected = ModuleBuilder::new(&mut table, start)
            .add_struct("Diagnostic", |b| {
                b.field("msg", Some(Mode::Owned), "String")
                    .field("level", None, "String")
            }).finish();

        eq(actual, expected, &table);

        Ok(())
    }

    fn eq(actual: ast::Module, expected: ast::Module, table: &ModuleTable) {
        let debug_actual = Debuggable::from(&actual, table);
        let debug_expected = Debuggable::from(&expected, table);

        assert!(
            actual == expected,
            format!(
                "actual != expected\nactual: {:#?}\nexpected: {:#?}\n\nabbreviated:\n    actual: {:?}\n  expected: {:?}\n",
                actual, expected, debug_actual, debug_expected
            )
        );
    }

    fn print_parse_error(e: ParseError, codemap: &CodeMap) -> ! {
        let error = Diagnostic::new(Severity::Error, e.description)
            .with_label(Label::new_primary(e.span.to_codespan()));
        let writer = StandardStream::stderr(ColorChoice::Auto);
        emit(
            &mut writer.lock(),
            &codemap,
            &error,
            &language_reporting::DefaultConfig,
        ).unwrap();
        panic!("Parse Error");
    }
}
