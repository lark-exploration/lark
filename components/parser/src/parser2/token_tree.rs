use crate::parser::program::StringId;

use derive_new::new;

#[derive(Debug, Copy, Clone)]
pub struct TokenSpan(usize, usize);

#[derive(Debug, Copy, Clone)]
pub struct TokenPos(usize);

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
}

#[derive(Debug, Copy, Clone)]
pub struct Handle(usize);

impl TokenTree {
    pub fn start(&mut self) {
        self.stack.push(TokenSpan(self.current, self.current));
        self.tick();
    }

    pub fn mark_expr(&mut self) {
        self.kind = Some(TokenKind::Expr);
    }

    pub fn mark_type(&mut self) {
        self.kind = Some(TokenKind::Type);
    }

    pub fn mark_macro(&mut self) {
        self.kind = Some(TokenKind::Macro(TokenPos(self.current)));
        self.current += 1;
    }

    pub fn tick(&mut self) {
        self.current += 1;
    }

    pub fn end(&mut self) -> Handle {
        let mut current = self
            .stack
            .pop()
            .expect("Can't end an event if none is started");

        current.1 = self.current;
        self.tick();

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
        self.current += 1;
    }
}
