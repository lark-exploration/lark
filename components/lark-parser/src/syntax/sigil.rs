use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::syntax::{Delimiter, NonEmptySyntax, Syntax};

use lark_debug_derive::DebugWith;
use lark_error::ErrorReported;
use lark_span::Spanned;

macro_rules! sigil_type {
    ($($v:vis struct $name:ident = ($kind:path, $token:expr);)*) => {
        $(
            #[derive(DebugWith)]
            $v struct $name;

            impl $name {
                $v const KIND: LexToken = $kind;
                $v const TEXT: &'static str = $token;
            }

            impl Syntax<'parse> for $name {
                type Data = Spanned<LexToken>;

                fn test(&self, parser: &Parser<'parse>) -> bool {
                    parser.is($kind) && parser.peek_str() == $name::TEXT
                }

                fn expect(&self, parser: &mut Parser<'parse>) -> Result<Self::Data, ErrorReported> {
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

            impl NonEmptySyntax<'parse> for $name { }
        )*
    }
}

sigil_type! {
    pub struct OpenCurly = (LexToken::Sigil, "{");
    pub struct CloseCurly = (LexToken::Sigil, "}");
    pub struct OpenParenthesis = (LexToken::Sigil, "(");
    pub struct CloseParenthesis = (LexToken::Sigil, ")");
    pub struct OpenSquare = (LexToken::Sigil, "[");
    pub struct CloseSquare = (LexToken::Sigil, "]");
    pub struct Colon = (LexToken::Sigil, ":");
    pub struct Comma = (LexToken::Sigil, ",");
    pub struct RightArrow = (LexToken::Sigil, "->");
}

#[derive(DebugWith)]
pub struct Curlies;

impl Delimiter<'parse> for Curlies {
    type Open = OpenCurly;
    type Close = CloseCurly;

    fn open_syntax(&self) -> Self::Open {
        OpenCurly
    }

    fn close_syntax(&self) -> Self::Close {
        CloseCurly
    }
}

#[derive(DebugWith)]
pub struct Parentheses;

impl Delimiter<'parse> for Parentheses {
    type Open = OpenParenthesis;
    type Close = CloseParenthesis;

    fn open_syntax(&self) -> Self::Open {
        OpenParenthesis
    }

    fn close_syntax(&self) -> Self::Close {
        CloseParenthesis
    }
}
