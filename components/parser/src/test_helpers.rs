#![allow(unused_variables)]
#![allow(unreachable_patterns)]

crate mod annotate_lines;

use crate::ast::DebugModuleTable;
use crate::lexer_helpers::{consume, consume_n, reconsume};
use crate::lexer_helpers::{
    LexerAccumulate, LexerAction, LexerDelegateTrait, LexerNext, LexerToken, ParseError, Tokenizer,
};
use crate::program::ModuleTable;
use crate::program::StringId;

use codespan::ByteSpan;
use derive_new::new;
use log::trace;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum LexerState {
    Top,
    Underline,
    SecondaryUnderline,
    Whitespace,
    Name,
    Sigil,
}

#[derive(Debug, Copy, Clone)]
pub enum Token {
    Underline,
    Sigil(StringId),
    Name(StringId),
    Whitespace,
    WsKeyword,
}

impl DebugModuleTable for Token {
    fn debug(&self, f: &mut fmt::Formatter<'_>, table: &'table ModuleTable) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type LineTokenizer<'source> = Tokenizer<'source, LexerState>;

fn tk_underline(_: StringId) -> Token {
    Token::Underline
}

fn tk_name(id: StringId) -> Token {
    Token::Name(id)
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
                    if c == '^' {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::Underline)
                    } else if c == '~' {
                        LexerNext::Transition(
                            LexerAccumulate::Begin,
                            LexerState::SecondaryUnderline,
                        )
                    } else if c == '#' {
                        consume().and_continue().and_transition(LexerState::Sigil)
                    } else if c == '@' {
                        consume().and_continue().and_transition(LexerState::Name)
                    } else if c == ' ' {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::Whitespace)
                    } else if rest.starts_with("ws") {
                        consume_n(2).and_emit(Token::WsKeyword).and_remain()
                    } else {
                        LexerNext::Error(Some(c))
                    }
                }
            },

            LexerState::Sigil => match c {
                None => LexerNext::Error(c),

                Some(c) => match c {
                    '#' => consume()
                        .and_emit_dynamic(Token::Sigil)
                        .and_transition(LexerState::Top),

                    _ => consume().and_remain(),
                },
            },

            LexerState::Name => match c {
                Some('@') => consume()
                    .and_emit_dynamic(Token::Name)
                    .and_transition(LexerState::Top),
                Some(_) => consume().and_remain(),
                None => LexerNext::Error(None),
            },

            LexerState::Underline => match c {
                None => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
                Some(' ') | Some('~') => reconsume()
                    .and_emit(Token::Underline)
                    .and_transition(LexerState::Top),
                Some('^') => consume().and_remain(),
                _ => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
            },

            LexerState::SecondaryUnderline => match c {
                None => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
                Some(' ') | Some('^') => reconsume()
                    .and_emit(Token::Underline)
                    .and_transition(LexerState::Top),
                Some('~') => consume().and_remain(),
                _ => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(c) => {
                    if c == ' ' {
                        LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1)))
                    } else {
                        reconsume()
                            .and_emit(Token::Whitespace)
                            .and_transition(LexerState::Top)
                    }
                }
            },
            other => unimplemented!("{:?}", other),
        };

        Ok(out)
    }
}
