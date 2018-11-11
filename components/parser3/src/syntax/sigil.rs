use crate::lexer::token::LexToken;
use crate::parser::Parser;
use crate::span::Spanned;
use crate::syntax::Syntax;

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

                fn parse(&self, parser: &mut Parser<'_>) -> Option<Self::Data> {
                    if parser.is($kind) && parser.peek_str() == $name::TEXT {
                        Some(parser.shift())
                    } else {
                        None
                    }
                }

                fn singular_name(&self) -> String {
                    format!("`{}`", $name::TEXT)
                }

                fn plural_name(&self) -> String {
                    self.singular_name()
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
