use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Syntax;
use intern::Intern;
use lark_string::global::GlobalIdentifier;

pub struct SpannedGlobalIdentifier;

impl Syntax for SpannedGlobalIdentifier {
    type Data = Spanned<GlobalIdentifier>;

    fn parse(&self, parser: &mut Parser<'_>) -> Option<Self::Data> {
        if parser.is(LexToken::Identifier) {
            let Spanned { span, value: _ } = parser.shift();
            Some(Spanned {
                value: parser.input()[span].intern(parser),
                span: span,
            })
        } else {
            None
        }
    }

    fn singular_name(&self) -> String {
        "identifier".to_string()
    }

    fn plural_name(&self) -> String {
        "identifiers".to_string()
    }
}
