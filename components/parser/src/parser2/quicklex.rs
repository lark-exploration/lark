use codespan::ByteOffset;
use crate::parser::ast::{DebugModuleTable, Debuggable};
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::lexer_helpers::{begin, consume, consume_n, reconsume};
use crate::parser::lexer_helpers::{
    LexerAccumulate, LexerAction, LexerDelegateTrait, LexerNext, LexerToken, ParseError,
    Tokenizer as GenericTokenizer,
};
use crate::parser::program::StringId;
use crate::parser::{ModuleTable, Span, Spanned};
use derive_new::new;
use lazy_static::lazy_static;
use log::{trace, warn};
use std::fmt;
use unicode_xid::UnicodeXID;

token! {
    Whitespace: String,
    Identifier: String,
    Sigil: String,
    Comment: String,
    String: String,
    Newline,
    EOF,
}

impl Token {
    pub fn data(&self) -> StringId {
        match *self {
            Token::Whitespace(s) => s,
            Token::Identifier(s) => s,
            Token::Sigil(s) => s,
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

    pub fn is_sigil_named(&self, name: StringId) -> bool {
        match self {
            Token::Sigil(id) if *id == name => true,
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
            Sigil(s) => write!(f, "#{:?}#", Debuggable::from(s, table)),
            Comment(_) => write!(f, "/* ... */"),
            String(s) => write!(f, "\"{:?}\"", Debuggable::from(s, table)),
            Newline => write!(f, "<newline>"),
            EOF => write!(f, "<EOF>"),
        }
    }
}

pub type Tokenizer<'table> = GenericTokenizer<'table, LexerState>;

// impl Tokenizer<'table> {
//     fn tokens(self) -> Result<Vec<Spanned<Token>>, ParseError> {
//         self.map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
//             .collect()
//     }
// }

#[derive(Debug, Copy, Clone)]
pub enum LexerState {
    Top,
    Whitespace,
    StartIdent,
    ContinueIdent,
    StringLiteral,
    Sigil,
    Comment(u32),
}

impl LexerDelegateTrait for LexerState {
    type Token = Token;

    fn top() -> LexerState {
        LexerState::Top
    }

    fn next<'input>(
        &self,
        c: Option<char>,
        rest: &'input str,
    ) -> Result<LexerNext<Self>, ParseError> {
        use self::LexerState::*;

        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    c if UnicodeXID::is_xid_start(c) => LexerNext::begin(StartIdent),
                    c if is_sigil_char(c) => {
                        LexerNext::begin(Sigil)
                        // LexerNext::dynamic_sigil(Token::Sigil)
                    }
                    '"' => consume().and_transition(StringLiteral),
                    '\n' => LexerNext::sigil(Token::Newline),
                    c if c.is_whitespace() => LexerNext::begin(Whitespace),
                    _ if rest.starts_with("/*") => consume_n(2).and_push(Comment(1)),
                    c => LexerNext::Error(Some(c)),
                },
            },

            LexerState::Sigil => match c {
                None => reconsume()
                    .and_emit_dynamic(Token::Sigil)
                    .and_transition(LexerState::Top),
                Some(c) if is_sigil_char(c) => consume().and_remain(),
                _ => reconsume()
                    .and_emit_dynamic(Token::Sigil)
                    .and_transition(LexerState::Top),
            },

            LexerState::StringLiteral => match c {
                None => LexerNext::Error(c),
                Some(c) => match c {
                    '"' => consume()
                        .and_emit_dynamic(Token::String)
                        .and_transition(LexerState::Top),
                    _ => consume().and_remain(),
                },
            },

            LexerState::StartIdent => match c {
                None => LexerNext::emit_dynamic(Token::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => {
                        consume().and_transition(LexerState::ContinueIdent)
                    }

                    // TODO: Should this be a pop, so we don't have to reiterate
                    // the state name?
                    _ => reconsume()
                        .and_emit_dynamic(Token::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::ContinueIdent => match c {
                None => LexerNext::emit_dynamic(Token::Identifier, LexerState::Top),
                Some(c) => match c {
                    c if UnicodeXID::is_xid_continue(c) => consume().and_remain(),
                    _ => reconsume()
                        .and_emit_dynamic(Token::Identifier)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => match c {
                    '\n' => reconsume().and_discard().and_transition(LexerState::Top),
                    c if c.is_whitespace() => consume().and_remain(),
                    _ => reconsume()
                        .and_emit_dynamic(Token::Whitespace)
                        .and_transition(LexerState::Top),
                },
            },

            LexerState::Comment(1) => {
                if rest.starts_with("/*") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(2))
                } else if rest.starts_with("*/") {
                    consume_n(2)
                        .and_emit_dynamic(Token::Comment)
                        .and_transition(LexerState::Top)
                } else {
                    consume().and_remain()
                }
            }

            LexerState::Comment(n) => {
                if rest.starts_with("/*") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(n + 1))
                } else if rest.starts_with("*/") {
                    consume_n(2)
                        .and_continue()
                        .and_transition(LexerState::Comment(n - 1))
                } else {
                    consume().and_remain()
                }
            }
        };

        Ok(out)
    }
}

fn is_sigil_char(c: char) -> bool {
    match c {
        '{' | '}' | '(' | ')' | '+' | '-' | '*' | '/' | ':' | ',' | '>' | '<' | '=' => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, Tokenizer};
    use crate::parser::ast::DebuggableVec;
    use crate::parser::lexer_helpers::ParseError;
    use crate::parser::{Span, Spanned};
    use crate::parser2::test_helpers::{process, Annotations, Position};

    use log::trace;
    use unindent::unindent;

    #[test]
    fn test_quicklex() -> Result<(), ParseError> {
        crate::init_logger();

        let source = unindent(
            r##"
            struct Diagnostic {
            ^^^^^^~^^^^^^^^^^~^ @struct@ ws @Diagnostic@ ws #{#
              msg: String,
              ^^^~^~~~~~~^ @msg@ #:# ws @String@ #,#
              level: String,
              ^^^^^~^~~~~~~^ @level@ #:# ws @String@ #,#
            }
            ^ #}#
            "##,
        );

        let (source, mut ann) = process(&source);

        let filemap = ann.codemap().add_filemap("test".into(), source.clone());
        let start = filemap.span().start().0;

        let lexed = Tokenizer::new(ann.table(), &source, start);

        let tokens: Result<Vec<Spanned<Token>>, ParseError> = lexed
            .map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
            .collect();

        trace!("{:#?}", DebuggableVec::from(&tokens.clone()?, ann.table()));

        Ok(())
    }

}
