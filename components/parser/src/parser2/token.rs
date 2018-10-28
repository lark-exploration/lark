use crate::prelude::*;

use crate::{ModuleTable, StringId};

use std::fmt;

token! {
    Whitespace: String,
    Identifier: String,
    Sigil: Sigil,
    Comment: String,
    String: String,
    Newline,
    EOF,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Sigil(pub StringId);

impl DebugModuleTable for Sigil {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "#{:?}#", Debuggable::from(&self.0, table))
    }
}

impl Sigil {
    pub fn classify(&self, table: &ModuleTable) -> ClassifiedSigil {
        let string = table.lookup(&self.0);

        match string {
            "{" => ClassifiedSigil::OpenCurly,
            "}" => ClassifiedSigil::CloseCurly,
            "(" => ClassifiedSigil::OpenRound,
            ")" => ClassifiedSigil::CloseRound,
            "[" => ClassifiedSigil::OpenSquare,
            "]" => ClassifiedSigil::CloseSquare,
            _ => ClassifiedSigil::Other(self.0),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ClassifiedSigil {
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
    OpenRound,
    CloseRound,
    Other(StringId),
}

impl Token {
    pub fn sigil(id: StringId) -> Token {
        Token::Sigil(Sigil(id))
    }

    pub fn data(&self) -> StringId {
        match *self {
            Token::Whitespace(s) => s,
            Token::Identifier(s) => s,
            Token::Sigil(Sigil(s)) => s,
            Token::Comment(s) => s,
            Token::String(s) => s,
            Token::Newline => panic!("Can't get data from newline (TODO?)"),
            Token::EOF => panic!("Can't get data from EOF (TODO?)"),
        }
    }

    pub fn is_id(&self) -> bool {
        match self {
            Token::Identifier(_) => true,
            _ => false,
        }
    }

    pub fn is_id_named(&self, name: StringId) -> bool {
        match self {
            Token::Identifier(id) if *id == name => true,
            _ => false,
        }
    }

    pub fn is_sigil(&self) -> bool {
        match self {
            Token::Sigil(..) => true,
            _ => false,
        }
    }

    pub fn is_sigil_named(&self, name: StringId) -> bool {
        match self {
            Token::Sigil(Sigil(id)) if *id == name => true,
            _ => false,
        }
    }

    pub fn is_whitespace(&self) -> bool {
        match self {
            Token::Newline | Token::Whitespace(..) => true,
            _ => false,
        }
    }
}

impl Spanned<Token> {
    pub fn as_id(self) -> Result<Spanned<StringId>, ParseError> {
        match self.0 {
            Token::Identifier(id) => Ok(Spanned::wrap_span(id, self.1)),
            other => Err(ParseError::new(
                format!("Unexpected token {:?}, expected id", other),
                self.1,
            )),
        }
    }

    pub fn expect_id(self) -> Result<Spanned<Token>, ParseError> {
        match self.0 {
            Token::Identifier(_) => Ok(self),
            other => Err(ParseError::new(
                format!("Unexpected token {:?}, expected id", other),
                self.1,
            )),
        }
    }
}

impl DebugModuleTable for Token {
    fn debug(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        table: &'table crate::parser::ModuleTable,
    ) -> std::fmt::Result {
        use self::Token::*;

        match self {
            Whitespace(_) => write!(f, "<whitespace>"),
            Identifier(s) => s.debug(f, table),
            Sigil(s) => write!(f, "{:?}", Debuggable::from(s, table)),
            Comment(_) => write!(f, "/* ... */"),
            String(s) => write!(f, "\"{:?}\"", Debuggable::from(s, table)),
            Newline => write!(f, "<newline>"),
            EOF => write!(f, "<EOF>"),
        }
    }
}

#[cfg(test)]
pub fn token_pos_at(line: usize, pos: usize, tokens: &[Spanned<Token>]) -> crate::TokenPos {
    let mut current_line = 1;
    let mut current_pos = 0;

    for (i, ann) in tokens.iter().enumerate() {
        if current_line == line && current_pos == pos {
            return crate::TokenPos(i);
        } else if let Token::Newline = ann.node() {
            current_line += 1;
            current_pos = 0;
        } else {
            current_pos += 1;
        }
    }

    panic!("Couldn't find a token at {}:{}", line, pos);
}
