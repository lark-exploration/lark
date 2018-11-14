use crate::parser::Parser;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::Syntax;
use lark_debug_derive::DebugWith;
use lark_error::{ErrorReported, ErrorSentinel};
use lark_span::{CurrentFile, Span, Spanned};
use lark_string::GlobalIdentifier;

#[derive(DebugWith)]
pub struct TypeReference;

/// Parsed form of a type.
#[derive(Copy, Clone, DebugWith)]
pub enum ParsedTypeReference {
    Named(NamedTypeReference),
    Elided(Span<CurrentFile>),
    Error,
}

/// Named type like `String` or (eventually) `Vec<u32>`
#[derive(Copy, Clone, DebugWith)]
pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier>,
}

impl Syntax<'parse> for TypeReference {
    type Data = ParsedTypeReference;

    fn test(&self, parser: &Parser<'parse>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&self, parser: &mut Parser<'parse>) -> Result<ParsedTypeReference, ErrorReported> {
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
