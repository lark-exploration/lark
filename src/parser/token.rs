use crate::parser::pos::Spanned;
use crate::parser::program::StringId;
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Token {
    Underscore,
    CurlyBraceOpen,
    CurlyBraceClose,
    Colon,
    Semicolon,
    Comma,
    Equals,
    ThinArrow,
    DoubleColon,
    Period,
    KeywordFor,
    KeywordLoop,
    KeywordWhile,
    KeywordDef,
    KeywordLet,
    KeywordStruct,
    KeywordIf,
    KeywordElse,
    KeywordOwn,
    KeywordSelf,
    Identifier(StringId),
    Newline,
    Unimplemented,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
