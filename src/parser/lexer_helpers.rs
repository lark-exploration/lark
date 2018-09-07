use codespan::ByteIndex;
use crate::parser::program::Program;
use crate::parser::program::StringId;
use derive_new::new;
use log::trace;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum LexerNext<State: LexerStateTrait> {
    WholeToken(u32, State::Token),
    FinalizeButDontEmitToken(u32, State),
    EmitCurrent(u32, fn(StringId) -> State::Token, State),
    Transition(u32, State),
    Continue(u32),
    Error(char),
    EOF,
}

impl<LexerState: LexerStateTrait> LexerNext<LexerState> {
    pub fn finalize_no_emit(next_state: LexerState) -> LexerNext<LexerState> {
        LexerNext::FinalizeButDontEmitToken(1, next_state)
    }

    pub fn consume() -> LexerNext<LexerState> {
        LexerNext::Continue(1)
    }

    pub fn emit(
        tok: fn(StringId) -> LexerState::Token,
        next_state: LexerState,
    ) -> LexerNext<LexerState> {
        LexerNext::EmitCurrent(1, tok, next_state)
    }

    pub fn transition_to(next_state: LexerState) -> LexerNext<LexerState> {
        LexerNext::Transition(1, next_state)
    }

    pub fn reconsume(self) -> LexerNext<LexerState> {
        match self {
            LexerNext::WholeToken(_, tok) => LexerNext::WholeToken(0, tok),
            LexerNext::FinalizeButDontEmitToken(_, tok) => {
                LexerNext::FinalizeButDontEmitToken(0, tok)
            }
            LexerNext::EmitCurrent(_, tok, state) => LexerNext::EmitCurrent(0, tok, state),
            LexerNext::Transition(_, state) => LexerNext::Transition(0, state),
            LexerNext::Continue(_) => LexerNext::Continue(1),
            LexerNext::Error(c) => LexerNext::Error(c),
            LexerNext::EOF => LexerNext::EOF,
        }
    }
}

pub trait LexerStateTrait: fmt::Debug + Sized {
    type Token: fmt::Debug;

    fn next(&self, c: Option<char>, rest: &'input str) -> Result<LexerNext<Self>, ParseError>;

    fn top() -> Self;
}

#[derive(Debug, new)]
pub struct Tokenizer<'program, State: LexerStateTrait> {
    program: &'program mut Program,
    input: &'program str,
    codespan_start: u32,

    #[new(value = "input")]
    rest: &'program str,

    #[new(value = "input")]
    token_start: &'program str,

    #[new(default)]
    start_pos: u32,

    #[new(default)]
    token_size: u32,

    #[new(default)]
    pos: u32,

    #[new(value = "State::top()")]
    state: State,

    #[new(default)]
    token: PhantomData<State::Token>,
}

impl<State: LexerStateTrait + Debug> Iterator for Tokenizer<'program, State> {
    type Item = Result<(ByteIndex, State::Token, ByteIndex), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = {
                let Tokenizer { state, rest, .. } = self;

                state.next(rest.chars().next(), rest)
            };

            self.trace("+");
            trace!(target: "lark::tokenize", "-> {:?}", next);

            let next = match next {
                Ok(n) => n,
                Err(e) => return Some(Err(e)),
            };

            match next {
                LexerNext::EOF => {
                    self.trace("-");
                    return None;
                }

                LexerNext::WholeToken(size, token) => {
                    let file_start = self.codespan_start;
                    let start = self.start_pos;
                    let end = self.pos + size;

                    self.consume(size);
                    self.token_start = self.rest;

                    self.trace("-");
                    trace!(target: "lark::tokenize", "-> token={:?}", token);
                    return Some(Ok((
                        ByteIndex(start + file_start),
                        token,
                        ByteIndex(end + file_start),
                    )));
                }

                LexerNext::EmitCurrent(size, tok, next_state) => {
                    let file_start = self.codespan_start;
                    let (start, id, end) = self.finalize_current(size, next_state);

                    return Some(Ok((
                        ByteIndex(start + file_start),
                        tok(id),
                        ByteIndex(end + file_start),
                    )));
                }

                LexerNext::FinalizeButDontEmitToken(size, next_state) => {
                    self.finalize_current(size, next_state);
                    // Parser doesn't handle WS tokens
                    // return Some((0, Tok::WS(token), 0));
                }

                LexerNext::Continue(size) => {
                    self.accumulate(size);

                    self.trace("-");
                }

                LexerNext::Transition(size, state) => {
                    self.accumulate(size);
                    self.state = state;

                    self.trace("-");
                }

                LexerNext::Error(c) => return Some(Err(self.error(c))),
            };
        }
    }
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
        LexerNext::EmitCurrent(size, tok, next_state)
    }
}

impl<State: LexerStateTrait + Debug> Tokenizer<'program, State> {
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
        // and advance it to the current position
        self.start_pos = self.pos;

        // reset the token size
        self.token_size = 0;

        let ret = (start_pos, self.pos);

        self.consume(size);

        ret
    }

    fn finalize_current(&mut self, size: u32, next_state: State) -> (u32, StringId, u32) {
        let token = &self.token_start[..self.token_size as usize];
        let id = self.program.intern(token);
        self.token_start = self.rest;
        self.state = next_state;
        let (start_pos, end_pos) = self.consume_token(size);

        self.trace("-");
        trace!(target: "lark::tokenize", "-> token=WS body={:?}", token.clone());
        (start_pos, id, end_pos)
    }

    fn error(&mut self, c: char) -> ParseError {
        let token = &self.token_start[..self.token_size as usize];
        let (start_pos, end_pos) = self.consume_token(1);

        ParseError(format!(
            "Unexpected char `{:?}` at {} in state {:?}",
            c, start_pos, self.state
        ))
    }

    fn trace(&self, prefix: &str) {
        trace!(target: "lark::tokenize", "input={:?}", self.input);

        trace!(
            target: "lark::tokenize",
            "{} rest={:?} token-start={:?} token-size={:?} state={:?}",
            prefix,
            self.rest,
            self.token_start,
            self.token_size,
            self.state
        );
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.0)
    }
}
