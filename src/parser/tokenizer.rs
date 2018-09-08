use codespan::ByteOffset;
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::lexer_helpers::LexerStateTrait;
use crate::parser::lexer_helpers::{LexerNext, ParseError, Tokenizer as GenericTokenizer};
use crate::parser::program::StringId;
use crate::parser::{Program, Span, Token};
use derive_new::new;
use lazy_static::lazy_static;
use log::trace;
use std::fmt;
use unicode_xid::UnicodeXID;

pub type Tokenizer<'program> = GenericTokenizer<'program, LexerState>;

#[derive(Debug)]
pub enum LexerState {
    Top,
    Integer,
    Whitespace,
    StartIdent,
    ContinueIdent,
}

impl LexerStateTrait for LexerState {
    type Token = Token;

    fn top() -> LexerState {
        LexerState::Top
    }

    fn next<'input>(
        &self,
        c: Option<char>,
        rest: &'input str,
    ) -> Result<LexerNext<Self>, ParseError> {
        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => {
                    if let Some((tok, size)) = KEYWORDS.match_keyword(rest) {
                        LexerNext::emit_token(tok, size)
                    } else if let Some((tok, size)) = SIGILS.match_keyword(rest) {
                        LexerNext::emit_token(tok, size)
                    } else if c.is_digit(10) {
                        LexerNext::transition_to(LexerState::Integer).reconsume()
                    } else if c == '\n' {
                        LexerNext::emit_char(Token::Newline)
                    } else if c.is_whitespace() {
                        LexerNext::transition_to(LexerState::Whitespace)
                    } else if UnicodeXID::is_xid_start(c) {
                        LexerNext::transition_to(LexerState::StartIdent).reconsume()
                    } else {
                        LexerNext::Error(c)
                    }
                }
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => {
                    if c == '\n' {
                        LexerNext::finalize_no_emit(LexerState::Top).reconsume()
                    } else if c.is_whitespace() {
                        LexerNext::consume()
                    } else {
                        LexerNext::finalize_no_emit(LexerState::Top).reconsume()
                    }
                }
            },

            LexerState::StartIdent => match c {
                None => LexerNext::emit(tk_id, LexerState::Top).reconsume(),
                Some(c) => {
                    if UnicodeXID::is_xid_continue(c) {
                        LexerNext::transition_to(LexerState::ContinueIdent)
                    } else {
                        LexerNext::emit(tk_id, LexerState::Top).reconsume()
                    }
                }
            },

            LexerState::ContinueIdent => match c {
                None => LexerNext::emit(tk_id, LexerState::Top).reconsume(),
                Some(c) => {
                    if UnicodeXID::is_xid_continue(c) {
                        LexerNext::consume()
                    } else {
                        LexerNext::emit(tk_id, LexerState::Top).reconsume()
                    }
                }
            },

            // LexerState::Integer => match c {
            //     None => LexerNext::emit_current(0, tk_int, LexerState::Top),
            //     Some(c) => {
            //         if c.is_digit(10) {
            //             LexerNext::consume()
            //         } else if c == '.' {
            //             LexerNext::transition_to(LexerState::Decimal)
            //         } else {
            //             LexerNext::emit(tk_int, LexerState::Top).reconsume()
            //         }
            //     }
            // },

            // LexerState::Decimal => match c {
            //     None => LexerNext::emit_current(0, tk_float, LexerState::Top),
            //     Some(c) => {
            //         if c.is_digit(10) {
            //             LexerNext::consume()
            //         } else {
            //             LexerNext::emit(tk_float, LexerState::Top).reconsume()
            //         }
            //     }
            // },
            other => unimplemented!("{:?}", other),
        };

        Ok(out)
    }
}

fn tk_id(token: StringId) -> Token {
    Token::Identifier(token)
}
