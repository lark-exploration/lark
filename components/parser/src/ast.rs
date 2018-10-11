mod debug;

#[cfg(test)]
mod test_helpers;

use codespan::ByteIndex;
use crate::pos::HasSpan;
use crate::pos::Span;
use crate::pos::Spanned;
use crate::{Environment, ModuleTable, StringId, Token};
use derive_new::new;
use std::fmt;
use std::sync::Arc;

pub use self::debug::{DebugModuleTable, Debuggable, DebuggableVec};

pub type Identifier = Spanned<StringId>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
    Def(Def),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockItem {
    Item(Arc<Item>),
    Decl(Declaration),
    Expr(Expression),
}

impl BlockItem {
    pub fn let_decl(
        pattern: Spanned<Pattern>,
        ty: Option<Spanned<Type>>,
        init: Option<Expression>,
    ) -> BlockItem {
        BlockItem::Decl(Declaration::Let(Let::new(pattern, ty, init)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Declaration {
    Let(Let),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Module {
    crate items: Vec<Arc<Item>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    name: Spanned<StringId>,
    fields: Vec<Field>,
    span: Span,
}

impl HasSpan for Struct {
    type Inner = Struct;

    fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Field {
    name: Identifier,
    ty: Spanned<Type>,
    span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConstructField {
    Longhand(Field),
    Shorthand(Identifier),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    mode: Option<Spanned<Mode>>,
    name: Spanned<StringId>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    Owned,
    Shared,
    Borrowed,
}

impl From<&str> for Mode {
    fn from(input: &str) -> Mode {
        match input {
            "own" => Mode::Owned,
            "share" => Mode::Shared,
            "borrow" => Mode::Borrowed,
            other => panic!("Invalid mode string {}", other),
        }
    }
}

impl From<Mode> for &'static str {
    fn from(input: Mode) -> &'static str {
        match input {
            Mode::Owned => "own",
            Mode::Shared => "share",
            Mode::Borrowed => "borrow",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub enum Pattern {
    Underscore,
    Identifier(Identifier, Option<Spanned<Mode>>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Path {
    components: Vec<Identifier>,
}

pub enum Statement {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Def {
    crate name: Identifier,
    crate parameters: Vec<Field>,
    crate ret: Option<Spanned<Type>>,
    crate body: Spanned<Block>,
    crate span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Expression {
    Block(Spanned<Block>),
    ConstructStruct(ConstructStruct),
    Call(Spanned<Call>),
    Ref(Identifier),
    Binary(Spanned<Op>, Box<Expression>, Box<Expression>),
    Interpolation(Vec<InterpolationElement>, Span),
    Literal(Literal),
}

impl Expression {
    pub fn binary(op: Spanned<Op>, left: Expression, right: Expression) -> Expression {
        Expression::Binary(op, box left, box right)
    }

    pub fn call(callee: impl Into<Callee>, args: Vec<Expression>, span: Span) -> Expression {
        let callee = callee.into();
        let call = Call::new(callee, args);
        Expression::Call(Spanned::wrap_span(call, span))
    }

    pub fn string(node: Spanned<StringId>) -> Expression {
        Expression::Literal(Literal::String(node))
    }
}

impl HasSpan for Expression {
    type Inner = Expression;

    fn span(&self) -> Span {
        use self::Expression::*;

        match self {
            Block(block) => block.span(),
            ConstructStruct(construct) => construct.span(),
            Call(call) => call.span(),
            Ref(id) => id.span(),
            Binary(_, left, right) => left.span().to(right.span()),
            Interpolation(_, span) => *span,
            Literal(lit) => lit.span(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Op::Add => "+",
                Op::Sub => "-",
                Op::Mul => "*",
                Op::Div => "/",
            }
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InterpolationElement {
    String(Spanned<StringId>),
    Expression(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Literal {
    String(Spanned<StringId>),
}

impl HasSpan for Literal {
    type Inner = Literal;

    fn span(&self) -> Span {
        match self {
            Literal::String(string) => string.span(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ConstructStruct {
    name: Identifier,
    fields: Vec<ConstructField>,
    span: Span,
}

impl HasSpan for ConstructStruct {
    type Inner = ConstructStruct;

    fn span(&self) -> Span {
        self.span
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]

pub struct Call {
    callee: Callee,
    arguments: Vec<Expression>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub enum Callee {
    Identifier(Identifier),
}

impl From<Identifier> for Callee {
    fn from(id: Identifier) -> Callee {
        Callee::Identifier(id)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Let {
    pattern: Spanned<Pattern>,
    ty: Option<Spanned<Type>>,
    init: Option<Expression>,
}

pub enum If {
    If(Box<Expression>, Block, Option<ChainedElse>),
    IfLet(Pattern, Box<Expression>, Block, Option<ChainedElse>),
}

pub enum ChainedElse {
    Block(Block),
    If(Box<If>),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Block {
    expressions: Vec<BlockItem>,
}

impl Block {
    pub fn spanned(expressions: Vec<BlockItem>, span: Span) -> Spanned<Block> {
        let block = Block::new(expressions);
        Spanned::wrap_span(block, span)
    }
}
