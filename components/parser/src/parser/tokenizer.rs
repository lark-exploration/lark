use crate::prelude::*;

use crate::lexer::tools::Tokenizer as GenericTokenizer;
use crate::lexer::tools::*;
use crate::parser::keywords::{KEYWORDS, SIGILS};
use crate::parser::Token;
use unicode_xid::UnicodeXID;

pub type Tokenizer<'table> = GenericTokenizer<'table, LexerState>;

#[derive(Debug, Copy, Clone)]
pub enum LexerState {
    Top,
    Integer,
    StartStringLiteral,
    StringLiteral,
    Whitespace,
    StartIdent,
    ContinueIdent,
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
        let out = match self {
            LexerState::Top => match c {
                None => LexerNext::EOF,
                Some(c) => {
                    if let Some((tok, size)) = KEYWORDS.match_token(rest) {
                        LexerNext::Remain(LexerAccumulate::Emit {
                            before: Some(LexerAction::Consume(size)),
                            after: None,
                            token: LexerToken::Fixed(tok),
                        })
                    } else if let Some((tok, size)) = SIGILS.match_token(rest) {
                        LexerNext::Remain(LexerAccumulate::Emit {
                            before: Some(LexerAction::Consume(size)),
                            after: None,
                            token: LexerToken::Fixed(tok),
                        })
                    } else if c.is_digit(10) {
                        LexerNext::Transition(LexerAccumulate::Nothing, LexerState::Integer)
                    } else if c == '"' {
                        LexerNext::Transition(
                            LexerAccumulate::Begin,
                            LexerState::StartStringLiteral,
                        )
                    } else if c.is_whitespace() {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::Whitespace)
                    } else if UnicodeXID::is_xid_start(c) {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::StartIdent)
                    } else {
                        LexerNext::Error(Some(c))
                    }
                }
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => {
                    if c == '\n' {
                        LexerNext::Transition(
                            LexerAccumulate::Skip(LexerAction::Reconsume),
                            LexerState::Top,
                        )
                    } else if c.is_whitespace() {
                        LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1)))
                    } else {
                        LexerNext::Transition(
                            LexerAccumulate::Skip(LexerAction::Reconsume),
                            LexerState::Top,
                        )
                    }
                }
            },

            LexerState::StringLiteral => match c {
                None => LexerNext::EOF,

                Some(c) => match c {
                    '"' => LexerNext::Transition(
                        LexerAccumulate::Emit {
                            before: Some(LexerAction::Consume(1)),
                            after: None,
                            token: LexerToken::Dynamic(Token::StringLiteral),
                        },
                        LexerState::Top,
                    ),

                    other => LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1))),
                },
            },

            LexerState::StartStringLiteral => LexerNext::Transition(
                LexerAccumulate::Continue(LexerAction::Consume(1)),
                LexerState::StringLiteral,
            ),

            LexerState::StartIdent => match c {
                None => {
                    LexerNext::Transition(LexerAccumulate::emit_dynamic(tk_id), LexerState::Top)
                }
                Some(c) => {
                    if UnicodeXID::is_xid_continue(c) {
                        LexerNext::Transition(
                            LexerAccumulate::Continue(LexerAction::Consume(1)),
                            LexerState::ContinueIdent,
                        )
                    } else {
                        LexerNext::Transition(LexerAccumulate::emit_dynamic(tk_id), LexerState::Top)
                    }
                }
            },

            LexerState::ContinueIdent => match c {
                None => {
                    LexerNext::Transition(LexerAccumulate::emit_dynamic(tk_id), LexerState::Top)
                }
                Some(c) => {
                    if UnicodeXID::is_xid_continue(c) {
                        LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1)))
                    } else {
                        LexerNext::Transition(LexerAccumulate::emit_dynamic(tk_id), LexerState::Top)
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
