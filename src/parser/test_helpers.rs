#![allow(unused_variables)]
#![allow(unreachable_patterns)]

crate mod annotate_lines;

use crate::parser::ast::DebugModuleTable;
use crate::parser::lexer_helpers::{
    LexerAccumulate, LexerAction, LexerDelegateTrait, LexerNext, LexerToken, ParseError, Tokenizer,
};
use crate::parser::program::ModuleTable;
use crate::parser::program::StringId;

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
}

#[derive(Debug, Copy, Clone)]
pub enum Token {
    Underline,
    Name(StringId),
    Whitespace,
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
                    } else if c == ' ' {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::Whitespace)
                    } else {
                        LexerNext::Transition(LexerAccumulate::Begin, LexerState::Name)
                    }
                }
            },

            LexerState::Name => match c {
                None => LexerNext::Transition(
                    LexerAccumulate::Emit {
                        before: None,
                        after: None,
                        token: LexerToken::Dynamic(tk_name),
                    },
                    LexerState::Top,
                ),
                Some(' ') => LexerNext::Transition(
                    LexerAccumulate::Emit {
                        before: None,
                        after: None,
                        token: LexerToken::Dynamic(tk_name),
                    },
                    LexerState::Top,
                ),
                Some(c) if c.is_alphabetic() => {
                    LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1)))
                }
                Some('-') => LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1))),

                Some(other) => LexerNext::Error(other),
            },

            LexerState::Underline => match c {
                None => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
                Some(' ') | Some('~') => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
                Some('^') => LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1))),
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
                Some(' ') | Some('^') => LexerNext::Transition(
                    LexerAccumulate::emit_dynamic(tk_underline),
                    LexerState::Top,
                ),
                Some('~') => LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1))),
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
                        LexerNext::Transition(
                            LexerAccumulate::Skip(LexerAction::Reconsume),
                            LexerState::Top,
                        )
                    }
                }
            },
            other => unimplemented!("{:?}", other),
        };

        Ok(out)
    }
}
