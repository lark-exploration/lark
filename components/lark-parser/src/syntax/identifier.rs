use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Syntax;
use intern::Intern;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_string::global::GlobalIdentifier;

#[derive(DebugWith)]
pub struct SpannedGlobalIdentifier;

impl Syntax for SpannedGlobalIdentifier {
    type Data = Spanned<GlobalIdentifier>;

    fn test(&self, parser: &Parser<'_>) -> bool {
        parser.is(LexToken::Identifier)
    }

    fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        if self.test(parser) {
            let Spanned { span, value: _ } = parser.shift();
            Ok(Spanned {
                value: parser.input()[span].intern(parser),
                span: span,
            })
        } else {
            Err(parser.report_error("expected an identifier", parser.peek_span()))
        }
    }
}
