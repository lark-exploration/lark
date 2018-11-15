use crate::parser::Parser;
use crate::syntax::Syntax;

use lark_debug_derive::DebugWith;
use lark_error::{ErrorReported, ErrorSentinel};
use lark_span::{FileName, Span, Spanned, SpannedGlobalIdentifier};
use lark_string::GlobalIdentifier;

#[derive(DebugWith)]
pub struct TypeReference;

/// Parsed form of a type.
#[derive(Copy, Clone, DebugWith)]
pub enum ParsedTypeReference {
    Named(NamedTypeReference),
    Elided(Span<FileName>),
    Error,
}

/// Named type like `String` or (eventually) `Vec<u32>`
#[derive(Copy, Clone, DebugWith)]
pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier, FileName>,
}

impl Syntax for TypeReference {
    type Data = ParsedTypeReference;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&self, parser: &mut Parser<'_>) -> Result<ParsedTypeReference, ErrorReported> {
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
