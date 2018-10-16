#![allow(unused_variables)]

use crate::parser::ast::{DebugModuleTable, Debuggable};
use crate::parser::pos::{Span, Spanned};
use crate::parser::program::{ModuleTable, StringId};

use codespan::ByteIndex;
use derive_new::new;
use log::{debug, trace, warn};
use std::fmt::{self, Debug};
use std::marker::PhantomData;

/// This enum describes which state to transition into next.
/// The LexerEmit nested inside of the LexerNext describes
/// what happens before the transition
#[derive(Debug)]
pub enum LexerNext<Delegate: LexerDelegateTrait> {
    EOF,
    Remain(LexerAccumulate<Delegate>),
    Transition(LexerAccumulate<Delegate>, Delegate),
    PushState(LexerAccumulate<Delegate>, Delegate),
    PopState(LexerAccumulate<Delegate>),
    Error(Option<char>),
}

trait EmitToken<Delegate: LexerDelegateTrait> {
    fn token_for(self, s: StringId) -> Delegate::Token;
}

impl<Delegate: LexerDelegateTrait> LexerNext<Delegate> {
    pub fn begin(state: Delegate) -> Self {
        LexerNext::Transition(LexerAccumulate::Begin, state)
    }

    /// A sigil is a single character that represents a token
    pub fn sigil(token: Delegate::Token) -> Self {
        LexerNext::Remain(LexerAccumulate::Emit {
            before: Some(LexerAction::Consume(1)),
            after: None,
            token: LexerToken::Fixed(token),
        })
    }

    pub fn dynamic_sigil(token: fn(StringId) -> Delegate::Token) -> Self {
        LexerNext::Remain(LexerAccumulate::Emit {
            before: Some(LexerAction::Consume(1)),
            after: None,
            token: LexerToken::Dynamic(token),
        })
    }

    pub fn emit(token: Delegate::Token, state: Delegate) -> Self {
        LexerNext::Transition(
            LexerAccumulate::Emit {
                before: None,
                after: None,
                token: LexerToken::Fixed(token),
            },
            state,
        )
    }

    pub fn emit_dynamic(token: fn(StringId) -> Delegate::Token, state: Delegate) -> Self {
        LexerNext::Transition(
            LexerAccumulate::Emit {
                before: None,
                after: None,
                token: LexerToken::Dynamic(token),
            },
            state,
        )
    }

    pub fn discard(state: Delegate) -> Self {
        LexerNext::Transition(LexerAccumulate::Skip(LexerAction::Reconsume), state)
    }

    pub fn consume() -> Self {
        LexerNext::Remain(LexerAccumulate::Continue(LexerAction::Consume(1)))
    }

    pub fn transition(state: Delegate) -> Self {
        LexerNext::Transition(LexerAccumulate::Continue(LexerAction::Consume(1)), state)
    }
}

/// This enum describes whether to emit the current token.
#[derive(Debug)]
pub enum LexerAccumulate<Delegate: LexerDelegateTrait> {
    /// Start a new token. There should be no accumulated characters
    /// yet.
    Begin,

    /// Don't accumulate any characters or emit anything
    Nothing,

    /// Don't emit anything, but continue to accumulate characters
    /// in the current token
    Continue(LexerAction),

    /// Start a new token after possibly consuming some characters.
    /// Those characters are ignored, and are not part of any token.
    Skip(LexerAction),

    /// Emit a token after accumulating some characters into it.
    /// Possibly skip some characters afterward.
    Emit {
        before: Option<LexerAction>,
        after: Option<LexerAction>,
        token: LexerToken<Delegate::Token>,
    },
}

impl<Delegate: LexerDelegateTrait> From<LexerAccumulate<Delegate>> for LexerNext<Delegate> {
    fn from(accum: LexerAccumulate<Delegate>) -> LexerNext<Delegate> {
        LexerNext::Remain(accum)
    }
}

impl<Delegate: LexerDelegateTrait> LexerAccumulate<Delegate> {
    pub fn and_remain(self) -> LexerNext<Delegate> {
        LexerNext::Remain(self)
    }

    pub fn and_transition(self, state: Delegate) -> LexerNext<Delegate> {
        LexerNext::Transition(self, state)
    }

    pub fn and_push(self, state: Delegate) -> LexerNext<Delegate> {
        LexerNext::PushState(self, state)
    }

    pub fn and_pop(self) -> LexerNext<Delegate> {
        LexerNext::PopState(self)
    }
}

impl<Delegate: LexerDelegateTrait> From<LexerAction> for LexerAccumulate<Delegate> {
    fn from(action: LexerAction) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Continue(action)
    }
}

impl<Delegate: LexerDelegateTrait> From<LexerAction> for LexerNext<Delegate> {
    fn from(action: LexerAction) -> LexerNext<Delegate> {
        LexerNext::Remain(LexerAccumulate::Continue(action))
    }
}

impl<Delegate: LexerDelegateTrait> LexerAccumulate<Delegate> {
    pub fn emit_dynamic(token: fn(StringId) -> Delegate::Token) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Emit {
            before: None,
            after: None,
            token: LexerToken::Dynamic(token),
        }
    }

    pub fn emit(token: Delegate::Token) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Emit {
            before: None,
            after: None,
            token: LexerToken::Fixed(token),
        }
    }

    pub fn before(self, action: LexerAction) -> LexerAccumulate<Delegate> {
        match self {
            LexerAccumulate::Emit {
                before: None,
                after,
                token,
            } => LexerAccumulate::Emit {
                before: Some(action),
                after,
                token,
            },

            other => panic!("Can't add a before action to {:?}", other),
        }
    }

    pub fn and_then(self, action: LexerAction) -> LexerAccumulate<Delegate> {
        match self {
            LexerAccumulate::Emit {
                before,
                after: None,
                token,
            } => LexerAccumulate::Emit {
                before,
                after: Some(action),
                token,
            },

            other => panic!("Can't add an after action to {:?}", other),
        }
    }
}

#[derive(Debug)]
pub enum LexerToken<Token: Copy> {
    Dynamic(fn(StringId) -> Token),
    Fixed(Token),
}

impl<Token: Copy> LexerToken<Token> {
    fn string(&self, id: StringId) -> Token {
        match self {
            LexerToken::Dynamic(f) => f(id),
            LexerToken::Fixed(tok) => *tok,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LexerAction {
    Consume(u32),
    Reconsume,
}

impl LexerAction {
    pub fn and_remain<Delegate: LexerDelegateTrait>(self) -> LexerNext<Delegate> {
        LexerNext::Remain(LexerAccumulate::Continue(self))
    }

    pub fn and_transition<Delegate: LexerDelegateTrait>(
        self,
        state: Delegate,
    ) -> LexerNext<Delegate> {
        LexerNext::Transition(LexerAccumulate::Continue(self), state)
    }

    pub fn and_push<Delegate: LexerDelegateTrait>(self, state: Delegate) -> LexerNext<Delegate> {
        LexerNext::PushState(LexerAccumulate::Continue(self), state)
    }

    pub fn and_pop<Delegate: LexerDelegateTrait>(self) -> LexerNext<Delegate> {
        LexerNext::PopState(LexerAccumulate::Continue(self))
    }

    pub fn and_continue<Delegate: LexerDelegateTrait>(self) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Continue(self)
    }

    pub fn and_discard<Delegate: LexerDelegateTrait>(self) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Skip(self)
    }

    pub fn and_emit<Delegate: LexerDelegateTrait>(
        self,
        token: Delegate::Token,
    ) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Emit {
            before: Some(self),
            after: None,
            token: LexerToken::Fixed(token),
        }
    }

    pub fn and_emit_dynamic<Delegate: LexerDelegateTrait>(
        self,
        token: fn(StringId) -> Delegate::Token,
    ) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Emit {
            before: Some(self),
            after: None,
            token: LexerToken::Dynamic(token),
        }
    }
}

pub fn reconsume() -> LexerAction {
    LexerAction::Reconsume
}

pub fn consume() -> LexerAction {
    LexerAction::Consume(1)
}

pub fn consume_n(size: u32) -> LexerAction {
    LexerAction::Consume(size)
}

pub fn begin<Delegate: LexerDelegateTrait>() -> LexerAccumulate<Delegate> {
    LexerAccumulate::Begin
}

pub fn eof<Delegate: LexerDelegateTrait>() -> LexerNext<Delegate> {
    LexerNext::EOF
}

pub trait LexerDelegateTrait: fmt::Debug + Clone + Copy + Sized {
    type Token: fmt::Debug + Copy + DebugModuleTable;

    fn next(&self, c: Option<char>, rest: &'input str) -> Result<LexerNext<Self>, ParseError>;

    fn top() -> Self;
}

#[derive(Debug, new)]
pub struct Tokenizer<'table, Delegate: LexerDelegateTrait> {
    table: &'table mut ModuleTable,
    input: &'table str,
    codespan_start: u32,

    /// The rest of the input
    #[new(value = "input")]
    rest: &'table str,

    /// The beginning of the current token
    #[new(value = "input")]
    token_start: &'table str,

    /// The position of the token_start offset from the input
    #[new(default)]
    start_pos: u32,

    /// The current size of the token
    #[new(default)]
    token_len: u32,

    #[new(value = "Delegate::top()")]
    state: Delegate,

    #[new(value = "vec![]")]
    stack: Vec<Delegate>,

    #[new(default)]
    token: PhantomData<Delegate::Token>,
}

pub type TokenizerItem<Token> = Result<(ByteIndex, Token, ByteIndex), ParseError>;

impl<Delegate: LexerDelegateTrait + Debug> Iterator for Tokenizer<'table, Delegate> {
    type Item = TokenizerItem<Delegate::Token>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut count = 0;
        loop {
            count += 1;

            if count > 1000 {
                return None;
            }

            // Get an action from the delegate
            let next = {
                let Tokenizer { state, rest, .. } = self;
                let next = rest.chars().next();

                trace!("next");
                trace!("          char={:?}", next);
                trace!("          rest={:?}", rest);

                state.next(next, rest)
            };

            self.trace("start");

            let next = match next {
                // If the delegate returned an error, it's an error
                Err(e) => return Some(Err(e)),

                // Otherwise, process the action
                Ok(n) => n,
            };

            match self.step(next) {
                LoopCompletion::Return(v) => return self.emit(v),
                LoopCompletion::Continue => {}
            }
        }
    }
}

enum LoopCompletion<T> {
    Continue,
    Return(T),
}

impl<Delegate: LexerDelegateTrait + Debug> Tokenizer<'table, Delegate> {
    fn intern(&mut self, source: &str) -> StringId {
        self.table.intern(&source)
    }

    pub fn tokens(self) -> Result<Vec<Spanned<Delegate::Token>>, ParseError> {
        self.map(|result| result.map(|(start, tok, end)| Spanned::from(tok, start, end)))
            .collect()
    }

    fn error(&mut self, c: Option<char>) -> ParseError {
        let file_start = self.codespan_start;
        let state = self.state.clone();
        // let token = &self.token_start[..self.token_size() as usize];
        // let (start_pos, end_pos) = self.consume_token(1);

        let error = ParseError::new(
            format!(
                "Unexpected char `{}` in state {:?}",
                c.map(|item| item.to_string())
                    .unwrap_or_else(|| "EOF".to_string()),
                state
            ),
            Span::from_pos(
                file_start + self.start_pos,
                file_start + self.start_pos + self.token_len,
            ),
        );

        debug!("lark::tokenize::error {:?}", error);

        error
    }

    fn trace(&self, prefix: &str) {
        let start = self.codespan_start;

        trace!(target: "lark::tokenize", "{}", prefix);

        trace!(target: "lark::tokenize", "          pos={:?}", start + self.start_pos + self.token_len);

        let token_start = (self.start_pos) as usize;
        let token_end = token_start + self.token_len as usize;

        trace!(
            target: "lark::tokenize",
            "          token-start={:?}",
            &self.input[token_start..token_end]
        );

        trace!(target: "lark::tokenize", "          token-size={:?} state={:?}", self.token_len, self.state)
    }

    /// Take a LexerNext and process it
    fn step(
        &mut self,
        next: LexerNext<Delegate>,
    ) -> LoopCompletion<Option<TokenizerItem<Delegate::Token>>> {
        debug!("next: {:?}", next);
        match next {
            // If the delegate returned EOF, we're done
            LexerNext::EOF => {
                self.trace("EOF");
                return LoopCompletion::Return(None);
            }

            LexerNext::Remain(accum) => self.accumulate(accum),

            LexerNext::Transition(accum, state) => {
                let ret = self.accumulate(accum);
                self.transition(state);

                self.trace("transition");

                ret
            }

            LexerNext::PushState(action, state) => {
                let ret = self.accumulate(action);
                self.stack.push(state.clone());
                self.transition(state);

                ret
            }

            LexerNext::PopState(action) => {
                let ret = self.accumulate(action);
                let state = self.stack.pop().expect("Unexpected empty state stack");
                self.transition(state);

                ret
            }

            LexerNext::Error(c) => LoopCompletion::Return(Some(Err(self.error(c)))),
        }
    }

    fn accumulate(
        &mut self,
        accum: LexerAccumulate<Delegate>,
    ) -> LoopCompletion<Option<TokenizerItem<Delegate::Token>>> {
        use self::LexerAccumulate::*;

        match accum {
            Begin => {
                assert!(
                    self.token_len == 0,
                    "Cannot begin a new token when there are already accumulated characters"
                );

                LoopCompletion::Continue
            }
            Nothing => LoopCompletion::Continue,
            Continue(action) => {
                self.action(action);
                LoopCompletion::Continue
            }
            Skip(action) => {
                self.action(action);

                // TODO: Extract into "finalization"?
                self.start_pos = self.start_pos + self.token_len;
                self.token_len = 0;
                LoopCompletion::Continue
            }

            Emit {
                before,
                after,
                token,
            } => {
                if let Some(before) = before {
                    self.action(before);
                }

                let start = self.start_pos as usize;
                let len = self.token_len as usize;

                let source = &self.input[start..start + len];
                self.start_pos = start as u32 + len as u32;
                self.token_len = 0;

                let id = self.intern(source);
                let token = token.string(id);

                LoopCompletion::Return(Some(Ok((
                    ByteIndex(self.codespan_start + start as u32),
                    token,
                    ByteIndex(self.codespan_start + start as u32 + len as u32),
                ))))
            }
        }
    }

    fn action(&mut self, action: LexerAction) {
        match action {
            LexerAction::Consume(n) => {
                self.token_len += n;
                self.rest = &self.rest[(n as usize)..];
            }
            LexerAction::Reconsume => {}
        }
    }

    fn transition(&mut self, state: Delegate) {
        debug!("transition {:?} -> {:?}", self.state, state);
        self.state = state;
    }

    fn emit(
        &mut self,
        token: Option<TokenizerItem<Delegate::Token>>,
    ) -> Option<TokenizerItem<Delegate::Token>> {
        match &token {
            None => debug!("-> EOF"),
            Some(Err(e)) => debug!("parse error {:?}", e),
            Some(Ok(tok)) => debug!("emit {:?}", Debuggable::from(&tok.1, self.table)),
        };

        token
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
