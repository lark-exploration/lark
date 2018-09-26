use codespan::ByteOffset;
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::lexer_helpers::LexerStateTrait;
use crate::parser::lexer_helpers::{
    LexerAction, LexerNext, ParseError, Tokenizer as GenericTokenizer,
};
use crate::parser::program::StringId;
use crate::parser::{ModuleTable, Span, Token};
use derive_new::new;
use lazy_static::lazy_static;
use log::{trace, warn};
use std::fmt;
use unicode_xid::UnicodeXID;

pub type Tokenizer<'table> = GenericTokenizer<'table, LexerState>;

#[derive(Debug, Copy, Clone)]
pub enum LexerState {
    Top,
    Integer,
    StringLiteral,
    AfterStringFragment,
    OpenCurly,
    InterpolateExpression,
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
                    if let Some((tok, size)) = KEYWORDS.match_token(rest) {
                        LexerNext::emit_token(tok, size)
                    } else if rest.starts_with("}}") {
                        LexerNext::PopState(LexerAction::Finalize(2))
                    } else if let Some((tok, size)) = SIGILS.match_token(rest) {
                        LexerNext::emit_token(tok, size)
                    } else if c.is_digit(10) {
                        LexerNext::transition_to(LexerState::Integer).reconsume()
                    } else if c == '"' {
                        LexerNext::transition_to(LexerState::StringLiteral)
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

            LexerState::StringLiteral => match c {
                None => LexerNext::EOF,

                Some(c) => match c {
                    '"' => LexerNext::emit(Token::StringLiteral, LexerState::Top),

                    other if rest.starts_with("{{") => {
                        LexerNext::PushState(LexerAction::Consume(0), LexerState::OpenCurly)
                            .reconsume()
                    }

                    other => LexerNext::consume(),
                },
            },

            LexerState::OpenCurly => {
                if let Some('"') = c {
                    LexerNext::emit(Token::EndString, LexerState::Top)
                } else if rest.starts_with("{{") {
                    LexerNext::Skip(
                        2,
                        box LexerNext::PushState(
                            LexerAction::EmitCurrent(0, Token::StringFragment),
                            LexerState::Top,
                        ),
                    )
                } else if rest.starts_with("}}") {
                    LexerNext::Transition(LexerAction::Finalize(2), LexerState::StringLiteral)
                } else {
                    warn!("unreachable rest={:?}", rest);
                    unreachable!()
                }
            }

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
