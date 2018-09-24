#![allow(unused_variables)]

use crate::parser::pos::Span;
use crate::parser::program::{ModuleTable, StringId};

use codespan::ByteIndex;
use derive_new::new;
use log::{debug, trace};
use std::fmt::{self, Debug};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum LexerNext<State: LexerStateTrait> {
    Skip(u32, Box<LexerNext<State>>),
    // PushState(State, Box<LexerNext<State>>),
    // PopState(Box<LexerNext<State>>),
    Transition(LexerAction<State>, State),
    Continue(LexerAction<State>),
    WholeToken(u32, State::Token),
    Error(char),
    EOF,
}

#[derive(Debug)]
pub enum LexerAction<State: LexerStateTrait> {
    Finalize(u32),
    EmitCurrent(u32, fn(StringId) -> State::Token),
    Consume(u32),
}

impl<LexerState: LexerStateTrait> LexerAction<LexerState> {
    fn reconsume(self) -> LexerAction<LexerState> {
        match self {
            LexerAction::Finalize(..) => LexerAction::Finalize(0),
            LexerAction::EmitCurrent(_, tk) => LexerAction::EmitCurrent(0, tk),
            LexerAction::Consume(..) => LexerAction::Consume(0),
        }
    }
}

impl<LexerState: LexerStateTrait> LexerNext<LexerState> {
    pub fn finalize_no_emit(next_state: LexerState) -> LexerNext<LexerState> {
        LexerNext::Transition(LexerAction::Finalize(1), next_state)
    }

    pub fn consume() -> LexerNext<LexerState> {
        LexerNext::Continue(LexerAction::Consume(1))
    }

    pub fn emit(
        tok: fn(StringId) -> LexerState::Token,
        next_state: LexerState,
    ) -> LexerNext<LexerState> {
        LexerNext::Transition(LexerAction::EmitCurrent(1, tok), next_state)
    }

    pub fn emit_and_skip(
        tok: fn(StringId) -> LexerState::Token,
        next_state: LexerState,
        skip: u32,
    ) -> LexerNext<LexerState> {
        LexerNext::Skip(
            skip,
            box LexerNext::Transition(LexerAction::EmitCurrent(0, tok), next_state),
        )
    }

    pub fn transition_to(next_state: LexerState) -> LexerNext<LexerState> {
        LexerNext::Transition(LexerAction::Consume(1), next_state)
    }

    pub fn reconsume(self) -> LexerNext<LexerState> {
        match self {
            LexerNext::WholeToken(_, tok) => LexerNext::WholeToken(0, tok),
            LexerNext::Transition(action, state) => {
                LexerNext::Transition(action.reconsume(), state)
            }
            LexerNext::Skip(..) => panic!("Skip and reconsume are not compatible"),
            LexerNext::Continue(action) => LexerNext::Continue(action.reconsume()),
            LexerNext::Error(c) => LexerNext::Error(c),
            LexerNext::EOF => LexerNext::EOF,
        }
    }

    pub fn skip(self, num: u32) -> LexerNext<LexerState> {
        LexerNext::Skip(num, Box::new(self))
    }
}

pub trait LexerStateTrait: fmt::Debug + Clone + Sized {
    type Token: fmt::Debug;

    fn next(&self, c: Option<char>, rest: &'input str) -> Result<LexerNext<Self>, ParseError>;

    fn top() -> Self;
}

#[derive(Debug, new)]
pub struct Tokenizer<'table, State: LexerStateTrait> {
    table: &'table mut ModuleTable,
    input: &'table str,
    codespan_start: u32,

    #[new(value = "input")]
    rest: &'table str,

    #[new(value = "input")]
    token_start: &'table str,

    #[new(default)]
    start_pos: u32,

    #[new(default)]
    token_size: u32,

    #[new(default)]
    pos: u32,

    #[new(value = "State::top()")]
    state: State,

    #[new(value = "vec![]")]
    stack: Vec<State>,

    #[new(default)]
    token: PhantomData<State::Token>,
}

pub type TokenizerItem<Token> = Result<(ByteIndex, Token, ByteIndex), ParseError>;

impl<State: LexerStateTrait + Debug> Iterator for Tokenizer<'table, State> {
    type Item = TokenizerItem<State::Token>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = {
                let Tokenizer { state, rest, .. } = self;
                let next = rest.chars().next();

                trace!("next char={:?} rest={:?}", next, rest);

                state.next(next, rest)
            };

            self.trace("start");

            let next = match next {
                Ok(n) => n,
                Err(e) => return Some(Err(e)),
            };

            match self.step(next) {
                LoopCompletion::Return(v) => return v,
                LoopCompletion::Continue => {}
            }
        }
    }
}

enum LoopCompletion<T> {
    Continue,
    Return(T),
}

impl<'a, State: LexerStateTrait> LexerNext<State> {
    pub fn emit_token(t: State::Token, size: u32) -> LexerNext<State> {
        LexerNext::WholeToken(size, t)
    }

    pub fn emit_char(t: State::Token) -> LexerNext<State> {
        LexerNext::WholeToken(1, t)
    }

    pub fn emit_current(
        size: u32,
        tok: fn(StringId) -> State::Token,
        next_state: State,
    ) -> LexerNext<State> {
        LexerNext::Transition(LexerAction::EmitCurrent(size, tok), next_state)
    }
}

impl<State: LexerStateTrait + Debug> Tokenizer<'table, State> {
    fn accumulate(&mut self, size: u32) {
        self.consume(size);
        self.token_size += size;
    }

    fn consume(&mut self, size: u32) {
        self.pos += size;
        self.rest = &self.rest[(size as usize)..];
    }

    fn consume_token(&mut self, size: u32) -> (u32, u32) {
        // get the starting position
        let start_pos = self.start_pos;

        self.consume(size);

        // and advance it to the current position
        self.start_pos = self.pos;

        // reset the token size
        self.token_size = 0;

        let ret = (start_pos, self.pos);

        ret
    }

    fn whole_token(&mut self, size: u32) -> (u32, u32) {
        let token = &self.token_start[..size as usize];
        let (start, end) = self.consume_token(size);
        self.token_start = self.rest;

        self.trace("whole");

        trace!(target: "lark::tokenize", "-> token={:?}", token.clone());

        (start, end)
    }

    fn discard_current(&mut self, size: u32) -> (u32, u32) {
        let (start_pos, end_pos) = self.consume_token(size);
        self.token_start = self.rest;

        self.trace("discard");
        (start_pos, end_pos)
    }

    fn finalize_current(&mut self, size: u32) -> (u32, StringId, u32) {
        let token = &self.token_start[..self.token_size as usize];
        let id = self.table.intern(token);
        self.token_start = self.rest;
        let (start_pos, end_pos) = self.consume_token(size);

        self.trace("finalize");
        trace!(target: "lark::tokenize", "-> token={:?}", token.clone());
        (start_pos, id, end_pos)
    }

    fn error(&mut self, c: char) -> ParseError {
        let file_start = self.codespan_start;
        let state = self.state.clone();
        let token = &self.token_start[..self.token_size as usize];
        let (start_pos, end_pos) = self.consume_token(1);

        let error = ParseError::new(
            format!("Unexpected char `{}` in state {:?}", c, state),
            Span::from_pos(file_start + start_pos, file_start + end_pos),
        );

        debug!("lark::tokenize::error {:?}", error);

        error
    }

    fn trace(&self, prefix: &str) {
        let start = self.codespan_start;

        trace!(target: "lark::tokenize", "{}", prefix);

        trace!(target: "lark::tokenize", "          input={:?}", self.input);

        trace!(target: "lark::tokenize", "          pos={:?} start_pos={:?}", start + self.pos, start + self.start_pos);

        trace!(
            target: "lark::tokenize",
            "          rest={:?} token-start={:?} token-size={:?} state={:?}",
            self.rest,
            self.token_start,
            self.token_size,
            self.state
        );
    }

    fn step(
        &mut self,
        next: LexerNext<State>,
    ) -> LoopCompletion<Option<TokenizerItem<State::Token>>> {
        match next {
            LexerNext::EOF => {
                self.trace("EOF");
                return LoopCompletion::Return(None);
            }

            LexerNext::Skip(size, next) => {
                let tok = self.step(*next);

                self.discard_current(size);

                tok
            }

            LexerNext::WholeToken(size, token) => {
                self.trace(&format!("whole {} {:?}", size, token));

                let file_start = self.codespan_start;

                let (start, end) = self.whole_token(size);

                let token = Some(Ok((
                    ByteIndex(start + file_start),
                    token,
                    ByteIndex(end + file_start),
                )));

                debug!(target: "lark::tokenize::some", "WholeToken: {:?}", token);

                LoopCompletion::Return(token)
            }

            LexerNext::Continue(action) => {
                let ret = self.handle_action(action);
                self.trace("continue");

                ret
            }

            LexerNext::Transition(action, state) => {
                let ret = self.handle_action(action);
                self.state = state;

                self.trace("transition");

                ret
            }

            LexerNext::Error(c) => LoopCompletion::Return(Some(Err(self.error(c)))),
        }
    }

    fn handle_action(
        &mut self,
        action: LexerAction<State>,
    ) -> LoopCompletion<Option<TokenizerItem<State::Token>>> {
        match action {
            LexerAction::Consume(size) => {
                self.accumulate(size);

                LoopCompletion::Continue
            }

            LexerAction::Finalize(size) => {
                let (l, r) = self.discard_current(size);
                let file_start = self.codespan_start;

                debug!(target: "lark::tokenize::noemit", "NoEmit @ {}..{}", l + file_start, r + file_start);

                LoopCompletion::Continue
            }

            LexerAction::EmitCurrent(size, tok) => {
                let file_start = self.codespan_start;
                self.token_size += size;
                let (start, id, end) = self.finalize_current(size);

                let token = Some(Ok((
                    ByteIndex(start + file_start),
                    tok(id),
                    ByteIndex(end + file_start),
                )));

                debug!(target: "lark::tokenize::some", "EmitCurrent: {:?}", token);

                LoopCompletion::Return(token)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ParseError {
    pub description: String,
    pub span: Span,
}

impl ParseError {
    pub fn from_pos(description: impl Into<String>, left: impl Into<ByteIndex>) -> ParseError {
        let pos = left.into();
        ParseError {
            description: description.into(),
            span: Span::from(pos, pos),
        }
    }

    pub fn from_eof(description: impl Into<String>) -> ParseError {
        ParseError {
            description: description.into(),
            span: Span::EOF,
        }
    }

    pub fn from(
        description: impl Into<String>,
        left: impl Into<ByteIndex>,
        right: impl Into<ByteIndex>,
    ) -> ParseError {
        ParseError {
            description: description.into(),
            span: Span::from(left.into(), right.into()),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {} at {:?}", self.description, self.span)
    }
}
