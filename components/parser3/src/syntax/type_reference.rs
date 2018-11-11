use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_string::global::GlobalIdentifier;

pub struct TypeReference;

/// Parsed form of a type.
pub enum ParsedTypeReference {
    Named(NamedTypeReference),
    Error,
}

/// Named type like `String` or (eventually) `Vec<u32>`
pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier>,
}

impl Syntax for TypeReference {
    type Data = ParsedTypeReference;

    fn parse(&self, parser: &mut Parser<'_>) -> Option<ParsedTypeReference> {
        let identifier = parser.eat(SpannedGlobalIdentifier)?;
        Some(ParsedTypeReference::Named(NamedTypeReference {
            identifier,
        }))
    }

    fn singular_name(&self) -> String {
        "type".to_string()
    }

    fn plural_name(&self) -> String {
        "types".to_string()
    }
}

impl<Cx> ErrorSentinel<Cx> for ParsedTypeReference {
    fn error_sentinel(_cx: Cx, _report: ErrorReported) -> Self {
        ParsedTypeReference::Error
    }
}
