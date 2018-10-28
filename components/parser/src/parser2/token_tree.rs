use derive_new::new;
use log::trace;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TokenSpan(pub TokenPos, pub TokenPos);

impl fmt::Debug for TokenSpan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", (self.0).0, (self.1).0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TokenPos(pub usize);

#[derive(Debug, Copy, Clone)]
pub enum TokenNode {
    Token(TokenPos),
    Type(TokenSpan),
    Expr(TokenSpan),
    Macro(TokenPos, TokenSpan),
}

#[derive(Debug, Copy, Clone)]
pub enum TokenKind {
    Expr,
    Type,
    Macro(TokenPos),
}

#[derive(Debug, new)]
pub struct TokenTree {
    #[new(value = "vec![]")]
    nodes: Vec<TokenNode>,

    #[new(value = "vec![]")]
    stack: Vec<TokenSpan>,

    #[new(value = "None")]
    kind: Option<TokenKind>,

    #[new(value = "0")]
    current: usize,

    #[new(value = "0")]
    current_non_ws: usize,

    #[new(value = "0")]
    shape_start: usize,

    #[new(value = "None")]
    backtrack_point: Option<usize>,

    token_len: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct Handle(usize);

impl TokenTree {
    pub fn finalize(self) -> Vec<TokenNode> {
        self.nodes
    }

    pub fn start(&mut self, debug_name: &str) {
        self.start_at(self.current, debug_name)
    }

    pub fn start_at(&mut self, pos: usize, debug_name: &str) {
        trace!(target: "lark::reader", "starting {} at {}", debug_name, pos);

        self.stack.push(TokenSpan(TokenPos(pos), TokenPos(pos)));
    }

    pub fn pos(&self) -> usize {
        self.current
    }

    pub fn last_non_ws(&self) -> usize {
        self.current_non_ws
    }

    pub fn start_pos(&self) -> usize {
        let current = &self.stack[self.stack.len() - 1];
        (current.0).0
    }

    pub fn is_done(&self) -> bool {
        self.current >= self.token_len
    }

    pub fn mark_expr(&mut self) {
        self.kind = Some(TokenKind::Expr);
    }

    pub fn mark_type(&mut self) {
        self.kind = Some(TokenKind::Type);
    }

    pub fn mark_macro(&mut self) {
        self.kind = Some(TokenKind::Macro(TokenPos(self.current)));
    }

    pub fn tick(&mut self) {
        self.current += 1;
    }

    pub fn tick_non_ws(&mut self) {
        self.current_non_ws = self.current;
    }

    pub fn fast_forward(&mut self, to: usize, non_ws: usize) {
        self.current = to;
        self.current_non_ws = non_ws;
    }

    pub fn mark_backtrack_point(&mut self, debug_reason: &str) {
        // TODO: Perhaps RAII instead of assertion? Depends on whether backtracking is always pretty
        // static, which is so far true.

        trace!(target: "lark::reader", "Marking backtrack point; reason={:?}", debug_reason);
        assert!(
            self.backtrack_point == None,
            "Cannot set a backtrack point while another is active"
        );

        self.backtrack_point = Some(self.current);
    }

    pub fn backtrack(&mut self, debug_reason: &str) {
        trace!(
            target: "lark::reader",
            "Backtracking to {:?}; reason={:?}",
            self.backtrack_point,
            debug_reason
        );

        let to = self
            .backtrack_point
            .expect("Can only backtrack while a backtrack point is active");
        self.current = to;
        self.backtrack_point = None;
    }

    pub fn commit(&mut self, debug_reason: &str) {
        trace!(target: "lark::reader", "Committing backtrack point; reason={:?}", debug_reason);
        self.backtrack_point = None;
    }

    pub fn end(&mut self, debug_name: &str) -> Handle {
        self.end_at(self.current, debug_name)
    }

    pub fn end_at(&mut self, pos: usize, debug_name: &str) -> Handle {
        trace!(target: "lark::reader", "ending {} as {}", debug_name, pos);

        let mut current = self
            .stack
            .pop()
            .expect("Can't end an event if none is started");

        current.1 = TokenPos(pos);

        let node = match self
            .kind
            .expect("Can only end an event once its token kind is marked")
        {
            TokenKind::Expr => TokenNode::Expr(current),
            TokenKind::Type => TokenNode::Type(current),
            TokenKind::Macro(macro_pos) => TokenNode::Macro(macro_pos, current),
        };

        let handle = Handle(self.nodes.len());
        self.nodes.push(node);
        handle
    }

    pub fn single(&mut self) {
        self.nodes.push(TokenNode::Token(TokenPos(self.current)));
    }
}
