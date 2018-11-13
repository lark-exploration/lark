use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::NonEmptySyntax;
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

    fn expect(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
        let Spanned { span, value } = parser.shift();

        match value {
            LexToken::Identifier => Ok(Spanned {
                value: parser.input()[span].intern(parser),
                span: span,
            }),

            _ => Err(parser.report_error("expected an identifier", span)),
        }
    }
}

impl NonEmptySyntax for SpannedGlobalIdentifier {}
