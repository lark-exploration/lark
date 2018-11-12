use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_string::global::GlobalIdentifier;

#[derive(DebugWith)]
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

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Result<ParsedTypeReference, ErrorReported> {
        let identifier = parser.expect(SpannedGlobalIdentifier)?;
        Ok(ParsedTypeReference::Named(NamedTypeReference {
            identifier,
        }))
    }
}

impl<Cx> ErrorSentinel<Cx> for ParsedTypeReference {
    fn error_sentinel(_cx: Cx, _report: ErrorReported) -> Self {
        ParsedTypeReference::Error
    }
}
