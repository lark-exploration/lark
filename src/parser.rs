crate mod ast;
crate mod grammar;
crate mod grammar_helpers;
crate mod interned;
crate mod keywords;
crate mod lexer_helpers;
crate mod pos;
crate mod program;
crate mod token;
crate mod tokenizer;

crate use self::ast::Ast;
crate use self::grammar::ProgramParser;
crate use self::lexer_helpers::ParseError;
crate use self::pos::{Span, Spanned};
crate use self::program::{Environment, Module, NameId, Program, StringId};
crate use self::token::Token;
crate use self::tokenizer::Tokenizer;

use std::error::Error;

pub fn parse(
    source: impl Into<&'source str>,
    program: &'source mut Program,
) -> Result<ast::Module, Box<dyn Error>> {
    let tokenizer = Tokenizer::new(program, source.into(), 0);
    let parser = crate::parser::ProgramParser::new();
    let module = parser.parse(tokenizer);
    Ok(module?)
}

#[cfg(test)]
mod test {
    use super::parse;
    use codespan::ByteIndex;
    use crate::parser::ast::Debuggable;
    use crate::parser::pos::Span;
    use crate::parser::{self, ast, Ast};

    #[test]
    fn test_struct() -> Result<(), Box<dyn std::error::Error>> {
        let source = "struct Diagnostic {}";
        let mut program = parser::Program::new();

        let actual = parse(source, &mut program)?;
        let expected = ast::Module::build().add_struct(
            ast::Struct::build("Diagnostic", &mut program)
                .name_spanned(7, 17)
                .spanned(0, 20),
        );

        let debug_actual = Debuggable::from(&actual, &program);
        let debug_expected = Debuggable::from(&expected, &program);

        assert!(
            actual == expected,
            format!(
                "actual != expected\nactual: {:#?}\nexpected: {:#?}\n\nabbreviated:\n    actual: {:?}\n  expected: {:?}\n",
                actual, expected, debug_actual, debug_expected
            )
        );

        Ok(())
    }
}
