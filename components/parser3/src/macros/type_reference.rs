use crate::span::Spanned;
use lark_string::global::GlobalIdentifier;

pub enum ParsedTypeReference {
    Named(NamedTypeReference),
    Error,
}

pub struct NamedTypeReference {
    pub identifier: Spanned<GlobalIdentifier>,
}
