use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_string::global::GlobalIdentifier;

/// Represents a parse of something like `foo: Type`
pub struct Field {
    pub name: Spanned<GlobalIdentifier>,
    pub ty: TypeReference,
}

impl Syntax for Field {
    type Data = Self;

    fn parse(parser: &mut Parser<'_>) -> Option<Field> {
        let name = parser.eat_global_identifier()?;

        if let None = parser.eat_sigil(":") {
            parser.report_error("expected `:`", parser.peek_span());
        }

        let ty = parser.eat_required_syntax::<TypeReference>();
        return Some(Field { name, ty });
    }

    fn singular_name() -> String {
        "field".to_string()
    }

    fn plural_name() -> String {
        "fields".to_string()
    }
}
