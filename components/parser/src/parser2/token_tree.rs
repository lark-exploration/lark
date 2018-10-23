use crate::parser::program::StringId;

use derive_new::new;
use log::trace;

#[derive(Debug, Copy, Clone)]
pub struct TokenSpan(pub TokenPos, pub TokenPos);

#[derive(Debug, Copy, Clone)]
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

    #[new(value = "None")]
    backtrack_point: Option<usize>,

    token_len: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct Handle(usize);

impl TokenTree {
    pub fn start(&mut self, debug_name: &str) {
        trace!(target: "lark::reader", "starting {}", debug_name);

        self.stack
            .push(TokenSpan(TokenPos(self.current), TokenPos(self.current)));
    }

    pub fn pos(&self) -> usize {
        self.current
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
        trace!(target: "lark::reader", "ending {}", debug_name);

        let mut current = self
            .stack
            .pop()
            .expect("Can't end an event if none is started");

        current.1 = TokenPos(self.current);

        let node = match self
            .kind
            .expect("Can only end an event once its token kind is marked")
        {
            TokenKind::Expr => TokenNode::Expr(current),
            TokenKind::Type => TokenNode::Type(current),
            TokenKind::Macro(pos) => TokenNode::Macro(pos, current),
        };

        let handle = Handle(self.nodes.len());
        self.nodes.push(node);
        handle
    }

    pub fn single(&mut self) {
        self.nodes.push(TokenNode::Token(TokenPos(self.current)));
    }
}
