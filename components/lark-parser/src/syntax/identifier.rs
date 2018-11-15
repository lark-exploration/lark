use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::{NonEmptySyntax, Syntax};

use intern::Intern;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_span::Spanned;
use lark_string::global::GlobalIdentifier;

#[derive(DebugWith)]
pub struct SpannedGlobalIdentifier;

impl Syntax<'parse> for SpannedGlobalIdentifier {
    type Data = Spanned<GlobalIdentifier>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        SpannedLocalIdentifier.test(parser)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let Spanned { span, value } = SpannedLocalIdentifier.expect(parser)?;
        Ok(Spanned {
            value: value.intern(parser),
            span: span,
        })
    }
}

impl NonEmptySyntax<'parse> for SpannedGlobalIdentifier {}

#[derive(DebugWith)]
pub struct SpannedLocalIdentifier;

impl Syntax<'parse> for SpannedLocalIdentifier {
    type Data = Spanned<&'parse str>;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.is(LexToken::Identifier)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        if self.test(parser) {
            let Spanned { span, .. } = parser.shift();
            Ok(Spanned {
                value: &parser.input()[span],
                span: span,
            })
        } else {
            Err(parser.report_error("expected an identifier", parser.peek_span()))
        }
    }
}

impl NonEmptySyntax<'parse> for SpannedLocalIdentifier {}
