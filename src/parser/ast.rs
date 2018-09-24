mod debug;

#[cfg(test)]
mod test_helpers;

use codespan::ByteIndex;
use crate::parser::pos::HasSpan;
use crate::parser::pos::Span;
use crate::parser::pos::Spanned;
use crate::parser::{Environment, ModuleTable, StringId, Token};
use derive_new::new;
use std::fmt;

crate use self::debug::{DebugModuleTable, Debuggable, DebuggableVec};

pub type Identifier = Spanned<StringId>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
    Def(Def),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockItem {
    Item(Item),
    Decl(Declaration),
    Expr(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Declaration {
    Let,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Module {
    crate items: Vec<Item>,
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
    crate body: Block,
    crate span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Expression {
    Block(Block),
    ConstructStruct(ConstructStruct),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct ConstructStruct {
    name: Identifier,
    fields: Vec<ConstructField>,
    span: Span,
}

pub struct Let {
    pattern: Spanned<Pattern>,
    ty: Option<Type>,
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
