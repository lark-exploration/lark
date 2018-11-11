use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::sigil::Colon;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_error::ErrorReported;
use lark_error::ResultExt;
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
        let name = parser.eat(SpannedGlobalIdentifier)?;

        let ty_result: Result<_, ErrorReported> = try {
            parser.expect(Colon)?;
            parser.expect(TypeReference)?
        };

        let ty = ty_result.unwrap_or_error_sentinel(&*parser);

        return Some(ParsedField { name, ty });
    }

    fn singular_name(&self) -> String {
        "field".to_string()
    }

    fn plural_name(&self) -> String {
        "fields".to_string()
    }
}
