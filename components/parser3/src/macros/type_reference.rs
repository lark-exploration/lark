use crate::span::Spanned;
use lark_string::global::GlobalIdentifier;

pub struct ParsedTypeReference {
    pub identifier: Spanned<GlobalIdentifier>,
}
