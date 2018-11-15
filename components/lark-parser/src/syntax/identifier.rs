use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::{NonEmptySyntax, Syntax};

use intern::Intern;
use lark_error::ErrorReported;
use lark_span::{FileName, Spanned, SpannedGlobalIdentifier};
use lark_string::global::GlobalIdentifier;

impl Syntax for SpannedGlobalIdentifier {
    type Data = Spanned<GlobalIdentifier, FileName>;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.is(LexToken::Identifier)
    }

    fn expect(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        if self.test(parser) {
            let Spanned { span, .. } = parser.shift();
            Ok(Spanned {
                value: parser.input()[span].intern(parser),
                span: span,
            })
        } else {
            Err(parser.report_error("expected an identifier", parser.peek_span()))
        }
    }
}

impl NonEmptySyntax for SpannedGlobalIdentifier {}
