use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::sigil::Colon;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_string::global::GlobalIdentifier;

pub struct Field;

/// Represents a parse of something like `foo: Type`
pub struct ParsedField {
    pub name: Spanned<GlobalIdentifier>,
    pub ty: ParsedTypeReference,
}

impl Syntax for Field {
    type Data = ParsedField;

    fn parse(&self, parser: &mut Parser<'_>) -> Option<ParsedField> {
        let name = parser.eat_global_identifier()?;

        parser.expect(Colon);

        let ty = parser.eat_required(TypeReference);
        return Some(ParsedField { name, ty });
    }

    fn singular_name(&self) -> String {
        "field".to_string()
    }

    fn plural_name(&self) -> String {
        "fields".to_string()
    }
}
