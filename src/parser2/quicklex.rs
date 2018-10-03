use codespan::ByteOffset;
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::lexer_helpers::{begin, consume, consume_n, reconsume};
use crate::parser::lexer_helpers::{
    LexerAccumulate, LexerAction, LexerDelegateTrait, LexerNext, LexerToken, ParseError,
    Tokenizer as GenericTokenizer,
};
use crate::parser::program::StringId;
use crate::parser::{ModuleTable, Span};
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
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    Newline,
}

pub type Tokenizer<'table> = GenericTokenizer<'table, LexerState>;

#[derive(Debug, Copy, Clone)]
pub enum LexerState {
    Top,
    Whitespace,
    StartIdent,
    ContinueIdent,
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
                    '{' => LexerNext::sigil(Token::OpenCurly),
                    '}' => LexerNext::sigil(Token::CloseCurly),
                    '(' => LexerNext::sigil(Token::OpenParen),
                    ')' => LexerNext::sigil(Token::CloseParen),
                    '+' | '-' | '*' | '/' => LexerNext::dynamic_sigil(Token::Sigil),
                    '\n' => LexerNext::sigil(Token::Newline),
                    c if c.is_whitespace() => LexerNext::begin(Whitespace),
                    _ if rest.starts_with("/*") => consume_n(2).and_push(Comment(1)),
                    _ => LexerNext::Error(c),
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
                    _ => reconsume().and_discard().and_transition(LexerState::Top),
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

#[cfg(test)]
mod tests {}
