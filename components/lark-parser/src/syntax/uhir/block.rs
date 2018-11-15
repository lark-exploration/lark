use crate::uhir::{Field, Identifier, Pattern, Span, Spanned, Type};

use derive_new::new;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Block {
    pub expressions: Vec<BlockItem>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockItem {
    Decl(Declaration),
    Expr(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Declaration {
    Let(Let),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Expression {
    Block(Spanned<Block>),
    ConstructStruct(ConstructStruct),
    Call(Spanned<Call>),
    Ref(Identifier),
    Binary(Spanned<Op>, Box<Expression>, Box<Expression>),
    Literal(Literal),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Literal {
    String(Identifier),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ConstructStruct {
    name: Identifier,
    fields: Vec<ConstructField>,
    span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Call {
    pub callee: Callee,
    pub arguments: Vec<Expression>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub enum Callee {
    Identifier(Identifier),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Let {
    pub pattern: Spanned<Pattern>,
    pub ty: Option<Spanned<Type>>,
    pub init: Option<Expression>,
}

pub enum If {
    If(Box<Expression>, Block, Option<ChainedElse>),
    IfLet(Pattern, Box<Expression>, Block, Option<ChainedElse>),
}

pub enum ChainedElse {
    Block(Block),
    If(Box<If>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConstructField {
    Longhand(Field),
    Shorthand(Identifier),
}
