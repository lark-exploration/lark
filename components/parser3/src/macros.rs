use crate::parsed_entity::ParsedEntity;
use crate::parser::Parser;
use lark_error::WithError;

crate trait MacroDefinition {
    fn parse(&self, parser: &mut Parser<'me>) -> WithError<ParsedEntity>;
}
