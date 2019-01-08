use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::expression::scope::ExpressionScope;
use crate::syntax::Syntax;
use derive_new::new;
use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_hir as hir;
use lark_intern::Intern;

#[derive(new, DebugWith)]
crate struct Literal<'me, 'parse> {
    scope: &'me mut ExpressionScope<'parse>,
}

impl Syntax<'parse> for Literal<'me, 'parse> {
    type Data = hir::Expression;

    fn test(&mut self, parser: &Parser<'parse>) -> bool {
        parser.is(LexToken::Integer) || parser.is(LexToken::String)
    }

    fn expect(&mut self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
        let text = parser.peek_str();
        let token = parser.shift();
        let kind = match token.value {
            LexToken::Integer => hir::LiteralKind::UnsignedInteger,
            LexToken::String => hir::LiteralKind::String,
            _ => return Err(parser.report_error("expected a literal", token.span)),
        };
        let value = text.intern(parser);
        let data = hir::LiteralData { kind, value };
        Ok(self
            .scope
            .add(token.span, hir::ExpressionData::Literal { data }))
    }
}
