use codespan::ByteIndex;
use crate::parser::pos::HasSpan;
use crate::parser::pos::Span;
use crate::parser::pos::Spanned;
use crate::parser::{Environment, Program, StringId, Token};
use derive_new::new;
use std::fmt;

pub type Identifier = Spanned<StringId>;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ast<'input> {
    input: &'input str,
    module: Module,
}

impl Ast<'input> {
    crate fn empty() -> Ast<'input> {
        Ast {
            input: "",
            module: Module { items: vec![] },
        }
    }

    crate fn module(self) -> Module {
        self.module
    }
}

pub struct Debuggable<'owner, T: DebugProgram + 'owner> {
    inner: &'owner T,
    program: &'owner Program,
}

impl<T: DebugProgram + 'owner> Debuggable<'owner, T> {
    pub fn from(inner: &'owner T, program: &'owner Program) -> Debuggable<'owner, T> {
        Debuggable { inner, program }
    }
}

impl<T: DebugProgram> fmt::Debug for Debuggable<'program, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.debug(f, &self.program)
    }
}

pub trait DebugProgram {
    fn debug(&self, f: &mut fmt::Formatter<'_>, program: &'program Program) -> fmt::Result;
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Item {
    Struct(Struct),
}

impl DebugProgram for Item {
    fn debug(&self, f: &mut fmt::Formatter<'_>, program: &'program Program) -> fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "{:?}", Debuggable::from(s, program)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Module {
    items: Vec<Item>,
}

impl DebugProgram for Module {
    fn debug(&self, f: &mut fmt::Formatter<'_>, program: &'program Program) -> fmt::Result {
        let entries: Vec<_> = self
            .items
            .iter()
            .map(|i| Debuggable::from(i, program))
            .collect();

        write!(f, "{:?}", entries)
    }
}

#[cfg(test)]
impl Module {
    crate fn build() -> Module {
        Module { items: vec![] }
    }

    crate fn add_struct(mut self, s: Struct) -> Module {
        self.items.push(Item::Struct(s));
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Struct {
    name: Spanned<StringId>,
    fields: Vec<Field>,
    span: Span,
}

struct DebugStruct<'a> {
    name: &'a str,
    fields: Vec<DebugField<'a>>,
}

impl DebugProgram for Struct {
    fn debug(&self, f: &mut fmt::Formatter<'_>, program: &'program Program) -> fmt::Result {
        f.debug_struct("Struct")
            .field("name", &program.lookup(self.name.node))
            .field("fields", &format_args!("{:?}", self.fields))
            .finish()
    }
}

impl HasSpan for Struct {
    type Inner = Struct;

    fn span(&self) -> Span {
        self.span
    }
}

#[cfg(test)]
impl Struct {
    crate fn build(name: &'input str, program: &mut Program) -> Struct {
        Struct {
            name: Spanned::synthetic(program.intern(name)),
            fields: vec![],
            span: Span::Synthetic,
        }
    }

    crate fn spanned(mut self, start: u32, end: u32) -> Struct {
        self.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn name_spanned(mut self, start: u32, end: u32) -> Struct {
        self.name.span = Span::from(ByteIndex(start), ByteIndex(end));
        self
    }

    crate fn field(mut self, field: Field) -> Struct {
        self.fields.push(field);
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Field {
    name: Spanned<StringId>,
    ty: Spanned<Type>,
}

struct DebugField<'a> {
    name: &'a str,
    ty: DebugType<'a>,
}

#[cfg(test)]
impl Field {
    crate fn build(name: &'input str, ty: Type, program: &mut Program) -> Field {
        Field {
            name: Spanned::synthetic(program.intern(name)),
            ty: Spanned::synthetic(ty),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Type {
    mode: Option<Spanned<Mode>>,
    name: Spanned<StringId>,
}

struct DebugType<'a> {
    name: &'a str,
}

#[cfg(test)]
impl Type {
    crate fn build(name: &'input str, mode: Mode, program: &mut Program) -> Type {
        Type {
            mode: Some(Spanned::synthetic(mode)),
            name: Spanned::synthetic(program.intern(name)),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    Owned,
    Shared,
    Borrowed,
}

pub struct Pattern {}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Path {
    components: Vec<Spanned<StringId>>,
}

pub enum Statement {}

pub struct Def {}

pub enum Expression {
    Block(Block),
}

pub struct Let {
    pattern: Pattern,
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

pub struct Block {
    expressions: Vec<Expression>,
}
