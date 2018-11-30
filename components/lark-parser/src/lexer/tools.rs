use debug::DebugWith;
use derive_new::new;
use lark_span::{CurrentFile, Span, Spanned};
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
            token,
        })
    }

    /// Move into the state `state`, and once it recognizes something,
    /// emit the token `token` -- this is for multicharacter tokens.
    pub fn emit(token: Delegate::Token, state: Delegate) -> Self {
        LexerNext::Transition(
            LexerAccumulate::Emit {
                before: None,
                after: None,
                token,
            },
            state,
        )
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
        token: Delegate::Token,
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
    pub fn emit(token: Delegate::Token) -> LexerAccumulate<Delegate> {
        LexerAccumulate::Emit {
            before: None,
            after: None,
            token: token,
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

#[derive(Debug, Copy, Clone)]
pub enum LexerAction {
    Consume(usize),
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
            token: token,
        }
    }
}

pub fn reconsume() -> LexerAction {
    LexerAction::Reconsume
}

pub fn consume(c: char) -> LexerAction {
    LexerAction::Consume(c.len_utf8())
}

pub fn consume_str(s: &str) -> LexerAction {
    LexerAction::Consume(s.len())
}

pub fn begin<Delegate: LexerDelegateTrait>() -> LexerAccumulate<Delegate> {
    LexerAccumulate::Begin
}

pub fn eof<Delegate: LexerDelegateTrait>() -> LexerNext<Delegate> {
    LexerNext::EOF
}

pub trait LexerDelegateTrait: fmt::Debug + Clone + Copy + Sized {
    type Token: fmt::Debug + DebugWith + Copy;

    fn next(&self, c: Option<char>, rest: &'input str) -> LexerNext<Self>;

    fn top() -> Self;
}

#[derive(Debug, new)]
pub struct Tokenizer<'table, Delegate: LexerDelegateTrait> {
    input: &'table str,

    /// The rest of the input
    #[new(value = "input")]
    rest: &'table str,

    /// The beginning of the current token
    #[new(value = "input")]
    token_start: &'table str,

    /// The position of the token_start offset from the input
    #[new(default)]
    start_pos: usize,

    /// The current size of the token
    #[new(default)]
    token_len: usize,

    #[new(value = "Delegate::top()")]
    state: Delegate,

    #[new(value = "vec![]")]
    stack: Vec<Delegate>,

    #[new(default)]
    token: PhantomData<Delegate::Token>,
}

pub type TokenizerItem<Token> = Result<Spanned<Token, CurrentFile>, Span<CurrentFile>>;

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

                log::trace!("next");
                log::trace!("          char={:?}", next);
                log::trace!("          rest={:?}", rest);

                state.next(next, rest)
            };

            self.trace("start");

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
    pub fn tokens(self) -> Result<Vec<Spanned<Delegate::Token, CurrentFile>>, Span<CurrentFile>> {
        self.collect()
    }

    fn error(&mut self, _: Option<char>) -> Span<CurrentFile> {
        // let token = &self.token_start[..self.token_size() as usize];
        // let (start_pos, end_pos) = self.consume_token(1);

        let span = Span::new(CurrentFile, self.start_pos, self.start_pos + self.token_len);

        log::debug!("lark::tokenize::error {:?}", span);

        span
    }

    fn trace(&self, prefix: &str) {
        log::trace!(target: "lark::tokenize", "{}", prefix);

        log::trace!(target: "lark::tokenize", "          pos={:?}", self.start_pos + self.token_len);

        let token_start = (self.start_pos) as usize;
        let token_end = token_start + self.token_len as usize;

        log::trace!(
            target: "lark::tokenize",
            "          token-start={:?}",
            &self.input[token_start..token_end]
        );

        log::trace!(target: "lark::tokenize", "          token-size={:?} state={:?}", self.token_len, self.state)
    }

    /// Take a LexerNext and process it
    fn step(
        &mut self,
        next: LexerNext<Delegate>,
    ) -> LoopCompletion<Option<TokenizerItem<Delegate::Token>>> {
        log::debug!("next: {:?}", next);
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

            Emit { before, token, .. } => {
                if let Some(before) = before {
                    self.action(before);
                }

                let start = self.start_pos;
                let len = self.token_len;
                self.start_pos = start + len;
                self.token_len = 0;

                LoopCompletion::Return(Some(Ok(Spanned::new(
                    token,
                    Span::new(CurrentFile, start, start + len),
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
        log::debug!("transition {:?} -> {:?}", self.state, state);
        self.state = state;
    }

    fn emit(
        &mut self,
        token: Option<TokenizerItem<Delegate::Token>>,
    ) -> Option<TokenizerItem<Delegate::Token>> {
        match &token {
            None => log::debug!("-> EOF"),
            Some(Err(e)) => log::debug!("parse error {:?}", e.debug_with(&self.state)),
            Some(Ok(tok)) => log::debug!("emit {:?}", tok.debug_with(&self.state)),
        };

        token
    }
}
