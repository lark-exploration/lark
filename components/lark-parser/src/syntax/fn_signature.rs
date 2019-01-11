use crate::macros::function_declaration::ParsedFunctionDeclaration;
use crate::parser::Parser;
use crate::syntax::delimited::Delimited;
use crate::syntax::guard::Guard;
use crate::syntax::identifier::SpannedGlobalIdentifier;
use crate::syntax::list::CommaList;
use crate::syntax::matched::Matched;
use crate::syntax::member::Field;
use crate::syntax::sigil::{Curlies, Parentheses, RightArrow};
use crate::syntax::skip_newline::SkipNewline;
use crate::syntax::type_reference::ParsedTypeReference;
use crate::syntax::type_reference::TypeReference;
use crate::syntax::Syntax;
use lark_collections::Seq;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_error::ResultExt;

#[derive(DebugWith)]
pub struct FunctionSignature;

impl Syntax<'parse> for FunctionSignature {
    type Data = ParsedFunctionDeclaration;

    fn test(&mut self, parser: &Parser<'_>) -> bool {
        parser.test(SpannedGlobalIdentifier)
    }

    fn expect(&mut self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        let parameters = parser
            .expect(SkipNewline(Delimited(Parentheses, CommaList(Field))))
            .unwrap_or_else(|ErrorReported(_)| Seq::default());

        let return_type = match parser
            .parse_if_present(SkipNewline(Guard(RightArrow, SkipNewline(TypeReference))))
        {
            Some(ty) => ty.unwrap_or_error_sentinel(&*parser),
            None => ParsedTypeReference::Elided(parser.elided_span()),
        };

        let body = parser.expect(SkipNewline(Matched(Curlies)));

        Ok(ParsedFunctionDeclaration {
            parameters,
            return_type,
            body,
        })
    }
}
