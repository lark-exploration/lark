use crate::parser::pos::Spanned;
use crate::parser::program::{ModuleTable, StringId};
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Token {
    Underscore,
    CurlyBraceOpen,
    CurlyBraceClose,
    ParenOpen,
    ParenClose,
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
    KeywordBorrow,
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

impl Token {
    crate fn source(&self, table: &'table ModuleTable) -> &'table str {
        use self::Token::*;

        match self {
            Underscore => "_",
            CurlyBraceOpen => "{",
            CurlyBraceClose => "}",
            ParenOpen => "(",
            ParenClose => ")",
            Colon => ":",
            Semicolon => ";",
            Comma => ",",
            Equals => "=",
            ThinArrow => "->",
            DoubleColon => "::",
            Period => ".",
            KeywordFor => "for",
            KeywordLoop => "loop",
            KeywordWhile => "while",
            KeywordDef => "def",
            KeywordLet => "let",
            KeywordStruct => "struct",
            KeywordIf => "if",
            KeywordElse => "else",
            KeywordOwn => "own",
            KeywordBorrow => "borrow",
            KeywordSelf => "self",
            Identifier(id) => table.lookup(*id),
            Newline => "newline",
            Unimplemented => "unimplemented",
        }
    }
}
