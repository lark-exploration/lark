use crate::parser::ast::DebugModuleTable;
use crate::parser::program::{ModuleTable, StringId};

use std::borrow::Cow;
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
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
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
    StringLiteral(StringId),
    StringFragment(StringId),
    EndString(StringId),
    Newline,
    Unimplemented,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Token {
    crate fn source(&self, table: &'table ModuleTable) -> Cow<'table, str> {
        use self::Token::*;

        let result = match self {
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
            OpAdd => "+",
            OpSub => "-",
            OpMul => "*",
            OpDiv => "/",
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
            Identifier(id) => table.lookup(id),
            StringLiteral(id) => return Cow::Owned(format!("String({})", table.lookup(id))),
            StringFragment(id) => {
                return Cow::Owned(format!("StringFragment({})", table.lookup(id)))
            }
            EndString(id) => "closequote",
            Newline => "newline",
            Unimplemented => "unimplemented",
        };

        Cow::Borrowed(result)
    }
}

impl DebugModuleTable for Token {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "{:?}", self.source(table))
    }
}
