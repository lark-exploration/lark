crate mod annotate_lines;

use crate::parser::lexer_helpers::{LexerNext, LexerStateTrait, ParseError, Tokenizer};
use crate::parser::program::StringId;

use codespan::ByteSpan;
use derive_new::new;
use log::trace;

#[derive(Debug, Clone)]
pub enum LexerState {
    Top,
    Underline,
    Whitespace,
    Name,
}

#[derive(Debug)]
pub enum Token {
    Underline,
    Name(StringId),
    Whitespace,
}

pub type LineTokenizer<'source> = Tokenizer<'source, LexerState>;

fn tk_underline(_: StringId) -> Token {
    Token::Underline
}

fn tk_name(id: StringId) -> Token {
    Token::Name(id)
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
                Some('^') => LexerNext::transition_to(LexerState::Underline).reconsume(),
                Some(' ') => LexerNext::transition_to(LexerState::Whitespace),
                _ => LexerNext::transition_to(LexerState::Name),
            },

            LexerState::Name => match c {
                None => LexerNext::emit(tk_name, LexerState::Top).reconsume(),
                Some(' ') => LexerNext::transition_to(LexerState::Top).reconsume(),
                Some(c) if c.is_alphabetic() => LexerNext::consume(),
                Some('-') | Some('_') => LexerNext::consume(),

                Some(other) => LexerNext::Error(other),
            },

            LexerState::Underline => match c {
                None => LexerNext::emit(tk_underline, LexerState::Top).reconsume(),
                Some(' ') => LexerNext::emit(tk_underline, LexerState::Top).reconsume(),
                Some('^') => LexerNext::consume(),
                _ => LexerNext::emit(tk_underline, LexerState::Top),
            },

            LexerState::Whitespace => match c {
                None => LexerNext::EOF,
                Some(' ') => LexerNext::consume(),
                _ => LexerNext::finalize_no_emit(LexerState::Top).reconsume(),
            },
            other => unimplemented!("{:?}", other),
        };

        Ok(out)
    }
}
