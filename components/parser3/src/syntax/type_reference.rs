use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Syntax;
use lark_error::Diagnostic;
use lark_error::ErrorSentinel;
use lark_string::global::GlobalIdentifier;

/// Parsed form of a type.
pub enum TypeReference {
    Named(NamedTypeReference),
    Error,
}

/// Named type like `String` or (eventually) `Vec<u32>`
pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier>,
}

impl Syntax for TypeReference {
    type Data = Self;

    fn parse(parser: &mut Parser<'_>) -> Option<TypeReference> {
        let identifier = parser.eat_global_identifier()?;
        Some(TypeReference::Named(NamedTypeReference { identifier }))
    }

    fn singular_name() -> String {
        "type".to_string()
    }

    fn plural_name() -> String {
        "types".to_string()
    }
}

impl<Cx> ErrorSentinel<Cx> for TypeReference {
    fn error_sentinel(_cx: Cx, _error_spans: &[Diagnostic]) -> Self {
        TypeReference::Error
    }
}
