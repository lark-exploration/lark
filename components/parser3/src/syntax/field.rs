use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::Syntax;
use lark_string::global::GlobalIdentifier;

/// Represents a parse of something like `foo: Type`
pub struct ParsedField {
    pub name: Spanned<GlobalIdentifier>,
    pub ty: ParsedTypeReference,
}

impl Syntax for ParsedField {
    type Data = Self;

    fn parse(parser: &mut Parser<'_>) -> Option<ParsedField> {
        let name = parser.eat_global_identifier()?;

        if let None = parser.eat_sigil(":") {
            parser.report_error("expected `:`", parser.peek_span());
        }

        let ty = parser.eat_required_syntax::<ParsedTypeReference>();
        return Some(ParsedField { name, ty });
    }

    fn singular_name() -> String {
        "field".to_string()
    }

    fn plural_name() -> String {
        "fields".to_string()
    }
}
