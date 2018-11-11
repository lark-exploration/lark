use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Syntax;
use lark_error::ErrorReported;

macro_rules! sigil_type {
    ($($v:vis struct $name:ident = ($kind:path, $token:expr);)*) => {
        $(
            $v struct $name;

            impl $name {
                $v const KIND: LexToken = $kind;
                $v const TEXT: &'static str = $token;
            }

            impl Syntax for $name {
                type Data = Spanned<LexToken>;

                fn test(&self, parser: &Parser<'_>) -> bool {
                    parser.is($kind) && parser.peek_str() == $name::TEXT
                }

                fn parse(&self, parser: &mut Parser<'_>) -> Result<Self::Data, ErrorReported> {
                    if self.test(parser) {
                        Ok(parser.shift())
                    } else {
                        Err(parser.report_error(
                            format!("expected `{}`", $name::TEXT),
                            parser.peek_span(),
                        ))
                    }
                }
            }
        )*
    }
}

sigil_type! {
    pub struct OpenCurly = (LexToken::Sigil, "{");
    pub struct CloseCurly = (LexToken::Sigil, "}");
    pub struct OpenParen = (LexToken::Sigil, "(");
    pub struct CloseParen = (LexToken::Sigil, ")");
    pub struct OpenSquare = (LexToken::Sigil, "[");
    pub struct CloseSquare = (LexToken::Sigil, "]");
    pub struct Colon = (LexToken::Sigil, ":");
    pub struct Comma = (LexToken::Sigil, ",");
}
